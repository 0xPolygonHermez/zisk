use chrono::{DateTime, Utc};
use consensus_common::{
    AggProofData, BlockContext, BlockId, ComputeCapacity, Error, Job, JobId, JobPhase, JobResult,
    JobResultData, JobState, ProverId, ProverState, Result,
};
use consensus_grpc_api::{
    coordinator_message, execute_task_request, prover_message, Challenges, CoordinatorMessage,
    ExecuteTaskResponse, Proof, ProofList, ProverMessage, TaskType,
};
use proofman::ContributionsInfo;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// Information about a connected prover - business logic only, no transport layer
#[derive(Debug)]
pub struct ProverConnection {
    pub prover_id: ProverId,
    pub state: ProverState,
    pub compute_capacity: ComputeCapacity,
    pub num_nodes: u32,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub message_sender: mpsc::Sender<CoordinatorMessage>,
}

/// Configuration for the coordinator functionality
#[derive(Debug, Clone)]
pub struct ProverManagerConfig {
    pub max_provers_per_job: u32,
    pub max_total_provers: u32,
    pub max_concurrent_connections: u32,
    pub message_buffer_size: u32,
    pub phase1_timeout_seconds: u64,
    pub phase2_timeout_seconds: u64,
}

impl ProverManagerConfig {
    /// Create from consensus_config::CoordinatorConfig
    pub fn from_config(config: &consensus_config::ProverManagerConfig) -> Self {
        Self {
            max_provers_per_job: config.max_provers_per_job,
            max_total_provers: config.max_total_provers,
            max_concurrent_connections: config.max_concurrent_connections,
            message_buffer_size: config.message_buffer_size,
            phase1_timeout_seconds: config.phase1_timeout_seconds,
            phase2_timeout_seconds: config.phase2_timeout_seconds,
        }
    }
}

impl Default for ProverManagerConfig {
    fn default() -> Self {
        Self {
            max_provers_per_job: 10,
            max_total_provers: 1000, // Default limit of 1000 total provers
            max_concurrent_connections: 500, // Default limit of 500 concurrent connections
            message_buffer_size: 1000, // Default bounded channel size
            phase1_timeout_seconds: 300, // 5 minutes
            phase2_timeout_seconds: 600, // 10 minutes
        }
    }
}

/// Prover manager for handling multiple provers
#[derive(Debug)]
pub struct ProverManager {
    provers: Arc<RwLock<HashMap<ProverId, ProverConnection>>>,
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
    config: ProverManagerConfig,
}

