use crate::{config::Config, ProversPool};

use anyhow::Result;
use chrono::{DateTime, Utc};
use distributed_common::{
    AggParamsDto, AggProofData, BlockContext, BlockId, ChallengesDto, ComputeCapacity,
    ContributionParamsDto, CoordinatorMessageDto, Error, ExecuteTaskRequestDto,
    ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto,
    HeartbeatAckDto, Job, JobId, JobPhase, JobResult, JobResultData, JobState, JobStatusDto,
    JobsListDto, LaunchProofRequestDto, LaunchProofResponseDto, MetricsDto, ProofDto,
    ProveParamsDto, ProverErrorDto, ProverId, ProverReconnectRequestDto, ProverRegisterRequestDto,
    ProverState, ProversListDto, StatusInfoDto, SystemStatusDto,
};
use proofman::ContributionsInfo;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::RwLock;
use tonic::Status;
use tracing::{debug, error, info, instrument, warn};

pub trait MessageSender {
    fn send(&self, msg: CoordinatorMessageDto) -> Result<()>;
}

/// Represents the runtime state of the service
pub struct CoordinatorService {
    /// Config data including Server, Logging and Coordinator settings
    config: Config,

    /// DateTime when the service was started
    start_time_utc: DateTime<Utc>,

    /// Pool of streaming connections
    provers_pool: ProversPool,

    /// Hashmap of jobs
    jobs: RwLock<HashMap<JobId, Job>>,
}

impl CoordinatorService {
    #[instrument(skip(config))]
    pub fn new(config: Config) -> Self {
        info!("Initializing service state");

        let start_time_utc = Utc::now();

        Self {
            config,
            start_time_utc,
            provers_pool: ProversPool::new(),
            jobs: RwLock::new(HashMap::new()),
        }
    }

    pub async fn handle_status_info(&self) -> StatusInfoDto {
        let uptime_seconds = (Utc::now() - self.start_time_utc).num_seconds() as u64;

        let metrics =
            MetricsDto { active_connections: self.provers_pool.num_provers().await as u32 };

        StatusInfoDto::new(
            "Distributed Prover Service".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds,
            self.start_time_utc,
            metrics,
        )
    }

    /// List all running jobs only
    pub async fn handle_jobs_list(&self) -> JobsListDto {
        let jobs = self
            .jobs
            .read()
            .await
            .values()
            .filter_map(|job| {
                if let JobState::Running(phase) = &job.state {
                    Some(JobStatusDto {
                        job_id: job.job_id.clone(),
                        block_id: job.block.block_id.clone(),
                        phase: Some(phase.clone()),
                        status: job.state.clone(),
                        assigned_provers: job.provers.clone(),
                        start_time: job.start_time.timestamp() as u64,
                        duration_ms: job.duration_ms.unwrap_or(0),
                    })
                } else {
                    None
                }
            })
            .collect();

        JobsListDto { jobs }
    }

    pub async fn handle_provers_list(&self) -> ProversListDto {
        self.provers_pool.provers_list().await
    }

    pub async fn handle_job_status(&self, job_id: &JobId) -> Result<JobStatusDto> {
        let job = self.jobs.read().await.get(job_id).cloned().ok_or_else(|| {
            Error::InvalidRequest(format!("Job with ID {} not found", job_id.as_string()))
        })?;

        Ok(JobStatusDto {
            job_id: job.job_id.clone(),
            block_id: job.block.block_id.clone(),
            status: job.state.clone(),
            phase: if let JobState::Running(phase) = &job.state {
                Some(phase.clone())
            } else {
                None
            },
            assigned_provers: job.provers.clone(),
            start_time: job.start_time.timestamp() as u64,
            duration_ms: job.duration_ms.unwrap_or(0),
        })
    }

    pub async fn handle_system_status(&self) -> SystemStatusDto {
        let total_provers = self.provers_pool.num_provers().await;
        let busy_provers = self.provers_pool.busy_provers().await;
        let active_jobs = self
            .jobs
            .read()
            .await
            .values()
            .filter(|j| matches!(j.state, JobState::Running(_)))
            .count();

        SystemStatusDto {
            total_provers: total_provers as u32,
            compute_capacity: self.provers_pool.compute_capacity().await,
            idle_provers: self.provers_pool.idle_provers().await as u32,
            busy_provers: busy_provers as u32,
            active_jobs: active_jobs as u32,
        }
    }

