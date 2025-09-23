//! # Coordinator Service
//!
//! The `CoordinatorService` is the core orchestration component of the distributed proof generation system.
//! It manages the entire lifecycle of proof jobs, from initial request validation through multi-phase
//! execution coordination to final proof aggregation.
//!
//! ## Architecture Overview
//!
//! The coordinator implements a three-phase proof generation workflow:
//!
//! ### Phase 1: Contributions (Challenge Generation)
//! - Distributes computation across selected workers based on capacity requirements
//! - Each worker generates cryptographic challenges for their assigned work partition
//!
//! ### Phase 2: Prove (Partial Proofs Generation)  
//! - Uses challenges from Phase 1 to generate individual proofs
//! - Each worker works on their designated portion of the overall proof
//!
//! ### Phase 3: Aggregate (Final Proof Assembly)
//! - Selects a single aggregator worker for the final phase (the first worker to finish its partial proof)
//! - Combines all individual proofs into a single final proof
//! - Triggers completion webhooks and cleanup processes
//!
//! ## Key Responsibilities
//!
//! - **Job Lifecycle Management**: Creating, tracking, and completing proof generation jobs
//! - **Worker Pool Coordination**: Managing worker registration, capacity allocation, and state tracking
//! - **Task Distribution**: Orchestrating work distribution across multiple computation phases
//! - **Error Handling & Recovery**: Managing failures, timeouts, and worker disconnections
//! - **Status Reporting**: Providing real-time system and job status information
//! - **Simulation Support**: Supporting simulated execution modes for testing and development

use crate::{
    config::Config,
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    hooks, WorkersPool,
};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use proofman::ContributionsInfo;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use zisk_distributed_common::{
    AggParamsDto, AggProofData, BlockId, ChallengesDto, ComputeCapacity, ContributionParamsDto,
    CoordinatorMessageDto, ExecuteTaskRequestDto, ExecuteTaskRequestTypeDto,
    ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, HeartbeatAckDto, Job,
    JobExecutionMode, JobId, JobPhase, JobResult, JobResultData, JobState, JobStatusDto,
    JobsListDto, LaunchProofRequestDto, LaunchProofResponseDto, MetricsDto, ProofDto,
    ProveParamsDto, StatusInfoDto, SystemStatusDto, WorkerErrorDto, WorkerId,
    WorkerReconnectRequestDto, WorkerRegisterRequestDto, WorkerState, WorkersListDto,
};

/// Trait for sending messages to workers through various communication channels.
///
/// This trait abstracts the message delivery mechanism, allowing different implementations
/// for various communication protocols (WebSocket, gRPC, etc.). Implementations should
/// be thread-safe (`Send + Sync`).
pub trait MessageSender {
    /// Sends a coordinator message to the connected worker.
    ///
    /// # Parameters
    ///
    /// * `msg` - The message to send, containing task assignments or control commands
    fn send(&self, msg: CoordinatorMessageDto) -> CoordinatorResult<()>;
}

/// The main coordination service for managing distributed proof generation.
///
/// `CoordinatorService` orchestrates the complex multi-phase proof generation workflow
/// across a pool of distributed workers. It maintains the runtime state of the system,
/// tracks job progress, and ensures reliable coordination between all participants.
///
/// # Architecture
///
/// The service operates as a central coordinator that:
/// - Accepts proof generation requests
/// - Manages bidirectional communication with workers via streaming protocols
/// - Tracks job state through three execution phases
/// - Handles worker failures and implements recovery strategies
/// - Provides real-time monitoring and status information
/// - All I/O and coordination logic uses async/await for non-blocking execution
///
/// # Lifecycle Management
///
/// 1. **Initialization**: Service starts with empty job queue and worker pool
/// 2. **Worker Registration**: Workers connect and register their compute capacity
/// 3. **Job Execution**: Proof requests trigger multi-phase job workflows
/// 4. **Cleanup**: Completed jobs trigger webhooks and resource cleanup
pub struct Coordinator {
    /// Configuration settings for the coordinator including server parameters,
    /// logging parameters and coordinator specific settings.
    config: Config,

    /// UTC timestamp when the service instance was started.
    start_time_utc: DateTime<Utc>,

    /// Manages the pool of connected workers and their communication channels.
    workers_pool: WorkersPool,

    /// Concurrent storage for active jobs with fine-grained locking.
    jobs: DashMap<JobId, RwLock<Job>>,
}

impl Coordinator {
    /// Creates a new coordinator service instance with the provided configuration.
    ///
    /// # Parameters
    ///
    /// * `config` - Configuration settings
    pub fn new(config: Config) -> Self {
        let start_time_utc = Utc::now();

        Self { config, start_time_utc, workers_pool: WorkersPool::new(), jobs: DashMap::new() }
    }

    /// Retrieves comprehensive status information about the coordinator service.
    ///
    /// # Returns
    ///
    /// A `StatusInfoDto` containing detailed information about the service name,
    /// version, uptime, and current metrics of the coordinator.
    pub async fn handle_status_info(&self) -> StatusInfoDto {
        let uptime_seconds = (Utc::now() - self.start_time_utc).num_seconds() as u64;

        let metrics =
            MetricsDto { active_connections: self.workers_pool.num_workers().await as u32 };

        StatusInfoDto::new(
            self.config.service.name.clone(),
            self.config.service.version.clone(),
            uptime_seconds,
            self.start_time_utc,
            metrics,
        )
    }