impl ProverManager {
    pub fn new(config: ProverManagerConfig) -> Self {
        Self {
            provers: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn num_provers(&self) -> usize {
        self.provers.read().await.len()
    }

    pub async fn compute_capacity(&self) -> ComputeCapacity {
        let provers = self.provers.read().await;
        let total_capacity: u32 = provers.values().map(|p| p.compute_capacity.compute_units).sum();
        ComputeCapacity { compute_units: total_capacity }
    }

    pub async fn get_available_compute_capacity(&self) -> ComputeCapacity {
        let total_capacity: u32 = self
            .provers
            .read()
            .await
            .values()
            .filter(|p| matches!(p.state, ProverState::Idle))
            .map(|p| p.compute_capacity.compute_units)
            .sum();
        ComputeCapacity { compute_units: total_capacity }
    }

    /// Get the coordinator configuration
    pub fn config(&self) -> &ProverManagerConfig {
        &self.config
    }

    /// Register a new prover connection - business logic only
    pub async fn register_prover(
        &self,
        prover_id: ProverId,
        compute_capacity: impl Into<ComputeCapacity>,
        num_nodes: u32,
        message_sender: mpsc::Sender<CoordinatorMessage>,
    ) -> Result<ProverId> {
        // Check if we've reached the maximum number of total provers
        let num_provers = self.num_provers().await;
        if num_provers >= self.config.max_total_provers as usize {
            return Err(Error::InvalidRequest(format!(
                "Maximum number of provers reached: {}/{}",
                num_provers, self.config.max_total_provers
            )));
        }

        let now = Utc::now();
        let connection = ProverConnection {
            prover_id: prover_id.clone(),
            compute_capacity: compute_capacity.into(),
            num_nodes,
            state: ProverState::Idle,
            connected_at: now,
            last_heartbeat: now,
            message_sender,
        };

        self.provers.write().await.insert(prover_id.clone(), connection);

        info!("Registered prover: {} (total: {})", prover_id, self.num_provers().await);

        Ok(prover_id)
    }

    async fn mark_provers_with_state(
        &self,
        prover_ids: &[ProverId],
        state: ProverState,
    ) -> Result<()> {
        let mut provers = self.provers.write().await;
        for prover_id in prover_ids {
            if let Some(prover) = provers.get_mut(prover_id) {
                prover.state = state.clone();
            } else {
                return Err(Error::InvalidRequest(format!("Prover {prover_id} not found")));
            }
        }

        Ok(())
    }

    /// Remove a prover connection
    pub async fn disconnect_prover(&self, prover_id: &ProverId) -> Result<()> {
        let mut provers = self.provers.write().await;
        match provers.remove(prover_id) {
            Some(prover) => {
                info!(
                    "Removed prover: {} (remaining: {}, was in state: {:?})",
                    prover_id,
                    provers.len(),
                    prover.state
                );
                // TODO: Handle job reassignment if prover was working ?
            }
            None => {
                warn!("Attempted to remove prover {prover_id} but it was not found");
            }
        }
        Ok(())
    }

    /// Start a proof job with the specified request
    pub async fn start_proof(
        &self,
        block_id: String,
        required_compute_capacity: ComputeCapacity,
        input_path: String,
    ) -> Result<JobId> {
        let available_compute_capacity = self.get_available_compute_capacity().await;

        if required_compute_capacity.compute_units == 0 {
            return Err(Error::InvalidRequest(
                "Compute capacity must be greater than 0".to_string(),
            ));
        }

        if required_compute_capacity > available_compute_capacity {
            return Err(Error::InvalidRequest(format!(
                "Not enough compute capacity available: need {required_compute_capacity}, have {available_compute_capacity}",
            )));
        }

        info!(
            "Starting proof for block {} using {} with input path: {}",
            block_id, required_compute_capacity, input_path
        );

        // Select only the required number of provers (not all of them)
        let (selected_provers, partitions) =
            self.partition_and_allocate_by_capacity(required_compute_capacity).await;

        // Create job
        let job_id = JobId::new();
        let block_id = BlockId::from(block_id);
        let job = Job {
            job_id: job_id.clone(),
            state: JobState::Running(JobPhase::Contributions),
            block: BlockContext {
                block_id: block_id.clone(),
                input_path: PathBuf::from(input_path.clone()),
            },
            compute_units: required_compute_capacity.compute_units,
            provers: selected_provers.clone(),
            partitions: partitions.clone(),
            results: HashMap::new(),
            challenges: None,
        };

        self.jobs.write().await.insert(job_id.clone(), job);

        // Send messages to selected provers
        let mut provers = self.provers.write().await;
        let provers_len = selected_provers.len() as u32;
        let mut table_id_acc = 0;

        let total_tables = selected_provers
            .iter()
            .map(|prover_id| provers.get(prover_id).map_or(0, |p| p.num_nodes))
            .sum::<u32>();

        for (rank_id, prover_id) in selected_provers.iter().enumerate() {
            let prover = provers.get_mut(prover_id).unwrap();

            // Convert range to ProverAllocation vector
            let range = &partitions[rank_id];
            let prover_allocation = range.clone();

            let table_id_start = table_id_acc;
            table_id_acc += prover.num_nodes;

            let message = CoordinatorMessage {
                payload: Some(coordinator_message::Payload::ExecuteTask(
                    consensus_grpc_api::ExecuteTaskRequest {
                        prover_id: prover_id.clone().into(),
                        job_id: job_id.clone().into(),
                        task_type: consensus_grpc_api::TaskType::PartialContribution as i32,
                        params: Some(execute_task_request::Params::ContributionParams(
                            consensus_grpc_api::ContributionParams {
                                block_id: block_id.clone().into(),
                                input_path: input_path.clone(),
                                rank_id: rank_id as u32,
                                total_provers: provers_len,
                                prover_allocation,
                                job_compute_units: required_compute_capacity.compute_units,
                                total_tables,
                                table_id_start,
                            },
                        )),
                    },
                )),
            };

            // Send message through the prover's channel (bounded channels require async send)
            match prover.message_sender.try_send(message) {
                Ok(()) => {
                    prover.state = ProverState::Computing(JobPhase::Contributions);
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // TODO: Handle backpressure - maybe queue for retry or mark prover as slow
                    // TODO: Make unbounded channel
                    return Err(Error::Internal(format!(
                        "Message buffer full for prover {prover_id}, dropping message"
                    )));
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // TODO: Handle closed channel - maybe reassign job
                    return Err(Error::Internal(format!(
                        "Channel closed for prover {prover_id}, dropping message"
                    )));
                }
            }
        }

        info!(
            "Assigned new job {} to {} provers with input path: {}",
            job_id, provers_len, input_path
        );

        Ok(job_id)
    }

    /// Handle incoming message from a prover
    pub async fn handle_prover_message(
        &self,
        prover_id: &ProverId,
        message: ProverMessage,
    ) -> Result<()> {
        // Update last heartbeat
        if let Some(prover) = self.provers.write().await.get_mut(prover_id) {
            prover.last_heartbeat = Utc::now();
        }

        // Handle specific message types
        if let Some(payload) = message.payload {
            match payload {
                prover_message::Payload::ExecuteTaskResponse(execute_task_response) => {
                    let job_id = JobId::from(execute_task_response.job_id.clone());

                    if !execute_task_response.success {
                        self.fail_job(&job_id, "Final proof generation failed".to_string())
                            .await
                            .map_err(|e| {
                            error!("Failed to mark job {} as failed: {}", job_id, e);
                            e
                        })?;

                        return Err(Error::Service(format!(
                            "Prover {} failed to execute task for job {}: {}",
                            prover_id,
                            execute_task_response.job_id,
                            execute_task_response.error_message
                        )));
                    }

                    info!(
                        "Execute task result success from prover {} (job_id: {})",
                        prover_id, job_id
                    );

                    match TaskType::try_from(execute_task_response.task_type) {
                        Ok(TaskType::PartialContribution) => {
                            self.handle_phase1_result(execute_task_response).await.map_err(
                                |e| {
                                    error!("Failed to handle Phase1 result: {}", e);
                                    e
                                },
                            )?;
                        }
                        Ok(TaskType::Prove) => {
                            // Handle Phase2 completion - wait for all provers to complete
                            self.handle_phase2_result(&job_id, execute_task_response)
                                .await
                                .map_err(|e| {
                                    error!(
                                        "Failed to handle Phase2 result for job {}: {}",
                                        job_id, e
                                    );
                                    e
                                })?;
                        }
                        Ok(TaskType::Aggregate) => {
                            // Handle Phase2 completion - wait for all provers to complete
                            self.handle_agregate_result(&job_id, execute_task_response)
                                .await
                                .map_err(|e| {
                                    error!(
                                        "Failed to handle Aggregation result for job {}: {}",
                                        job_id, e
                                    );
                                    e
                                })?;
                        }
                        Err(_) => {
                            warn!("Received TaskResult with unknown task_type (raw: {}) for job {} from prover {}", execute_task_response.task_type, job_id, prover_id);
                        }
                    }
                    // Store the Phase1 result and check if we can proceed to Phase2
                }
                prover_message::Payload::Error(prover_error) => {
                    error!("Prover {} error: {}", prover_id, prover_error.error_message);

                    // If the error includes a job_id, we should fail that job
                    if !prover_error.job_id.is_empty() {
                        let job_id = JobId::from(prover_error.job_id.clone());

                        self.fail_job(&job_id, prover_error.error_message.clone()).await.map_err(
                            |e| {
                                error!(
                                    "Failed to mark job {} as failed after prover error: {}",
                                    job_id, e
                                );
                                e
                            },
                        )?;
                    }
                }
                prover_message::Payload::HeartbeatAck(_) => {
                    // Already updated heartbeat above
                }
                _ => {
                    // Other message types handled elsewhere
                }
            }
        }

        Ok(())
    }

    /// Handle Phase2 result and check if the job is complete
    async fn handle_phase2_result(
        &self,
        job_id: &JobId,
        execute_task_response: ExecuteTaskResponse,
    ) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let prover_id = ProverId::from(execute_task_response.prover_id);

        // Store Phase2 result
        let phase2_results = job.results.entry(JobPhase::Prove).or_default();

        // Check if we already have a result from this prover
        if phase2_results.contains_key(&prover_id) {
            warn!("Received duplicate Phase2 result from prover {} for job {}", prover_id, job_id);
            return Err(Error::InvalidRequest(format!(
                "Duplicate Phase2 result from prover {prover_id} for job {job_id}"
            )));
        }

        let data = match execute_task_response.result_data {
            Some(consensus_grpc_api::execute_task_response::ResultData::Proofs(proof_list)) => {
                let agg_proofs: Vec<AggProofData> = proof_list
                    .proofs
                    .into_iter()
                    .map(|proof| AggProofData {
                        airgroup_id: proof.airgroup_id,
                        values: proof.values,
                        worker_idx: proof.worker_idx,
                    })
                    .collect();
                JobResultData::AggProofs(agg_proofs)
            }
            _ => {
                return Err(Error::InvalidRequest(
                    "Expected Proofs result data for Phase2".to_string(),
                ));
            }
        };

        phase2_results
            .insert(prover_id.clone(), JobResult { success: execute_task_response.success, data });

        info!("Stored Phase2 result for prover {prover_id} in job {job_id}.");

        // Check if we have results from all assigned provers
        if phase2_results.len() < job.provers.len() {
            info!(
                "Phase2 progress for job {}: {}/{} provers completed",
                job_id,
                phase2_results.len(),
                job.provers.len()
            );
            return Ok(());
        }

        // Check if all Phase2 results are successful
        let all_successful = phase2_results.values().all(|result| result.success);

        if all_successful {
            job.state = JobState::Running(JobPhase::Aggregate);

            // Get the assigned provers and release the jobs lock
            let assigned_provers = job.provers.clone();
            let job_id = job.job_id.clone();
            drop(jobs); // Release jobs lock early

            // Start Phase2 for all provers
            if let Err(e) = self.start_phase3(&job_id, &assigned_provers).await {
                error!("Failed to start Phase2 for job {}: {}", job_id, e);
                self.fail_job(&job_id, format!("Failed to start Phase2: {e}")).await.map_err(
                    |e| {
                        error!("Failed to mark job {} as failed: {}", job_id, e);
                        e
                    },
                )?;
            }
        } else {
            // Some Phase2 results failed
            let failed_provers: Vec<ProverId> = phase2_results
                .iter()
                .filter_map(
                    |(prover_id, result)| {
                        if !result.success {
                            Some(prover_id.clone())
                        } else {
                            None
                        }
                    },
                )
                .collect();

            warn!("Phase2 failed for provers {:?} in job {}", failed_provers, job_id);
            let reason = format!("Phase2 failed for provers: {failed_provers:?}");

            // Release jobs lock before calling fail_job
            let job_id_clone = job.job_id.clone();
            drop(jobs);

            self.fail_job(&job_id_clone, reason).await?;
        }

        Ok(())
    }

    /// Handle Phase2 result and check if the job is complete
    async fn handle_agregate_result(
        &self,
        job_id: &JobId,
        execute_task_response: ExecuteTaskResponse,
    ) -> Result<()> {
        info!("Handling aggregation result for job {}", job_id);

        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        if execute_task_response.success {
            job.state = JobState::Completed;

            // Get the assigned provers before releasing the lock
            let assigned_provers = job.provers.clone();

            // Reset prover statuses back to Idle
            self.mark_provers_with_state(&assigned_provers, ProverState::Idle).await?;

            info!("Completed job {} and freed {} provers", job_id, assigned_provers.len());
        } else {
            // Some Phase2 results failed
            match execute_task_response.result_data {
                Some(consensus_grpc_api::execute_task_response::ResultData::FinalProof(_)) => {}
                _ => {
                    return Err(Error::InvalidRequest(
                        "Expected Proofs result data for Phase2".to_string(),
                    ));
                }
            }

            warn!("Aggregation failed in job {}", job_id);
            let reason = "Aggregation failed".to_string();

            self.fail_job(&job.job_id, reason).await?;
        }

        drop(jobs);

        Ok(())
    }

    /// Mark a job as failed and reset prover statuses
    pub async fn fail_job(&self, job_id: &JobId, reason: String) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        job.state = JobState::Failed;

        // Reset prover statuses back to Idle
        self.mark_provers_with_state(&job.provers, ProverState::Idle).await?;

        error!(
            "Failed job {} (reason: {}) and freed {} provers",
            job_id,
            reason,
            job.provers.len()
        );

        Ok(())
    }

