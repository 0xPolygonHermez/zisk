use crate::{config::Config, hooks, ProversPool};

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use distributed_common::{
    AggParamsDto, AggProofData, BlockId, ChallengesDto, ComputeCapacity, ContributionParamsDto,
    CoordinatorMessageDto, Error, ExecuteTaskRequestDto, ExecuteTaskRequestTypeDto,
    ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, HeartbeatAckDto, Job,
    JobExecutionMode, JobId, JobPhase, JobResult, JobResultData, JobState, JobStatusDto,
    JobsListDto, LaunchProofRequestDto, LaunchProofResponseDto, MetricsDto, ProofDto,
    ProveParamsDto, ProverErrorDto, ProverId, ProverReconnectRequestDto, ProverRegisterRequestDto,
    ProverState, ProversListDto, StatusInfoDto, SystemStatusDto,
};
use proofman::ContributionsInfo;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};

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

    /// DashMap of jobs with individual RwLocks for better granularity
    jobs: DashMap<JobId, RwLock<Job>>,
}

impl CoordinatorService {
    #[instrument(skip(config))]
    pub fn new(config: Config) -> Self {
        let start_time_utc = Utc::now();

        Self { config, start_time_utc, provers_pool: ProversPool::new(), jobs: DashMap::new() }
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
        let mut jobs = Vec::new();

        for entry in self.jobs.iter() {
            let job_lock = entry.value();
            let job = job_lock.read().await;

            if let JobState::Running(phase) = &job.state() {
                jobs.push(JobStatusDto {
                    job_id: job.job_id.clone(),
                    block_id: job.block.block_id.clone(),
                    phase: Some(phase.clone()),
                    state: job.state().clone(),
                    assigned_provers: job.provers.clone(),
                    start_time: job.start_time.timestamp() as u64,
                    duration_ms: job.duration_ms.unwrap_or(0),
                });
            }
        }

        JobsListDto { jobs }
    }

    pub async fn handle_provers_list(&self) -> ProversListDto {
        self.provers_pool.provers_list().await
    }

