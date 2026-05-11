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

pub(crate) mod aggregate;
pub(crate) mod contributions;
pub(crate) mod prove;
pub(crate) mod worker_handlers;
pub(crate) mod wrap;

pub use worker_handlers::MessageSender;

use crate::{
    config::Config,
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    hooks,
    job_events::{CoordinatorExecutionStats, CoordinatorJobEvent},
    WorkersPool,
};
use chrono::{DateTime, Utc};
use std::{
    collections::{HashMap, HashSet},
    fs,
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use zisk_cluster_common::{
    ComputeCapacity, CoordinatorMessageDto, DataId, HintsModeDto, InputsModeDto, Job,
    JobExecutionMode, JobId, JobPhase, JobState, LaunchProofRequestDto, LaunchProofResponseDto,
    PhaseTimings, ProofKind, SetupProgramDto, WorkerId, WorkerState,
};
use zisk_common::{SetupKey, ZiskPaths};

struct SetupPendingState {
    pending: HashSet<WorkerId>,
    vks: Vec<(WorkerId, Vec<u8>)>,
    hash_id: String,
    program_name: String,
    with_hints: bool,
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

    /// Tracks in-flight setup jobs: maps job_id to per-job state.
    /// Removed once all workers have acknowledged (or the job is cancelled/failed).
    setup_pending: RwLock<HashMap<JobId, SetupPendingState>>,
    /// All programs that have been set up: maps SetupKey → program_name.
    /// Two setups for the same program (hints vs. no-hints) coexist as separate entries.
    active_setups: RwLock<HashMap<SetupKey, String>>,

    /// Per-job channel senders for gRPC-pushed hints (uri = "grpc://...").
    /// Dropping or sending `None` signals EOF to the relay thread.
    #[allow(clippy::type_complexity)]
    grpc_hints_senders: Arc<RwLock<HashMap<JobId, std::sync::mpsc::Sender<Option<Vec<u8>>>>>>,

    /// Workers that owe a `WorkerRecoveryComplete`. Decoupled from
    /// `WorkerState` so the intent survives a stream drop + reconnect
    /// (which resets `WorkerState` to `default_state`).
    pending_recovery: RwLock<HashSet<WorkerId>>,
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
            setup_pending: RwLock::new(HashMap::new()),
            active_setups: RwLock::new(HashMap::new()),
            grpc_hints_senders: Arc::new(RwLock::new(HashMap::new())),
            pending_recovery: RwLock::new(HashSet::new()),
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
            // Dropping the sender signals EOF to any running gRPC hints relay.
            self.grpc_hints_senders.write().await.remove(job_id);
        }
    }

    /// Cancels a running or queued job.
    ///
    /// Returns `true` if the job was cancelled, `false` if it was already in a terminal state.
    pub async fn cancel_job(&self, job_id: &JobId) -> CoordinatorResult<bool> {
        let jobs_map = self.jobs.read().await;
        let job_entry =
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        drop(jobs_map);

        let (worker_ids, phase1_start) = {
            let mut job = job_entry.write().await;
            if job.state().is_resolved() {
                return Ok(false);
            }
            job.change_state(JobState::Cancelled);
            (job.workers.clone(), job.phase_start_time(&JobPhase::Contributions))
        };

        // Park first, send JobCancelled second. The worker may emit
        // `WorkerRecoveryComplete` immediately on receipt; if we sent the
        // message before parking, that completion would arrive while the
        // coordinator still saw the worker as `Computing(_)` and be dropped,
        // wedging the worker once the parking finally lands.
        let parked = self.workers_pool.mark_computing_workers_settingup(&worker_ids).await;
        if !parked.is_empty() {
            let mut pending = self.pending_recovery.write().await;
            for wid in &parked {
                pending.insert(wid.clone());
            }
        }
        self.cancel_job_workers(&worker_ids, job_id, "cancelled by client").await;

        self.fire_job_event(job_id, CoordinatorJobEvent::Cancelled).await;

        crate::metrics::record_job_terminal(
            crate::metrics::OUTCOME_CANCELLED,
            &worker_ids,
            phase1_start,
        );

        info!("Cancelled job {}", job_id);

        Ok(true)
    }

    /// Content-addresses ELF bytes with blake3, writes to cache if absent, returns `hash_id`.
    pub fn register_guest_program(&self, elf_bytes: Vec<u8>) -> CoordinatorResult<String> {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&elf_bytes);
        let hash_id = hasher.finalize().to_hex().to_string();

        let path = ZiskPaths::global().elf_cache(&hash_id);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| CoordinatorError::Internal(format!("create cache dir: {e}")))?;
            }
            fs::write(&path, &elf_bytes)
                .map_err(|e| CoordinatorError::Internal(format!("write ELF cache: {e}")))?;
            metrics::gauge!("coordinator_registered_programs_total").increment(1.0);
        }

        Ok(hash_id)
    }

    /// Reads the cached ELF for `hash_id` and broadcasts `SetupProgram` to all connected workers.
    /// Returns a `JobId` that can be used to track completion via `subscribe_job_events`.
    pub async fn setup_program(
        &self,
        hash_id: &str,
        program_name: String,
        with_hints: bool,
    ) -> CoordinatorResult<JobId> {
        let path = ZiskPaths::global().elf_cache(hash_id);
        let elf_bytes =
            fs::read(&path).map_err(|_| CoordinatorError::ProgramNotFound(hash_id.to_string()))?;

        let job_id = JobId::new();
        let workers = self.workers_pool.connected_worker_ids().await;

        if workers.is_empty() {
            return Err(CoordinatorError::InsufficientCapacity);
        }

        // Allocate event channel before sending to workers so subscribers can't miss events.
        self.alloc_job_events(&job_id).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Started).await;

        // Track which workers must ACK before the setup is considered complete.
        let pending: HashSet<WorkerId> = workers.iter().cloned().collect();
        self.setup_pending.write().await.insert(
            job_id.clone(),
            SetupPendingState {
                pending,
                vks: Vec::new(),
                hash_id: hash_id.to_string(),
                program_name: program_name.clone(),
                with_hints,
            },
        );

        for worker_id in &workers {
            let msg = CoordinatorMessageDto::SetupProgram(SetupProgramDto {
                job_id: job_id.as_string(),
                elf_bytes: elf_bytes.clone(),
                hash_id: hash_id.to_string(),
                program_name: program_name.clone(),
                with_hints,
            });
            if let Err(e) = self.workers_pool.send_message(worker_id, msg).await {
                warn!("[Setup] Failed to send SetupProgram to worker {}: {}", worker_id, e);
                // Remove unreachable worker from pending set — don't block on it.
                self.setup_pending.write().await.entry(job_id.clone()).and_modify(|s| {
                    s.pending.remove(worker_id);
                });
            } else {
                // Mark the worker as SettingUp so it is excluded from job assignment
                // until its SetupProgramAck arrives.
                let _ = self
                    .workers_pool
                    .mark_worker_with_state(worker_id, WorkerState::SettingUp)
                    .await;
            }
        }

        // Edge case: all sends failed — complete immediately with failure.
        let should_complete = self
            .setup_pending
            .read()
            .await
            .get(&job_id)
            .map(|s| s.pending.is_empty())
            .unwrap_or(true);
        if should_complete {
            self.setup_pending.write().await.remove(&job_id);
            self.fire_job_event(
                &job_id,
                CoordinatorJobEvent::Failed("all workers unreachable during setup".into()),
            )
            .await;
        }

        Ok(job_id)
    }

    /// Returns all active setups as `SetupProgramDto`s (reading ELF bytes from the on-disk cache).
    /// Used to re-send all programs to reconnecting workers.
    async fn read_all_setup_dtos(&self) -> Vec<SetupProgramDto> {
        let setups = self.active_setups.read().await.clone();
        let mut result = Vec::with_capacity(setups.len());
        for (key, program_name) in setups {
            let (hash_id, with_hints) = (key.hash_id, key.with_hints);
            let path = ZiskPaths::global().elf_cache(&hash_id);
            match fs::read(&path) {
                Ok(elf_bytes) => result.push(SetupProgramDto {
                    job_id: JobId::new().as_string(),
                    elf_bytes,
                    hash_id,
                    program_name,
                    with_hints,
                }),
                Err(e) => warn!("[Setup] Failed to read cached ELF for {}: {}", hash_id, e),
            }
        }
        result
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
        let (requested, minimum) = self.resolve_capacity(&request).await?;

        // Create and configure a new job
        let mut job = self
            .create_job(
                request.data_id.clone(),
                request.hash_id.clone(),
                requested,
                minimum,
                request.inputs_mode,
                request.hints_mode,
                request.simulated_node,
                request.metadata.clone(),
                request.execution_only,
                request.proof_type,
            )
            .await?;

        info!(
            "[Job] Started {} successfully Capacity: {} Workers: {}",
            job.job_id,
            job.compute_capacity,
            job.workers.len(),
        );

        // Initialize job state
        job.change_state(JobState::Running(JobPhase::Contributions));

        // For execution-only jobs, record the Execution phase start time now
        // so the completion handler can compute the correct wall-clock duration.
        if job.execution_only {
            job.phase_timings.insert(
                JobPhase::Execution,
                PhaseTimings { start_time: Utc::now(), end_time: None },
            );
        }

        let job_id = job.job_id.clone();
        let active_workers = self.select_workers_for_execution(&job)?;

        // Store job in jobs map
        let job_arc = Arc::new(RwLock::new(job));
        self.jobs.write().await.insert(job_id.clone(), job_arc.clone());
        self.alloc_job_events(&job_id).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Queued).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Started).await;

        // Increment `coordinator_active_jobs` BEFORE dispatch: even if dispatch
        // fails, the job is already in `self.jobs` map and a later monitor
        // timeout will call `record_job_terminal` (which decrements). Without
        // the matching increment here, the gauge would underflow on the
        // dispatch-failure path.
        crate::metrics::record_job_started();

        let job = job_arc.read().await;
        self.dispatch_contributions_messages(&job, &active_workers).await?;

        info!("[Phase1] Started with {} workers for {}", active_workers.len(), job_id);

        Ok(LaunchProofResponseDto { job_id })
    }

    /// Resolve the compute capacity for an incoming job request.
    pub(crate) async fn resolve_capacity(
        &self,
        request: &LaunchProofRequestDto,
    ) -> CoordinatorResult<(ComputeCapacity, ComputeCapacity)> {
        let requested = &request.compute_capacity;
        let minimum = &request.minimal_compute_capacity;
        let cfg = &self.config.coordinator;

        // Explicit caller constraint: minimum must not exceed requested.
        if let (Some(req), Some(min)) = (requested, minimum) {
            if min > req {
                return Err(CoordinatorError::InvalidArgument(
                    "minimal_compute_capacity must not exceed compute_capacity".to_string(),
                ));
            }
        }

        let available = self.workers_pool.available_compute_capacity().await.compute_units;

        let default_requested =
            if cfg.default_compute_units == 0 { available } else { cfg.default_compute_units };

        let requested_units = requested.unwrap_or(default_requested);
        let minimum_units = minimum.unwrap_or(cfg.min_compute_units);

        // Clamp to available — not an error to ask for more than is free right now.
        let resolved = requested_units.min(available);

        if resolved < minimum_units {
            if self.workers_pool.setting_up_workers().await > 0 {
                return Err(CoordinatorError::WorkersSettingUp);
            }
            if self.workers_pool.idle_workers().await > 0 {
                return Err(CoordinatorError::WorkersNotSetup);
            }
            return Err(CoordinatorError::InsufficientCapacity);
        }

        Ok((ComputeCapacity::from(resolved), ComputeCapacity::from(minimum_units)))
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

        // Check if webhook URL is configured and spawn it in a separate task
        if let Some(webhook_url) = &self.config.coordinator.webhook_url {
            self.send_webhook(webhook_url.clone(), &job);
        }

        let state = job.state.clone();
        drop(job);
        let mut job = job_entry.write().await;

        // Save proof to disk
        if state == JobState::Completed && !self.config.server.no_save_proofs {
            let zisk_proof = job.proof.as_ref().ok_or_else(|| {
                CoordinatorError::Internal(
                    "Proof is missing during post-launch processing".to_string(),
                )
            })?;
            let folder = self.config.server.proofs_dir.clone();
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
        let executed_steps = job.executed_steps;
        let proof_data = job.proof.as_ref().and_then(|p| bincode::serialize(p).ok());

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
                        executed_steps,
                        proof_data.clone(),
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
        hash_id: String,
        required_compute_capacity: ComputeCapacity,
        minimal_compute_capacity: ComputeCapacity,
        inputs_mode: InputsModeDto,
        hints_mode: HintsModeDto,
        simulated_node: Option<u32>,
        metadata: std::collections::BTreeMap<String, String>,
        execution_only: bool,
        proof_type: ProofKind,
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
            hash_id,
            inputs_mode,
            hints_mode,
            required_compute_capacity,
            minimal_compute_capacity,
            selected_workers,
            partitions,
            execution_mode,
            metadata,
            execution_only,
            proof_type,
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

    /// Marks a job as failed and performs and cleans up all associated resources
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the failing job
    /// * `reason` - Human-readable description of the failure cause
    pub async fn fail_job(&self, job_id: &JobId, reason: impl AsRef<str>) -> CoordinatorResult<()> {
        self.fail_job_with_recovery(job_id, reason, None).await
    }

    /// Like `fail_job` but parks `recovering_worker` `SettingUp` until it
    /// emits `WorkerRecoveryComplete`, instead of flipping it `Ready` directly.
    pub async fn fail_job_with_recovery(
        &self,
        job_id: &JobId,
        reason: impl AsRef<str>,
        recovering_worker: Option<&WorkerId>,
    ) -> CoordinatorResult<()> {
        let jobs_map = self.jobs.read().await;
        let job_entry =
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        drop(jobs_map);

        let (worker_ids, phase1_start) = {
            let mut job = job_entry.write().await;

            // Prevent double-fail races (monitor + worker error racing)
            if job.state().is_resolved() {
                return Ok(());
            }

            job.change_state(JobState::Failed);
            (job.workers.clone(), job.phase_start_time(&JobPhase::Contributions))
            // job write lock released here
        };

        // Same ordering rule as `cancel_job`: insert `pending_recovery` and
        // park `recovering_worker` BEFORE sending JobCancelled, otherwise an
        // immediate `WorkerRecoveryComplete` from the worker can race ahead
        // of the parking and be dropped.
        match recovering_worker {
            Some(rec) => {
                self.pending_recovery.write().await.insert(rec.clone());
                self.ensure_workers_ready_except(&worker_ids, rec).await;
            }
            None => self.ensure_workers_ready(&worker_ids).await,
        }
        self.cancel_job_workers(&worker_ids, job_id, reason.as_ref()).await;

        self.fire_job_event(job_id, CoordinatorJobEvent::Failed(reason.as_ref().to_string())).await;

        crate::metrics::record_job_terminal(
            crate::metrics::OUTCOME_FAILURE,
            &worker_ids,
            phase1_start,
        );

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
                    .mark_worker_with_state(candidate_worker_id, WorkerState::Ready)
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

    // MONITOR METHODS
    // ---------------------------------------------------------------

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
        let removed = self
            .workers_pool
            .remove_stale_disconnected(chrono::Duration::seconds(threshold_secs as i64))
            .await;
        if !removed.is_empty() {
            let mut pending = self.pending_recovery.write().await;
            for w in &removed {
                pending.remove(w);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use std::collections::BTreeMap;
    use zisk_cluster_common::{
        ComputeCapacity, HintsModeDto, InputsModeDto, Job, JobExecutionMode, JobPhase, JobState,
        PhaseTimings, WorkerState,
    };

    fn test_config_with(overrides: impl FnOnce(&mut Config)) -> Config {
        let mut config = Config::load(None, None, None, true, None)
            .expect("Failed to create default test config");
        overrides(&mut config);
        config
    }

    fn create_test_job(workers: &[WorkerId]) -> Job {
        let partitions: Vec<Vec<u32>> =
            workers.iter().enumerate().map(|(i, _)| vec![i as u32]).collect();
        Job::new(
            Default::default(),
            String::new(),
            InputsModeDto::InputsNone,
            HintsModeDto::HintsNone,
            ComputeCapacity::from(workers.len() as u32),
            ComputeCapacity::from(1u32),
            workers.to_vec(),
            partitions,
            JobExecutionMode::Standard,
            BTreeMap::new(),
            false,
            ProofKind::VadcopFinal,
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
                .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Idle)
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
    async fn test_ensure_workers_ready_all_workers() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(3, JobPhase::Contributions, |_| {}).await;

        // Only worker 0 has "results" — but ensure_workers_ready should mark ALL 3 as Ready
        coordinator.fail_job(&job_id, "test").await.unwrap();

        for (wid, _) in &workers {
            let state = coordinator.workers_pool.worker_state(wid).await;
            assert_eq!(state, Some(WorkerState::Ready), "Worker {} should be Ready", wid);
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
        use zisk_cluster_common::{
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
                publics: vec![],
            }),
            worker_in_recovery: false,
        };

        // Should succeed (not error) — the late response is silently discarded
        coordinator.handle_stream_execute_task_response(late_response).await.unwrap();

        // Worker should be set back to Ready (setup is still valid after job failure)
        let state = coordinator.workers_pool.worker_state(&w0_id).await;
        assert_eq!(state, Some(WorkerState::Ready));

        // Job should still be Failed (not revived)
        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert_eq!(entry.read().await.state, JobState::Failed);
    }

    #[tokio::test]
    async fn test_late_task_response_with_recovery_parks_settingup() {
        use zisk_cluster_common::{
            ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, ExecutionResultDataDto,
            ZiskExecutorTimeDto,
        };

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |_| {}).await;

        coordinator.fail_job(&job_id, "intentional").await.unwrap();

        let w0_id = workers[0].0.clone();
        let late_response = ExecuteTaskResponseDto {
            job_id: job_id.clone(),
            worker_id: w0_id.clone(),
            success: false,
            error_message: Some("contribution failed".into()),
            result_data: ExecuteTaskResponseResultDataDto::Execution(ExecutionResultDataDto {
                instances: 0,
                executed_steps: 0,
                zisk_executor_time: ZiskExecutorTimeDto {
                    total_duration: 0.0,
                    execution_duration: 0.0,
                    count_and_plan_duration: 0.0,
                    count_and_plan_mo_duration: 0.0,
                    asm_execution_duration: None,
                    task_received_time: 0.0,
                },
                publics: vec![],
            }),
            worker_in_recovery: true,
        };

        coordinator.handle_stream_execute_task_response(late_response).await.unwrap();

        let state = coordinator.workers_pool.worker_state(&w0_id).await;
        assert_eq!(state, Some(WorkerState::SettingUp));
        assert!(coordinator.pending_recovery.read().await.contains(&w0_id));
    }

    #[tokio::test]
    async fn test_recovery_complete_survives_reconnect() {
        use zisk_cluster_common::WorkerReconnectRequestDto;

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;

        let w0_id = workers[0].0.clone();

        coordinator.fail_job_with_recovery(&job_id, "task failed", Some(&w0_id)).await.unwrap();
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
        assert!(coordinator.pending_recovery.read().await.contains(&w0_id));

        coordinator.workers_pool.disconnect_worker(&w0_id).await.unwrap();

        let (sender2, _msgs2) = MockMessageSender::new();
        let req = WorkerReconnectRequestDto {
            worker_id: w0_id.clone(),
            compute_capacity: 1u32.into(),
            last_known_job_id: None,
        };
        let (accepted, _msg, _directive, _setup) =
            coordinator.handle_stream_reconnection(req, Box::new(sender2)).await;
        assert!(accepted);

        // Reconnect must preserve the recovery intent.
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );

        coordinator.handle_stream_recovery_complete(&w0_id).await.unwrap();
        assert_eq!(coordinator.workers_pool.worker_state(&w0_id).await, Some(WorkerState::Ready));
        assert!(!coordinator.pending_recovery.read().await.contains(&w0_id));
    }

    /// `WorkerRecoveryComplete` arriving on the new stream before the failure
    /// response lands on the old stream must still flip the worker Ready.
    #[tokio::test]
    async fn test_recovery_complete_handles_cross_stream_race() {
        let (coordinator, workers, _job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator
            .workers_pool
            .mark_worker_with_state(&w0_id, WorkerState::SettingUp)
            .await
            .unwrap();
        assert!(!coordinator.pending_recovery.read().await.contains(&w0_id));

        coordinator.handle_stream_recovery_complete(&w0_id).await.unwrap();
        assert_eq!(coordinator.workers_pool.worker_state(&w0_id).await, Some(WorkerState::Ready));
    }

    /// `WorkerRecoveryComplete` must not clobber a `Computing(_)` state — a
    /// re-dispatched worker is owned by the dispatcher.
    #[tokio::test]
    async fn test_recovery_complete_does_not_clobber_computing() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.pending_recovery.write().await.insert(w0_id.clone());
        coordinator
            .workers_pool
            .mark_worker_with_state(
                &w0_id,
                WorkerState::Computing((job_id.clone(), JobPhase::Prove)),
            )
            .await
            .unwrap();

        coordinator.handle_stream_recovery_complete(&w0_id).await.unwrap();

        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::Computing((job_id, JobPhase::Prove)))
        );
        assert!(!coordinator.pending_recovery.read().await.contains(&w0_id));
    }

    /// A stray `WorkerRecoveryComplete` with no `pending_recovery` record
    /// must not pre-empt an in-flight `SetupProgramAck`.
    #[tokio::test]
    async fn test_recovery_complete_yields_to_setup_in_flight() {
        let (coordinator, workers, _job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator
            .workers_pool
            .mark_worker_with_state(&w0_id, WorkerState::SettingUp)
            .await
            .unwrap();
        let setup_job_id = JobId::new();
        coordinator.setup_pending.write().await.insert(
            setup_job_id,
            SetupPendingState {
                pending: [w0_id.clone()].into_iter().collect(),
                vks: Vec::new(),
                hash_id: "h".into(),
                program_name: "p".into(),
                with_hints: false,
            },
        );

        coordinator.handle_stream_recovery_complete(&w0_id).await.unwrap();

        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
    }

    #[tokio::test]
    async fn test_cancel_job_populates_pending_recovery() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |_| {}).await;

        let w_ids: Vec<_> = workers.iter().map(|(id, _)| id.clone()).collect();

        coordinator.cancel_job(&job_id).await.unwrap();

        for wid in &w_ids {
            assert_eq!(
                coordinator.workers_pool.worker_state(wid).await,
                Some(WorkerState::SettingUp)
            );
            assert!(
                coordinator.pending_recovery.read().await.contains(wid),
                "worker {} must be in pending_recovery after cancel",
                wid
            );
        }
    }

    /// Non-Computing workers don't owe a `WorkerRecoveryComplete` — they
    /// must not end up in `pending_recovery`.
    #[tokio::test]
    async fn test_cancel_job_skips_non_computing_workers() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |_| {}).await;
        let w0 = workers[0].0.clone();
        let w1 = workers[1].0.clone();

        coordinator.workers_pool.mark_worker_with_state(&w0, WorkerState::Ready).await.unwrap();

        coordinator.cancel_job(&job_id).await.unwrap();

        let pending = coordinator.pending_recovery.read().await;
        assert!(!pending.contains(&w0), "non-computing worker must not be in pending_recovery");
        assert!(pending.contains(&w1), "computing worker must be in pending_recovery");
    }

    #[tokio::test]
    async fn test_unregister_worker_clears_pending_recovery() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job_with_recovery(&job_id, "task failed", Some(&w0_id)).await.unwrap();
        assert!(coordinator.pending_recovery.read().await.contains(&w0_id));

        coordinator.unregister_worker(&w0_id).await.unwrap();

        assert!(!coordinator.pending_recovery.read().await.contains(&w0_id));
    }

    #[tokio::test]
    async fn test_register_worker_accepts_idle() {
        let config = test_config_with(|_| {});
        let coordinator = Coordinator::new(config);

        let worker_id = WorkerId::from("w-idle".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Idle)
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
            .register_worker(worker_id.clone(), 1u32, Box::new(sender2), WorkerState::Idle)
            .await
            .unwrap();

        // Worker should still be Idle with incremented generation
        assert_eq!(
            coordinator.workers_pool.worker_state(&worker_id).await,
            Some(WorkerState::Idle)
        );
    }
}