    /// Handle Phase1 result and check if we can proceed to Phase2
    pub async fn handle_phase1_result(
        &self,
        execute_task_response: ExecuteTaskResponse,
    ) -> Result<()> {
        let job_id = JobId::from(execute_task_response.job_id.clone());

        let mut jobs = self.jobs.write().await;

        let job = jobs
            .get_mut(&job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let prover_id = ProverId::from(execute_task_response.prover_id);

        let phase1_results = job.results.entry(JobPhase::Contributions).or_default();

        let data = match execute_task_response.result_data {
            Some(consensus_grpc_api::execute_task_response::ResultData::Challenges(challenges)) => {
                assert!(!challenges.challenges.is_empty());

                let mut cont = Vec::new();
                for challenge in challenges.challenges {
                    cont.push(ContributionsInfo {
                        worker_index: challenge.worker_index,
                        airgroup_id: challenge.airgroup_id as usize,
                        challenge: challenge
                            .challenge
                            .try_into()
                            .expect("Challenge length mismatch"),
                    });
                }
                JobResultData::Challenges(cont)
            }
            _ => {
                return Err(Error::InvalidRequest(
                    "Expected Challenges result data for Phase1".to_string(),
                ));
            }
        };

        phase1_results
            .insert(prover_id.clone(), JobResult { success: execute_task_response.success, data });

        info!("Stored Phase1 result for prover {prover_id} in job {job_id}.");

        // Check if we have results from ALL assigned provers
        if phase1_results.len() < job.provers.len() {
            info!(
                "Phase1 progress for job {}: {}/{} provers completed",
                job_id,
                phase1_results.len(),
                job.provers.len()
            );
            return Ok(());
        }

        // Check if all results are successful
        let all_successful = phase1_results.values().all(|result| result.success);

        if all_successful {
            let challenges: Vec<Vec<ContributionsInfo>> = phase1_results
                .values()
                .map(|results| match &results.data {
                    JobResultData::Challenges(values) => values.clone(),
                    _ => unreachable!("Expected Challenges data in Phase1 results"),
                })
                .collect();

            let challenges: Vec<ContributionsInfo> = challenges.into_iter().flatten().collect();
            job.challenges = Some(challenges.clone());

            job.state = JobState::Running(JobPhase::Prove);

            // Get the assigned provers and release the jobs lock
            let assigned_provers = job.provers.clone();
            let job_id = job.job_id.clone();
            drop(jobs); // Release jobs lock early

            // Start Phase2 for all provers
            if let Err(e) = self.start_phase2(&job_id, &assigned_provers, challenges).await {
                error!("Failed to start Phase2 for job {}: {}", job_id, e);
                self.fail_job(&job_id, format!("Failed to start Phase2: {e}")).await.map_err(
                    |e| {
                        error!("Failed to mark job {} as failed: {}", job_id, e);
                        e
                    },
                )?;
            }
        } else {
            // Some Phase1 results failed
            let failed_provers: Vec<ProverId> = phase1_results
                .iter()
                .filter_map(
                    |(prover_id, result)| {
                        if !result.success {
                            Some(prover_id.clone())
                        } else {
                            None
                        }
                    },
                )
                .collect();

            warn!("Phase1 failed for provers {:?} in job {}", failed_provers, job_id);
            let reason = format!("Phase1 failed for provers: {failed_provers:?}");

            // Release jobs lock before calling fail_job
            let job_id = job.job_id.clone();
            drop(jobs);

            self.fail_job(&job_id, reason).await.map_err(|e| {
                error!("Failed to mark job {} as failed: {}", job_id, e);
                e
            })?;
        }

        Ok(())
    }

    /// Start Phase2 for all provers that completed Phase1
    async fn start_phase2(
        &self,
        job_id: &JobId,
        assigned_provers: &[ProverId],
        challenges: Vec<ContributionsInfo>,
    ) -> Result<()> {
        // Update prover statuses and send Phase2 messages
        let mut provers = self.provers.write().await;

        let mut ch = Vec::new();

        for challenge in challenges {
            ch.push(Challenges {
                worker_index: challenge.worker_index,
                airgroup_id: challenge.airgroup_id as u32,
                challenge: challenge.challenge.to_vec(),
            })
        }

        for prover_id in assigned_provers {
            if let Some(prover) = provers.get_mut(prover_id) {
                // Prover should still be in Working status from Phase1
                if prover.state == ProverState::Computing(JobPhase::Contributions) {
                    prover.state = ProverState::Computing(JobPhase::Prove);

                    // Create the Phase2 message (use TaskType::Proof)
                    let message = CoordinatorMessage {
                        payload: Some(coordinator_message::Payload::ExecuteTask(
                            consensus_grpc_api::ExecuteTaskRequest {
                                prover_id: prover_id.clone().into(),
                                job_id: job_id.clone().into(),
                                task_type: consensus_grpc_api::TaskType::Prove as i32,
                                params: Some(execute_task_request::Params::ProveParams(
                                    consensus_grpc_api::ProveParams { challenges: ch.clone() },
                                )),
                            },
                        )),
                    };

                    // Send Phase2 message
                    match prover.message_sender.try_send(message) {
                        Ok(()) => {
                            info!("Sent ProvePhase2 message to prover {}", prover_id);
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            warn!(
                                "Message buffer full for prover {}, dropping Phase2 message",
                                prover_id
                            );
                            return Err(Error::InvalidRequest(format!(
                                "Failed to send Phase2 message to prover {prover_id}: buffer full"
                            )));
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            error!("Channel closed for prover {}", prover_id);
                            return Err(Error::InvalidRequest(format!(
                                "Failed to send Phase2 message to prover {prover_id}: channel closed"
                            )));
                        }
                    }
                } else {
                    warn!("Prover {} is not in working state for job {}", prover_id, job_id);
                    return Err(Error::InvalidRequest(format!(
                        "Prover {prover_id} is not in computing state for job {job_id}",
                    )));
                }
            } else {
                warn!("Prover {} not found when starting Phase2", prover_id);
                return Err(Error::InvalidRequest(format!(
                    "Prover {prover_id} not found when starting Phase2"
                )));
            }
        }

        info!(
            "Successfully started Phase2 for job {} with {} provers",
            job_id,
            assigned_provers.len()
        );
        Ok(())
    }

    /// Start Phase3 for all provers that completed Phase2
    async fn start_phase3(
        &self,
        job_id: &JobId,
        assigned_provers: &[ProverId],
        // proofs: Vec<Vec<AggProofData>>,
    ) -> Result<()> {
        // Update prover statuses and send Phase3 messages
        let mut provers = self.provers.write().await;

        // For the sake of simplicity, we use now only the first prover to aggregate the proofs
        let agg_prover = self.select_agg_prover(&provers);

        let jobs = self.jobs.read().await;
        let agg_proofs = jobs.get(job_id).unwrap().results.get(&JobPhase::Prove).unwrap();

        // Get all job_result from agg_proofs unless agg_prover contains the prover_id
        let proofs: Vec<Vec<AggProofData>> = agg_proofs
            .iter()
            .filter_map(|(prover_id, result)| {
                if !agg_prover.contains(prover_id) {
                    match &result.data {
                        JobResultData::AggProofs(values) => Some(values.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect();

        let proofs: Vec<AggProofData> = proofs.into_iter().flatten().collect();
        println!("Starting aggregation with {} proofs", proofs.len());

        for prover_id in agg_prover {
            if let Some(prover) = provers.get_mut(&prover_id) {
                // Prover should still be in Working status from Phase2
                if prover.state != ProverState::Computing(JobPhase::Prove) {
                    warn!("Prover {} is not in working state for job {}", prover_id, job_id);
                    return Err(Error::InvalidRequest(format!(
                        "Prover {prover_id} is not in computing state for job {job_id}",
                    )));
                }
                prover.state = ProverState::Computing(JobPhase::Aggregate);

                let proofs: Vec<Proof> = proofs
                    .clone() // This clone must be removed when using more than one prover
                    .into_iter()
                    .map(|p| Proof {
                        airgroup_id: p.airgroup_id,
                        values: p.values,
                        worker_idx: p.worker_idx,
                    })
                    .collect();

                // Create the Phase2 message (use TaskType::Proof)
                let message = CoordinatorMessage {
                    payload: Some(coordinator_message::Payload::ExecuteTask(
                        consensus_grpc_api::ExecuteTaskRequest {
                            prover_id: prover_id.clone().into(),
                            job_id: job_id.clone().into(),
                            task_type: consensus_grpc_api::TaskType::Aggregate as i32,
                            params: Some(execute_task_request::Params::AggParams(
                                consensus_grpc_api::AggParams {
                                    agg_proofs: Some(ProofList { proofs }),
                                    last_proof: true,
                                    final_proof: true,
                                    verify_constraints: true,
                                    aggregation: true,
                                    final_snark: false,
                                    verify_proofs: true,
                                    save_proofs: false,
                                    test_mode: false,
                                    output_dir_path: "".to_string(),
                                    minimal_memory: false,
                                },
                            )),
                        },
                    )),
                };

                // Send Phase2 message
                match prover.message_sender.try_send(message) {
                    Ok(()) => {
                        info!("Sent ProveAggregate message to prover {}", prover_id);
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        warn!(
                            "Message buffer full for prover {}, dropping ProveAggregate message",
                            prover_id
                        );
                        return Err(Error::InvalidRequest(format!(
                            "Failed to send ProveAggregate message to prover {prover_id}: buffer full"
                        )));
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        error!("Channel closed for prover {}", prover_id);
                        return Err(Error::InvalidRequest(format!(
                            "Failed to send ProveAggregate message to prover {prover_id}: channel closed"
                        )));
                    }
                }
            } else {
                warn!("Prover {} not found when starting ProveAggregate", prover_id);
                return Err(Error::InvalidRequest(format!(
                    "Prover {prover_id} not found when starting ProveAggregate"
                )));
            }
        }

        info!(
            "Successfully started ProveAggregate for job {} with {} provers",
            job_id,
            assigned_provers.len()
        );
        Ok(())
    }

    fn select_agg_prover(
        &self,
        available_provers: &HashMap<ProverId, ProverConnection>,
    ) -> Vec<ProverId> {
        // For the sake of simplicity, we use now only the first prover to aggregate the proofs
        vec![available_provers.iter().next().unwrap().0.clone()]
    }

    // async fn partition_and_allocate_by_capacity(
    //     &self,
    //     required_compute_capacity: ComputeCapacity,
    // ) -> (Vec<ProverId>, Vec<Range<u32>>) {
    //     let mut selected_provers = Vec::new();
    //     let mut partitions = Vec::new();
    //     let mut accumulated = 0;

    //     for (prover_id, prover_connection) in self.provers.write().await.iter() {
    //         selected_provers.push(prover_id.clone());

    //         // Compute new partition as a range
    //         let range_start = accumulated;
    //         let remaining_needed = required_compute_capacity.compute_units - accumulated;
    //         let prover_allocation =
    //             std::cmp::min(remaining_needed, prover_connection.compute_capacity.compute_units);
    //             println!("Prover {} capacity: {}", prover_id, prover_connection.compute_capacity.compute_units);
    //         let range_end = accumulated + prover_allocation;
    //         let prover_range = range_start..range_end;

    //         partitions.push(prover_range);

    //         accumulated += prover_allocation;

    //         if accumulated >= required_compute_capacity.compute_units {
    //             break;
    //         }
    //     }

    //     println!("Selected provers: {:?}", selected_provers);
    //     println!("Partitions: {:?}", partitions);

    //     (selected_provers, partitions)
    // }

    async fn partition_and_allocate_by_capacity(
        &self,
        required_compute_capacity: ComputeCapacity,
    ) -> (Vec<ProverId>, Vec<Vec<u32>>) {
        let mut selected_provers = Vec::new();
        let mut prover_capacities = Vec::new();
        let mut total_capacity = 0;

        // Step 1: Select provers that can cover the required compute capacity
        for (prover_id, prover_connection) in self.provers.write().await.iter() {
            if matches!(prover_connection.state, ProverState::Idle) {
                selected_provers.push(prover_id.clone());
                prover_capacities.push(prover_connection.compute_capacity.compute_units);
                total_capacity += prover_connection.compute_capacity.compute_units;

                println!(
                    "Prover {} capacity: {}",
                    prover_id, prover_connection.compute_capacity.compute_units
                );

                // Stop when we have enough capacity
                if total_capacity >= required_compute_capacity.compute_units {
                    break;
                }
            }
        }

        if selected_provers.is_empty() || total_capacity < required_compute_capacity.compute_units {
            return (vec![], vec![]);
        }

        // Step 2: Assign partitions using round-robin
        let num_provers = selected_provers.len();
        let total_units = required_compute_capacity.compute_units;
        let mut prover_allocations = vec![Vec::new(); num_provers];

        // Round-robin assignment of compute units
        for unit in 0..total_units {
            let prover_idx = (unit as usize) % num_provers;

            // Check if this prover still has capacity
            if prover_allocations[prover_idx].len() < prover_capacities[prover_idx] as usize {
                prover_allocations[prover_idx].push(unit);
            } else {
                // If this prover is at capacity, find the next available prover
                let mut found = false;
                for offset in 1..num_provers {
                    let next_idx = (prover_idx + offset) % num_provers;
                    if prover_allocations[next_idx].len() < prover_capacities[next_idx] as usize {
                        prover_allocations[next_idx].push(unit);
                        found = true;
                        break;
                    }
                }

                if !found {
                    warn!("Could not assign compute unit {} to any prover", unit);
                    break;
                }
            }
        }

        // // Step 3: Convert allocations to ranges (for protobuf compatibility)
        // let mut partitions = Vec::new();
        // for (i, allocation) in prover_allocations.iter().enumerate() {
        //     if allocation.is_empty() {
        //         partitions.push(0..0); // Empty range
        //     } else {
        //         // Create a range that represents the allocated units
        //         // Note: This creates non-contiguous ranges for round-robin
        //         let min_unit = *allocation.iter().min().unwrap();
        //         let max_unit = *allocation.iter().max().unwrap();
        //         partitions.push(min_unit..(max_unit + 1));

        //         println!(
        //             "Prover {} ({}) gets {} units: {:?}",
        //             selected_provers[i],
        //             prover_capacities[i],
        //             allocation.len(),
        //             allocation
        //         );
        //     }
        // }

        println!("Selected provers: {:?}", selected_provers);
        println!("Round-robin partitions: {:?}", prover_allocations);

        (selected_provers, prover_allocations)
    }
}
