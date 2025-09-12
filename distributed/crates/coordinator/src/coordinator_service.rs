use crate::ProversPool;

use anyhow::Result;
use chrono::{DateTime, Utc};
use distributed_common::{
    AggParamsDto, AggProofData, BlockContext, BlockId, ChallengesDto, ComputeCapacity,
    ContributionParamsDto, CoordinatorMessageDto, Error, ExecuteTaskRequestDto,
    ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto,
    HeartbeatAckDto, Job, JobId, JobPhase, JobResult, JobResultData, JobState, JobStatusDto,
    JobsListDto, MetricsDto, ProofDto, ProveParamsDto, ProverErrorDto, ProverId,
    ProverReconnectRequestDto, ProverRegisterRequestDto, ProverState, ProversListDto,
    StartProofRequestDto, StartProofResponseDto, StatusInfoDto, SystemStatusDto,
};

use distributed_config::Config;
use proofman::ContributionsInfo;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::RwLock;
use tonic::Status;
use tracing::{error, info, instrument, warn};

pub trait MessageSender {
    fn send(&self, msg: CoordinatorMessageDto) -> Result<()>;
}

/// Represents the runtime state of the service
pub struct CoordinatorService {
    config: Config,
    start_time_utc: DateTime<Utc>,
    active_connections: Arc<AtomicU32>,

    provers_pool: ProversPool,
    jobs: RwLock<HashMap<JobId, Job>>,
}

impl CoordinatorService {
    #[instrument(skip(config))]
    pub async fn new(config: Config) -> distributed_common::Result<Self> {
        info!("Initializing service state");

        let start_time_utc = Utc::now();

        // Create ProverManager with configuration from config
        let coordinator_config = config.coordinator.clone();

        Ok(Self {
            config,
            start_time_utc,
            active_connections: Arc::new(AtomicU32::new(0)),
            provers_pool: ProversPool::new(coordinator_config),
            jobs: RwLock::new(HashMap::new()),
        })
    }

    pub fn active_connections(&self) -> Arc<AtomicU32> {
        self.active_connections.clone()
    }

    pub fn max_concurrent_connections(&self) -> u32 {
        self.config.coordinator.max_concurrent_connections
    }

    pub fn status_info(&self) -> StatusInfoDto {
        let uptime_seconds = (Utc::now() - self.start_time_utc).num_seconds() as u64;

        let metrics =
            MetricsDto { active_connections: self.active_connections.load(Ordering::SeqCst) };

        StatusInfoDto::new(
            "Distributed Prover Service".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds,
            self.start_time_utc,
            metrics,
        )
    }

    pub fn jobs_list(&self) -> JobsListDto {
        // TODO: Implement actual job retrieval from database
        JobsListDto { jobs: Vec::new() }
    }

    pub fn provers_list(&self) -> ProversListDto {
        // TODO: Implement actual prover retrieval from database
        ProversListDto { provers: Vec::new() }
    }

    pub fn job_status(&self, job_id: &JobId) -> JobStatusDto {
        // TODO: Implement actual job retrieval from database
        JobStatusDto {
            job_id: job_id.to_string(),
            block_id: "block123".to_string(),
            phase: "proving".to_string(),
            status: "in_progress".to_string(),
            assigned_provers: vec!["prover1".to_string(), "prover2".to_string()],
            start_time: Utc::now().timestamp() as u64,
            duration_ms: 5000,
        }
    }

    pub async fn handle_system_status(&self) -> SystemStatusDto {
        // Get actual system status from ProverManager
        let total_provers = self.provers_pool.num_provers().await;
        let compute_capacity = self.provers_pool.compute_capacity().await;
        let idle_provers = self.provers_pool.num_provers().await;
        let busy_provers = total_provers.saturating_sub(idle_provers);

        SystemStatusDto {
            total_provers: total_provers as u32,
            compute_capacity,
            idle_provers: idle_provers as u32,
            busy_provers: busy_provers as u32,
            active_jobs: 0,                // TODO: Implement actual job counting
            pending_jobs: 0,               // TODO: Implement actual job counting
            completed_jobs_last_minute: 0, // TODO: Implement actual metrics
            job_completion_rate: 0.0,      // TODO: Implement actual metrics
            prover_utilization: if total_provers > 0 {
                (busy_provers as f64) / (total_provers as f64)
            } else {
                0.0
            },
        }
    }

    pub async fn start_proof(
        &self,
        request: StartProofRequestDto,
    ) -> Result<StartProofResponseDto> {
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
        Ok(StartProofResponseDto { job_id: job_id.as_string() })
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

    /// Handle registration directly in stream context (static version to avoid lifetime issues)
    pub async fn handle_stream_registration(
        &self,
        req: ProverRegisterRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Result<ProverId, Status> {
        self.provers_pool
            .register_prover(ProverId::from(req.prover_id), req.compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Registration failed: {e}")))
    }

    /// Handle reconnection directly in stream context (static version to avoid lifetime issues)
    pub async fn handle_stream_reconnection(
        &self,
        req: ProverReconnectRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> Result<ProverId, Status> {
        self.provers_pool
            .register_prover(ProverId::from(req.prover_id), req.compute_capacity, msg_sender)
            .await
            .map_err(|e| Status::internal(format!("Reconnection failed: {e}")))
    }

    /// Unregister a prover by its ID
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> Result<()> {
        Ok(self.provers_pool.unregister_prover(prover_id).await?)
    }

    pub async fn handle_stream_heartbeat_ack(
        &self,
        prover_id: &ProverId,
        message: HeartbeatAckDto,
    ) -> Result<()> {
        assert_eq!(prover_id, &message.prover_id);

        self.provers_pool
            .update_last_heartbeat(&ProverId::from(message.prover_id))
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn handle_stream_error(
        &self,
        prover_id: &ProverId,
        message: ProverErrorDto,
    ) -> Result<()> {
        assert_eq!(prover_id, &message.prover_id);

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

    pub async fn handle_stream_register(
        &self,
        prover_id: &ProverId,
        message: ProverRegisterRequestDto,
    ) -> Result<()> {
        assert_eq!(prover_id.as_string(), message.prover_id);

        let prover_id = ProverId::from(message.prover_id);

        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&prover_id).await?;

        // TODO: Handle prover registration if needed
        Ok(())
    }

    pub async fn handle_stream_reconnect(
        &self,
        prover_id: &ProverId,
        message: ProverReconnectRequestDto,
    ) -> Result<()> {
        assert_eq!(prover_id.as_string(), message.prover_id);

        let prover_id = ProverId::from(message.prover_id);

        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&prover_id).await?;

        // TODO: Handle prover reconnection if needed
        Ok(())
    }

    pub async fn handle_stream_execute_task_response(
        &self,
        prover_id: &ProverId,
        message: ExecuteTaskResponseDto,
    ) -> Result<()> {
        assert_eq!(prover_id, &message.prover_id);

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

        Ok(())
    }
}