    /// Retrieves a list of currently running proof generation jobs.
    ///
    /// Returns information about all jobs that are running.
    ///
    /// # Returns
    ///
    /// A `JobsListDto` containing an array of job status information including:
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
                    assigned_workers: job.workers.clone(),
                    start_time: job.start_time.timestamp() as u64,
                    duration_ms: job.duration_ms.unwrap_or(0),
                });
            }
        }

        JobsListDto { jobs }
    }

    /// Retrieves information about all registered workers in the system.
    ///
    /// # Returns
    ///
    /// A `WorkersListDto` containing detailed information about each registered worker.
    pub async fn handle_workers_list(&self) -> WorkersListDto {
        self.workers_pool.workers_list().await
    }

    /// Retrieves detailed status information for a specific job.
    ///
    /// # Parameters
    ///
    /// * `job_id` - Unique identifier of the job to query
    ///
    /// # Returns
    ///
    /// On success, returns a JobStatusDto with detailed job status information
    pub async fn handle_job_status(&self, job_id: &JobId) -> CoordinatorResult<JobStatusDto> {
        let job_entry = self.jobs.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
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
            assigned_workers: job.workers.clone(),
            start_time: job.start_time.timestamp() as u64,
            duration_ms: job.duration_ms.unwrap_or(0),
        })
    }

    /// Provides a high-level overview of the entire distributed system status.
    ///
    /// # Returns
    ///
    /// A `SystemStatusDto` containing information about total workers, compute capacity,
    /// idle and busy workers, and active jobs.
    pub async fn handle_system_status(&self) -> SystemStatusDto {
        let total_workers = self.workers_pool.num_workers().await;
        let busy_workers = self.workers_pool.busy_workers().await;

        let mut active_jobs = 0;
        for entry in self.jobs.iter() {
            let job = entry.value().read().await;
            if matches!(job.state(), JobState::Running(_)) {
                active_jobs += 1;
            }
        }

        SystemStatusDto {
            total_workers: total_workers as u32,
            compute_capacity: self.workers_pool.compute_capacity().await,
            idle_workers: self.workers_pool.idle_workers().await as u32,
            busy_workers: busy_workers as u32,
            active_jobs: active_jobs as u32,
        }
    }

    /// Pre-launch validation for proof generation requests.
    ///
    /// Performs a validation of proof generation parameters before
    /// allocating resources or starting the actual proof workflow.
    ///
    /// # Parameters
    ///
    /// * `request` - The proof launch request containing all necessary parameters
    pub fn pre_launch_proof(&self, request: &LaunchProofRequestDto) -> CoordinatorResult<()> {
        // Check if compute_capacity is within allowed limits
        if request.compute_capacity == 0 {
            error!("Invalid requested compute capacity");
            return Err(CoordinatorError::InvalidArgument(
                "Compute capacity must be greater than zero".to_string(),
            ));
        }

        // Check if we have enough capacity to compute the proof is already checked
        // in create_job > partition_and_allocate_by_capacity

        // Check if input_path file exists
        let input_path = PathBuf::from(&request.input_path);
        if !input_path.exists() {
            error!("Input path does not exist: {}", request.input_path);
            return Err(CoordinatorError::InvalidArgument(format!(
                "Input path does not exist: {}",
                request.input_path
            )));
        }

        Ok(())
    }

    /// Initiates a new distributed proof job.
    ///
    /// This is the main entry point for proof generation requests. It orchestrates the complete
    /// workflow from initial validation through resource allocation to phase 1 task distribution.
    /// The method implements a fail-fast approach with comprehensive error handling.
    ///
    /// # Parameters
    ///
    /// * `request` - Complete proof generation request containing:
    ///
    /// # Sucess
    ///
    /// * `LaunchProofResponseDto` - Contains the assigned job ID for tracking
    ///
    /// # Errors
    ///
    /// * `CoordinatorError` - Detailed error information for various failure modes
    ///
    /// # Workflow Overview
    ///
    /// 1. **Pre-launch Validation**: Validates request parameters and system state
    /// 2. **Job Creation**: Allocates workers and creates job with required resources
    /// 3. **State Initialization**: Sets initial job state to Contributions phase
    /// 4. **Worker Selection**: Determines active workers based on execution mode
    /// 5. **Task Distribution**: Sends phase 1 tasks to selected workers
    /// 6. **Response Generation**: Returns job ID for client tracking
    ///
    /// # Simulation Mode
    ///
    /// When `simulated_node` is specified, the system operates in simulation mode
    /// where one worker simulates the work of multiple nodes for testing purposes.
    pub async fn launch_proof(
        &self,
        request: LaunchProofRequestDto,
    ) -> CoordinatorResult<LaunchProofResponseDto> {
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
        let active_workers = self.select_workers_for_execution(&job)?;

        // Store job in jobs map with RwLock
        self.jobs.insert(job_id.clone(), RwLock::new(job));

        // Send Phase1 tasks to selected workers
        if let Some(job_entry) = self.jobs.get(&job_id) {
            let job = job_entry.read().await;
            self.dispatch_contributions_messages(
                request.block_id,
                required_compute_capacity,
                &job,
                &active_workers,
            )
            .await?;
        }

        info!("Successfully started Phase1 for {} with {} workers", job_id, active_workers.len());

        Ok(LaunchProofResponseDto { job_id })
    }

    /// Post-completion processing for proof generation jobs.
    ///
    /// Handles cleanup, notification, and finalization tasks that should occur after
    /// a job completes (successfully or with failure).
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the completed job
    ///
    /// # Webhook Notifications
    ///
    /// If a webhook URL is configured in the coordinator settings, this method will send a POST
    /// request to the webhook endpoint with job results.
    ///
    /// The webhook URL can be specified in two formats:
    ///
    /// - **With a placeholder** — contains `{$job_id}`, which will be replaced with the
    ///   actual job ID at runtime.
    /// - **Without a placeholder** — if the URL does not contain `{$job_id}`, the job ID
    ///   is appended as a path segment.
    ///
    /// If the placeholder is not present, the coordinator automatically
    /// appends `/{job_id}` to the end of the URL.
    ///
    /// Examples:
    ///   coordinator server --webhook-url 'http://example.com/notify?job_id={$job_id}'
    ///   # becomes 'http://example.com/notify?job_id=12345'
    ///   coordinator server --webhook-url 'http://example.com/notify'
    ///   # becomes 'http://example.com/notify/12345'
    pub async fn post_launch_proof(&self, job_id: &JobId) -> CoordinatorResult<()> {
        // Check if webhook URL is configured
        if let Some(webhook_url) = &self.config.coordinator.webhook_url {
            let webhook_url = webhook_url.clone();

            let (final_proof, success) = {
                let job_entry =
                    self.jobs.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
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

    /// Creates a new proof generation job with allocated resources.
    ///
    /// # Parameters
    ///
    /// * `block_id` - Unique identifier for the data block being processed
    /// * `required_compute_capacity` - Computational resources needed for the job
    /// * `input_path` - Filesystem path to the input data
    /// * `simulated_node` - Optional node index for simulation mode
    ///
    /// # Returns
    ///
    /// On success, returns a fully initialized job ready to start proof generation
    pub async fn create_job(
        &self,
        block_id: BlockId,
        required_compute_capacity: ComputeCapacity,
        input_path: String,
        simulated_node: Option<u32>,
    ) -> CoordinatorResult<Job> {
        let execution_mode = if let Some(node) = simulated_node {
            JobExecutionMode::Simulating(node)
        } else {
            JobExecutionMode::Standard
        };

        let (selected_workers, mut partitions) = self
            .workers_pool
            .partition_and_allocate_by_capacity(required_compute_capacity, execution_mode)
            .await?;

        if let Some(simulated_node) = simulated_node {
            partitions[0] = partitions[simulated_node as usize].clone();
        }

        Ok(Job::new(
            block_id,
            PathBuf::from(input_path),
            required_compute_capacity,
            selected_workers,
            partitions,
            execution_mode,
        ))
    }

    /// Selects the active workers for job execution based on the execution mode.
    ///
    /// Determines which workers from the job's allocated worker set should actually
    /// execute tasks. The selection strategy depends on whether the job is running
    /// in standard distributed mode or simulation mode.
    ///
    /// # Parameters
    ///
    /// * `job` - The job containing worker allocations and execution mode
    ///
    /// # Returns
    ///
    /// On success, returns a vector of worker IDs that should receive tasks.
    fn select_workers_for_execution(&self, job: &Job) -> CoordinatorResult<Vec<WorkerId>> {
        let selected_workers = match job.execution_mode {
            // In simulation mode we only use the first worker to simulate the execution of N nodes
            JobExecutionMode::Simulating(simulated_node) => {
                if simulated_node as usize >= job.workers.len() {
                    let msg = format!(
                        "Simulated mode index ({simulated_node}) exceeds available workers ({}).",
                        job.workers.len()
                    );
                    return Err(CoordinatorError::InvalidArgument(msg));
                }

                job.workers[0..1].to_vec()
            }
            // In standard mode use the already selected workers during the job creation
            JobExecutionMode::Standard => job.workers.clone(),
        };

        Ok(selected_workers)
    }

    /// Dispatches Phase 1 (Contributions) tasks to all selected workers.
    ///
    /// Orchestrates the distribution of initial computation tasks across the selected
    /// worker set. Each worker receives a customized task containing their specific
    /// work partition and coordination parameters.
    ///
    /// # Parameters
    ///
    /// * `block_id` - Identifier for the data block being processed
    /// * `required_compute_capacity` - Total computational requirements for the job
    /// * `job` - Job containing partition assignments and configuration
    /// * `active_workers` - List of workers that should receive tasks
    async fn dispatch_contributions_messages(
        &self,
        block_id: BlockId,
        required_compute_capacity: ComputeCapacity,
        job: &Job,
        active_workers: &[WorkerId],
    ) -> CoordinatorResult<()> {
        for (rank_id, worker_id) in active_workers.iter().enumerate() {
            // Create contribution task request
            let req = ExecuteTaskRequestDto {
                worker_id: worker_id.clone(),
                job_id: job.job_id.clone(),
                params: ExecuteTaskRequestTypeDto::ContributionParams(ContributionParamsDto {
                    block_id: block_id.clone(),
                    input_path: job.block.input_path.display().to_string(),
                    rank_id: rank_id as u32,
                    total_workers: active_workers.len() as u32,
                    worker_allocation: job.partitions[rank_id].clone(),
                    job_compute_units: required_compute_capacity,
                }),
            };
            let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

            // Send task to worker
            self.workers_pool.send_message(worker_id, req).await?;

            // Update worker state
            self.workers_pool
                .mark_worker_with_state(worker_id, WorkerState::Computing(JobPhase::Contributions))
                .await?;
        }

        Ok(())
    }

    /// Marks a job as failed and performs and cleans up all associated resources
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the failing job
    /// * `reason` - Human-readable description of the failure cause
    pub async fn fail_job(&self, job_id: &JobId, reason: impl AsRef<str>) -> CoordinatorResult<()> {
        let job_entry = self.jobs.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;
        job.change_state(JobState::Failed);

        // Reset worker statuses back to Idle
        self.workers_pool.mark_workers_with_state(&job.workers, WorkerState::Idle).await?;

        error!(
            "Failed job {} (reason: {}) and freed {} workers",
            job_id,
            reason.as_ref(),
            job.workers.len()
        );

        drop(job); // Release job lock before calling post_launch_proof

        // Add webhook notification for failed jobs
        self.post_launch_proof(job_id).await?;

        Ok(())
    }

    /// Handles new worker registration requests in streaming context.
    ///
    /// Processes incoming worker registration requests and manages the bidirectional
    /// communication channel setup. This method is called directly from stream handlers
    /// to provide immediate registration feedback without additional async coordination.
    ///
    /// # Parameters
    ///
    /// * `req` - Registration request containing worker ID and compute capacity
    /// * `msg_sender` - Communication channel for sending messages back to the worker
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `bool` - Whether registration was successful
    /// - `String` - Success confirmation or detailed error message
    ///
    /// # Registration Process
    ///
    /// 1. **Capacity Check**: Validates against maximum allowed concurrent connections
    /// 2. **Pool Registration**: Attempts to register worker in the active pool
    /// 3. **Channel Setup**: Associates the message sender with the worker ID
    /// 4. **Response Generation**: Returns immediate feedback for the stream handler
    ///
    /// # Connection Limits
    ///
    /// The coordinator enforces a maximum number of concurrent worker connections
    /// (configured via `max_total_workers`) to prevent resource exhaustion and
    /// maintain system stability under load.
    pub async fn handle_stream_registration(
        &self,
        req: WorkerRegisterRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String) {
        let max_connections = self.config.coordinator.max_total_workers as usize;
        if self.workers_pool.num_workers().await >= max_connections {
            return (
                false,
                format!("Maximum concurrent connections reached: ({})", max_connections),
            );
        }

        match self
            .workers_pool
            .register_worker(req.worker_id, req.compute_capacity, msg_sender)
            .await
        {
            Ok(()) => (true, "Registration successful".to_string()),
            Err(e) => (false, format!("Registration failed: {e}")),
        }
    }

    /// Handles worker reconnection requests in streaming context.
    ///
    /// Processes reconnection attempts from workers that have previously registered
    /// but lost their connection due to network issues, service restarts, or other
    /// transient failures. Maintains job continuity where possible.
    ///
    /// # Parameters
    ///
    /// * `req` - Reconnection request containing worker ID and current compute capacity
    /// * `msg_sender` - New communication channel for the reconnected worker
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `bool` - Whether reconnection was successful
    /// - `String` - Success confirmation or detailed error message
    ///
    /// # Reconnection Process
    ///
    /// 1. **Identity Validation**: Verifies the worker was previously registered
    /// 2. **State Recovery**: Attempts to restore worker to its previous operational state
    /// 3. **Channel Update**: Associates the new message sender with the existing worker entry
    /// 4. **Continuation**: Allows ongoing jobs to resume with the reconnected worker
    ///
    /// # State Preservation
    ///
    /// The coordinator attempts to maintain job continuity during reconnections:
    /// - Active job assignments are preserved where possible
    /// - Worker compute capacity can be updated to reflect current capabilities
    /// - Message queues and pending tasks are maintained across disconnections
    ///
    /// # Recovery Scenarios
    ///
    /// Successful reconnection depends on:
    /// - Worker ID matches a previously registered instance
    /// - No conflicting registrations for the same worker ID
    /// - System state consistency allows for safe state restoration
    pub async fn handle_stream_reconnection(
        &self,
        req: WorkerReconnectRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String) {
        match self
            .workers_pool
            .reconnect_worker(req.worker_id, req.compute_capacity, msg_sender)
            .await
        {
            Ok(()) => (true, "Reconnection successful".to_string()),
            Err(e) => (false, format!("Reconnection failed: {e}")),
        }
    }

    /// Removes a worker from the active pool and cleans up associated resources.
    ///
    /// Handles worker disconnection or removal by cleaning up state, reallocating
    /// work if necessary, and ensuring system consistency. This method is typically
    /// called when workers disconnect unexpectedly or during graceful shutdowns.
    ///
    /// # Parameters
    ///
    /// * `worker_id` - Unique identifier of the worker to remove
    ///
    /// # Cleanup Operations
    ///
    /// 1. **State Removal**: Removes worker from active pool and associated data structures
    /// 2. **Job Impact Assessment**: Identifies any active jobs that may be affected
    /// 3. **Resource Reallocation**: May trigger job failure or rebalancing depending on job state
    /// 4. **Connection Cleanup**: Releases communication channels and associated resources
    ///
    /// # Impact on Active Jobs
    ///
    /// When a worker is unregistered:
    /// - Jobs in progress may be marked as failed if the worker was critical
    /// - Work may be redistributed to remaining workers where possible
    /// - Aggregation phases may need to be restarted with different worker assignments
    pub async fn unregister_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        self.workers_pool.unregister_worker(worker_id).await
    }

    /// Handles heartbeat acknowledgments from workers to maintain liveness tracking.
    ///
    /// Updates the last known heartbeat timestamp for the worker.
    ///
    /// # Parameters
    ///
    /// * `message` - Heartbeat acknowledgment message containing worker ID
    pub async fn handle_stream_heartbeat_ack(
        &self,
        message: HeartbeatAckDto,
    ) -> CoordinatorResult<()> {
        self.workers_pool.update_last_heartbeat(&message.worker_id).await
    }

    /// Handles error reports from workers and marks associated jobs as failed.
    ///
    /// # Parameters
    ///
    /// * `message` - Worker error message containing job ID, worker ID, and error details
    pub async fn handle_stream_error(&self, message: WorkerErrorDto) -> CoordinatorResult<()> {
        // Update last heartbeat
        self.workers_pool.update_last_heartbeat(&message.worker_id).await?;

        error!("Worker {} error: {}", message.worker_id, message.error_message);

        self.fail_job(&message.job_id, message.error_message).await.map_err(|e| {
            error!("Failed to mark job {} as failed after worker error: {}", message.job_id, e);
            e
        })?;

        Ok(())
    }

    /// Handles task execution responses from workers and orchestrates job progression.
    ///
    /// # Parameters
    ///
    /// * `message` - Task execution response containing results or failure details
    pub async fn handle_stream_execute_task_response(
        &self,
        message: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
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

    /// Validates incoming task response and updates worker heartbeat.
    ///
    /// # Parameters
    ///
    /// * `message` - The task response message from a worker
    async fn validate_and_update_heartbeat(
        &self,
        message: &ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        // Update last heartbeat
        self.workers_pool.update_last_heartbeat(&message.worker_id).await?;

        // Check if job exists
        if !self.jobs.contains_key(&message.job_id) {
            warn!(
                "Received ExecuteTaskResponse for unknown job {} from worker {}",
                message.job_id, message.worker_id
            );
            return Err(CoordinatorError::NotFoundOrInaccessible);
        }

        Ok(())
    }

    /// Handles task execution failures by failing the job and generating appropriate errors.
    ///
    /// # Parameters
    ///
    /// * `message` - Task response containing failure details and context
    async fn handle_task_failure(&self, message: ExecuteTaskResponseDto) -> CoordinatorResult<()> {
        self.fail_job(&message.job_id, "Task execution failed").await?;

        Err(CoordinatorError::WorkerError(format!(
            "Worker {} failed to execute task for {}: {}",
            message.worker_id,
            message.job_id,
            message.error_message.unwrap_or_default()
        )))
    }

    /// Processes Phase 1 (Contributions) completion and orchestrates transition to Phase 2.
    ///
    /// Handles the coordination required when workers complete their initial
    /// contribution tasks.
    ///
    /// # Parameters
    ///
    /// * `execute_task_response` - Response containing contribution results from a worker
    pub async fn handle_contributions_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id.clone();

        let job_entry = self.jobs.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;

        // Store Contributions response
        self.store_contribution_response(&mut job, execute_task_response).await?;

        // Check if all contributions are complete
        if !self.check_phase1_completion(&job) {
            return Ok(());
        }

        // Validate and extract challenges in a single operation to minimize lock time
        let challenges = self.validate_and_extract_challenges(&job).await?;

        // Update job state to Phase2
        job.challenges = Some(challenges);
        job.change_state(JobState::Running(JobPhase::Prove));

        let challenges_dto = self.collect_challenges_dto(&job);

        let active_workers = self.select_workers_for_execution(&job)?;

        drop(job); // Release jobs lock early

        // Start Phase2 for all workers
        self.start_prove(&job_id, &active_workers, challenges_dto).await?;

        info!("Successfully started Phase2 for {} with {} workers", job_id, active_workers.len());

        Ok(())
    }

    /// Stores a single worker's Contribution response in the job state.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to update
    /// * `execute_task_response` - The response from the worker containing contribution data
    async fn store_contribution_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let contributions_results = job.results.entry(JobPhase::Contributions).or_default();

        let worker_id = execute_task_response.worker_id.clone();

        // Check for duplicate results
        if contributions_results.contains_key(&worker_id) {
            warn!(
                "Received duplicate Contribution result from worker {worker_id} for {}",
                job.job_id
            );
            return Err(CoordinatorError::InvalidRequest(format!(
                "Duplicate Contribution result from worker {worker_id} for {}",
                job.job_id
            )));
        }

        let data = self.extract_challenges_data(execute_task_response.result_data)?;

        contributions_results
            .insert(worker_id.clone(), JobResult { success: execute_task_response.success, data });

        Ok(())
    }

    /// Extracts challenge data from the worker's result response.
    ///
    /// # Parameters
    ///
    /// * `result_data` - The result data from the worker's response
    fn extract_challenges_data(
        &self,
        result_data: ExecuteTaskResponseResultDataDto,
    ) -> CoordinatorResult<JobResultData> {
        match result_data {
            ExecuteTaskResponseResultDataDto::Challenges(challenges) => {
                if challenges.is_empty() {
                    return Err(CoordinatorError::InvalidRequest(
                        "Received empty Challenges result data".to_string(),
                    ));
                }

                let contributions: Vec<ContributionsInfo> = challenges
                    .into_iter()
                    .map(|challenge| ContributionsInfo {
                        worker_index: challenge.worker_index,
                        airgroup_id: challenge.airgroup_id as usize,
                        challenge: challenge.challenge,
                    })
                    .collect();

                Ok(JobResultData::Challenges(contributions))
            }
            _ => Err(CoordinatorError::InvalidRequest(
                "Expected Challenges result data for Phase1".to_string(),
            )),
        }
    }

    /// Checks if all workers have completed Phase 1 contributions.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to check
    fn check_phase1_completion(&self, job: &Job) -> bool {
        let phase1_results_len =
            job.results.get(&JobPhase::Contributions).map(|r| r.len()).unwrap_or(0);

        info!(
            "Phase1 progress for {}: {}/{} workers completed",
            job.job_id,
            phase1_results_len,
            job.workers.len()
        );

        // Ensure we have results from all assigned workers before proceeding.
        // If not all workers have responded (and we're not in simulation mode),
        // return early and wait for more results.
        if !job.execution_mode.is_simulating() && phase1_results_len < job.workers.len() {
            return false;
        }

        true
    }

    /// Validates Phase 1 results and extracts challenge data with simulation mode handling.
    ///
    /// Performs comprehensive validation of all Phase 1 contribution results and extracts
    /// the cryptographic challenges needed for Phase 2 proof generation.
    ///
    /// # Parameters
    ///
    /// * `job` - Job containing all Phase 1 results to validate and process
    async fn validate_and_extract_challenges(
        &self,
        job: &Job,
    ) -> CoordinatorResult<Vec<ContributionsInfo>> {
        // Extract data we need while minimizing lock time
        let (simulating, phase1_results) = {
            let empty_results = HashMap::new();
            let phase1_results =
                job.results.get(&JobPhase::Contributions).unwrap_or(&empty_results).clone();
            let simulating = job.execution_mode.is_simulating();

            (simulating, phase1_results)
        };

        // Validate all results are successful
        // In simulation mode, we assume success since we're not running real distributed computation
        let all_successful =
            if simulating { true } else { phase1_results.values().all(|result| result.success) };

        if !all_successful {
            // Identify specific workers that failed for detailed error reporting
            let failed_workers: Vec<WorkerId> = phase1_results
                .iter()
                .filter_map(
                    |(worker_id, result)| {
                        if !result.success {
                            Some(worker_id.clone())
                        } else {
                            None
                        }
                    },
                )
                .collect();

            let reason =
                format!("Phase1 failed for workers: {failed_workers:?} in job {}", job.job_id);
            self.fail_job(&job.job_id, &reason).await?;

            return Err(CoordinatorError::WorkerError(reason));
        }

        // Extract and prepare challenges based on execution mode
        let challenges: Vec<ContributionsInfo> = if simulating {
            // Simulation mode: replicate single worker's challenges across all expected workers
            // This maintains algorithm correctness while using minimal computational resources
            let first_challenges = match phase1_results.values().next().unwrap().data {
                JobResultData::Challenges(ref values) => values,
                _ => unreachable!("Expected Challenges data in Phase1 results"),
            };

            // Create challenge sets for each simulated worker using the same base challenges
            vec![first_challenges.clone(); phase1_results.len()].into_iter().flatten().collect()
        } else {
            // Standard mode: aggregate challenges from all participating workers
            // Each worker contributes their portion of the overall challenge space
            let challenges: Vec<Vec<ContributionsInfo>> = phase1_results
                .values()
                .map(|results| match &results.data {
                    JobResultData::Challenges(values) => values.clone(),
                    _ => unreachable!("Expected Challenges data in Phase1 results"),
                })
                .collect();

            // Flatten all worker contributions into unified challenge vector
            // Maintains worker indexing and airgroup assignments for proper coordination
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

    /// Initiates Phase 2 (Prove) execution across all selected workers.
    ///
    /// Orchestrates the distribution of proof generation tasks using the challenges
    /// generated in Phase 1. This method ensures all workers receive the complete
    /// challenge set and transition properly to the proof generation phase.
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the job transitioning to Phase 2
    /// * `active_workers` - List of workers that should participate in Phase 2
    /// * `challenges` - Challenges generated from Phase 1 contributions
    async fn start_prove(
        &self,
        job_id: &JobId,
        active_workers: &[WorkerId],
        challenges: Vec<ChallengesDto>,
    ) -> CoordinatorResult<()> {
        // Send messages to active workers
        for worker_id in active_workers {
            if let Some(worker_state) = self.workers_pool.worker_state(worker_id).await {
                // Validate worker is in the expected Phase 1 computing state
                // This ensures proper phase sequencing and prevents race conditions
                if !matches!(worker_state, WorkerState::Computing(JobPhase::Contributions)) {
                    let reason =
                        format!("Worker {worker_id} is not in computing state for {}", job_id);
                    return Err(CoordinatorError::InvalidRequest(reason));
                }

                // Transition worker to Phase 2 computing state
                // This atomic update ensures consistent state tracking across the system
                self.workers_pool
                    .mark_worker_with_state(worker_id, WorkerState::Computing(JobPhase::Prove))
                    .await?;

                // Create Phase 2 task with complete challenge set
                // All workers receive the full challenge data regardless of their individual contributions
                let req = ExecuteTaskRequestDto {
                    worker_id: worker_id.clone(),
                    job_id: job_id.clone(),
                    params: ExecuteTaskRequestTypeDto::ProveParams(ProveParamsDto {
                        challenges: challenges.clone(), // Complete challenge set from Phase 1 aggregation
                    }),
                };
                let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

                // Send start prove message to worker
                // Network failures here will cause the method to fail and require retry logic
                self.workers_pool.send_message(worker_id, req).await?;
            } else {
                // Worker disappeared between Phase 1 completion and Phase 2 start
                // This can happen due to disconnections or system state changes
                warn!("Worker {} not found when starting Phase2", worker_id);
                return Err(CoordinatorError::NotFoundOrInaccessible);
            }
        }

        Ok(())
    }

    /// Processes Phase 2 (Proofs) completion and orchestrates transition to Phase 3.
    ///
    /// Handles the coordination required when workers complete their proof generation tasks.
    ///
    /// # Parameters
    ///
    /// * `execute_task_response` - Response containing proof results from a worker
    async fn handle_proofs_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id.clone();
        let worker_id = execute_task_response.worker_id.clone();

        let job_entry = self.jobs.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        let mut job = job_entry.write().await;

        // If in simulation mode, complete the job
        if job.execution_mode.is_simulating() {
            return self.complete_simulated_job(&mut job).await;
        }

        // Store Proof response
        self.store_proof_response(&mut job, execute_task_response).await?;

        // Assign aggregator worker if not already assigned
        let agg_worker_id = self.resolve_aggregator_assignment(&mut job, &worker_id).await?;

        let all_done = self.check_phase2_completion(&job).await?;

        let proofs = self.collect_worker_proofs(&job, &agg_worker_id, &worker_id)?;

        drop(job); // Release jobs lock early

        self.send_aggregation_task(&job_id, &agg_worker_id, proofs, all_done).await?;

        Ok(())
    }

    /// Stores a single worker's Contribution response in the job state.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to update
    /// * `execute_task_response` - The response from the worker containing proof data
    async fn store_proof_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id;
        let worker_id = execute_task_response.worker_id;

        let phase2_results = job.results.entry(JobPhase::Prove).or_default();

        // Check for duplicate results
        if phase2_results.contains_key(&worker_id) {
            let msg =
                format!("Received duplicate Proof result from worker {} for {}", worker_id, job_id);
            warn!(msg);
            return Err(CoordinatorError::InvalidRequest(msg));
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
                return Err(CoordinatorError::InvalidRequest(
                    "Expected Proofs result data for Phase2".to_string(),
                ));
            }
        };

        phase2_results
            .insert(worker_id.clone(), JobResult { success: execute_task_response.success, data });

        Ok(())
    }

    /// Completes a simulated job by marking it as completed and freeing resources.
    ///
    /// # Parameters
    ///
    /// * `job` - Mutable reference to job for state updates
    async fn complete_simulated_job(&self, job: &mut Job) -> CoordinatorResult<()> {
        job.change_state(JobState::Completed);

        let assigned_workers = job.workers.clone();

        // Reset worker statuses back to Idle
        self.workers_pool.mark_workers_with_state(&assigned_workers, WorkerState::Idle).await?;

        info!(
            "Completed simulated job {} and freed {} workers",
            job.job_id,
            assigned_workers.len()
        );

        Ok(())
    }

    /// Determines aggregator assignment and manages worker state transitions for Phase 3.
    ///
    /// # Parameters
    ///
    /// * `job` - Mutable reference to job for state updates
    /// * `candidate_worker_id` - Worker that just completed Phase 2 and could become aggregator
    ///
    /// # Returns
    ///
    /// * The worker ID of the worker assigned as aggregator
    ///
    /// # Aggregator Selection Strategy
    ///
    /// The system uses a "first-to-complete" aggregator selection approach, so the first worker
    /// to complete Phase 2 becomes the aggregator
    async fn resolve_aggregator_assignment(
        &self,
        job: &mut Job,
        candidate_worker_id: &WorkerId,
    ) -> CoordinatorResult<WorkerId> {
        match job.agg_worker_id.as_ref() {
            Some(existing_aggregator_id) => {
                // Aggregator already exists - mark the candidate as idle since it's not the aggregator
                // This immediately frees up the worker's resources for other jobs
                self.workers_pool
                    .mark_worker_with_state(candidate_worker_id, WorkerState::Idle)
                    .await?;
                Ok(existing_aggregator_id.clone())
            }
            None => {
                // No aggregator yet - assign the candidate as aggregator
                // This represents the first worker to complete Phase 2, implementing "first-wins" selection
                job.agg_worker_id = Some(candidate_worker_id.clone());
                job.change_state(JobState::Running(JobPhase::Aggregate));

                // Update worker state
                self.workers_pool
                    .mark_worker_with_state(
                        candidate_worker_id,
                        WorkerState::Computing(JobPhase::Aggregate),
                    )
                    .await?;

                info!(
                    "Assigned worker {} as aggregator for job {}",
                    candidate_worker_id, job.job_id
                );

                Ok(candidate_worker_id.clone())
            }
        }
    }

    /// Checks if all workers have completed Phase 2 proofs and validates success.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to check
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - All workers completed successfully, ready for aggregation
    /// * `Ok(false)` - Still waiting for more workers to complete
    ///
    /// # Completion Criteria
    ///
    /// Phase 2 is considered complete when:
    /// - All assigned workers have submitted proof results
    /// - All submitted proofs report successful generation
    async fn check_phase2_completion(&self, job: &Job) -> CoordinatorResult<bool> {
        let empty_results = HashMap::new();
        let phase2_results = job.results.get(&JobPhase::Prove).unwrap_or(&empty_results);

        // Provide operational visibility into Phase 2 progress
        // This logging helps with monitoring long-running proof generation jobs
        info!(
            "Phase2 progress for {}: {}/{} workers completed",
            job.job_id,
            phase2_results.len(),
            job.workers.len()
        );

        // Check if all assigned workers have completed their proof generation
        // Early return allows other workers to continue working while we wait
        if phase2_results.len() < job.workers.len() {
            return Ok(false);
        }

        // Validate that all completed proofs are successful
        // Any failure triggers job-level failure to prevent invalid aggregation
        let all_successful = phase2_results.values().all(|result| result.success);

        if !all_successful {
            // Build comprehensive failure report identifying all failed workers
            // This detailed error context helps with debugging and system improvement
            let failed_workers: Vec<WorkerId> = phase2_results
                .iter()
                .filter_map(
                    |(worker_id, result)| {
                        if !result.success {
                            Some(worker_id.clone())
                        } else {
                            None
                        }
                    },
                )
                .collect();

            // Trigger job failure with detailed context about which workers failed
            let reason =
                format!("Phase2 failed for workers {:?} in job {}", failed_workers, job.job_id);
            self.fail_job(&job.job_id, reason).await?;

            // Returns error to prevent further processing of this failed job
            return Err(CoordinatorError::Internal("Phase2 failed".to_string()));
        }

        Ok(true)
    }

    /// Collects the proofs stored from a worker for aggregation.
    ///     
    /// # Parameters
    ///
    /// * `job` - Reference to the job containing proof results
    /// * `agg_worker_id` - Worker ID assigned as the aggregator
    /// * `worker_id` - Worker ID whose proofs are being collected
    fn collect_worker_proofs(
        &self,
        job: &Job,
        agg_worker_id: &WorkerId,
        worker_id: &WorkerId,
    ) -> CoordinatorResult<Vec<AggProofData>> {
        Ok(if worker_id == agg_worker_id {
            vec![]
        } else {
            let job_results = job.results.get(&JobPhase::Prove).unwrap();

            let job_result = job_results.get(worker_id).ok_or(CoordinatorError::InvalidRequest(
                format!("Worker {worker_id} has not completed Phase2 for {}", job.job_id),
            ))?;

            match &job_result.data {
                JobResultData::AggProofs(values) => values.clone(),
                _ => {
                    return Err(CoordinatorError::InvalidRequest(
                        "Expected AggProofs data for Phase2".to_string(),
                    ));
                }
            }
        })
    }

    /// Sends an aggregation task to the designated aggregator worker.
    ///    
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the job being processed
    /// * `agg_worker_id` - Worker ID assigned as the aggregator
    /// * `proofs` - List of proofs to aggregate
    /// * `all_done` - Indicates if this is the final aggregation step
    async fn send_aggregation_task(
        &self,
        job_id: &JobId,
        agg_worker_id: &WorkerId,
        proofs: Vec<AggProofData>,
        all_done: bool,
    ) -> CoordinatorResult<()> {
        let proofs: Vec<ProofDto> = proofs
            .into_iter()
            .map(|p| ProofDto {
                airgroup_id: p.airgroup_id,
                values: p.values,
                worker_idx: p.worker_idx,
            })
            .collect();

        let req = ExecuteTaskRequestDto {
            worker_id: agg_worker_id.clone(),
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

        self.workers_pool.send_message(agg_worker_id, message).await?;

        Ok(())
    }

    /// Handles aggregation completion, finalizes the job if all steps are done.
    ///    
    /// # Parameters
    ///
    /// * `execute_task_response` - Response containing final proof or failure details
    async fn handle_aggregation_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = &execute_task_response.job_id;

        // An aggregation request has failed, fail the job
        if !execute_task_response.success {
            let reason = format!("Aggregation failed in job {}", job_id);
            self.fail_job(job_id, &reason).await?;

            return Err(CoordinatorError::Internal(reason));
        }

        // Extract the proof data
        let mut proof_data = match execute_task_response.result_data {
            ExecuteTaskResponseResultDataDto::FinalProof(final_proof) => final_proof,
            _ => {
                return Err(CoordinatorError::InvalidRequest(
                    "Expected FinalProof result data for Aggregation".to_string(),
                ));
            }
        };

        // Check if the final proof has no values.
        // An empty proof means this was not the last aggregation step,
        // so we need to wait for additional results to complete the job.
        if proof_data.is_empty() {
            return Ok(());
        }

        let job_entry = self.jobs.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;

        // Mark the aggregation worker as Idle
        self.workers_pool
            .mark_worker_with_state(job.agg_worker_id.as_ref().unwrap(), WorkerState::Idle)
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
