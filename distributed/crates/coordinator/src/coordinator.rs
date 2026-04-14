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
    hooks,
    job_events::{CoordinatorExecutionStats, CoordinatorJobEvent, CoordinatorJobResult},
    PrecompileHintsRelay, WorkersPool,
};

use chrono::{DateTime, Utc};
use colored::Colorize;
use proofman::{ContributionsInfo, WitnessInfo};
use std::{
    collections::HashMap,
    fs,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use zisk_common::io::{StreamSource, ZiskStream};
use zisk_common::AsmExecutionInfo;
use zisk_common::ZiskExecutorTime;
use zisk_common::ZiskProofWithPublicValues;
use zisk_distributed_common::{
    AggParamsDto, AggProofData, ChallengesDto, ComputeCapacity, ContributionParamsDto,
    ContributionsResult, CoordinatorMessageDto, DataId, ExecuteTaskRequestDto,
    ExecuteTaskRequestTypeDto, ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto,
    ExecutionResult, HeartbeatAckDto, HintsModeDto, HintsSourceDto, InputSourceDto, InputsModeDto,
    Job, JobExecutionMode, JobId, JobPhase, JobResult, JobResultData, JobState, JobStatusDto,
    JobsListDto, LaunchProofRequestDto, LaunchProofResponseDto, MetricsDto, PhaseTimings, ProofDto,
    ProveParamsDto, ReconnectionDirectiveDto, SetupProgramAckDto, StatusInfoDto, StreamMessageKind,
    SystemStatusDto, WorkerErrorDto, WorkerId, WorkerReconnectRequestDto, WorkerRegisterRequestDto,
    WorkerState, WorkersListDto, ZiskExecutorTimeDto,
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
    workers_pool: Arc<WorkersPool>,

    /// Concurrent storage for active jobs.
    jobs: RwLock<HashMap<JobId, Arc<RwLock<Job>>>>,

    /// Number of registrations accumulated.
    registrations: AtomicU64,

    /// Number of reconnections accumulated.
    reconnections: AtomicU64,

    /// Per-job event broadcast channels. Populated on job creation, cleaned up on terminal state.
    job_events: RwLock<HashMap<JobId, broadcast::Sender<CoordinatorJobEvent>>>,
}

fn exec_stats_from_job(job: &Job) -> CoordinatorExecutionStats {
    CoordinatorExecutionStats {
        steps: job.executed_steps.unwrap_or(0),
        duration_nanos: job.duration_ms.unwrap_or(0).saturating_mul(1_000_000),
        ..Default::default()
    }
}

impl Coordinator {
    /// Creates a new coordinator service instance with the provided configuration.
    ///
    /// # Parameters
    ///
    /// * `config` - Configuration settings
    pub fn new(config: Config) -> Self {
        let start_time_utc = Utc::now();

        Self {
            config,
            start_time_utc,
            workers_pool: Arc::new(WorkersPool::new()),
            jobs: RwLock::new(HashMap::new()),
            registrations: AtomicU64::new(0),
            reconnections: AtomicU64::new(0),
            job_events: RwLock::new(HashMap::new()),
        }
    }

    /// Returns a reference to the workers pool.
    pub fn workers_pool(&self) -> &WorkersPool {
        &self.workers_pool
    }

    /// Returns a reference to the jobs map.
    pub fn jobs(&self) -> &RwLock<HashMap<JobId, Arc<RwLock<Job>>>> {
        &self.jobs
    }

    /// Returns a reference to the coordinator config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    // -------------------------------------------------------------------------
    // Job event broadcast helpers
    // -------------------------------------------------------------------------

    /// Allocates a broadcast channel for the given job. Must be called before any event is fired.
    async fn alloc_job_events(&self, job_id: &JobId) {
        let (tx, _) = broadcast::channel(64);
        self.job_events.write().await.insert(job_id.clone(), tx);
    }

    /// Returns a live receiver for the job's event channel, or `None` if the job is unknown.
    pub async fn subscribe_job_events(
        &self,
        job_id: &JobId,
    ) -> Option<broadcast::Receiver<CoordinatorJobEvent>> {
        self.job_events.read().await.get(job_id).map(|tx| tx.subscribe())
    }

    /// Fires an event on the job's channel. Drops silently when there are no receivers.
    /// Removes the channel from the map after a terminal event.
    async fn fire_job_event(&self, job_id: &JobId, event: CoordinatorJobEvent) {
        let terminal = matches!(
            event,
            CoordinatorJobEvent::Completed(_)
                | CoordinatorJobEvent::Failed(_)
                | CoordinatorJobEvent::Cancelled
        );

        {
            let map = self.job_events.read().await;
            if let Some(tx) = map.get(job_id) {
                // send() only errors when there are no receivers — safe to ignore
                let _ = tx.send(event);
            }
        }

        if terminal {
            self.job_events.write().await.remove(job_id);
        }
    }

    // -------------------------------------------------------------------------
    // cancel_job
    // -------------------------------------------------------------------------

    /// Cancels a running or queued job.
    ///
    /// Returns `true` if the job was cancelled, `false` if it was already in a terminal state.
    pub async fn cancel_job(&self, job_id: &JobId) -> CoordinatorResult<bool> {
        let jobs_map = self.jobs.read().await;
        let job_entry =
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        drop(jobs_map);

        let worker_ids = {
            let mut job = job_entry.write().await;
            if job.state().is_resolved() {
                return Ok(false);
            }
            job.change_state(JobState::Cancelled);
            job.workers.clone()
        };

        self.cancel_job_workers(&worker_ids, job_id, "cancelled by client").await;
        self.ensure_workers_idle(&worker_ids).await;

        self.fire_job_event(job_id, CoordinatorJobEvent::Cancelled).await;

        info!("Cancelled job {}", job_id);

        Ok(true)
    }

    /// Content-addresses ELF bytes with blake3, writes to cache if absent, returns `hash_id`.
    pub fn register_guest_program(&self, elf_bytes: Vec<u8>) -> CoordinatorResult<String> {
        use blake3::Hasher;
        use zisk_distributed_common::elf_cache_path;

        let mut hasher = Hasher::new();
        hasher.update(&elf_bytes);
        let hash_id = hasher.finalize().to_hex().to_string();

        let path = elf_cache_path(&hash_id);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| CoordinatorError::Internal(format!("create cache dir: {e}")))?;
            }
            fs::write(&path, &elf_bytes)
                .map_err(|e| CoordinatorError::Internal(format!("write ELF cache: {e}")))?;
        }

        Ok(hash_id)
    }

    /// Reads the cached ELF for `hash_id` and broadcasts `SetupProgram` to all connected workers.
    /// Returns a synthetic `JobId` (setup tracking is async via acks).
    pub async fn setup_program(&self, hash_id: &str) -> CoordinatorResult<JobId> {
        use zisk_distributed_common::{elf_cache_path, SetupProgramDto};

        let path = elf_cache_path(hash_id);
        let elf_bytes = fs::read(&path).map_err(|e| {
            CoordinatorError::Internal(format!("ELF not found for hash_id {hash_id}: {e}"))
        })?;

        let job_id = JobId::new();
        let workers = self.workers_pool.connected_worker_ids().await;
        for worker_id in &workers {
            let msg = CoordinatorMessageDto::SetupProgram(SetupProgramDto {
                job_id: job_id.as_string(),
                elf_bytes: elf_bytes.clone(),
                hash_id: hash_id.to_string(),
            });
            if let Err(e) = self.workers_pool.send_message(worker_id, msg).await {
                warn!("[Setup] Failed to send SetupProgram to worker {}: {}", worker_id, e);
            }
        }

        Ok(job_id)
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

        let jobs_map = self.jobs.read().await;
        for job_lock in jobs_map.values() {
            let job = job_lock.read().await;

            if let JobState::Running(phase) = &job.state() {
                let start_time = job
                    .phase_start_time(phase)
                    .map(|t| t.timestamp() as u64)
                    .unwrap_or_else(|| {
                        error!(
                            "Start time for phase {:?} is missing for job {}",
                            phase, job.job_id
                        );
                        0
                    });

                jobs.push(JobStatusDto {
                    job_id: job.job_id.clone(),
                    data_id: job.data_id.clone(),
                    phase: Some(phase.clone()),
                    state: job.state().clone(),
                    assigned_workers: job.workers.clone(),
                    start_time,
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
        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        let job = job_entry.read().await;

        let phase = JobPhase::Contributions;
        let start_time =
            job.phase_start_time(&phase).map(|t| t.timestamp() as u64).unwrap_or_else(|| {
                error!("Start time for phase {:?} is missing for job {}", phase, job.job_id);
                0
            });

        Ok(JobStatusDto {
            job_id: job.job_id.clone(),
            data_id: job.data_id.clone(),
            state: job.state().clone(),
            phase: if let JobState::Running(phase) = &job.state() {
                Some(phase.clone())
            } else {
                None
            },
            assigned_workers: job.workers.clone(),
            start_time,
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
        let jobs_map = self.jobs.read().await;
        for job_lock in jobs_map.values() {
            let job = job_lock.read().await;
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

        if request.minimal_compute_capacity > request.compute_capacity {
            error!("Invalid requested minimal compute capacity");
            return Err(CoordinatorError::InvalidArgument(
                "Minimal compute capacity must not exceed compute capacity".to_string(),
            ));
        }

        // Check if we have enough capacity to compute the proof is already checked
        // in create_job > partition_and_allocate_by_capacity

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
        let minimal_compute_capacity = ComputeCapacity::from(request.minimal_compute_capacity);

        // Create and configure a new job
        let mut job = self
            .create_job(
                request.data_id.clone(),
                required_compute_capacity,
                minimal_compute_capacity,
                request.inputs_mode,
                request.hints_mode,
                request.simulated_node,
                request.metadata.clone(),
                request.execution_only,
            )
            .await?;

        info!(
            "[Job] Started {} successfully Inputs: {:?} Hints: {:?} Capacity: {} Workers: {}",
            job.job_id,
            job.inputs_mode,
            job.hints_mode,
            job.compute_capacity,
            job.workers.len(),
        );

        // Initialize job state
        job.change_state(JobState::Running(JobPhase::Contributions));

        let job_id = job.job_id.clone();
        let active_workers = self.select_workers_for_execution(&job)?;

        // Store job in jobs map
        let job_arc = Arc::new(RwLock::new(job));
        self.jobs.write().await.insert(job_id.clone(), job_arc.clone());
        self.alloc_job_events(&job_id).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Queued).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Started).await;

        // Send Phase1 tasks to selected workers
        let job = job_arc.read().await;
        self.dispatch_contributions_messages(&job, &active_workers).await?;

        info!("[Phase1] Started with {} workers for {}", active_workers.len(), job_id);

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
        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        let job = job_entry.read().await;

        // Clone job.final_proof and final_verkey and error if does not exist
        let final_proof = if job.state == JobState::Completed {
            Some(job.final_proof.clone().ok_or_else(|| {
                CoordinatorError::Internal(
                    "Final proof is missing during post-launch processing".to_string(),
                )
            })?)
        } else {
            None
        };

        let final_verkey = if job.state == JobState::Completed {
            Some(job.final_verkey.clone().ok_or_else(|| {
                CoordinatorError::Internal(
                    "Final verification key is missing during post-launch processing".to_string(),
                )
            })?)
        } else {
            None
        };

        // Check if webhook URL is configured and spawn it in a separate task
        if let Some(webhook_url) = &self.config.coordinator.webhook_url {
            self.send_webhook(webhook_url.clone(), &job);
        }

        let state = job.state.clone();
        drop(job);
        let mut job = job_entry.write().await;

        // Save proof to disk
        if state == JobState::Completed && !self.config.server.no_save_proofs {
            let folder = self.config.server.proofs_dir.clone();

            let zisk_proof = ZiskProofWithPublicValues::new_from_vadcop_proof(
                &final_proof.unwrap(),
                self.config.coordinator.minimal_proofs,
                final_verkey.unwrap(),
            )
            .map_err(|e| CoordinatorError::Internal(format!("Failed to create proof: {}", e)))?;
            fs::create_dir_all(&folder).map_err(|e| {
                CoordinatorError::Internal(format!("Failed to create proofs directory: {}", e))
            })?;
            let raw_path = folder.join(format!("proof_{}.bin", job_id.as_str()));
            zisk_proof
                .save(raw_path)
                .map_err(|e| CoordinatorError::Internal(format!("Failed to save proof: {}", e)))?;
        }

        // Clean up process data for the job
        job.cleanup();

        Ok(())
    }

    /// Sends webhook notifications for job completion or failure.
    ///
    /// # Parameters
    ///
    /// * `webhook_url` - The URL to send the webhook to.
    /// * `job_id` - The ID of the job.
    ///
    fn send_webhook(&self, webhook_url: String, job: &Job) {
        // Errors from webhook sending are logged but not reported
        let job_id = job.job_id.clone();
        let duration_ms = job.duration_ms.unwrap_or(0);
        let job_state = job.state.clone();
        let final_proof = job.final_proof.clone();
        let executed_steps = job.executed_steps;

        tokio::spawn(async move {
            const MAX_RETRIES: usize = 10;
            const INITIAL_BACKOFF_MS: u64 = 50;
            const MAX_BACKOFF_MS: u64 = 2000;

            let mut attempt = 0;

            while attempt < MAX_RETRIES {
                let result = if job_state == JobState::Failed {
                    hooks::send_failure_webhook(
                        webhook_url.clone(),
                        job_id.clone(),
                        duration_ms,
                        "JOB_FAILED".to_string(),
                        "The job has failed during execution.".to_string(),
                    )
                    .await
                } else {
                    hooks::send_completion_webhook(
                        webhook_url.clone(),
                        job_id.clone(),
                        duration_ms,
                        final_proof.clone(),
                        executed_steps,
                    )
                    .await
                };

                match result {
                    Ok(_) => {
                        info!("Successfully sent webhook {} for job {}", webhook_url, job_id);
                        break;
                    }
                    Err(e) => {
                        attempt += 1;

                        if attempt >= MAX_RETRIES {
                            error!(
                                "Failed to send webhook {} for job {} after {} attempts: {}",
                                webhook_url, job_id, MAX_RETRIES, e
                            );
                            break;
                        }

                        // Exponential backoff: 50ms, 100ms, 200ms, 400ms, 800ms, 1600ms, 2000ms (capped)
                        let wait_ms = (INITIAL_BACKOFF_MS * 2_u64.pow(attempt as u32 - 1))
                            .min(MAX_BACKOFF_MS);

                        warn!(
                            "Failed to send webhook {} for job {} (attempt {}/{}): {}. Retrying in {}ms",
                            webhook_url, job_id, attempt, MAX_RETRIES, e, wait_ms
                        );

                        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                    }
                }
            }
        });
    }

    /// Creates a new proof generation job with allocated resources.
    ///
    /// # Parameters
    ///
    /// * `data_id` - Unique identifier for the data being processed
    /// * `required_compute_capacity` - Computational resources needed for the job
    /// * `input_path` - Filesystem path to the input data
    /// * `simulated_node` - Optional node index for simulation mode
    ///
    /// # Returns
    ///
    /// On success, returns a fully initialized job ready to start proof generation
    #[allow(clippy::too_many_arguments)]
    pub async fn create_job(
        &self,
        data_id: DataId,
        required_compute_capacity: ComputeCapacity,
        minimal_compute_capacity: ComputeCapacity,
        inputs_mode: InputsModeDto,
        hints_mode: HintsModeDto,
        simulated_node: Option<u32>,
        metadata: std::collections::BTreeMap<String, String>,
        execution_only: bool,
    ) -> CoordinatorResult<Job> {
        let execution_mode = if let Some(node) = simulated_node {
            JobExecutionMode::Simulating(node)
        } else {
            JobExecutionMode::Standard
        };

        let (selected_workers, mut partitions) = self
            .workers_pool
            .partition_and_allocate_by_capacity(
                required_compute_capacity,
                minimal_compute_capacity,
                execution_mode,
            )
            .await?;

        if let Some(simulated_node) = simulated_node {
            partitions[0] = partitions[simulated_node as usize].clone();
        }

        Ok(Job::new(
            data_id,
            inputs_mode,
            hints_mode,
            required_compute_capacity,
            minimal_compute_capacity,
            selected_workers,
            partitions,
            execution_mode,
            metadata,
            execution_only,
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
    /// * `job` - Job containing partition assignments and configuration
    /// * `active_workers` - List of workers that should receive tasks
    async fn dispatch_contributions_messages(
        &self,
        job: &Job,
        active_workers: &[WorkerId],
    ) -> CoordinatorResult<()> {
        let input_source = match job.inputs_mode {
            InputsModeDto::InputsPath(ref inputs_path) => {
                InputSourceDto::InputPath(inputs_path.clone())
            }
            InputsModeDto::InputsData(ref inputs_uri) => {
                let inputs = tokio::fs::read(inputs_uri).await.map_err(|e| {
                    CoordinatorError::Internal(format!(
                        "Failed to read input data for job {}: {}",
                        job.job_id, e
                    ))
                })?;
                InputSourceDto::InputData(inputs)
            }
            InputsModeDto::InputsNone => InputSourceDto::InputNull,
        };

        let hints_source = match &job.hints_mode {
            HintsModeDto::HintsPath(ref hints_uri) => HintsSourceDto::HintsPath(hints_uri.clone()),
            HintsModeDto::HintsStream(hints_uri) => {
                // Hints will be streamed separately
                HintsSourceDto::HintsStream(hints_uri.clone())
            }
            HintsModeDto::HintsNone => HintsSourceDto::HintsNull,
        };

        // Use Arc to avoid expensive clones
        let active_workers = active_workers.to_vec();
        let total_workers = active_workers.len() as u32;

        let cloned_active_workers = active_workers.clone();
        let execution_only = job.execution_only;
        let tasks = active_workers.into_iter().enumerate().map(|(rank_id, worker_id)| {
            let job_id = job.job_id.clone();
            let data_id = job.data_id.clone();
            let input_source = input_source.clone();
            let hints_source = hints_source.clone();
            let worker_allocation = job.partitions[rank_id].clone();
            let job_compute_capacity = job.compute_capacity;
            let workers_pool = &self.workers_pool;

            async move {
                let contribution_params = ContributionParamsDto {
                    data_id,
                    input_source,
                    hints_source,
                    rank_id: rank_id as u32,
                    total_workers,
                    worker_allocation,
                    job_compute_units: job_compute_capacity,
                };

                let params = if execution_only {
                    ExecuteTaskRequestTypeDto::ExecutionParams(contribution_params)
                } else {
                    ExecuteTaskRequestTypeDto::ContributionParams(contribution_params)
                };

                let req = ExecuteTaskRequestDto {
                    worker_id: worker_id.clone(),
                    job_id: job_id.clone(),
                    params,
                };
                let req = CoordinatorMessageDto::ExecuteTaskRequest(req);

                let send_result = workers_pool.send_message(&worker_id, req).await;
                let state_result = workers_pool
                    .mark_worker_with_state(
                        &worker_id,
                        WorkerState::Computing((job_id.clone(), JobPhase::Contributions)),
                    )
                    .await;

                (worker_id, send_result, state_result)
            }
        });

        // Process tasks with a concurrency limit
        use futures::stream::StreamExt;

        let results: Vec<_> = futures::stream::iter(tasks).buffer_unordered(16).collect().await;

        // Check for any errors
        for (worker_id, send_result, state_result) in results {
            send_result.map_err(|e| {
                CoordinatorError::Internal(format!(
                    "Failed to send message to worker {}: {}",
                    worker_id, e
                ))
            })?;

            state_result.map_err(|e| {
                CoordinatorError::Internal(format!(
                    "Failed to update state for worker {}: {}",
                    worker_id, e
                ))
            })?;
        }

        if matches!(hints_source, HintsSourceDto::HintsStream(_)) {
            self.initialize_stream(job, cloned_active_workers)?;
        }

        Ok(())
    }

    fn initialize_stream(
        &self,
        job: &Job,
        cloned_active_workers: Vec<WorkerId>,
    ) -> Result<(), CoordinatorError> {
        let hints_uri = match &job.hints_mode {
            HintsModeDto::HintsStream(uri) => uri,
            _ => unreachable!(),
        };
        let job_id_clone = job.job_id.clone();
        let workers_clone = Arc::new(cloned_active_workers.clone());
        let workers_pool = Arc::clone(&self.workers_pool);

        // Async dispatcher - no blocking, pure async flow for maximum performance
        let dispatcher =
            move |sequence_number: u32, stream_type: StreamMessageKind, payload: Vec<u8>| {
                use futures::future::join_all;
                use zisk_distributed_common::{StreamDataDto, StreamPayloadDto};

                let job_id = job_id_clone.clone();
                let workers = Arc::clone(&workers_clone);
                let pool = Arc::clone(&workers_pool);

                Box::pin(async move {
                    let sends = workers.iter().map(|worker_id| {
                        let job_id = job_id.clone();
                        let worker_id = worker_id.clone();
                        let payload = payload.clone();
                        let pool = Arc::clone(&pool);
                        let stream_type = stream_type.clone();

                        async move {
                            let msg = CoordinatorMessageDto::StreamData(StreamDataDto {
                                job_id: job_id.clone(),
                                stream_type,
                                stream_payload: Some(StreamPayloadDto { sequence_number, payload }),
                            });

                            if let Err(e) = pool.send_message(&worker_id, msg).await {
                                error!(
                                    "Failed to send hints to worker {} for job {}: {}",
                                    worker_id, job_id, e
                                );
                            }
                        }
                    });

                    join_all(sends).await;
                })
            };
        let hints_relay = PrecompileHintsRelay::new(dispatcher);
        let mut stream = ZiskStream::new(hints_relay);
        let stream_reader = StreamSource::from_uri(hints_uri).map_err(|e| {
            CoordinatorError::Internal(format!(
                "Failed to create hints stream reader for job {}: {}",
                job.job_id, e
            ))
        })?;
        stream.set_hints_stream_src(stream_reader).map_err(|e| {
            CoordinatorError::Internal(format!(
                "Failed to set hints stream for job {}: {}",
                job.job_id, e
            ))
        })?;
        stream.start_stream().map_err(|e| {
            CoordinatorError::Internal(format!(
                "Failed to start hints stream for job {}: {}",
                job.job_id, e
            ))
        })?;
        Ok(())
    }

    /// Marks a job as failed and performs and cleans up all associated resources
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the failing job
    /// * `reason` - Human-readable description of the failure cause
    pub async fn fail_job(&self, job_id: &JobId, reason: impl AsRef<str>) -> CoordinatorResult<()> {
        let jobs_map = self.jobs.read().await;
        let job_entry =
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        drop(jobs_map);

        let worker_ids = {
            let mut job = job_entry.write().await;

            // Prevent double-fail races (monitor + worker error racing)
            if job.state().is_resolved() {
                return Ok(());
            }

            job.change_state(JobState::Failed);
            job.workers.clone()
            // job write lock released here
        };

        // These operations only need the worker IDs, not the job lock.
        self.cancel_job_workers(&worker_ids, job_id, reason.as_ref()).await;
        self.ensure_workers_idle(&worker_ids).await;

        self.fire_job_event(
            job_id,
            CoordinatorJobEvent::Failed(reason.as_ref().to_string()),
        )
        .await;

        error!("Failed job {} (reason: {})", job_id, reason.as_ref());

        drop(job_entry);

        // post_launch_proof may fail (e.g. proof serialization, webhook).
        // Ensure cleanup always runs even if it does.
        if let Err(e) = self.post_launch_proof(job_id).await {
            warn!("post_launch_proof failed for job {}: {} — forcing cleanup", job_id, e);
            let jobs_map = self.jobs.read().await;
            if let Some(job_entry) = jobs_map.get(job_id) {
                job_entry.write().await.cleanup();
            }
        }

        Ok(())
    }

    /// Handles a setup program acknowledgement from a worker.
    ///
    /// Called when a worker reports that it has completed (or failed) a setup operation.
    pub async fn handle_stream_setup_program_ack(
        &self,
        ack: SetupProgramAckDto,
    ) -> CoordinatorResult<()> {
        if ack.success {
            info!(
                "[Setup] Worker {} completed setup for job_id {} hash_id {}",
                ack.worker_id, ack.job_id, ack.hash_id
            );
        } else {
            error!(
                "[Setup] Worker {} failed setup for job_id {} hash_id {}: {}",
                ack.worker_id,
                ack.job_id,
                ack.hash_id,
                ack.error_message.as_deref().unwrap_or("unknown error")
            );
        }
        // TODO: track per-job setup completion and fire Completed(Setup) event
        Ok(())
    }

    /// Sends cancellation messages to all workers assigned to a job.
    /// Best-effort: logs warnings on failure but continues.
    async fn cancel_job_workers(&self, worker_ids: &[WorkerId], job_id: &JobId, reason: &str) {
        for worker_id in worker_ids {
            let msg =
                CoordinatorMessageDto::JobCancelled(zisk_distributed_common::JobCancelledDto {
                    job_id: job_id.clone(),
                    reason: reason.to_string(),
                });
            if let Err(e) = self.workers_pool.send_message(worker_id, msg).await {
                warn!("Failed to send cancellation to worker {}: {}", worker_id, e);
            }
        }
    }

    /// Marks all Computing workers in the list as Idle.
    async fn ensure_workers_idle(&self, worker_ids: &[WorkerId]) {
        self.workers_pool.mark_computing_workers_idle(worker_ids).await;
    }

    /// Handles new worker registration. Returns `(accepted, message)`.
    pub async fn handle_stream_registration(
        &self,
        req: WorkerRegisterRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String) {
        self.registrations.fetch_add(1, Ordering::Relaxed);

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

    /// Handles worker reconnection with state reconciliation.
    ///
    /// When a worker reconnects (process survived a disconnect), it may hold stale
    /// `current_job` state. The coordinator reconciles by checking the claimed job
    /// against its own state and returning a directive:
    ///
    /// | Worker claims job X | Coordinator state          | Directive                |
    /// |---------------------|----------------------------|--------------------------|
    /// | None                | —                          | None (idle)              |
    /// | Some(X)             | Job X unknown              | CancelStaleJob           |
    /// | Some(X)             | Job X terminal             | CancelStaleJob           |
    /// | Some(X)             | Job X active, not assigned | CancelStaleJob           |
    /// | Some(X)             | Job X active, assigned     | ResumeComputing          |
    pub async fn handle_stream_reconnection(
        &self,
        req: WorkerReconnectRequestDto,
        msg_sender: Box<dyn MessageSender + Send + Sync>,
    ) -> (bool, String, Option<ReconnectionDirectiveDto>) {
        self.reconnections.fetch_add(1, Ordering::Relaxed);

        // Check max connections — but allow if the worker already exists (reconnection)
        let max_connections = self.config.coordinator.max_total_workers as usize;
        if self.workers_pool.num_workers().await >= max_connections
            && self.workers_pool.worker_state(&req.worker_id).await.is_none()
        {
            return (
                false,
                format!("Maximum concurrent connections reached: ({})", max_connections),
                None,
            );
        }

        let worker_id = req.worker_id.clone();
        let last_known_job_id = req.last_known_job_id.clone();

        if let Err(e) =
            self.workers_pool.register_worker(req.worker_id, req.compute_capacity, msg_sender).await
        {
            return (false, format!("Reconnection failed: {e}"), None);
        }

        // Reconcile stale job state
        let directive = self.compute_reconnection_directive(&worker_id, last_known_job_id).await;

        if let Some(ref d) = directive {
            match d {
                ReconnectionDirectiveDto::CancelStaleJob => {
                    info!("Reconnection of {worker_id}: directing cancellation of stale job");
                }
                ReconnectionDirectiveDto::KeepComputing => {
                    info!("Reconnection of {worker_id}: job still active, keep computing");
                }
                ReconnectionDirectiveDto::Idle => {}
            }
        }

        (true, "Reconnection successful".to_string(), directive)
    }

    /// Computes the reconciliation directive for a reconnecting worker based on
    /// its claimed `last_known_job_id` vs the coordinator's current job state.
    async fn compute_reconnection_directive(
        &self,
        worker_id: &WorkerId,
        last_known_job_id: Option<JobId>,
    ) -> Option<ReconnectionDirectiveDto> {
        let claimed_job_id = last_known_job_id?;

        let job_entry = {
            let jobs_map = self.jobs.read().await;
            match jobs_map.get(&claimed_job_id) {
                None => {
                    // Coordinator has no record (restarted or job expired)
                    return Some(ReconnectionDirectiveDto::CancelStaleJob);
                }
                Some(entry) => entry.clone(),
            }
        };

        let job = job_entry.read().await;

        if job.state.is_resolved() {
            return Some(ReconnectionDirectiveDto::CancelStaleJob);
        }

        if !job.workers.contains(worker_id) {
            return Some(ReconnectionDirectiveDto::CancelStaleJob);
        }

        // Job is active and worker is still assigned — process survived the
        // disconnect so the computation may still be running. Let the worker
        // continue; the re-established channel will deliver the result.
        Some(ReconnectionDirectiveDto::KeepComputing)
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
    /// If the worker was computing, fail the associated job.
    /// Returns Ok(()) if the worker was not computing or if the job was already terminal.
    async fn fail_job_if_computing(
        &self,
        worker_id: &WorkerId,
        worker_state: Option<WorkerState>,
        reason: &str,
    ) -> CoordinatorResult<()> {
        if let Some(WorkerState::Computing((job_id, phase))) = worker_state {
            error!(
                "Worker {} {} while computing for job {} in phase {:?}",
                worker_id, reason, job_id, phase
            );
            self.fail_job(&job_id, format!("Worker {} {}", worker_id, reason)).await?;
        }
        Ok(())
    }

    /// Unregisters a worker. If it was computing, fails the associated job.
    pub async fn unregister_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let worker_state = self.workers_pool.worker_state(worker_id).await;
        self.fail_job_if_computing(worker_id, worker_state, "unregistered").await?;
        self.workers_pool.unregister_worker(worker_id).await
    }

    pub async fn disconnect_worker(&self, worker_id: &WorkerId) -> CoordinatorResult<()> {
        let worker_state = self.workers_pool.worker_state(worker_id).await;
        self.fail_job_if_computing(worker_id, worker_state, "disconnected").await?;
        self.workers_pool.disconnect_worker(worker_id).await
    }

    /// Generation-aware disconnect for [`ConnectionDropGuard`].
    ///
    /// Checks if the worker's current connection generation matches the expected
    /// generation. If it does, checks if the worker was computing and fails the
    /// associated job, then disconnects the worker. If the generation doesn't match,
    /// this is a stale guard and the call is a no-op.
    pub async fn disconnect_worker_if_generation(
        &self,
        worker_id: &WorkerId,
        expected_generation: u64,
    ) -> CoordinatorResult<()> {
        // Read the worker state + generation atomically under one lock
        let (generation_matches, worker_state) = {
            match self.workers_pool.worker_state_and_generation(worker_id).await {
                Some((state, gen)) if gen == expected_generation => (true, Some(state)),
                _ => (false, None),
            }
        };

        if !generation_matches {
            return Ok(());
        }

        // If the worker was computing, fail the associated job
        if let Err(e) =
            self.fail_job_if_computing(worker_id, worker_state, "connection dropped").await
        {
            warn!("Failed to fail job on worker {} disconnect: {}", worker_id, e);
        }

        // Disconnect with generation check (re-validates under write lock)
        self.workers_pool.disconnect_worker_if_generation(worker_id, expected_generation).await
    }

    /// Starts the background job monitor that periodically checks for
    /// phase timeouts, stale heartbeats, and disconnected worker cleanup.
    pub fn start_job_monitor(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let coordinator = Arc::clone(self);
        let interval_secs = coordinator.config.coordinator.job_monitor_interval_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                coordinator.run_monitor_sweep().await;
            }
        })
    }

    /// Runs a single monitor sweep: checks phase timeouts, stale heartbeats,
    /// and cleans up stale disconnected workers.
    pub async fn run_monitor_sweep(&self) {
        self.check_phase_timeouts().await;
        self.check_stale_heartbeats().await;
        self.cleanup_stale_disconnected_workers().await;
    }

    /// Checks all running jobs for phase timeouts and fails them if exceeded.
    pub async fn check_phase_timeouts(&self) {
        // Clone job entries to avoid holding the read lock during async operations
        let entries: Vec<_> = {
            let jobs_map = self.jobs.read().await;
            jobs_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };

        let mut timed_out: Vec<(JobId, String)> = Vec::new();

        for (job_id, job_lock) in entries {
            let job = job_lock.read().await;
            if let JobState::Running(ref phase) = job.state {
                let timeout_secs = self.phase_timeout_secs(phase);
                if timeout_secs == 0 {
                    continue;
                }

                if let Some(start_time) = job.phase_start_time(phase) {
                    let elapsed = Utc::now().signed_duration_since(start_time);
                    if elapsed >= chrono::Duration::seconds(timeout_secs as i64) {
                        let reason = format!(
                            "[Monitor] Phase {:?} timed out for job {} ({}s > {}s)",
                            phase,
                            job.job_id,
                            elapsed.num_seconds(),
                            timeout_secs
                        );
                        timed_out.push((job_id.clone(), reason));
                    }
                }
            }
        }

        for (job_id, reason) in timed_out {
            warn!("{}", reason);
            if let Err(e) = self.fail_job(&job_id, &reason).await {
                error!("Failed to abort timed-out job {}: {}", job_id, e);
            }
        }
    }

    /// Returns the configured timeout in seconds for a given phase.
    fn phase_timeout_secs(&self, phase: &JobPhase) -> u64 {
        match phase {
            JobPhase::Execution => self.config.coordinator.execution_timeout_seconds,
            JobPhase::Contributions
            | JobPhase::ContributionsInputsStream
            | JobPhase::ContributionsHintsStream => self.config.coordinator.phase1_timeout_seconds,
            JobPhase::Prove => self.config.coordinator.phase2_timeout_seconds,
            JobPhase::Aggregate => self.config.coordinator.phase3_timeout_seconds,
        }
    }

    /// Checks for computing workers with stale heartbeats and fails their jobs.
    pub async fn check_stale_heartbeats(&self) {
        let threshold = chrono::Duration::seconds(
            (self.config.coordinator.heartbeat_interval_seconds
                * self.config.coordinator.heartbeat_max_missed as u64) as i64,
        );
        let stale = self.workers_pool.get_stale_computing_workers(threshold).await;

        // Deduplicate by job_id
        let mut failed_jobs = std::collections::HashSet::new();
        for (worker_id, job_id, _phase) in &stale {
            if failed_jobs.insert(job_id.clone()) {
                let reason =
                    format!("[Monitor] Worker {} missed heartbeats for job {}", worker_id, job_id);
                warn!("{}", reason);
                if let Err(e) = self.fail_job(job_id, &reason).await {
                    error!("Failed to abort job {} due to stale heartbeat: {}", job_id, e);
                }
            }
        }
    }

    /// Removes worker entries that have been Disconnected for longer than the configured threshold.
    async fn cleanup_stale_disconnected_workers(&self) {
        let threshold_secs = self.config.coordinator.stale_disconnected_threshold_seconds;
        self.workers_pool
            .remove_stale_disconnected(chrono::Duration::seconds(threshold_secs as i64))
            .await;
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

    pub async fn handle_stream_job_cancelled_ack(
        &self,
        worker_id: &WorkerId,
        job_id: &JobId,
    ) -> CoordinatorResult<()> {
        self.workers_pool.update_last_heartbeat(worker_id).await?;
        info!("Worker {} acknowledged cancellation of job {}", worker_id, job_id);
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

        // If the job is already terminal (Failed/Completed), this is a late arrival
        // (e.g. spawn_blocking finished after JobCancelled). Mark worker Idle and discard.
        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(&message.job_id).cloned()
        };
        if let Some(job_entry) = job_entry {
            let job = job_entry.read().await;
            if job.state().is_resolved() {
                info!(
                    "Ignoring late ExecuteTaskResponse from worker {} for resolved job {}",
                    message.worker_id, message.job_id
                );
                drop(job);
                drop(job_entry);
                self.workers_pool
                    .mark_worker_with_state(&message.worker_id, WorkerState::Idle)
                    .await?;
                return Ok(());
            }
        }

        // Handle task failure if needed
        if !message.success {
            return self.handle_task_failure(message).await;
        }

        match message.result_data {
            ExecuteTaskResponseResultDataDto::Execution(_) => {
                self.handle_execution_completion(message).await
            }
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
        if !self.jobs.read().await.contains_key(&message.job_id) {
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

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;

        let worker_id = execute_task_response.worker_id.clone();

        // If job has Failed, mark worker as Idle and return early
        if matches!(job.state(), JobState::Failed) {
            self.workers_pool.mark_worker_with_state(&worker_id, WorkerState::Idle).await?;
            return Ok(());
        }

        // Store Contributions response and extract instances
        let instances = self.store_contribution_response(&mut job, execute_task_response).await?;
        job.instances = Some(instances);

        // Check if all contributions are complete
        if !self.check_phase1_completion(&job, &worker_id) {
            return Ok(());
        }

        // Print execution summary from Phase 1 completion
        self.print_execution_summary(&job);

        // Validate and extract challenges in a single operation to minimize lock time
        let challenges = self.validate_and_extract_challenges(&job).await?;

        // Update job state to Phase2
        job.challenges = Some(challenges);
        job.change_state(JobState::Running(JobPhase::Prove));

        let challenges_dto = self.collect_challenges_dto(&job);

        let active_workers = self.select_workers_for_execution(&job)?;

        drop(job); // Release jobs lock early

        self.fire_job_event(&job_id, CoordinatorJobEvent::Progress(JobPhase::Prove)).await;

        // Start Phase2 for all workers
        self.start_prove(&job_id, &active_workers, challenges_dto).await?;

        info!("[Phase2] Started with {} workers for {}", active_workers.len(), job_id);

        Ok(())
    }

    pub async fn handle_execution_completion(
        &self,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<()> {
        let job_id = execute_task_response.job_id.clone();

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;

        let worker_id = execute_task_response.worker_id.clone();

        // If job has Failed, mark worker as Idle and return early
        if matches!(job.state(), JobState::Failed) {
            self.workers_pool.mark_worker_with_state(&worker_id, WorkerState::Idle).await?;
            return Ok(());
        }

        // Store Execution response and extract instances and executed_steps
        let (instances, executed_steps) =
            self.store_execution_response(&mut job, execute_task_response).await?;
        job.instances = Some(instances);
        job.executed_steps = Some(executed_steps);

        // Check if all execution results are complete
        if !self.check_execution_completion(&job, &worker_id) {
            return Ok(());
        }

        // Print execution summary
        self.print_execution_summary(&job);

        // Mark job as completed (execution-only, no proof generation)
        job.change_state(JobState::Completed);

        // Calculate total execution time
        let end_time = Utc::now();
        let start_time = job.phase_start_time(&JobPhase::Execution).unwrap_or(end_time);
        let total_duration = end_time.signed_duration_since(start_time);
        let duration = Duration::from_millis(total_duration.num_milliseconds() as u64);

        let header = format!("[Execution] Job {} completed successfully ✔", job_id).green();
        let duration_str = format!("Duration: {:.3}s", duration.as_secs_f32()).bold();
        let steps_str = if let Some(executed_steps) = job.executed_steps {
            format!("Steps: {}", Self::format_number_with_dots(executed_steps)).bold()
        } else {
            "Steps: N/A".to_string().red().bold()
        };
        let instances_str = if let Some(instances) = job.instances {
            format!("Instances: {}", Self::format_number_with_dots(instances)).bold()
        } else {
            "Instances: N/A".to_string().red().bold()
        };

        let metadata_str = if job.metadata.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> =
                job.metadata.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
            format!(" {}", pairs.join(", "))
        };

        info!(
            "{} {} {} {} Capacity: {}{}",
            header, duration_str, steps_str, instances_str, job.compute_capacity, metadata_str
        );

        // Print ASM execution statistics if multiple workers
        let workers = job.workers.clone();
        if workers.len() > 1 {
            if let Some(results) = job.results.get(&JobPhase::Execution) {
                // Extract overall execution times (phase duration from task received to completion)
                let mut execution_durations: Vec<(WorkerId, i64)> = results
                    .iter()
                    .filter_map(|(worker_id, result)| {
                        if let JobResultData::Execution(exec_result) = &result.data {
                            exec_result.task_received_time.map(|task_received| {
                                let duration = result.end_time.signed_duration_since(task_received);
                                (worker_id.clone(), duration.num_milliseconds())
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                if execution_durations.len() > 1 {
                    execution_durations.sort_by_key(|(_, duration)| *duration);
                    let (best_worker, best_duration) = &execution_durations[0];
                    let (worst_worker, worst_duration) = execution_durations.last().unwrap();
                    let avg_duration = execution_durations.iter().map(|(_, d)| d).sum::<i64>()
                        as f64
                        / execution_durations.len() as f64;

                    let diff_percentage = if *best_duration > 0 {
                        ((*worst_duration - *best_duration) as f64 / *best_duration as f64) * 100.0
                    } else {
                        0.0
                    };

                    info!(
                        "[Execution] Performance for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                        job_id,
                        avg_duration / 1000.0,
                        best_worker,
                        *best_duration as f64 / 1000.0,
                        worst_worker,
                        *worst_duration as f64 / 1000.0,
                        diff_percentage
                    );
                }

                // Extract ASM execution times
                let mut asm_times: Vec<(WorkerId, f32, f32)> = results
                    .iter()
                    .filter_map(|(worker_id, result)| {
                        if let JobResultData::Execution(exec_result) = &result.data {
                            exec_result
                                .zisk_executor_time
                                .asm_execution_duration
                                .as_ref()
                                .map(|asm| (worker_id.clone(), asm.time, asm.mhz))
                        } else {
                            None
                        }
                    })
                    .collect();

                if !asm_times.is_empty() {
                    asm_times.sort_by(|(_, a, _), (_, b, _)| {
                        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    let (best_asm_worker, best_asm, best_mhz) = &asm_times[0];
                    let (worst_asm_worker, worst_asm, worst_mhz) = asm_times.last().unwrap();
                    let avg_asm = asm_times.iter().map(|(_, t, _)| *t as f64).sum::<f64>()
                        / asm_times.len() as f64;

                    let asm_diff_percentage = if *best_asm > 0.0 {
                        ((*worst_asm - *best_asm) as f64 / *best_asm as f64) * 100.0
                    } else {
                        0.0
                    };

                    info!(
                        "[Execution] ASM for {} - Avg: {:.3}s, Best: {} ({:.3}s @ {:.1}MHz), Worst: {} ({:.3}s @ {:.1}MHz), Diff: {:.1}%",
                        job_id,
                        avg_asm,
                        best_asm_worker,
                        *best_asm,
                        *best_mhz,
                        worst_asm_worker,
                        *worst_asm,
                        *worst_mhz,
                        asm_diff_percentage
                    );
                }
            }
        }

        // Mark all workers as idle
        for worker_id in &job.workers {
            self.workers_pool.mark_worker_with_state(worker_id, WorkerState::Idle).await?;
        }

        let exec_stats = exec_stats_from_job(&job);

        // Release job lock before cleanup
        drop(job);

        self.fire_job_event(
            &job_id,
            CoordinatorJobEvent::Completed(CoordinatorJobResult::Execute {
                stats: exec_stats,
                public_outputs: vec![], // TODO: thread public outputs through distributed exec path
            }),
        )
        .await;

        let mut job = job_entry.write().await;

        // Clean up process data for the job (no webhook for execution-only)
        job.cleanup();

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
    ) -> CoordinatorResult<u64> {
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
        let instances =
            if let JobResultData::Challenges(ref contrib) = data { contrib.instances } else { 0 };

        contributions_results.insert(
            worker_id.clone(),
            JobResult { success: execute_task_response.success, data, end_time: Utc::now() },
        );

        Ok(instances)
    }

    /// Stores a single worker's Execution-only response in the job state.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to update
    /// * `execute_task_response` - The response from the worker containing execution data
    async fn store_execution_response(
        &self,
        job: &mut Job,
        execute_task_response: ExecuteTaskResponseDto,
    ) -> CoordinatorResult<(u64, u64)> {
        let execution_results = job.results.entry(JobPhase::Execution).or_default();

        let worker_id = execute_task_response.worker_id.clone();

        // Check for duplicate results
        if execution_results.contains_key(&worker_id) {
            warn!("Received duplicate Execution result from worker {worker_id} for {}", job.job_id);
            return Err(CoordinatorError::InvalidRequest(format!(
                "Duplicate Execution result from worker {worker_id} for {}",
                job.job_id
            )));
        }

        let data = self.extract_execution_data(execute_task_response.result_data)?;
        let (instances, executed_steps) = if let JobResultData::Execution(ref exec_result) = data {
            (exec_result.instances, exec_result.executed_steps)
        } else {
            (0, 0)
        };

        execution_results.insert(
            worker_id.clone(),
            JobResult { success: execute_task_response.success, data, end_time: Utc::now() },
        );

        Ok((instances, executed_steps))
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
            ExecuteTaskResponseResultDataDto::Challenges(ch_list) => {
                if ch_list.challenges.is_empty() {
                    return Err(CoordinatorError::InvalidRequest(
                        "Received empty Challenges result data".to_string(),
                    ));
                }

                let contributions: Vec<ContributionsInfo> = ch_list
                    .challenges
                    .into_iter()
                    .map(|challenge| ContributionsInfo {
                        worker_index: challenge.worker_index,
                        airgroup_id: challenge.airgroup_id as usize,
                        challenge: challenge.challenge,
                        aggregated: false,
                    })
                    .collect();

                let witness_info = WitnessInfo {
                    summary_info: ch_list.witness_info.summary_info,
                    publics: ch_list.witness_info.publics,
                    proof_values: ch_list.witness_info.proof_values,
                    witness_time: ch_list.witness_info.witness_time,
                    total_instances: ch_list.witness_info.total_instances as usize,
                };

                let zisk_executor_time = Self::extract_execution_info(&ch_list.zisk_executor_time);

                Ok(JobResultData::Challenges(ContributionsResult {
                    witness_info,
                    challenges: contributions,
                    zisk_executor_time,
                    task_received_time: chrono::DateTime::<Utc>::from_timestamp(
                        (ch_list.zisk_executor_time.task_received_time / 1000.0) as i64,
                        ((ch_list.zisk_executor_time.task_received_time % 1000.0) * 1_000_000.0)
                            as u32,
                    ),
                    instances: ch_list.witness_info.total_instances,
                }))
            }
            _ => Err(CoordinatorError::InvalidRequest(
                "Expected Challenges result data for Phase1".to_string(),
            )),
        }
    }

    /// Extracts execution-only data from the worker's result response.
    ///
    /// # Parameters
    ///
    /// * `result_data` - The result data from the worker's response
    fn extract_execution_data(
        &self,
        result_data: ExecuteTaskResponseResultDataDto,
    ) -> CoordinatorResult<JobResultData> {
        match result_data {
            ExecuteTaskResponseResultDataDto::Execution(exec_data) => {
                let zisk_executor_time =
                    Self::extract_execution_info(&exec_data.zisk_executor_time);
                let instances = exec_data.instances;
                let executed_steps = exec_data.executed_steps;

                let task_received_time = chrono::DateTime::<Utc>::from_timestamp(
                    (exec_data.zisk_executor_time.task_received_time / 1000.0) as i64,
                    ((exec_data.zisk_executor_time.task_received_time % 1000.0) * 1_000_000.0)
                        as u32,
                );

                Ok(JobResultData::Execution(ExecutionResult {
                    instances,
                    executed_steps,
                    zisk_executor_time,
                    task_received_time,
                }))
            }
            _ => {
                Err(CoordinatorError::InvalidRequest("Expected Execution result data".to_string()))
            }
        }
    }

    /// Extracts and converts execution timing information from DTO to internal representation.
    ///
    /// # Parameters
    ///
    /// * `exec_time_dto` - The execution time DTO from the worker's response
    fn extract_execution_info(exec_time_dto: &ZiskExecutorTimeDto) -> ZiskExecutorTime {
        ZiskExecutorTime {
            total_duration: Duration::from_secs_f32(exec_time_dto.total_duration / 1000.0),
            execution_duration: Duration::from_secs_f32(exec_time_dto.execution_duration / 1000.0),
            count_and_plan_duration: Duration::from_secs_f32(
                exec_time_dto.count_and_plan_duration / 1000.0,
            ),
            count_and_plan_mo_duration: Duration::from_secs_f32(
                exec_time_dto.count_and_plan_mo_duration / 1000.0,
            ),
            asm_execution_duration: exec_time_dto
                .asm_execution_duration
                .as_ref()
                .map(|asm_info| AsmExecutionInfo { time: asm_info.time, mhz: asm_info.mhz }),
        }
    }

    /// Prints execution summary information from Phase 1 completion.
    ///
    /// Extracts and displays execution information from the first completed worker's
    /// contribution results, including timing, summary info, and key metrics.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job containing Phase 1 results
    fn print_execution_summary(&self, job: &Job) {
        // Find the first completed contribution result to extract WitnessInfo summary
        if let Some(contributions_results) = job.results.get(&JobPhase::Contributions) {
            if let Some((_worker_id, job_result)) = contributions_results.iter().next() {
                if let JobResultData::Challenges(contributions_result) = &job_result.data {
                    info!("Execution Summary: {}", contributions_result.witness_info.summary_info);
                }
            }
        }
    }

    /// Checks if all workers have completed Phase 1 contributions.
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to check
    fn check_phase1_completion(&self, job: &Job, worker_id: &WorkerId) -> bool {
        let phase1_results_len =
            job.results.get(&JobPhase::Contributions).map(|r| r.len()).unwrap_or(0);

        let end_time = Utc::now();
        let phase_start_time =
            job.phase_start_time(&JobPhase::Contributions).unwrap_or_else(|| {
                error!("Missing start time for Phase1 in job {}", job.job_id);
                end_time
            });
        let duration = end_time.signed_duration_since(phase_start_time);
        let duration_ms = Duration::from_millis(duration.num_milliseconds() as u64);

        // Get execution info from the worker's result
        let worker_result =
            job.results.get(&JobPhase::Contributions).and_then(|results| results.get(worker_id));

        let (asm_info_str, witness_time_str, delay_time_str) = if let Some(job_result) =
            worker_result
        {
            match &job_result.data {
                JobResultData::Challenges(contributions_result) => {
                    // Calculate delay: time from coordinator sending job to worker receiving task
                    let delay_duration = contributions_result
                        .task_received_time
                        .map(|task_received| task_received.signed_duration_since(phase_start_time))
                        .unwrap_or_else(chrono::Duration::zero);
                    let delay_ms = delay_duration.num_milliseconds().max(0) as f32;
                    let delay_str = format!(", Delay: {:.3}s", delay_ms / 1000.0);

                    let asm_str = contributions_result
                        .zisk_executor_time
                        .asm_execution_duration
                        .as_ref()
                        .map(|asm_info| {
                            format!(
                                ", Asm Execution: {:.3}s at {} MHz",
                                asm_info.time, asm_info.mhz
                            )
                        })
                        .unwrap_or_default();

                    let witness_str = format!(
                        ", Witness: {:.3}s",
                        contributions_result.witness_info.witness_time / 1000.0
                    );

                    (asm_str, witness_str, delay_str)
                }
                _ => (String::new(), String::new(), String::new()),
            }
        } else {
            (String::new(), String::new(), String::new())
        };

        info!(
            "[Phase1] {} finished phase 1 for {} ({}/{} workers done, Phase: {:.3}s{}{}{})",
            worker_id,
            job.job_id,
            phase1_results_len,
            job.workers.len(),
            duration_ms.as_secs_f32(),
            delay_time_str,
            witness_time_str,
            asm_info_str,
        );

        // Ensure we have results from all assigned workers before proceeding.
        // If not all workers have responded (and we're not in simulation mode),
        // return early and wait for more results.
        job.execution_mode.is_simulating() || phase1_results_len >= job.workers.len()
    }

    /// Checks if all workers have completed Execution phase (execution-only, no proofs).
    ///
    /// # Parameters
    ///
    /// * `job` - Reference to the job to check
    fn check_execution_completion(&self, job: &Job, worker_id: &WorkerId) -> bool {
        let execution_results_len =
            job.results.get(&JobPhase::Execution).map(|r| r.len()).unwrap_or(0);

        let end_time = Utc::now();
        let phase_start_time = job.phase_start_time(&JobPhase::Execution).unwrap_or_else(|| {
            error!("Missing start time for Execution phase in job {}", job.job_id);
            end_time
        });
        let duration = end_time.signed_duration_since(phase_start_time);
        let duration_ms = Duration::from_millis(duration.num_milliseconds() as u64);

        // Get execution info from the worker's result
        let worker_result =
            job.results.get(&JobPhase::Execution).and_then(|results| results.get(worker_id));

        let (asm_info_str, delay_time_str) = if let Some(job_result) = worker_result {
            match &job_result.data {
                JobResultData::Execution(execution_result) => {
                    // Calculate delay: time from coordinator sending job to worker receiving task
                    let delay_duration = execution_result
                        .task_received_time
                        .map(|task_received| task_received.signed_duration_since(phase_start_time))
                        .unwrap_or_else(chrono::Duration::zero);
                    let delay_ms = delay_duration.num_milliseconds().max(0) as f32;
                    let delay_str = format!(", Delay: {:.3}s", delay_ms / 1000.0);

                    let asm_str = execution_result
                        .zisk_executor_time
                        .asm_execution_duration
                        .as_ref()
                        .map(|asm_info| {
                            format!(
                                ", Asm Execution: {:.3}s at {} MHz",
                                asm_info.time, asm_info.mhz
                            )
                        })
                        .unwrap_or_default();

                    (asm_str, delay_str)
                }
                _ => (String::new(), String::new()),
            }
        } else {
            (String::new(), String::new())
        };

        info!(
            "[Execution] {} finished execution for {} ({}/{} workers done, Phase: {:.3}s{}{})",
            worker_id,
            job.job_id,
            execution_results_len,
            job.workers.len(),
            duration_ms.as_secs_f32(),
            delay_time_str,
            asm_info_str,
        );

        // Ensure we have results from all assigned workers before proceeding.
        job.execution_mode.is_simulating() || execution_results_len >= job.workers.len()
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
                JobResultData::Challenges(ref values) => &values.challenges,
                _ => unreachable!("Expected Challenges data in Phase1 results"),
            };

            // Create challenge sets for each simulated worker using the same base challenges
            vec![first_challenges.clone(); phase1_results.len()].into_iter().flatten().collect()
        } else {
            // Standard mode: aggregate challenges from all participating workers
            // Each worker contributes their portion of the overall challenge space
            let (challenges, witness_info): (Vec<Vec<ContributionsInfo>>, Vec<WitnessInfo>) =
                phase1_results
                    .values()
                    .map(|results| match &results.data {
                        JobResultData::Challenges(values) => {
                            (values.challenges.clone(), values.witness_info.clone())
                        }
                        _ => unreachable!("Expected Challenges data in Phase1 results"),
                    })
                    .unzip();

            let first = witness_info.first().ok_or_else(|| {
                CoordinatorError::Internal(format!("No witness info found in job {}", job.job_id))
            })?;

            let mut mismatched_workers = Vec::new();

            for (worker_idx, info) in witness_info.iter().enumerate() {
                if info.publics != first.publics || info.proof_values != first.proof_values {
                    mismatched_workers.push((worker_idx, info));
                }
            }

            if !mismatched_workers.is_empty() {
                // Format detailed mismatch report
                let mismatch_report: Vec<String> = mismatched_workers
                    .iter()
                    .map(|(idx, info)| {
                        format!(
                            "Worker {} differs: publics={:?}, proof_values={:?}",
                            idx, info.publics, info.proof_values
                        )
                    })
                    .collect();

                return Err(CoordinatorError::Internal(format!(
                    "WitnessInfo mismatch in job {}:\n{}",
                    job.job_id,
                    mismatch_report.join("\n")
                )));
            }

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
                if !matches!(worker_state, WorkerState::Computing((_, JobPhase::Contributions))) {
                    let reason =
                        format!("Worker {worker_id} is not in computing state for {}", job_id);
                    return Err(CoordinatorError::InvalidRequest(reason));
                }

                // Transition worker to Phase 2 computing state
                // This atomic update ensures consistent state tracking across the system
                self.workers_pool
                    .mark_worker_with_state(
                        worker_id,
                        WorkerState::Computing((job_id.clone(), JobPhase::Prove)),
                    )
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

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(&job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        let mut job = job_entry.write().await;

        // If in simulation mode, complete the job
        if job.execution_mode.is_simulating() {
            return self.complete_simulated_job(&mut job, &worker_id).await;
        }

        // If job has Failed, mark worker as Idle and return early
        if matches!(job.state(), JobState::Failed) {
            self.workers_pool
                .mark_worker_with_state(&execute_task_response.worker_id, WorkerState::Idle)
                .await?;
            return Ok(());
        }

        // Store Proof response
        self.store_proof_response(&mut job, execute_task_response).await?;

        // Assign aggregator worker if not already assigned
        let agg_worker_id = self.resolve_aggregator_assignment(&mut job, &worker_id).await?;

        let all_done = self.check_phase2_completion(&job, &worker_id).await?;

        if all_done {
            job.phase_timings.insert(
                JobPhase::Aggregate,
                PhaseTimings { start_time: Utc::now(), end_time: None },
            );
        }

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

        phase2_results.insert(
            worker_id.clone(),
            JobResult { success: execute_task_response.success, data, end_time: Utc::now() },
        );

        Ok(())
    }

    /// Completes a simulated job by marking it as completed and freeing resources.
    ///
    /// # Parameters
    ///
    /// * `job` - Mutable reference to job for state updates
    async fn complete_simulated_job(
        &self,
        job: &mut Job,
        worker_id: &WorkerId,
    ) -> CoordinatorResult<()> {
        job.change_state(JobState::Completed);

        let assigned_workers = job.workers.clone();

        // Reset worker statuses back to Idle
        self.workers_pool.mark_workers_with_state(&assigned_workers, WorkerState::Idle).await?;

        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(
            job.phase_start_time(&JobPhase::Prove).unwrap_or_else(|| {
                error!("Missing start time for Phase2 in job {}", job.job_id);
                end_time
            }),
        );

        let duration_ms = Duration::from_millis(duration.num_milliseconds() as u64);

        // Provide operational visibility into Phase 2 progress
        // This logging helps with monitoring long-running proof generation jobs
        info!(
            "[Phase2 progress] Worker {} done. (duration: {:.3}s)",
            worker_id,
            duration_ms.as_secs_f32()
        );

        let duration_simulation = Duration::from_millis(job.duration_ms.unwrap_or(0));

        info!(
            "[Simulated Job Finished] {} (duration: {:.3}s)",
            job.job_id,
            duration_simulation.as_secs_f32()
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

                let job_id = job.job_id.clone();

                // Update worker state
                self.workers_pool
                    .mark_worker_with_state(
                        candidate_worker_id,
                        WorkerState::Computing((job_id.clone(), JobPhase::Aggregate)),
                    )
                    .await?;

                self.fire_job_event(&job_id, CoordinatorJobEvent::Progress(JobPhase::Aggregate))
                    .await;

                info!(
                    "[Phase3] Assigned worker {} as aggregator for job {}",
                    candidate_worker_id, job_id
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
    async fn check_phase2_completion(
        &self,
        job: &Job,
        worker_id: &WorkerId,
    ) -> CoordinatorResult<bool> {
        let empty_results = HashMap::new();
        let phase2_results = job.results.get(&JobPhase::Prove).unwrap_or(&empty_results);

        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(
            job.phase_start_time(&JobPhase::Prove).unwrap_or_else(|| {
                error!("Missing start time for Phase2 in job {}", job.job_id);
                end_time
            }),
        );
        let duration_ms = Duration::from_millis(duration.num_milliseconds() as u64);

        // Provide operational visibility into Phase 2 progress
        // This logging helps with monitoring long-running proof generation jobs
        info!(
            "[Phase2] {} finished phase 2 for {} ({} / {} workers done, {:.3}s)",
            worker_id,
            job.job_id,
            phase2_results.len(),
            job.workers.len(),
            duration_ms.as_secs_f32()
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
                minimal: self.config.coordinator.minimal_proofs,
            }),
        };

        let message = CoordinatorMessageDto::ExecuteTaskRequest(req);

        self.workers_pool.send_message(agg_worker_id, message).await?;

        Ok(())
    }

    /// Formats a number with dots as thousand separators (e.g., 12.345.567).
    fn format_number_with_dots(n: u64) -> String {
        let s = n.to_string();
        let mut result = String::new();
        let len = s.len();

        for (i, c) in s.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                result.push('.');
            }
            result.push(c);
        }
        result
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

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        let job = job_entry.write().await;

        // If job has Failed, mark worker as Idle and return early
        if matches!(job.state(), JobState::Failed) {
            self.workers_pool
                .mark_worker_with_state(&execute_task_response.worker_id, WorkerState::Idle)
                .await?;
            return Ok(());
        }

        drop(job);

        // An aggregation request has failed, fail the job
        if !execute_task_response.success {
            let reason = format!("Aggregation failed in job {}", job_id);
            self.fail_job(job_id, &reason).await?;

            return Err(CoordinatorError::Internal(reason));
        }

        // Extract the proof data
        let proof_data = match execute_task_response.result_data {
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
        if proof_data.values.is_empty() {
            return Ok(());
        }

        let jobs_map = self.jobs.read().await;
        let job_entry = jobs_map.get(job_id).ok_or(CoordinatorError::NotFoundOrInaccessible)?;

        let mut job = job_entry.write().await;

        let agg_worker_id = &job.agg_worker_id.as_ref().unwrap().clone();

        // Mark the aggregation worker as Idle
        self.workers_pool.mark_worker_with_state(agg_worker_id, WorkerState::Idle).await?;

        // Finalize completed job
        job.final_proof = Some(proof_data.values);
        job.final_verkey = Some(proof_data.verkey);
        job.executed_steps = Some(proof_data.executed_steps);
        job.instances = Some(proof_data.instances);

        job.change_state(JobState::Completed);

        let end_time = Utc::now();

        let phase1_time = job.phase_start_time(&JobPhase::Contributions).unwrap_or_else(|| {
            error!("Missing start time for Phase1 in job {}", job.job_id);
            end_time
        });
        let phase2_time = job.phase_start_time(&JobPhase::Prove).unwrap_or_else(|| {
            error!("Missing start time for Phase2 in job {}", job.job_id);
            end_time
        });
        let phase3_time = job.phase_start_time(&JobPhase::Aggregate).unwrap_or_else(|| {
            error!("Missing start time for Phase3 in job {}", job.job_id);
            end_time
        });

        let phase1_duration = phase2_time.signed_duration_since(phase1_time);
        let phase2_duration = phase3_time.signed_duration_since(phase2_time);
        let phase3_duration = end_time.signed_duration_since(phase3_time);

        info!(
            "[Phase3] WorkerId {} done, phase 3 completed for {} ({:.3}s)",
            agg_worker_id,
            job_id,
            phase3_duration.as_seconds_f32()
        );

        let duration = Duration::from_millis(job.duration_ms.unwrap_or(0));

        let header = format!("[Job] Finished {} successfully ✔", job_id).green();
        let duration_str = format!("Duration: {:.3}s", duration.as_secs_f32()).bold();
        let steps_str = if let Some(executed_steps) = job.executed_steps {
            format!("Steps: {}", Self::format_number_with_dots(executed_steps)).bold()
        } else {
            "Steps: N/A".to_string().red().bold()
        };
        let instances_str = if let Some(instances) = job.instances {
            format!("Instances: {}", Self::format_number_with_dots(instances)).bold()
        } else {
            "Instances: N/A".to_string().red().bold()
        };

        let metadata_str = if job.metadata.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> =
                job.metadata.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
            format!(" {}", pairs.join(", "))
        };

        info!(
            "{} {} ({:.3}s+{:.3}s+{:.3}s) {} {} Capacity: {}{}",
            header,
            duration_str,
            phase1_duration.as_seconds_f32(),
            phase2_duration.as_seconds_f32(),
            phase3_duration.as_seconds_f32(),
            steps_str,
            instances_str,
            job.compute_capacity,
            metadata_str,
        );

        let workers = job.workers.clone();

        if workers.len() > 1 {
            for phase in [JobPhase::Contributions, JobPhase::Prove] {
                if let Some(results) = job.results.get(&phase) {
                    if let Some(start_time) = job.phase_start_time(&phase) {
                        let mut durations_ms: Vec<(WorkerId, i64)> = results
                            .iter()
                            .map(|(worker_id, result)| {
                                let duration = result.end_time.signed_duration_since(start_time);
                                (worker_id.clone(), duration.num_milliseconds())
                            })
                            .collect();

                        if durations_ms.len() > 1 {
                            durations_ms.sort_by_key(|(_, duration)| *duration);

                            let (best_worker, best_duration) = &durations_ms[0];
                            let (worst_worker, worst_duration) = durations_ms.last().unwrap();

                            let avg_duration = durations_ms.iter().map(|(_, d)| d).sum::<i64>()
                                as f64
                                / durations_ms.len() as f64;

                            let diff_percentage = if *best_duration > 0 {
                                ((*worst_duration - *best_duration) as f64 / *best_duration as f64)
                                    * 100.0
                            } else {
                                0.0
                            };

                            info!(
                                "[Job] {:?} Performance for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                                phase,
                                job_id,
                                avg_duration / 1000.0,
                                best_worker,
                                *best_duration as f64 / 1000.0,
                                worst_worker,
                                *worst_duration as f64 / 1000.0,
                                diff_percentage
                            );
                        }

                        // For Phase 1, also show delay, witness, and ASM execution statistics
                        if phase == JobPhase::Contributions && durations_ms.len() > 1 {
                            // Extract delay times (coordinator send to worker start)
                            let mut delays_ms: Vec<(WorkerId, i64)> = results
                                .iter()
                                .filter_map(|(worker_id, result)| {
                                    if let JobResultData::Challenges(contrib) = &result.data {
                                        contrib.task_received_time.map(|task_received| {
                                            let delay =
                                                task_received.signed_duration_since(start_time);
                                            (worker_id.clone(), delay.num_milliseconds().max(0))
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if !delays_ms.is_empty() {
                                delays_ms.sort_by_key(|(_, delay)| *delay);
                                let (best_delay_worker, best_delay) = &delays_ms[0];
                                let (worst_delay_worker, worst_delay) = delays_ms.last().unwrap();
                                let avg_delay = delays_ms.iter().map(|(_, d)| d).sum::<i64>()
                                    as f64
                                    / delays_ms.len() as f64;

                                let delay_diff_percentage = if *best_delay > 0 {
                                    ((*worst_delay - *best_delay) as f64 / *best_delay as f64)
                                        * 100.0
                                } else {
                                    0.0
                                };

                                info!(
                                    "[Job] Contributions Delay for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                                    job_id,
                                    avg_delay / 1000.0,
                                    best_delay_worker,
                                    *best_delay as f64 / 1000.0,
                                    worst_delay_worker,
                                    *worst_delay as f64 / 1000.0,
                                    delay_diff_percentage
                                );
                            }

                            // Extract witness times
                            let mut witness_times: Vec<(WorkerId, f32)> = results
                                .iter()
                                .filter_map(|(worker_id, result)| {
                                    if let JobResultData::Challenges(contrib) = &result.data {
                                        Some((worker_id.clone(), contrib.witness_info.witness_time))
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if !witness_times.is_empty() {
                                witness_times.sort_by(|(_, a), (_, b)| {
                                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                let (best_witness_worker, best_witness) = &witness_times[0];
                                let (worst_witness_worker, worst_witness) =
                                    witness_times.last().unwrap();
                                let avg_witness =
                                    witness_times.iter().map(|(_, t)| *t as f64).sum::<f64>()
                                        / witness_times.len() as f64;

                                let witness_diff_percentage = if *best_witness > 0.0 {
                                    ((*worst_witness - *best_witness) as f64 / *best_witness as f64)
                                        * 100.0
                                } else {
                                    0.0
                                };

                                info!(
                                    "[Job] Contributions Witness for {} - Avg: {:.3}s, Best: {} ({:.3}s), Worst: {} ({:.3}s), Diff: {:.1}%",
                                    job_id,
                                    avg_witness / 1000.0,
                                    best_witness_worker,
                                    *best_witness as f64 / 1000.0,
                                    worst_witness_worker,
                                    *worst_witness as f64 / 1000.0,
                                    witness_diff_percentage
                                );
                            }

                            // Extract ASM execution times
                            let mut asm_times: Vec<(WorkerId, f32, f32)> = results
                                .iter()
                                .filter_map(|(worker_id, result)| {
                                    if let JobResultData::Challenges(contrib) = &result.data {
                                        contrib
                                            .zisk_executor_time
                                            .asm_execution_duration
                                            .as_ref()
                                            .map(|asm| (worker_id.clone(), asm.time, asm.mhz))
                                    } else {
                                        None
                                    }
                                })
                                .collect();

                            if !asm_times.is_empty() {
                                asm_times.sort_by(|(_, a, _), (_, b, _)| {
                                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                                });
                                let (best_asm_worker, best_asm, best_mhz) = &asm_times[0];
                                let (worst_asm_worker, worst_asm, worst_mhz) =
                                    asm_times.last().unwrap();
                                let avg_asm =
                                    asm_times.iter().map(|(_, t, _)| *t as f64).sum::<f64>()
                                        / asm_times.len() as f64;

                                let asm_diff_percentage = if *best_asm > 0.0 {
                                    ((*worst_asm - *best_asm) as f64 / *best_asm as f64) * 100.0
                                } else {
                                    0.0
                                };

                                info!(
                                    "[Job] Contributions ASM for {} - Avg: {:.3}s, Best: {} ({:.3}s @ {:.1}MHz), Worst: {} ({:.3}s @ {:.1}MHz), Diff: {:.1}%",
                                    job_id,
                                    avg_asm,
                                    best_asm_worker,
                                    *best_asm,
                                    *best_mhz,
                                    worst_asm_worker,
                                    *worst_asm,
                                    *worst_mhz,
                                    asm_diff_percentage
                                );
                            }
                        }
                    }
                }
            }
        }

        let duration = Utc::now().signed_duration_since(self.start_time_utc);
        let total_secs = duration.num_seconds().max(0) as u64; // avoid negative durations
        let uptime = humantime::format_duration(Duration::from_secs(total_secs)).to_string();

        info!(
            "[Coordinator] Started at {} UTC — Uptime: {}",
            self.start_time_utc.format("%Y-%m-%d %H:%M:%S"),
            uptime
        );

        info!(
            "[Coordinator] Registrations: {} Reconnections: {}",
            self.registrations.load(Ordering::Relaxed),
            self.reconnections.load(Ordering::Relaxed)
        );

        // Build proof bytes and stats for the event before releasing the lock
        let prove_event = {
            let proof_bytes = match (job.final_proof.as_ref(), job.final_verkey.as_ref()) {
                (Some(final_proof), Some(final_verkey)) => {
                    match ZiskProofWithPublicValues::new_from_vadcop_proof(
                        final_proof,
                        self.config.coordinator.minimal_proofs,
                        final_verkey.clone(),
                    ) {
                        Ok(p) => bincode::serialize(&p).unwrap_or_default(),
                        Err(e) => {
                            warn!("Failed to serialize proof for event on job {}: {}", job_id, e);
                            vec![]
                        }
                    }
                }
                _ => vec![],
            };
            let stats = exec_stats_from_job(&job);
            CoordinatorJobEvent::Completed(CoordinatorJobResult::Prove { proof_bytes, stats })
        };

        // Release job lock before calling post_launch_proof
        drop(job);

        self.fire_job_event(job_id, prove_event).await;

        self.post_launch_proof(job_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use std::collections::BTreeMap;
    use zisk_distributed_common::{
        ComputeCapacity, HintsModeDto, InputsModeDto, Job, JobExecutionMode, JobPhase, JobState,
        PhaseTimings, WorkerState,
    };

    fn test_config_with(overrides: impl FnOnce(&mut Config)) -> Config {
        let mut config = Config::load(None, None, None, true, false, None)
            .expect("Failed to create default test config");
        overrides(&mut config);
        config
    }

    fn create_test_job(workers: &[WorkerId]) -> Job {
        let partitions: Vec<Vec<u32>> =
            workers.iter().enumerate().map(|(i, _)| vec![i as u32]).collect();
        Job::new(
            Default::default(),
            InputsModeDto::InputsNone,
            HintsModeDto::HintsNone,
            ComputeCapacity::from(workers.len() as u32),
            ComputeCapacity::from(1u32),
            workers.to_vec(),
            partitions,
            JobExecutionMode::Standard,
            BTreeMap::new(),
            false,
        )
    }

    /// Helper: create a Coordinator with workers and a Running job inserted.
    async fn setup_coordinator_with_job(
        n_workers: usize,
        phase: JobPhase,
        config_overrides: impl FnOnce(&mut Config),
    ) -> (
        Coordinator,
        Vec<(WorkerId, std::sync::Arc<std::sync::Mutex<Vec<CoordinatorMessageDto>>>)>,
        JobId,
    ) {
        let config = test_config_with(config_overrides);
        let coordinator = Coordinator::new(config);

        let mut workers = Vec::with_capacity(n_workers);
        for i in 0..n_workers {
            let worker_id = WorkerId::from(format!("w{}", i));
            let (sender, messages) = MockMessageSender::new();
            coordinator
                .workers_pool
                .register_worker(worker_id.clone(), 1u32, Box::new(sender))
                .await
                .unwrap();
            workers.push((worker_id, messages));
        }

        let worker_ids: Vec<_> = workers.iter().map(|(id, _)| id.clone()).collect();
        let mut job = create_test_job(&worker_ids);
        job.change_state(JobState::Running(phase.clone()));
        let job_id = job.job_id.clone();

        for wid in &worker_ids {
            coordinator
                .workers_pool
                .mark_worker_with_state(
                    wid,
                    WorkerState::Computing((job_id.clone(), phase.clone())),
                )
                .await
                .unwrap();
        }

        coordinator.jobs.write().await.insert(job_id.clone(), Arc::new(RwLock::new(job)));

        (coordinator, workers, job_id)
    }

    #[tokio::test]
    async fn test_fail_job_is_idempotent() {
        let (coordinator, _workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |_| {}).await;

        // First fail succeeds
        coordinator.fail_job(&job_id, "first").await.unwrap();
        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert_eq!(entry.read().await.state, JobState::Failed);

        // Second fail is a no-op (no panic, returns Ok)
        coordinator.fail_job(&job_id, "second").await.unwrap();
    }

    #[tokio::test]
    async fn test_fail_job_sends_cancellation() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |_| {}).await;

        coordinator.fail_job(&job_id, "test reason").await.unwrap();

        // Both workers should have received at least one JobCancelled message
        for (_, msgs) in &workers {
            let cancellations: usize = msgs
                .lock()
                .unwrap()
                .iter()
                .filter(|m| matches!(m, CoordinatorMessageDto::JobCancelled(_)))
                .count();
            assert!(cancellations >= 1, "Expected at least one JobCancelled message");
        }
    }

    #[tokio::test]
    async fn test_ensure_workers_idle_all_workers() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(3, JobPhase::Contributions, |_| {}).await;

        // Only worker 0 has "results" — but ensure_workers_idle should mark ALL 3 as Idle
        coordinator.fail_job(&job_id, "test").await.unwrap();

        for (wid, _) in &workers {
            let state = coordinator.workers_pool.worker_state(wid).await;
            assert_eq!(state, Some(WorkerState::Idle), "Worker {} should be Idle", wid);
        }
    }

    #[tokio::test]
    async fn test_check_phase_timeouts() {
        let (coordinator, _workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |c| {
                c.coordinator.phase1_timeout_seconds = 300;
            })
            .await;

        // Backdate start_time to 10 minutes ago
        {
            let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
            let mut job = entry.write().await;
            job.phase_timings.insert(
                JobPhase::Contributions,
                PhaseTimings {
                    start_time: Utc::now() - chrono::Duration::seconds(600),
                    end_time: None,
                },
            );
        }

        coordinator.check_phase_timeouts().await;

        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert_eq!(entry.read().await.state, JobState::Failed);
    }

    #[tokio::test]
    async fn test_check_phase_timeouts_no_false_positive() {
        let (coordinator, _workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |c| {
                c.coordinator.phase1_timeout_seconds = 300;
            })
            .await;

        // start_time is fresh (just set) — should NOT timeout
        coordinator.check_phase_timeouts().await;

        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert_eq!(entry.read().await.state, JobState::Running(JobPhase::Contributions),);
    }

    #[tokio::test]
    async fn test_check_stale_heartbeats() {
        let (coordinator, _workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |c| {
                c.coordinator.heartbeat_interval_seconds = 30;
                c.coordinator.heartbeat_max_missed = 3;
            })
            .await;

        // Set worker 0's heartbeat to 100 seconds ago
        let w0 = &_workers[0].0;
        coordinator
            .workers_pool
            .set_last_heartbeat(w0, Utc::now() - chrono::Duration::seconds(100))
            .await
            .unwrap();

        coordinator.check_stale_heartbeats().await;

        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert_eq!(entry.read().await.state, JobState::Failed);
    }

    #[tokio::test]
    async fn test_late_task_response_ignored_for_failed_job() {
        use zisk_distributed_common::{
            ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, ExecutionResultDataDto,
            ZiskExecutorTimeDto,
        };

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |_| {}).await;

        // Fail the job first
        coordinator.fail_job(&job_id, "intentional").await.unwrap();

        // Now simulate a late task response from worker 0
        let w0_id = workers[0].0.clone();
        let late_response = ExecuteTaskResponseDto {
            job_id: job_id.clone(),
            worker_id: w0_id.clone(),
            success: true,
            error_message: None,
            result_data: ExecuteTaskResponseResultDataDto::Execution(ExecutionResultDataDto {
                instances: 1,
                executed_steps: 100,
                zisk_executor_time: ZiskExecutorTimeDto {
                    total_duration: 0.0,
                    execution_duration: 0.0,
                    count_and_plan_duration: 0.0,
                    count_and_plan_mo_duration: 0.0,
                    asm_execution_duration: None,
                    task_received_time: 0.0,
                },
            }),
        };

        // Should succeed (not error) — the late response is silently discarded
        coordinator.handle_stream_execute_task_response(late_response).await.unwrap();

        // Worker should be set to Idle
        let state = coordinator.workers_pool.worker_state(&w0_id).await;
        assert_eq!(state, Some(WorkerState::Idle));

        // Job should still be Failed (not revived)
        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert_eq!(entry.read().await.state, JobState::Failed);
    }

    #[tokio::test]
    async fn test_register_worker_accepts_idle() {
        let config = test_config_with(|_| {});
        let coordinator = Coordinator::new(config);

        let worker_id = WorkerId::from("w-idle".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender))
            .await
            .unwrap();

        // Worker is Idle
        assert_eq!(
            coordinator.workers_pool.worker_state(&worker_id).await,
            Some(WorkerState::Idle)
        );

        // Re-registering an Idle worker should succeed (M4 fix)
        let (sender2, _msgs2) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender2))
            .await
            .unwrap();

        // Worker should still be Idle with incremented generation
        assert_eq!(
            coordinator.workers_pool.worker_state(&worker_id).await,
            Some(WorkerState::Idle)
        );
    }
}