    pub fn pre_launch_proof(&self, _request: &LaunchProofRequestDto) {
        debug!("Pre-launch hook called");
    }

    pub async fn launch_proof(
        &self,
        request: LaunchProofRequestDto,
    ) -> Result<LaunchProofResponseDto> {
        self.pre_launch_proof(&request);

        let block_id = BlockId::from(request.block_id.clone());
        let required_compute_capacity = ComputeCapacity { compute_units: request.compute_units };
        let job = self.create_job(block_id, required_compute_capacity, request.input_path).await?;

        let job_id = job.job_id.clone();
        let block_id = job.block.block_id.clone();

        // Send messages to selected provers
        let provers_len = job.provers.len() as u32;

        for (rank_id, prover_id) in job.provers.iter().enumerate() {
            let req = ExecuteTaskRequestDto {
                prover_id: prover_id.clone().into(),
                job_id: job_id.clone().into(),
                params: ExecuteTaskRequestTypeDto::ContributionParams(ContributionParamsDto {
                    block_id: block_id.clone().into(),
                    input_path: job.block.input_path.display().to_string(),
                    rank_id: rank_id as u32,
                    total_provers: provers_len,
                    prover_allocation: job.partitions[rank_id].clone(),
                    job_compute_units: required_compute_capacity,
                }),
            };
            let req = CoordinatorMessageDto::ExecuteTaskRequest(req);
            let message = req.into();

            self.provers_pool.send_message(prover_id, message).await?;

            self.provers_pool
                .mark_prover_with_state(prover_id, ProverState::Computing(JobPhase::Contributions))
                .await?;
        }

        info!(
            "Assigned new job {} to {} provers with input path: {}",
            job_id,
            provers_len,
            job.block.input_path.display()
        );

        self.jobs.write().await.insert(job_id.clone(), job);

        info!("Successfully started proof job: {}", job_id.as_string());
        Ok(LaunchProofResponseDto { job_id })
    }

    pub fn post_launch_proof(&self) {
        debug!("Post-launch hook called");
    }

