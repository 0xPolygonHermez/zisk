use crate::{BlockId, ComputeCapacity, Error, JobId, ProverId, Result};
use chrono::{DateTime, Utc};
use consensus_grpc_api::{
    coordinator_message, execute_task_request, prover_message, CoordinatorMessage,
    ExecuteTaskResponse, ProverAllocation, ProverMessage, RowData, TaskType,
};
use consensus_common::{BlockContext, Job, JobPhase, JobResult, JobState, ProverState};
use std::{collections::HashMap, ops::Range, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// Information about a connected prover - business logic only, no transport layer
#[derive(Debug)]
pub struct ProverConnection {
    pub prover_id: ProverId,
    pub state: ProverState,
    pub compute_capacity: ComputeCapacity,
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
            state: JobState::Running(JobPhase::Phase1),
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
        for (rank, prover_id) in selected_provers.iter().enumerate() {
            let prover = provers.get_mut(prover_id).unwrap();

            // Convert range to ProverAllocation vector
            let range = &partitions[rank];
            let prover_allocation =
                vec![ProverAllocation { range_start: range.start, range_end: range.end }];

            let message = CoordinatorMessage {
                payload: Some(coordinator_message::Payload::ExecuteTask(
                    consensus_grpc_api::ExecuteTaskRequest {
                        prover_id: prover_id.clone().into(),
                        job_id: job_id.clone().into(),
                        task_type: consensus_grpc_api::TaskType::PartialContribution as i32,
                        params: Some(execute_task_request::Params::PartialContribution(
                            consensus_grpc_api::PartialContributionParams {
                                block_id: block_id.clone().into(),
                                input_path: input_path.clone(),
                                rank_id: rank as u32,
                                total_provers: provers_len,
                                prover_allocation,
                                job_compute_units: required_compute_capacity.compute_units,
                            },
                        )),
                    },
                )),
            };

            // Send message through the prover's channel (bounded channels require async send)
            match prover.message_sender.try_send(message) {
                Ok(()) => {
                    prover.state = ProverState::Computing(JobPhase::Phase1);
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
                // message ExecuteTaskResponse {
                //   string prover_id = 2;
                //   string job_id = 1;
                //   TaskType task_type = 3;
                //   bool success = 4;
                //   string error_message = 5; // Optional error message if success is false
                //   repeated uint64 result_data = 6; // Serialized result data
                // }
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
                            // Mark job as complete and free provers
                            self.complete_job(&job_id).await.map_err(|e| {
                                error!("Failed to complete job {}: {}", job_id, e);
                                e
                            })?;
                        }
                        Ok(other) => {
                            warn!("Received TaskResult with unexpected task_type {:?} for job {} from prover {}", other, job_id, prover_id);
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

    /// Complete a job and reset all assigned provers back to Idle status
    async fn complete_job(&self, job_id: &JobId) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        job.state = JobState::Completed;

        // Reset prover statuses back to Idle
        self.mark_provers_with_state(&job.provers, ProverState::Idle).await?;

        info!("Completed job {} and freed {} provers", job_id, job.provers.len());

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

        let phase1_results = job.results.entry(JobPhase::Phase1).or_default();
        phase1_results.insert(
            prover_id.clone(),
            JobResult {
                success: execute_task_response.success,
                data: execute_task_response.result_data.into_iter().map(Into::into).collect(),
            },
        );

        info!("Stored Phase1 result for prover {prover_id} in job {job_id}.");

        if phase1_results.len() < self.num_provers().await {
            return Ok(());
        }

        // Check if all results are successful
        let all_successful = phase1_results.values().all(|result| result.success);

        if all_successful {
            let mut challenges = Vec::new();
            for results in phase1_results.values() {
                challenges.push(results.data[0].values.clone());
            }
            job.challenges = Some(challenges.clone());
            job.state = JobState::Running(JobPhase::Phase2);

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
        challenges: Vec<Vec<u64>>,
    ) -> Result<()> {
        // Update prover statuses and send Phase2 messages
        let mut provers = self.provers.write().await;
        for prover_id in assigned_provers {
            if let Some(prover) = provers.get_mut(prover_id) {
                // Prover should still be in Working status from Phase1
                if prover.state == ProverState::Computing(JobPhase::Phase1) {
                    prover.state = ProverState::Computing(JobPhase::Phase2);

                    // Create the Phase2 message (use TaskType::Proof)
                    let message = CoordinatorMessage {
                        payload: Some(coordinator_message::Payload::ExecuteTask(
                            consensus_grpc_api::ExecuteTaskRequest {
                                prover_id: prover_id.clone().into(),
                                job_id: job_id.clone().into(),
                                task_type: consensus_grpc_api::TaskType::Prove as i32,
                                params: Some(execute_task_request::Params::Prove(
                                    consensus_grpc_api::ProveParams {
                                        challenges: vec![RowData { values: challenges[0].clone() }],
                                    },
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

    async fn partition_and_allocate_by_capacity(
        &self,
        required_compute_capacity: ComputeCapacity,
    ) -> (Vec<ProverId>, Vec<Range<u32>>) {
        let mut selected_provers = Vec::new();
        let mut partitions = Vec::new();
        let mut accumulated = 0;

        for (prover_id, prover_connection) in self.provers.write().await.iter() {
            selected_provers.push(prover_id.clone());

            // Compute new partition as a range
            let range_start = accumulated;
            let remaining_needed = required_compute_capacity.compute_units - accumulated;
            let prover_allocation =
                std::cmp::min(remaining_needed, prover_connection.compute_capacity.compute_units);
            let range_end = accumulated + prover_allocation;
            let prover_range = range_start..range_end;

            partitions.push(prover_range);

            accumulated += prover_allocation;

            if accumulated >= required_compute_capacity.compute_units {
                break;
            }
        }

        (selected_provers, partitions)
    }
}