    pub async fn handle_job_status(&self, job_id: &JobId) -> Result<JobStatusDto> {
        let job_entry = self.jobs.get(job_id).ok_or_else(|| {
            Error::InvalidRequest(format!("Job with ID {} not found", job_id.as_string()))
        })?;

        let job = job_entry.read().await;

        Ok(JobStatusDto {
            job_id: job.job_id.clone(),
            block_id: job.block.block_id.clone(),
            state: job.state().clone(),
            phase: if let JobState::Running(phase) = &job.state() {
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

        let mut active_jobs = 0;
        for entry in self.jobs.iter() {
            let job = entry.value().read().await;
            if matches!(job.state(), JobState::Running(_)) {
                active_jobs += 1;
            }
        }

        SystemStatusDto {
            total_provers: total_provers as u32,
            compute_capacity: self.provers_pool.compute_capacity().await,
            idle_provers: self.provers_pool.idle_provers().await as u32,
            busy_provers: busy_provers as u32,
            active_jobs: active_jobs as u32,
        }
    }

    /// Proof Generation
    /// -------------------------------------------------------------------------------------
    /// The `launch_proof` function is the entry point for creating and orchestrating a new
    /// proof workflow.  
    /// - `pre_launch_proof` runs beforehand for validation and setup.  
    /// - `post_launch_proof` runs afterward for cleanup, logging, or extra processing.
    /// -------------------------------------------------------------------------------------
    pub fn pre_launch_proof(&self, request: &LaunchProofRequestDto) -> Result<()> {
        // Check if compute_units is within allowed limits
        if request.compute_capacity == 0 {
            error!("Requested compute_units is 0, which is invalid.");
            return Err(anyhow::anyhow!("compute_units must be greater than 0".to_string()));
        }

        // Check if we have enough capacity to compute the proof is already checked
        // in create_job > partition_and_allocate_by_capacity

        // Check if input_path file exists
        let input_path = PathBuf::from(&request.input_path);
        if !input_path.exists() {
            error!("Input path does not exist: {}", request.input_path);
            return Err(anyhow::anyhow!("Input path does not exist: {}", request.input_path));
        }

        Ok(())
    }

    pub async fn launch_proof(
        &self,
        request: LaunchProofRequestDto,
    ) -> Result<LaunchProofResponseDto> {
        self.pre_launch_proof(&request)?;

        let required_compute_capacity = ComputeCapacity::from(request.compute_capacity);

        // Create and configure a new job
        let mut job = self
            .create_job(
                request.block_id.clone(),
                required_compute_capacity,
                request.input_path,
                request.simulated_node,
            )
            .await?;

        info!("Successfully started Prove job {}", job.job_id);

        // Initialize job state
        job.change_state(JobState::Running(JobPhase::Contributions));

        let job_id = job.job_id.clone();
        let active_provers = self.select_provers_for_execution(&job)?;

        // Store job in jobs map with RwLock
        self.jobs.insert(job_id.clone(), RwLock::new(job));

        // Send Phase1 tasks to selected provers
        if let Some(job_entry) = self.jobs.get(&job_id) {
            let job = job_entry.read().await;
            self.dispatch_contributions_messages(
                request.block_id,
                required_compute_capacity,
                &job,
                &active_provers,
            )
            .await?;
        }

        info!("Successfully started Phase1 for {} with {} provers", job_id, active_provers.len());

        Ok(LaunchProofResponseDto { job_id })
    }

    pub async fn post_launch_proof(&self, job_id: &JobId) -> Result<()> {
        // Check if webhook URL is configured
        if let Some(webhook_url) = &self.config.coordinator.webhook_url {
            let webhook_url = webhook_url.clone();

            let (final_proof, success) = {
                let job_entry = self
                    .jobs
                    .get(job_id)
                    .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;
                let job = job_entry.read().await;
                (job.final_proof.clone(), matches!(job.state(), JobState::Completed))
            };

            let job_id = job_id.clone();

            // Spawn a non-blocking task
            tokio::spawn(async move {
                if let Err(e) =
                    hooks::send_completion_webhook(webhook_url, job_id, final_proof, success).await
                {
                    error!("Failed to send webhook notification: {}", e);
                }
            });
        }

        Ok(())
    }

    pub async fn create_job(
        &self,
        block_id: BlockId,
        required_compute_capacity: ComputeCapacity,
        input_path: String,
        simulated_node: Option<u32>,
    ) -> Result<Job> {
        let execution_mode = if let Some(node) = simulated_node {
            JobExecutionMode::Simulating(node)
        } else {
            JobExecutionMode::Standard
        };

        let (selected_provers, mut partitions) = self
            .provers_pool
            .partition_and_allocate_by_capacity(required_compute_capacity, execution_mode)
            .await?;

        if let Some(simulated_node) = simulated_node {
            partitions[0] = partitions[simulated_node as usize].clone();
        }

        Ok(Job::new(
            block_id,
            PathBuf::from(input_path),
            required_compute_capacity,
            selected_provers,
            partitions,
            execution_mode,
        ))
    }

    fn select_provers_for_execution(&self, job: &Job) -> Result<Vec<ProverId>> {
        let selected_provers = match job.execution_mode {
            // In simulation mode we only use the first prover to simulate the execution of N nodes
            JobExecutionMode::Simulating(simulated_node) => {
                if simulated_node as usize >= job.provers.len() {
                    let msg = format!(
                        "Simulated mode index ({simulated_node}) exceeds available provers ({}).",
                        job.provers.len()
                    );
                    error!(msg);
                    return Err(anyhow::anyhow!(Error::InvalidRequest(msg)));
                }

                job.provers[0..1].to_vec()
            }
            // In standard mode use the already selected provers during the job creation
            JobExecutionMode::Standard => job.provers.clone(),
        };
        Ok(selected_provers)
    }

    async fn dispatch_contributions_messages(
        &self,
        block_id: BlockId,
        required_compute_capacity: ComputeCapacity,
        job: &Job,
        active_provers: &[ProverId],
    ) -> Result<(), anyhow::Error> {
        for (rank_id, prover_id) in active_provers.iter().enumerate() {
            // Create contribution task request
            let req = ExecuteTaskRequestDto {
                prover_id: prover_id.clone(),
                job_id: job.job_id.clone(),
                params: ExecuteTaskRequestTypeDto::ContributionParams(ContributionParamsDto {
                    block_id: block_id.clone(),
                    input_path: job.block.input_path.display().to_string(),
                    rank_id: rank_id as u32,
                    total_provers: active_provers.len() as u32,
                    prover_allocation: job.partitions[rank_id].clone(),
                    job_compute_units: required_compute_capacity,
                }),
            };
            let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

            // Send task to prover
            self.provers_pool.send_message(prover_id, req).await?;

            // Update prover state
            self.provers_pool
                .mark_prover_with_state(prover_id, ProverState::Computing(JobPhase::Contributions))
                .await?;
        }

        Ok(())
    }

    /// Mark a job as failed and reset prover statuses
    pub async fn fail_job(&self, job_id: &JobId, reason: impl AsRef<str>) -> Result<()> {
        let job_entry = self
            .jobs
            .get(job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let mut job = job_entry.write().await;
        job.change_state(JobState::Failed);

        // Reset prover statuses back to Idle
        self.provers_pool.mark_provers_with_state(&job.provers, ProverState::Idle).await?;

        error!(
            "Failed job {} (reason: {}) and freed {} provers",
            job_id,
            reason.as_ref(),
            job.provers.len()
        );

        drop(job); // Release job lock before calling post_launch_proof

        // Add webhook notification for failed jobs
        self.post_launch_proof(job_id).await?;

        Ok(())
    }

    /// Handle registration directly in stream context
    pub async fn handle_stream_registration(
        &self,
        req: ProverRegisterRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String) {
        let max_connections = self.config.coordinator.max_total_provers as usize;
        if self.provers_pool.num_provers().await >= max_connections {
            return (
                false,
                format!("Maximum concurrent connections reached: ({})", max_connections),
            );
        }

        match self
            .provers_pool
            .register_prover(req.prover_id, req.compute_capacity, msg_sender)
            .await
        {
            Ok(()) => (true, "Registration successful".to_string()),
            Err(e) => (false, format!("Registration failed: {e}")),
        }
    }

    /// Handle reconnection directly in stream context
    pub async fn handle_stream_reconnection(
        &self,
        req: ProverReconnectRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String) {
        match self
            .provers_pool
            .reconnect_prover(req.prover_id, req.compute_capacity, msg_sender)
            .await
        {
            Ok(()) => (true, "Reconnection successful".to_string()),
            Err(e) => (false, format!("Reconnection failed: {e}")),
        }
    }

    /// Unregister a prover by its ID
    pub async fn unregister_prover(&self, prover_id: &ProverId) -> Result<()> {
        Ok(self.provers_pool.unregister_prover(prover_id).await?)
    }

    pub async fn handle_stream_heartbeat_ack(&self, message: HeartbeatAckDto) -> Result<()> {
        self.provers_pool
            .update_last_heartbeat(&message.prover_id)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn handle_stream_error(&self, message: ProverErrorDto) -> Result<()> {
        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&message.prover_id).await?;

        error!("Prover {} error: {}", message.prover_id, message.error_message);

        self.fail_job(&message.job_id, message.error_message).await.map_err(|e| {
            error!("Failed to mark job {} as failed after prover error: {}", message.job_id, e);
            e
        })?;

        Ok(())
    }

    pub async fn handle_stream_execute_task_response(
        &self,
        message: ExecuteTaskResponseDto,
    ) -> Result<()> {
        // Validate and update heartbeat
        self.validate_and_update_heartbeat(&message).await?;

        // Handle task failure if needed
        if !message.success {
            return self.handle_task_failure(message).await;
        }

        match message.result_data {
            ExecuteTaskResponseResultDataDto::Challenges(_) => {
                self.handle_contributions_completion(message).await
            }
            ExecuteTaskResponseResultDataDto::Proofs(_) => {
                self.handle_proofs_completion(message).await
            }
            ExecuteTaskResponseResultDataDto::FinalProof(_) => {
                self.handle_aggregation_completion(message).await
            }
        }
    }

    /// Validate request and update prover heartbeat
    async fn validate_and_update_heartbeat(&self, message: &ExecuteTaskResponseDto) -> Result<()> {
        // Update last heartbeat
        self.provers_pool.update_last_heartbeat(&message.prover_id).await?;

        // Check if job exists
        if !self.jobs.contains_key(&message.job_id) {
            warn!(
                "Received ExecuteTaskResponse for unknown job {} from prover {}",
                message.job_id, message.prover_id
            );
            return Err(Error::InvalidRequest(format!("Job {} not found", message.job_id)).into());
        }

        Ok(())
    }

    /// Handle task failure by failing the job and returning appropriate error
    async fn handle_task_failure(&self, message: ExecuteTaskResponseDto) -> Result<()> {
        self.fail_job(&message.job_id, "Task execution failed").await.map_err(|e| {
            error!("Failed to mark job {} as failed: {}", message.job_id, e);
            e
        })?;

        Err(Error::Service(format!(
            "Prover {} failed to execute task for {}: {}",
            message.prover_id,
            message.job_id,
            message.error_message.unwrap_or_default()
        ))
        .into())
    }

    /// Handle Phase1 result and check if we can proceed to Phase2
    pub async fn handle_contributions_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let job_id = execute_task_response.job_id.clone();

        let job_entry = self
            .jobs
            .get(&job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let mut job = job_entry.write().await;

        // Store Contributions response
        self.store_contribution_response(&mut job, execute_task_response).await?;

        // Check if all contributions are complete
        if !self.check_phase1_completion(&job).await? {
            return Ok(());
        }

        // Validate and extract challenges in a single operation to minimize lock time
        let challenges = self.validate_and_extract_challenges(&job).await?;

        // Update job state to Phase2
        job.challenges = Some(challenges);
        job.change_state(JobState::Running(JobPhase::Prove));

        let challenges_dto = self.collect_challenges_dto(&job);

        let active_provers = self.select_provers_for_execution(&job)?;

        drop(job); // Release jobs lock early

        // Start Phase2 for all provers
        self.start_prove(&job_id, &active_provers, challenges_dto).await?;

        info!("Successfully started Phase2 for {} with {} provers", job_id, active_provers.len());

        Ok(())
    }

    /// Store Phase1 contribution result from a prover
    async fn store_contribution_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let contributions_results = job.results.entry(JobPhase::Contributions).or_default();

        let prover_id = execute_task_response.prover_id.clone();

        // Check for duplicate results
        if contributions_results.contains_key(&prover_id) {
            warn!(
                "Received duplicate Contribution result from prover {prover_id} for {}",
                job.job_id
            );
            return Err(Error::InvalidRequest(format!(
                "Duplicate Contribution result from prover {prover_id} for {}",
                job.job_id
            ))
            .into());
        }

        let data = self.extract_challenges_data(execute_task_response.result_data)?;

        contributions_results
            .insert(prover_id.clone(), JobResult { success: execute_task_response.success, data });

        Ok(())
    }

    /// Extract and validate challenges data from Contribution response
    fn extract_challenges_data(
        &self,
        result_data: ExecuteTaskResponseResultDataDto,
    ) -> Result<JobResultData> {
        match result_data {
            ExecuteTaskResponseResultDataDto::Challenges(challenges) => {
                if challenges.is_empty() {
                    return Err(Error::InvalidRequest(
                        "Received empty Challenges result data".to_string(),
                    )
                    .into());
                }

                let cont: Result<Vec<ContributionsInfo>, Error> = challenges
                    .into_iter()
                    .map(|challenge| {
                        let challenge_array = challenge.challenge.try_into().map_err(|_| {
                            Error::InvalidRequest("Challenge length mismatch".to_string())
                        })?;

                        Ok(ContributionsInfo {
                            worker_index: challenge.worker_index,
                            airgroup_id: challenge.airgroup_id as usize,
                            challenge: challenge_array,
                        })
                    })
                    .collect();

                let cont = cont.map_err(anyhow::Error::from)?;
                Ok(JobResultData::Challenges(cont))
            }
            _ => {
                Err(Error::InvalidRequest("Expected Challenges result data for Phase1".to_string())
                    .into())
            }
        }
    }

    /// Check if all Phase1 contributions are complete for a job
    async fn check_phase1_completion(&self, job: &Job) -> Result<bool> {
        let phase1_results_len =
            job.results.get(&JobPhase::Contributions).map(|r| r.len()).unwrap_or(0);

        info!(
            "Phase1 progress for {}: {}/{} provers completed",
            job.job_id,
            phase1_results_len,
            job.provers.len()
        );

        // Ensure we have results from all assigned provers before proceeding.
        // If not all provers have responded (and we're not in simulation mode),
        // return early and wait for more results.
        if !job.execution_mode.is_simulating() && phase1_results_len < job.provers.len() {
            return Ok(false);
        }
        Ok(true)
    }

    /// Validate Phase1 results and extract challenges in a single operation
    async fn validate_and_extract_challenges(&self, job: &Job) -> Result<Vec<ContributionsInfo>> {
        // Extract data we need while minimizing lock time
        let (simulating, phase1_results) = {
            let empty_results = HashMap::new();
            let phase1_results =
                job.results.get(&JobPhase::Contributions).unwrap_or(&empty_results).clone();
            let simulating = job.execution_mode.is_simulating();

            (simulating, phase1_results)
        };

        // Validate all results are successful
        let all_successful =
            if simulating { true } else { phase1_results.values().all(|result| result.success) };

        if !all_successful {
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

            let reason =
                format!("Phase1 failed for provers: {failed_provers:?} in job {}", job.job_id);
            self.fail_job(&job.job_id, &reason).await?;

            return Err(Error::Service(reason).into());
        }

        // Extract and prepare challenges
        let challenges: Vec<ContributionsInfo> = if simulating {
            // If we are simulating the execution of N nodes but not actually running them
            // we just repeat the same challenges for all provers to simplify the logic
            let first_challenges = match phase1_results.values().next().unwrap().data {
                JobResultData::Challenges(ref values) => values,
                _ => unreachable!("Expected Challenges data in Phase1 results"),
            };

            vec![first_challenges.clone(); phase1_results.len()].into_iter().flatten().collect()
        } else {
            let challenges: Vec<Vec<ContributionsInfo>> = phase1_results
                .values()
                .map(|results| match &results.data {
                    JobResultData::Challenges(values) => values.clone(),
                    _ => unreachable!("Expected Challenges data in Phase1 results"),
                })
                .collect();

            challenges.into_iter().flatten().collect()
        };

        Ok(challenges)
    }

    fn collect_challenges_dto(&self, job: &Job) -> Vec<ChallengesDto> {
        let mut challenges_dto = Vec::new();

        for challenge in job.challenges.as_ref().unwrap() {
            challenges_dto.push(ChallengesDto {
                worker_index: challenge.worker_index,
                airgroup_id: challenge.airgroup_id as u32,
                challenge: challenge.challenge.to_vec(),
            })
        }

        challenges_dto
    }

    /// Start Phase2 for all provers that completed Phase1
    async fn start_prove(
        &self,
        job_id: &JobId,
        active_provers: &[ProverId],
        ch: Vec<ChallengesDto>,
    ) -> Result<()> {
        // Send messages to active provers
        for prover_id in active_provers {
            if let Some(prover_state) = self.provers_pool.prover_state(prover_id).await {
                // Prover should still be in Working status from Phase1
                if !matches!(prover_state, ProverState::Computing(JobPhase::Contributions)) {
                    let reason =
                        format!("Prover {prover_id} is not in computing state for {}", job_id);
                    return Err(Error::InvalidRequest(reason).into());
                }

                self.provers_pool
                    .mark_prover_with_state(prover_id, ProverState::Computing(JobPhase::Prove))
                    .await?;

                let req = ExecuteTaskRequestDto {
                    prover_id: prover_id.clone(),
                    job_id: job_id.clone(),
                    params: ExecuteTaskRequestTypeDto::ProveParams(ProveParamsDto {
                        challenges: ch.clone(),
                    }),
                };
                let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

                // Send start prove message
                self.provers_pool.send_message(prover_id, req).await?;
            } else {
                warn!("Prover {} not found when starting Phase2", prover_id);
                return Err(Error::InvalidRequest(format!(
                    "Prover {prover_id} not found when starting Phase2"
                ))
                .into());
            }
        }

        Ok(())
    }

    /// Handle Phase2 result and check if the job is complete
    async fn handle_proofs_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let job_id = execute_task_response.job_id.clone();
        let prover_id = execute_task_response.prover_id.clone();

        let job_entry = self
            .jobs
            .get(&job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let mut job = job_entry.write().await;

        // If in simulation mode, complete the job
        if job.execution_mode.is_simulating() {
            return self.complete_simulated_job(&mut job).await;
        }

        // Store Proof response
        self.store_proof_response(&mut job, execute_task_response).await?;

        // Assign aggregator prover if not already assigned
        let agg_prover = self.ensure_aggregator_assigned(&mut job, &prover_id).await?;

        let all_done = self.check_phase2_completion(&job).await?;

        let proofs = self.collect_prover_proofs(&job, &agg_prover, &prover_id).await?;

        drop(job); // Release jobs lock early

        self.send_aggregation_task(&job_id, &agg_prover, proofs, all_done).await?;

        Ok(())
    }

    /// Store Phase2 proof result from a prover
    async fn store_proof_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let job_id = execute_task_response.job_id;
        let prover_id = execute_task_response.prover_id;

        let phase2_results = job.results.entry(JobPhase::Prove).or_default();

        // Check for duplicate results
        if phase2_results.contains_key(&prover_id) {
            let msg =
                format!("Received duplicate Proof result from prover {} for {}", prover_id, job_id);
            warn!(msg);
            return Err(Error::InvalidRequest(msg).into());
        }

        // Extract and validate proofs data from Phase2 response
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

        Ok(())
    }

    /// Finish a simulated job by marking it as completed and freeing provers
    async fn complete_simulated_job(&self, job: &mut Job) -> Result<()> {
        job.change_state(JobState::Completed);

        let assigned_provers = job.provers.clone();

        // Reset prover statuses back to Idle
        self.provers_pool.mark_provers_with_state(&assigned_provers, ProverState::Idle).await?;

        info!(
            "Completed simulated job {} and freed {} provers",
            job.job_id,
            assigned_provers.len()
        );

        Ok(())
    }

    /// Assign if it's possible the aggregator prover for Phase3
    async fn ensure_aggregator_assigned(
        &self,
        job: &mut Job,
        prover_id: &ProverId,
    ) -> Result<ProverId> {
        // The first prover that completes Phase2 becomes the aggregator
        let agg_prover = if job.agg_prover.is_none() {
            job.agg_prover = Some(prover_id.clone());
            job.change_state(JobState::Running(JobPhase::Aggregate));

            self.provers_pool
                .mark_prover_with_state(prover_id, ProverState::Computing(JobPhase::Aggregate))
                .await?;

            prover_id.clone()
        } else {
            // The prover_id is not the aggregator, mark it as Idle
            self.provers_pool.mark_prover_with_state(prover_id, ProverState::Idle).await?;

            job.agg_prover.as_ref().unwrap().clone()
        };

        Ok(agg_prover)
    }

    async fn check_phase2_completion(&self, job: &Job) -> Result<bool> {
        let empty_results = HashMap::new();
        let phase2_results = job.results.get(&JobPhase::Prove).unwrap_or(&empty_results);

        info!(
            "Phase2 progress for {}: {}/{} provers completed",
            job.job_id,
            phase2_results.len(),
            job.provers.len()
        );

        // Ensure we have results from all assigned provers before proceeding.
        // If not all provers have responded, return early and wait for more results.
        if phase2_results.len() < job.provers.len() {
            return Ok(false);
        }

        let all_successful = phase2_results.values().all(|result| result.success);

        if !all_successful {
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

            let reason =
                format!("Phase2 failed for provers {:?} in job {}", failed_provers, job.job_id);
            self.fail_job(&job.job_id, reason).await?;

            return Err(Error::Service("Phase2 failed".to_string()).into());
        }

        Ok(true)
    }

    /// Start Phase3 for all provers that completed Phase2
    async fn collect_prover_proofs(
        &self,
        job: &Job,
        agg_prover_id: &ProverId,
        prover_id: &ProverId,
    ) -> Result<Vec<AggProofData>> {
        Ok(if prover_id == agg_prover_id {
            vec![]
        } else {
            let job_results = job.results.get(&JobPhase::Prove).unwrap();

            let job_result = job_results.get(prover_id).ok_or_else(|| {
                Error::InvalidRequest(format!(
                    "Prover {prover_id} has not completed Phase2 for {}",
                    job.job_id
                ))
            })?;

            match &job_result.data {
                JobResultData::AggProofs(values) => values.clone(),
                _ => {
                    return Err(Error::InvalidRequest(
                        "Expected AggProofs data for Phase2".to_string(),
                    )
                    .into());
                }
            }
        })
    }

    async fn send_aggregation_task(
        &self,
        job_id: &JobId,
        agg_prover_id: &ProverId,
        proofs: Vec<AggProofData>,
        all_done: bool,
    ) -> Result<(), anyhow::Error> {
        let proofs: Vec<ProofDto> = proofs
            .into_iter()
            .map(|p| ProofDto {
                airgroup_id: p.airgroup_id,
                values: p.values,
                worker_idx: p.worker_idx,
            })
            .collect();

        let req = ExecuteTaskRequestDto {
            prover_id: agg_prover_id.clone(),
            job_id: job_id.clone(),
            params: ExecuteTaskRequestTypeDto::AggParams(AggParamsDto {
                agg_proofs: proofs,
                last_proof: all_done,
                final_proof: all_done,
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

        let message = CoordinatorMessageDto::ExecuteTaskRequest(req);

        self.provers_pool.send_message(agg_prover_id, message).await?;

        Ok(())
    }

    /// Handle Contributions result and check if the job is complete
    async fn handle_aggregation_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> Result<()> {
        let job_id = &execute_task_response.job_id;

        // An aggregation request has failed, fail the job
        if !execute_task_response.success {
            let reason = format!("Aggregation failed in job {}", job_id);
            self.fail_job(job_id, &reason).await?;

            return Err(Error::Service(reason).into());
        }

        // Extract the proof data
        let mut proof_data = match execute_task_response.result_data {
            ExecuteTaskResponseResultDataDto::FinalProof(final_proof) => final_proof,
            _ => {
                return Err(Error::InvalidRequest(
                    "Expected FinalProof result data for Aggregation".to_string(),
                )
                .into());
            }
        };

        // Check if the final proof has no values.
        // An empty proof means this was not the last aggregation step,
        // so we need to wait for additional results to complete the job.
        if proof_data.is_empty() {
            return Ok(());
        }

        let job_entry = self
            .jobs
            .get(job_id)
            .ok_or_else(|| Error::InvalidRequest(format!("Job {job_id} not found")))?;

        let mut job = job_entry.write().await;

        // Mark the aggregation prover as Idle
        self.provers_pool
            .mark_prover_with_state(job.agg_prover.as_ref().unwrap(), ProverState::Idle)
            .await?;

        // Finalize completed job
        job.final_proof = Some(proof_data.swap_remove(0));
        job.change_state(JobState::Completed);

        drop(job);

        info!("Job completed successfully {}", job_id);

        self.post_launch_proof(job_id).await?;

        Ok(())
    }
}
