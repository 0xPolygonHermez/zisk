use distributed_common::{
    AggProofData, BlockContext, BlockId, ComputeCapacity, Error, Job, JobId, JobPhase, JobResult,
    JobResultData, JobState, ProverId, ProverState, Result,
};
use distributed_config::CoordinatorConfig;
use distributed_grpc_api::{
    coordinator_message, execute_task_request, prover_message, Challenges, CoordinatorMessage,
    ExecuteTaskResponse, Proof, ProofList, ProverMessage, TaskType,
};
use proofman::ContributionsInfo;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

use crate::ProversPool;

/// Prover manager for handling multiple provers
pub struct Coordinator {
    provers_pool: ProversPool,
    jobs: RwLock<HashMap<JobId, Job>>,
    config: CoordinatorConfig,
}

impl Coordinator {
    pub fn new(config: CoordinatorConfig) -> Self {
        Self {
            provers_pool: ProversPool::new(config.clone()),
            jobs: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Register a new prover connection
    pub async fn register_prover(
        &self,
        prover_id: ProverId,
        compute_capacity: impl Into<ComputeCapacity>,
        message_sender: mpsc::Sender<CoordinatorMessage>,
    ) -> Result<ProverId> {
        self.provers_pool.register_prover(prover_id, compute_capacity, message_sender).await
    }

    /// Remove an existing prover connection
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> Result<()> {
        self.provers_pool.unregister_prover(prover_id).await
    }

    /// Start a proof job with the specified request
    pub async fn start_proof(
        &self,
        block_id: String,
        required_compute_capacity: ComputeCapacity,
        input_path: String,
    ) -> Result<JobId> {
        // Select only the required number of provers (not all of them)
        let (selected_provers, partitions) =
            self.provers_pool.partition_and_allocate_by_capacity(required_compute_capacity).await?;

        info!(
            "Starting proof for block {} using {} with input path: {}",
            block_id, required_compute_capacity, input_path
        );

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
        let provers_len = selected_provers.len() as u32;

        for (rank_id, prover_id) in selected_provers.iter().enumerate() {
            // Convert range to ProverAllocation vector
            let range = &partitions[rank_id];
            let prover_allocation = range.clone();

            let message = CoordinatorMessage {
                payload: Some(coordinator_message::Payload::ExecuteTask(
                    distributed_grpc_api::ExecuteTaskRequest {
                        prover_id: prover_id.clone().into(),
                        job_id: job_id.clone().into(),
                        task_type: distributed_grpc_api::TaskType::PartialContribution as i32,
                        params: Some(execute_task_request::Params::ContributionParams(
                            distributed_grpc_api::ContributionParams {
                                block_id: block_id.clone().into(),
                                input_path: input_path.clone(),
                                rank_id: rank_id as u32,
                                total_provers: provers_len,
                                prover_allocation,
                                job_compute_units: required_compute_capacity.compute_units,
                            },
                        )),
                    },
                )),
            };

            // Send message through the prover's channel (bounded channels require async send)
            self.provers_pool.send_message(prover_id, message).await?;

            self.provers_pool
                .mark_prover_with_state(prover_id, ProverState::Computing(JobPhase::Contributions))
                .await?;
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
        self.provers_pool.update_last_heartbeat(prover_id).await?;

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
            Some(distributed_grpc_api::execute_task_response::ResultData::Proofs(proof_list)) => {
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
            self.provers_pool.mark_provers_with_state(&assigned_provers, ProverState::Idle).await?;

            info!("Completed job {} and freed {} provers", job_id, assigned_provers.len());
        } else {
            // Some Phase2 results failed
            match execute_task_response.result_data {
                Some(distributed_grpc_api::execute_task_response::ResultData::FinalProof(_)) => {}
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
        self.provers_pool.mark_provers_with_state(&job.provers, ProverState::Idle).await?;

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
            Some(distributed_grpc_api::execute_task_response::ResultData::Challenges(
                challenges,
            )) => {
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
        let mut ch = Vec::new();

        for challenge in challenges {
            ch.push(Challenges {
                worker_index: challenge.worker_index,
                airgroup_id: challenge.airgroup_id as u32,
                challenge: challenge.challenge.to_vec(),
            })
        }

        for prover_id in assigned_provers {
            if let Some(prover_state) = self.provers_pool.prover_state(prover_id).await {
                // Prover should still be in Working status from Phase1
                if prover_state == ProverState::Computing(JobPhase::Contributions) {
                    self.provers_pool
                        .mark_prover_with_state(prover_id, ProverState::Computing(JobPhase::Prove))
                        .await?;

                    // Create the Phase2 message (use TaskType::Proof)
                    let message = CoordinatorMessage {
                        payload: Some(coordinator_message::Payload::ExecuteTask(
                            distributed_grpc_api::ExecuteTaskRequest {
                                prover_id: prover_id.clone().into(),
                                job_id: job_id.clone().into(),
                                task_type: distributed_grpc_api::TaskType::Prove as i32,
                                params: Some(execute_task_request::Params::ProveParams(
                                    distributed_grpc_api::ProveParams { challenges: ch.clone() },
                                )),
                            },
                        )),
                    };

                    // Send Phase2 message
                    self.provers_pool.send_message(prover_id, message).await?;
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

        // For the sake of simplicity, we use now only the first prover to aggregate the proofs
        let agg_prover = self.provers_pool.select_agg_prover().await;

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
            if let Some(prover_state) = self.provers_pool.prover_state(&prover_id).await {
                // Prover should still be in Working status from Phase2
                if prover_state != ProverState::Computing(JobPhase::Prove) {
                    warn!("Prover {} is not in working state for job {}", prover_id, job_id);
                    return Err(Error::InvalidRequest(format!(
                        "Prover {prover_id} is not in computing state for job {job_id}",
                    )));
                }
                self.provers_pool
                    .mark_prover_with_state(&prover_id, ProverState::Computing(JobPhase::Aggregate))
                    .await?;

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
                        distributed_grpc_api::ExecuteTaskRequest {
                            prover_id: prover_id.clone().into(),
                            job_id: job_id.clone().into(),
                            task_type: distributed_grpc_api::TaskType::Aggregate as i32,
                            params: Some(execute_task_request::Params::AggParams(
                                distributed_grpc_api::AggParams {
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
                self.provers_pool.send_message(&prover_id, message).await?;
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

    // TODO! remove these functions ????
    pub async fn num_provers(&self) -> usize {
        self.provers_pool.num_provers().await
    }

    pub async fn compute_capacity(&self) -> ComputeCapacity {
        self.provers_pool.compute_capacity().await
    }

    pub async fn config(&self) -> CoordinatorConfig {
        self.config.clone()
    }
}