    pub async fn create_job(
        &self,
        block_id: BlockId,
        required_compute_capacity: ComputeCapacity,
        input_path: String,
    ) -> Result<Job> {
        let (selected_provers, partitions) =
            self.provers_pool.partition_and_allocate_by_capacity(required_compute_capacity).await?;

        info!(
            "Starting proof for block {} using {} with input path: {}",
            block_id, required_compute_capacity, input_path
        );

        // Create job
        let job_id = JobId::new();
        let block_id = BlockId::from(block_id);

        Ok(Job {
            job_id: job_id.clone(),
            start_time: Utc::now(),
            duration_ms: None,
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
        })
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

    /// Handle registration directly in stream context
    pub async fn handle_stream_registration(
        &self,
        req: ProverRegisterRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Result<ProverId, Status> {
        let max_connections = self.config.coordinator.max_total_provers as usize;
        if self.provers_pool.num_provers().await >= max_connections {
            return Err(Status::resource_exhausted(format!(
                "Maximum concurrent connections reached: {}/{}",
                self.provers_pool.num_provers().await,
                max_connections
            )));
        }

        let prover_id = ProverId::from(req.prover_id);

        // TODO: Check if prover_id is already registered

        self.provers_pool
            .register_prover(prover_id, req.compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Registration failed: {e}")))
    }

    /// Handle reconnection directly in stream context
    pub async fn handle_stream_reconnection(
        &self,
        req: ProverReconnectRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Result<ProverId, Status> {
        let prover_id = ProverId::from(req.prover_id);

        // TODO: Check if prover_id is already registered

        self.provers_pool
            .register_prover(prover_id, req.compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Reconnection failed: {e}")))
    }

    /// Unregister a prover by its ID
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> Result<()> {
        Ok(self.provers_pool.unregister_prover(prover_id).await?)
    }

    pub async fn handle_stream_heartbeat_ack(&self, message: HeartbeatAckDto) -> Result<()> {
        self.provers_pool
            .update_last_heartbeat(&ProverId::from(message.prover_id))
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn handle_stream_error(&self, message: ProverErrorDto) -> Result<()> {
        let prover_id = ProverId::from(message.prover_id);

        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&prover_id).await?;

        error!("Prover {} error: {}", prover_id, message.error_message);

        // If the error includes a job_id, we should fail that job
        let job_id = JobId::from(message.job_id.clone());

        self.fail_job(&job_id, message.error_message.clone()).await.map_err(|e| {
            error!("Failed to mark job {} as failed after prover error: {}", job_id, e);
            e
        })?;

        Ok(())
    }

    pub async fn handle_stream_register(&self, message: ProverRegisterRequestDto) -> Result<()> {
        let prover_id = ProverId::from(message.prover_id);

        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&prover_id).await?;

        // TODO: Handle prover registration if needed
        Ok(())
    }

    pub async fn handle_stream_reconnect(&self, message: ProverReconnectRequestDto) -> Result<()> {
        let prover_id = ProverId::from(message.prover_id);

        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&prover_id).await?;

        // TODO: Handle prover reconnection if needed
        Ok(())
    }

    pub async fn handle_stream_execute_task_response(
        &self,
        message: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let prover_id = ProverId::from(message.prover_id.clone());

        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&prover_id).await?;

        let job_id = JobId::from(message.job_id.clone());

        // Check if job exists
        if !self.jobs.read().await.contains_key(&job_id) {
            warn!(
                "Received ExecuteTaskResponse for unknown job {} from prover {}",
                job_id, prover_id
            );
            return Err(Error::InvalidRequest(format!("Job {job_id} not found")).into());
        }

        if !message.success {
            self.fail_job(&job_id, "Final proof generation failed".to_string()).await.map_err(
                |e| {
                    error!("Failed to mark job {} as failed: {}", job_id, e);
                    e
                },
            )?;

            return Err(Error::Service(format!(
                "Prover {} failed to execute task for job {}: {}",
                prover_id,
                message.job_id,
                message.error_message.unwrap()
            ))
            .into());
        }

        info!("Execute task result success from prover {} (job_id: {})", prover_id, job_id);

        match message.result_data {
            ExecuteTaskResponseResultDataDto::Challenges(_) => {
                self.handle_phase1_result(message).await.map_err(|e| {
                    error!("Failed to handle Phase1 result: {}", e);
                    e
                })
            }
            ExecuteTaskResponseResultDataDto::Proofs(_) => {
                // Handle Phase2 completion - wait for all provers to complete
                self.handle_phase2_result(message).await.map_err(|e| {
                    error!("Failed to handle Phase2 result for job {}: {}", job_id, e);
                    e
                })
            }
            ExecuteTaskResponseResultDataDto::FinalProof(_) => {
                // Handle Aggregation completion - wait for all provers to complete
                self.handle_agregate_result(message).await.map_err(|e| {
                    error!("Failed to handle Aggregation result for job {}: {}", job_id, e);
                    e
                })
            }
        }
        // Store the Phase1 result and check if we can proceed to Phase2
    }

    /// Handle Phase1 result and check if we can proceed to Phase2
    pub async fn handle_phase1_result(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let job_id = JobId::from(execute_task_response.job_id.clone());

        let mut jobs = self.jobs.write().await;

        let job = jobs
            .get_mut(&job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let prover_id = ProverId::from(execute_task_response.prover_id);

        let phase1_results = job.results.entry(JobPhase::Contributions).or_default();

        let data = match execute_task_response.result_data {
            ExecuteTaskResponseResultDataDto::Challenges(challenges) => {
                assert!(!challenges.is_empty());

                let mut cont = Vec::new();
                for challenge in challenges {
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
                )
                .into());
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
            ch.push(ChallengesDto {
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

                    let req = ExecuteTaskRequestDto {
                        prover_id: prover_id.clone().into(),
                        job_id: job_id.clone().into(),
                        params: ExecuteTaskRequestTypeDto::ProveParams(ProveParamsDto {
                            challenges: ch.clone(),
                        }),
                    };
                    let req = CoordinatorMessageDto::ExecuteTaskRequest(req);
                    let message = req.into();

                    // Send Phase2 message
                    self.provers_pool.send_message(prover_id, message).await?;
                } else {
                    warn!("Prover {} is not in working state for job {}", prover_id, job_id);
                    return Err(Error::InvalidRequest(format!(
                        "Prover {prover_id} is not in computing state for job {job_id}",
                    ))
                    .into());
                }
            } else {
                warn!("Prover {} not found when starting Phase2", prover_id);
                return Err(Error::InvalidRequest(format!(
                    "Prover {prover_id} not found when starting Phase2"
                ))
                .into());
            }
        }

        info!(
            "Successfully started Phase2 for job {} with {} provers",
            job_id,
            assigned_provers.len()
        );
        Ok(())
    }

    /// Handle Phase2 result and check if the job is complete
    async fn handle_phase2_result(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let job_id = execute_task_response.job_id.clone();
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let prover_id = ProverId::from(execute_task_response.prover_id);

        // Store Phase2 result
        let phase2_results = job.results.entry(JobPhase::Prove).or_default();

        // Check if we already have a result from this prover
        if phase2_results.contains_key(&prover_id) {
            warn!("Received duplicate Phase2 result from prover {} for job {}", prover_id, job_id);
            return Err(Error::InvalidRequest(format!(
                "Duplicate Phase2 result from prover {prover_id} for job {job_id}"
            ))
            .into());
        }

        let data = match execute_task_response.result_data {
            ExecuteTaskResponseResultDataDto::Proofs(proof_list) => {
                let agg_proofs: Vec<AggProofData> = proof_list
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
                )
                .into());
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
                    ))
                    .into());
                }
                self.provers_pool
                    .mark_prover_with_state(&prover_id, ProverState::Computing(JobPhase::Aggregate))
                    .await?;

                let proofs: Vec<ProofDto> = proofs
                    .clone() // This clone must be removed when using more than one prover
                    .into_iter()
                    .map(|p| ProofDto {
                        airgroup_id: p.airgroup_id,
                        values: p.values,
                        worker_idx: p.worker_idx,
                    })
                    .collect();

                let req = ExecuteTaskRequestDto {
                    prover_id: prover_id.clone().into(),
                    job_id: job_id.clone().into(),
                    params: ExecuteTaskRequestTypeDto::AggParams(AggParamsDto {
                        agg_proofs: proofs,
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
                    }),
                };

                let req = CoordinatorMessageDto::ExecuteTaskRequest(req);
                let message = req.into();

                // Send Phase2 message
                self.provers_pool.send_message(&prover_id, message).await?;
            } else {
                warn!("Prover {} not found when starting ProveAggregate", prover_id);
                return Err(Error::InvalidRequest(format!(
                    "Prover {prover_id} not found when starting ProveAggregate"
                ))
                .into());
            }
        }

        info!(
            "Successfully started ProveAggregate for job {} with {} provers",
            job_id,
            assigned_provers.len()
        );
        Ok(())
    }

    /// Handle Phase2 result and check if the job is complete
    async fn handle_agregate_result(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let job_id = execute_task_response.job_id.clone();

        info!("Handling aggregation result for job {}", job_id);

        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let _result = match execute_task_response.result_data {
            ExecuteTaskResponseResultDataDto::FinalProof(final_proof) => final_proof,
            _ => {
                return Err(Error::InvalidRequest(
                    "Expected Proofs result data for Phase2".to_string(),
                )
                .into());
            }
        };

        if execute_task_response.success {
            job.state = JobState::Completed;

            // Get the assigned provers before releasing the lock
            let assigned_provers = job.provers.clone();

            // Reset prover statuses back to Idle
            self.provers_pool.mark_provers_with_state(&assigned_provers, ProverState::Idle).await?;

            info!("Completed job {} and freed {} provers", job_id, assigned_provers.len());
        } else {
            // Some Phase2 results failed
            warn!("Aggregation failed in job {}", job_id);
            let reason = "Aggregation failed".to_string();

            self.fail_job(&job.job_id, reason).await?;
        }

        drop(jobs);

        self.post_launch_proof();

        Ok(())
    }
}
