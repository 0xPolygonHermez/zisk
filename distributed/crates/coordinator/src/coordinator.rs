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
use tracing::{debug, error, info, warn};
use zisk_cluster_common::{
    ComputeCapacity, CoordinatorMessageDto, DataId, HintsModeDto, InputsModeDto, Job,
    JobExecutionMode, JobId, JobPhase, JobResultData, JobState, LaunchProofRequestDto,
    LaunchProofResponseDto, PhaseTimings, ProofKind, SetupProgramDto, StatsCostPerType, WorkerId,
    WorkerState,
};
use zisk_common::{AirInstanceCount, SetupKey, ZiskExecutorTime, ZiskPaths};

struct SetupPendingState {
    pending: HashSet<WorkerId>,
    vks: Vec<(WorkerId, Vec<u8>)>,
    hash_id: String,
    program_name: String,
    with_hints: bool,
    emulator_only: bool,
}

/// Per-program record for setups already applied to the cluster.
#[derive(Clone)]
pub(crate) struct ActiveSetup {
    pub program_name: String,
    pub vk: Vec<u8>,
}

/// Per-job event channel: live broadcast sender plus a one-slot stash for the
/// terminal event so subscribers that arrive after termination can still read
/// the final outcome (status + payload). Only the terminal event is retained —
/// intermediate events are not.
struct JobEventChannel {
    tx: broadcast::Sender<CoordinatorJobEvent>,
    /// Set exactly once when a terminal event fires; remains until the job is
    /// evicted by the retention sweep.
    terminal: Option<CoordinatorJobEvent>,
    /// Wall-clock time the terminal event landed. Drives TTL eviction —
    /// authoritative for both proof jobs (which also have `Job.terminated_at`)
    /// and setup jobs (which don't live in `self.jobs`).
    terminated_at: Option<DateTime<Utc>>,
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

    /// Per-job event channels. Populated on job creation; the live broadcast
    /// sender stays alive across termination so late subscribers don't get
    /// `None`. The terminal event itself is stashed inside the same entry at
    /// termination time so late subscribers can read the final outcome.
    /// Intermediate events are not retained. Entries are evicted by
    /// `cleanup_expired_jobs` on the same TTL as `jobs`.
    job_events: RwLock<HashMap<JobId, JobEventChannel>>,

    /// Tracks in-flight setup jobs: maps job_id to per-job state.
    /// Removed once all workers have acknowledged (or the job is cancelled/failed).
    setup_pending: RwLock<HashMap<JobId, SetupPendingState>>,
    /// All programs that have been set up: maps SetupKey → (program_name, vk).
    /// VK is retained so a follow-up `setup_program` call for an already-set-up
    /// program returns success immediately without re-broadcasting.
    pub(crate) active_setups: RwLock<HashMap<SetupKey, ActiveSetup>>,

    /// Per-job channel senders for gRPC-pushed hints (uri = "grpc://...").
    /// Dropping or sending `None` signals EOF to the relay thread.
    #[allow(clippy::type_complexity)]
    grpc_hints_senders: Arc<RwLock<HashMap<JobId, std::sync::mpsc::Sender<Option<Vec<u8>>>>>>,

    /// Workers parked `SettingUp` while we wait for `WorkerRecoveryComplete`
    /// to confirm they've drained the cancelled job. Decoupled from
    /// `WorkerState` so the intent survives a stream drop + reconnect
    /// (which resets `WorkerState` to `default_state`). The value is the
    /// timestamp the worker was parked, used by the stuck-recovery sweep
    /// to evict workers that never confirm.
    pending_recovery: RwLock<HashMap<WorkerId, DateTime<Utc>>>,
}

/// Bookkeeping captured by `Coordinator::terminate_job` and consumed by
/// `fail_job` / `cancel_job` for terminal-event firing and metrics.
struct TerminationOutcome {
    worker_ids: Vec<WorkerId>,
    phase1_start: Option<DateTime<Utc>>,
}

fn exec_stats_from_job(job: &Job) -> CoordinatorExecutionStats {
    let cost = cost_per_type_from_job(job);
    CoordinatorExecutionStats {
        steps: job.executed_steps.unwrap_or(0),
        duration_nanos: job.duration_ms.unwrap_or(0).saturating_mul(1_000_000),
        main_cost: cost.main_cost,
        opcode_cost: cost.opcode_cost,
        memory_cost: cost.memory_cost,
        precompile_cost: cost.precompile_cost,
        tables_cost: cost.tables_cost,
        other_cost: cost.other_cost,
        executor_time: executor_time_from_job(job),
        plan: plan_from_job(job),
    }
}

/// Extracts the per-AIR instance plan from a job's stored execution result.
/// Only execute jobs carry a plan (the `Execution` result); prove jobs return empty.
fn plan_from_job(job: &Job) -> Vec<AirInstanceCount> {
    job.results
        .get(&JobPhase::Execution)
        .and_then(|m| m.values().next())
        .and_then(|r| match &r.data {
            JobResultData::Execution(e) => Some(e.plan.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Extracts the per-phase executor timing from a job's stored worker results.
/// Mirrors [`cost_per_type_from_job`]: Execute-only jobs carry it on the
/// `Execution` result, prove jobs on the `Challenges` (contributions) result.
fn executor_time_from_job(job: &Job) -> ZiskExecutorTime {
    let from_phase = |phase: &JobPhase| {
        job.results.get(phase).and_then(|m| m.values().next()).and_then(|r| match &r.data {
            JobResultData::Execution(e) => Some(e.zisk_executor_time.clone()),
            JobResultData::Challenges(c) => Some(c.zisk_executor_time.clone()),
            _ => None,
        })
    };
    from_phase(&JobPhase::Execution)
        .or_else(|| from_phase(&JobPhase::Contributions))
        .unwrap_or_default()
}

/// Extracts the per-type execution cost from a job's stored worker results.
///
/// Execute-only jobs store the cost on the `Execution` result; prove jobs carry
/// it on the `Challenges` (contributions) result. The cost reflects the whole
/// program (computed from full execution stats), so any one worker's result is
/// representative.
fn cost_per_type_from_job(job: &Job) -> StatsCostPerType {
    let from_phase = |phase: &JobPhase| {
        job.results.get(phase).and_then(|m| m.values().next()).and_then(|r| match &r.data {
            JobResultData::Execution(e) => Some(e.cost_per_type.clone()),
            JobResultData::Challenges(c) => Some(c.cost_per_type.clone()),
            _ => None,
        })
    };
    from_phase(&JobPhase::Execution)
        .or_else(|| from_phase(&JobPhase::Contributions))
        .unwrap_or_default()
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
            pending_recovery: RwLock::new(HashMap::new()),
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
        self.job_events
            .write()
            .await
            .insert(job_id.clone(), JobEventChannel { tx, terminal: None, terminated_at: None });
    }

    /// Returns a live receiver for the job's event channel, or `None` if the job is unknown.
    pub async fn subscribe_job_events(
        &self,
        job_id: &JobId,
    ) -> Option<broadcast::Receiver<CoordinatorJobEvent>> {
        self.job_events.read().await.get(job_id).map(|chan| chan.tx.subscribe())
    }

    /// Returns a clone of the stashed terminal event for a job, if one was recorded.
    /// Used by API endpoints to read the final terminal outcome (with full result
    /// payload or failure reason) for jobs that have already terminated. Survives
    /// until the job is evicted by the retention sweep.
    pub async fn get_terminal_event(&self, job_id: &JobId) -> Option<CoordinatorJobEvent> {
        self.job_events.read().await.get(job_id).and_then(|chan| chan.terminal.clone())
    }

    /// Fires an event on the job's channel. Drops silently when there are no receivers.
    /// For terminal events, the event is also stashed inside the channel entry so
    /// late subscribers can read it; the entry itself is kept alive (and evicted
    /// later by `cleanup_expired_jobs`).
    async fn fire_job_event(&self, job_id: &JobId, event: CoordinatorJobEvent) {
        let terminal = matches!(
            event,
            CoordinatorJobEvent::Completed(_)
                | CoordinatorJobEvent::Failed(_)
                | CoordinatorJobEvent::Cancelled
        );

        if terminal {
            {
                let mut map = self.job_events.write().await;
                if let Some(chan) = map.get_mut(job_id) {
                    // Skip the broadcast clone when nothing is listening —
                    // terminal payloads (proof bytes) can be large and most
                    // jobs terminate with no live watcher attached.
                    if chan.tx.receiver_count() > 0 {
                        let _ = chan.tx.send(event.clone());
                    }
                    chan.terminal.get_or_insert(event);
                    chan.terminated_at.get_or_insert_with(Utc::now);
                }
            }
            // Dropping the sender signals EOF to any running gRPC hints relay.
            self.grpc_hints_senders.write().await.remove(job_id);
        } else if let Some(chan) = self.job_events.read().await.get(job_id) {
            let _ = chan.tx.send(event);
        }
    }

    /// Cancels a running or queued job.
    ///
    /// Returns `true` if the job was cancelled, `false` if it was already in a terminal state.
    pub async fn cancel_job(&self, job_id: &JobId) -> CoordinatorResult<bool> {
        let Some(outcome) =
            self.terminate_job(job_id, JobState::Cancelled, "cancelled by client").await?
        else {
            return Ok(false);
        };

        self.fire_job_event(job_id, CoordinatorJobEvent::Cancelled).await;
        crate::metrics::record_job_terminal(
            crate::metrics::OUTCOME_CANCELLED,
            &outcome.worker_ids,
            outcome.phase1_start,
        );
        info!("Cancelled job {}", job_id);

        Ok(true)
    }

    /// Shared transition path for `fail_job` and `cancel_job`: flips the job
    /// to a terminal state, parks every still-`Computing` assigned worker in
    /// `SettingUp`, registers them in `pending_recovery`, and dispatches
    /// `JobCancelled`. Returns `None` if the job was already resolved (so the
    /// caller can short-circuit without firing duplicate terminal events).
    ///
    /// Ordering invariant: park workers BEFORE sending `JobCancelled`. The
    /// worker side emits `WorkerRecoveryComplete` in response to
    /// `JobCancelled`; if we sent the message first, that completion could
    /// arrive while the coordinator still saw the worker as `Computing(_)`
    /// and be dropped, wedging the worker once the parking finally lands.
    async fn terminate_job(
        &self,
        job_id: &JobId,
        terminal_state: JobState,
        cancel_reason: &str,
    ) -> CoordinatorResult<Option<TerminationOutcome>> {
        debug_assert!(
            matches!(terminal_state, JobState::Failed | JobState::Cancelled),
            "terminate_job only handles Failed/Cancelled terminal states"
        );

        let jobs_map = self.jobs.read().await;
        let job_entry =
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?;
        drop(jobs_map);

        let (worker_ids, phase1_start) = {
            let mut job = job_entry.write().await;
            if job.state().is_resolved() {
                return Ok(None);
            }
            job.change_state(terminal_state);
            (job.workers.clone(), job.phase_start_time(&JobPhase::Contributions))
        };

        let parked = self.workers_pool.mark_computing_workers_settingup(job_id, &worker_ids).await;
        if !parked.is_empty() {
            let now = Utc::now();
            let mut pending = self.pending_recovery.write().await;
            for wid in &parked {
                pending.entry(wid.clone()).or_insert(now);
            }
        }
        self.cancel_job_workers(&worker_ids, job_id, cancel_reason).await;

        Ok(Some(TerminationOutcome { worker_ids, phase1_start }))
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
    ///
    /// Refuses (with `InvalidRequest`) if any worker is currently
    /// `Computing(_)`. The worker side's `run_setup` operates on the same
    /// prover that's in use for an in-flight task, so racing the two would
    /// corrupt the running job. Operators must wait for active Prove jobs to
    /// finish (or cancel them) before deploying a new program.
    pub async fn setup_program(
        &self,
        hash_id: &str,
        program_name: String,
        with_hints: bool,
        emulator_only: bool,
    ) -> CoordinatorResult<JobId> {
        let job_id = JobId::new();

        // Idempotent fast-path: this exact program is already set up.
        // Fire a synthetic Completed event with the recorded VK and skip
        // worker reservation entirely — safe to call even while a Prove
        // job is running.
        if let Some(setup) = self.active_setups.read().await.get(&SetupKey::new(
            hash_id.to_string(),
            with_hints,
            emulator_only,
        )) {
            let vk = setup.vk.clone();
            self.alloc_job_events(&job_id).await;
            self.fire_job_event(&job_id, CoordinatorJobEvent::Started).await;
            self.fire_job_event(
                &job_id,
                CoordinatorJobEvent::Completed(crate::job_events::CoordinatorJobResult::Setup {
                    vk,
                }),
            )
            .await;
            info!("[Setup] Program {} already set up; returning cached VK", hash_id);
            return Ok(job_id);
        }

        let path = ZiskPaths::global().elf_cache(hash_id);
        let elf_bytes =
            fs::read(&path).map_err(|_| CoordinatorError::ProgramNotFound(hash_id.to_string()))?;

        // Atomic check + reserve under one workers-pool write lock:
        // refuses if any worker is `Computing` or any recovery is pending,
        // and otherwise flips every reachable worker to `SettingUp`. This
        // closes the TOCTOU race where a concurrent `partition_and_reserve`
        // could flip workers `Ready → Computing` between separate check
        // and mark-SettingUp steps.
        let reserved = self.workers_pool.try_reserve_all_for_setup(&self.pending_recovery).await?;

        if reserved.is_empty() {
            return Err(CoordinatorError::InsufficientCapacity);
        }

        // Allocate event channel before sending to workers so subscribers can't miss events.
        self.alloc_job_events(&job_id).await;
        self.fire_job_event(&job_id, CoordinatorJobEvent::Started).await;

        // Track which workers must ACK before the setup is considered complete.
        let pending: HashSet<WorkerId> = reserved.iter().cloned().collect();
        self.setup_pending.write().await.insert(
            job_id.clone(),
            SetupPendingState {
                pending,
                vks: Vec::new(),
                hash_id: hash_id.to_string(),
                program_name: program_name.clone(),
                with_hints,
                emulator_only,
            },
        );

        for worker_id in &reserved {
            let msg = CoordinatorMessageDto::SetupProgram(SetupProgramDto {
                job_id: job_id.as_string(),
                elf_bytes: elf_bytes.clone(),
                hash_id: hash_id.to_string(),
                program_name: program_name.clone(),
                with_hints,
                emulator_only,
            });
            if let Err(e) = self.workers_pool.send_message(worker_id, msg).await {
                warn!("[Setup] Failed to send SetupProgram to worker {}: {}", worker_id, e);
                self.setup_pending.write().await.entry(job_id.clone()).and_modify(|s| {
                    s.pending.remove(worker_id);
                });
                // Best-effort disconnect: if the worker didn't receive the message, it won't ACK and the setup will never complete, so disconnect it to unblock future setups. If the worker is still alive but just flakey, it will re-register and be healthy for the next setup attempt.
                if let Some(gen) = self.workers_pool.connection_generation(worker_id).await {
                    if let Err(de) =
                        self.workers_pool.disconnect_worker_if_generation(worker_id, gen).await
                    {
                        warn!(
                            "[Setup] Failed to disconnect {} after failed send: {}",
                            worker_id, de
                        );
                    }
                }
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
        for (key, setup) in setups {
            let (hash_id, with_hints, emulator_only) =
                (key.hash_id, key.with_hints, key.emulator_only);
            let path = ZiskPaths::global().elf_cache(&hash_id);
            match fs::read(&path) {
                Ok(elf_bytes) => result.push(SetupProgramDto {
                    job_id: JobId::new().as_string(),
                    elf_bytes,
                    hash_id,
                    program_name: setup.program_name,
                    with_hints,
                    emulator_only,
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
        self: &Arc<Self>,
        request: LaunchProofRequestDto,
    ) -> CoordinatorResult<LaunchProofResponseDto> {
        let (requested, minimum) = self.resolve_capacity(&request).await?;

        // Generate job_id up-front so we can atomically reserve workers
        // under the same identity. `create_job` will use this id for both
        // the worker-pool reservation and the Job struct.
        let job_id = JobId::new();

        // Create and configure a new job. Workers are reserved atomically
        // inside `create_job` (flipped to Computing(job_id, Contributions)
        // under the workers-pool write lock); failures inside `create_job`
        // release the reservation before returning.
        let mut job = self
            .create_job(
                job_id.clone(),
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

        // `select_workers_for_execution` can fail in simulation mode if the
        // sim node index is out of range. The reservation has already been
        // made; release it before propagating.
        let active_workers = match self.select_workers_for_execution(&job) {
            Ok(workers) => workers,
            Err(e) => {
                self.workers_pool.release_reservation(&job.workers, &job_id).await;
                return Err(e);
            }
        };

        // Store job in jobs map. From this point, error paths can fail the
        // job via `fail_job` which itself releases worker reservations via
        // `terminate_job` → `mark_computing_workers_settingup` →
        // `pending_recovery`.
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
        if let Err(e) = self.dispatch_contributions_messages(&job, &active_workers).await {
            drop(job);
            // Dispatch flaked (e.g. a worker's channel broke). The job is
            // already stored and workers are `Computing`; fail it now so
            // the canonical recovery path (`terminate_job` →
            // `mark_computing_workers_settingup` → `pending_recovery`)
            // releases them immediately rather than waiting for the
            // monitor's Phase 1 timeout to fire.
            let reason = format!("Failed to dispatch Phase 1 to workers: {}", e);
            let _ = self.fail_job(&job_id, &reason).await;
            return Err(e);
        }

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
        let job_entry = {
            let jobs_map = self.jobs.read().await;
            jobs_map.get(job_id).cloned().ok_or(CoordinatorError::NotFoundOrInaccessible)?
        };
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
        let proof_data = job
            .proof
            .as_ref()
            .and_then(|p| bincode::serde::encode_to_vec(p, bincode::config::standard()).ok());

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
        job_id: JobId,
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
            .partition_and_reserve(
                required_compute_capacity,
                minimal_compute_capacity,
                execution_mode,
                &job_id,
            )
            .await?;

        // Defense in depth: the pool's selection filter is `state == Ready`,
        // and workers in `pending_recovery` are always parked `SettingUp`.
        // The atomic reservation in `partition_and_reserve` makes this
        // collision unreachable in correct operation. If it does fire, it
        // means the state filter and the pending-recovery set are out of
        // sync (a bug); release the reservation and refuse the dispatch.
        {
            let pending = self.pending_recovery.read().await;
            for wid in &selected_workers {
                if pending.contains_key(wid) {
                    error!(
                        "[Dispatch] Worker {} was selected for a new job but is still in \
                         pending_recovery; refusing dispatch. This indicates a pool/recovery \
                         state mismatch.",
                        wid
                    );
                    drop(pending);
                    self.workers_pool.release_reservation(&selected_workers, &job_id).await;
                    return Err(CoordinatorError::InsufficientCapacity);
                }
            }
        }

        if let Some(simulated_node) = simulated_node {
            partitions[0] = partitions[simulated_node as usize].clone();
        }

        Ok(Job::new(
            job_id,
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

    /// Marks a job as failed and cleans up all associated resources.
    ///
    /// Parks every currently-`Computing` worker assigned to the job in
    /// `SettingUp` and adds them to `pending_recovery`. Each worker will
    /// receive `JobCancelled`, tear down its in-flight task, and emit
    /// `WorkerRecoveryComplete` — that signal is what flips the worker back
    /// to `Ready` (see [`handle_stream_recovery_complete`]). Until then the
    /// dispatcher cannot re-task them, which is what prevents a stale
    /// `ExecuteTaskResponse` for the failed job from racing a fresh
    /// `Computing(new_job, _)` state on the same worker.
    ///
    /// # Parameters
    ///
    /// * `job_id` - Identifier of the failing job
    /// * `reason` - Human-readable description of the failure cause
    pub async fn fail_job(&self, job_id: &JobId, reason: impl AsRef<str>) -> CoordinatorResult<()> {
        let reason = reason.as_ref();
        // Idempotent under monitor + worker-error races: returns None when the
        // job was already resolved, so we don't fire duplicate terminal events.
        let Some(outcome) = self.terminate_job(job_id, JobState::Failed, reason).await? else {
            return Ok(());
        };

        self.fire_job_event(job_id, CoordinatorJobEvent::Failed(reason.to_string())).await;
        crate::metrics::record_job_terminal(
            crate::metrics::OUTCOME_FAILURE,
            &outcome.worker_ids,
            outcome.phase1_start,
        );
        error!("Failed job {} (reason: {})", job_id, reason);

        // post_launch_proof may fail (e.g. proof serialization, webhook).
        // Ensure cleanup always runs even if it does.
        if let Err(e) = self.post_launch_proof(job_id).await {
            warn!("post_launch_proof failed for job {}: {} — forcing cleanup", job_id, e);
            let cleanup_entry = {
                let jobs_map = self.jobs.read().await;
                jobs_map.get(job_id).cloned()
            };
            if let Some(job_entry) = cleanup_entry {
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
        self.cleanup_stuck_recovery_workers().await;
        self.cleanup_stale_disconnected_workers().await;
        self.cleanup_expired_jobs().await;
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

    /// Unregisters workers that have been parked in `pending_recovery` for
    /// longer than `stuck_recovery_threshold_seconds`. The worker side caps
    /// its own recovery at 300s, after which it intentionally stops emitting
    /// `WorkerRecoveryComplete` and continues heartbeating — without this
    /// sweep the entry would leak forever and the worker would stay
    /// permanently `SettingUp` from the coordinator's view.
    ///
    /// Setting `stuck_recovery_threshold_seconds = 0` disables the sweep
    /// (operator-action recovery only — the worker stays wedged until
    /// manually unregistered).
    ///
    /// # Race handling
    ///
    /// We do NOT proactively drain `pending_recovery`: doing so loses the
    /// "owes recovery" bookkeeping for workers that are momentarily
    /// `Disconnected` (mid-network-blip), which would let them reconnect as
    /// eligible-for-dispatch when they might still be internally wedged.
    /// Instead, we collect candidate IDs, then re-check per worker that
    /// (a) the entry is still in `pending_recovery` (recovery hasn't raced
    /// ahead) and (b) the worker is still `SettingUp` (Disconnected workers
    /// are left for the stale-disconnected sweep; recovered workers are
    /// left alone). Only if both still hold do we call `unregister_worker`,
    /// which drains the entry as part of its normal cleanup.
    async fn cleanup_stuck_recovery_workers(&self) {
        let threshold_secs = self.config.coordinator.stuck_recovery_threshold_seconds;
        if threshold_secs == 0 {
            return;
        }
        let cutoff = Utc::now() - chrono::Duration::seconds(threshold_secs as i64);

        let candidates: Vec<WorkerId> = {
            let pending = self.pending_recovery.read().await;
            pending
                .iter()
                .filter(|(_, parked_at)| **parked_at <= cutoff)
                .map(|(wid, _)| wid.clone())
                .collect()
        };

        for wid in candidates {
            if !self.pending_recovery.read().await.contains_key(&wid) {
                // Recovery completed between candidate collection and now.
                continue;
            }
            let state = self.workers_pool.worker_state(&wid).await;
            match state {
                Some(WorkerState::SettingUp) => {
                    error!(
                        "[Recovery] Worker {} stuck in pending_recovery > {}s; unregistering",
                        wid, threshold_secs
                    );
                    if let Err(e) = self.unregister_worker(&wid).await {
                        warn!("Failed to unregister stuck-recovery worker {}: {}", wid, e);
                    }
                }
                Some(WorkerState::Disconnected) => {
                    // Disconnected mid-recovery — let the stale-disconnected
                    // sweep reap the worker (and its `pending_recovery`
                    // entry) on its own schedule. Preserving the entry here
                    // means that a reconnect within the disconnect window
                    // still parks the worker `SettingUp`, blocking dispatch
                    // until recovery actually completes.
                    info!(
                        "[Recovery] Worker {} stuck in pending_recovery > {}s but \
                         currently Disconnected; deferring to stale-disconnected sweep",
                        wid, threshold_secs
                    );
                }
                other => {
                    // Ready / Idle / Computing — recovery effectively
                    // completed (or worker was re-tasked); the
                    // `pending_recovery` entry is stale bookkeeping. Drop
                    // it without unregistering.
                    info!(
                        "[Recovery] Worker {} pending_recovery entry expired but worker \
                         is in state {:?}; clearing entry without unregister",
                        wid, other
                    );
                    self.pending_recovery.write().await.remove(&wid);
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

    /// Evicts entries whose terminal event landed more than `job_ttl_seconds`
    /// ago. Drives off `JobEventChannel.terminated_at` so it catches both
    /// proof/execute/wrap jobs (which also live in `self.jobs`) and setup
    /// jobs (which don't — they only have an event channel).
    pub async fn cleanup_expired_jobs(&self) {
        let retention_secs = self.config.coordinator.job_ttl_seconds;
        let cutoff = Utc::now() - chrono::Duration::seconds(retention_secs as i64);

        let expired: Vec<JobId> = {
            let events = self.job_events.read().await;
            events
                .iter()
                .filter_map(|(id, chan)| {
                    chan.terminated_at.filter(|t| *t <= cutoff).map(|_| id.clone())
                })
                .collect()
        };

        if expired.is_empty() {
            return;
        }

        // Phase 2: remove in batches under each map's write lock.
        // Order matches `fire_job_event`: drop auxiliary state first so the canonical
        // `jobs` entry is the last thing to disappear for any racing observer.
        {
            let mut events = self.job_events.write().await;
            for id in &expired {
                events.remove(id);
            }
        }
        {
            let mut senders = self.grpc_hints_senders.write().await;
            for id in &expired {
                senders.remove(id);
            }
        }
        {
            let mut jobs_map = self.jobs.write().await;
            for id in &expired {
                jobs_map.remove(id);
            }
        }

        debug!(
            "[Monitor] Evicted {} job(s) past retention window ({}s)",
            expired.len(),
            retention_secs
        );
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
            JobId::new(),
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
    async fn test_fail_job_parks_all_workers_for_recovery() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(3, JobPhase::Contributions, |_| {}).await;

        // `fail_job` parks every Computing worker in `SettingUp` and seeds
        // `pending_recovery`, so the dispatcher cannot re-task them until
        // each sends `WorkerRecoveryComplete`.
        coordinator.fail_job(&job_id, "test").await.unwrap();

        let pending = coordinator.pending_recovery.read().await;
        for (wid, _) in &workers {
            let state = coordinator.workers_pool.worker_state(wid).await;
            assert_eq!(
                state,
                Some(WorkerState::SettingUp),
                "Worker {} should be SettingUp after fail_job",
                wid
            );
            assert!(
                pending.contains_key(wid),
                "Worker {} should be in pending_recovery after fail_job",
                wid
            );
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
            result_data: Some(ExecuteTaskResponseResultDataDto::Execution(
                ExecutionResultDataDto {
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
                    cost_per_type: StatsCostPerType::default(),
                    plan: Vec::new(),
                },
            )),
            worker_in_recovery: false,
        };

        // Should succeed (not error) — the late response is silently discarded
        coordinator.handle_stream_execute_task_response(late_response).await.unwrap();

        // Worker stays `SettingUp` and in `pending_recovery` until its
        // `WorkerRecoveryComplete` arrives. The late response is purely
        // informational; it does not move worker state.
        let state = coordinator.workers_pool.worker_state(&w0_id).await;
        assert_eq!(state, Some(WorkerState::SettingUp));
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        // Job should still be Failed (not revived)
        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert_eq!(entry.read().await.state, JobState::Failed);
    }

    #[tokio::test]
    async fn test_late_task_response_with_recovery_parks_settingup() {
        use zisk_cluster_common::ExecuteTaskResponseDto;

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(2, JobPhase::Contributions, |_| {}).await;

        coordinator.fail_job(&job_id, "intentional").await.unwrap();

        let w0_id = workers[0].0.clone();
        let late_response = ExecuteTaskResponseDto {
            job_id: job_id.clone(),
            worker_id: w0_id.clone(),
            success: false,
            error_message: Some("contribution failed".into()),
            result_data: None,
            worker_in_recovery: true,
        };

        coordinator.handle_stream_execute_task_response(late_response).await.unwrap();

        let state = coordinator.workers_pool.worker_state(&w0_id).await;
        assert_eq!(state, Some(WorkerState::SettingUp));
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));
    }

    /// A non-KeepComputing reconnect (here last_known_job_id=None, directive=None)
    /// must drop the stale pending_recovery entry rather than rely on a WRC the
    /// worker won't reliably send. A stray late WRC on the new stream is then a
    /// no-op (no pending_recovery record).
    #[tokio::test]
    async fn test_reconnect_without_live_job_clears_pending_recovery() {
        use zisk_cluster_common::WorkerReconnectRequestDto;

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;

        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "task failed").await.unwrap();
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

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
        assert!(!coordinator.pending_recovery.read().await.contains_key(&w0_id));

        // A late WRC is now a no-op since pending_recovery is empty.
        coordinator.handle_stream_recovery_complete(&w0_id).await.unwrap();
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
        assert!(!coordinator.pending_recovery.read().await.contains_key(&w0_id));

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

        coordinator.pending_recovery.write().await.insert(w0_id.clone(), Utc::now());
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
        assert!(!coordinator.pending_recovery.read().await.contains_key(&w0_id));
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
                emulator_only: false,
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
                coordinator.pending_recovery.read().await.contains_key(wid),
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
        assert!(!pending.contains_key(&w0), "non-computing worker must not be in pending_recovery");
        assert!(pending.contains_key(&w1), "computing worker must be in pending_recovery");
    }

    #[tokio::test]
    async fn test_unregister_worker_clears_pending_recovery() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "task failed").await.unwrap();
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        coordinator.unregister_worker(&w0_id).await.unwrap();

        assert!(!coordinator.pending_recovery.read().await.contains_key(&w0_id));
    }

    /// Workers stuck in `pending_recovery` past the configured threshold get
    /// unregistered by the monitor sweep — without this cap, a worker whose
    /// `WorkerRecoveryComplete` is lost (e.g. its own RECOVERY_TIMEOUT fired)
    /// would stay `SettingUp` forever.
    #[tokio::test]
    async fn test_cleanup_stuck_recovery_unregisters_workers() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |c| {
                c.coordinator.stuck_recovery_threshold_seconds = 1;
            })
            .await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "intentional").await.unwrap();
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        // Backdate the park timestamp to before the threshold.
        {
            let mut pending = coordinator.pending_recovery.write().await;
            let entry = pending.get_mut(&w0_id).expect("worker in pending_recovery");
            *entry = Utc::now() - chrono::Duration::seconds(60);
        }

        coordinator.cleanup_stuck_recovery_workers().await;

        assert!(
            !coordinator.pending_recovery.read().await.contains_key(&w0_id),
            "stuck worker must be evicted from pending_recovery"
        );
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            None,
            "stuck worker must be unregistered from the pool"
        );
    }

    /// Fresh entries in `pending_recovery` (within the threshold) survive the
    /// sweep — only the timed-out ones are evicted.
    #[tokio::test]
    async fn test_cleanup_stuck_recovery_skips_fresh_entries() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |c| {
                c.coordinator.stuck_recovery_threshold_seconds = 600;
            })
            .await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "intentional").await.unwrap();

        coordinator.cleanup_stuck_recovery_workers().await;

        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
    }

    /// If `WorkerRecoveryComplete` arrives between candidate collection and
    /// the per-worker re-check, the recovered worker has already left
    /// `SettingUp` — the sweep must not unregister it. It also clears the
    /// stale `pending_recovery` entry so the dispatch-boundary defense
    /// doesn't keep refusing this worker forever.
    #[tokio::test]
    async fn test_cleanup_stuck_recovery_skips_recovered_workers() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |c| {
                c.coordinator.stuck_recovery_threshold_seconds = 1;
            })
            .await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "intentional").await.unwrap();
        {
            let mut pending = coordinator.pending_recovery.write().await;
            let entry = pending.get_mut(&w0_id).expect("worker in pending_recovery");
            *entry = Utc::now() - chrono::Duration::seconds(60);
        }
        // Simulate the race: recovery completed (worker is Ready) before
        // the sweep ran.
        coordinator.workers_pool.mark_worker_with_state(&w0_id, WorkerState::Ready).await.unwrap();

        coordinator.cleanup_stuck_recovery_workers().await;

        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::Ready),
            "recovered worker must not be unregistered"
        );
        assert!(
            !coordinator.pending_recovery.read().await.contains_key(&w0_id),
            "stale pending_recovery entry must be cleared"
        );
    }

    /// `setup_program` returns success immediately when the requested
    /// program is already in `active_setups`, even with workers Computing.
    /// Workers are not disturbed; the caller gets the cached VK.
    #[tokio::test]
    async fn test_setup_program_idempotent_for_active_setup() {
        let (coordinator, workers, _job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();
        let computing_state_before = coordinator.workers_pool.worker_state(&w0_id).await.unwrap();

        // Seed an active_setup record (as if a prior setup completed).
        let hash_id = "cached-hash";
        let cached_vk = vec![0xA, 0xB, 0xC];
        coordinator.active_setups.write().await.insert(
            SetupKey::new(hash_id.to_string(), false, false),
            ActiveSetup { program_name: "p".into(), vk: cached_vk.clone() },
        );

        // setup_program for the SAME (hash_id, with_hints) must succeed
        // without touching the worker — even though the worker is Computing.
        let job_id = coordinator
            .setup_program(hash_id, "p".to_string(), false, false)
            .await
            .expect("idempotent setup_program must succeed");

        // Worker state untouched.
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(computing_state_before)
        );

        // Terminal event must contain the cached VK.
        let terminal =
            coordinator.get_terminal_event(&job_id).await.expect("terminal event must be stashed");
        match terminal {
            CoordinatorJobEvent::Completed(crate::job_events::CoordinatorJobResult::Setup {
                vk,
            }) => {
                assert_eq!(vk, cached_vk);
            }
            other => panic!("expected Completed(Setup), got {other:?}"),
        }
    }

    /// `setup_program` must refuse if any worker is `Computing(_)`. The
    /// worker side's `run_setup` uses the same prover as in-flight tasks;
    /// running both concurrently corrupts the job.
    #[tokio::test]
    async fn test_setup_program_refused_while_workers_computing() {
        use std::fs;

        let (coordinator, workers, _job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        // Sanity: setup_coordinator_with_job leaves the worker in
        // Computing(job, Contributions).
        let w0_id = workers[0].0.clone();
        assert!(matches!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::Computing(_))
        ));

        // Stage a fake ELF so `setup_program` gets past the file read.
        let elf_bytes = b"fake-elf".to_vec();
        let hash_id = coordinator.register_guest_program(elf_bytes).unwrap();
        // Cross-check the ELF cache exists.
        assert!(fs::metadata(zisk_common::ZiskPaths::global().elf_cache(&hash_id)).is_ok());

        let err = coordinator
            .setup_program(&hash_id, "test-program".to_string(), false, false)
            .await
            .expect_err("setup_program must refuse while a worker is Computing");
        assert!(
            matches!(err, CoordinatorError::InvalidRequest(_)),
            "expected InvalidRequest, got {err:?}"
        );

        // Worker state must be untouched — the running job's assignment
        // survives the refused setup attempt.
        assert!(matches!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::Computing(_))
        ));
    }

    /// Concurrent-race regression: a `launch_proof` racing with
    /// `setup_program` for the same workers must not result in `setup_program`
    /// proceeding when the workers have just been reserved for a new job.
    /// With the atomic `try_reserve_all_for_setup`, exactly one of the two
    /// succeeds — either `setup_program` reserves all workers `SettingUp`
    /// (so `launch_proof` returns `InsufficientCapacity`) or `launch_proof`
    /// marks workers `Computing` first (so `setup_program` returns
    /// `InvalidRequest`).
    #[tokio::test]
    async fn test_concurrent_setup_program_vs_launch_proof_serializes() {
        use std::fs;
        use zisk_cluster_common::LaunchProofRequestDto;

        let config = test_config_with(|_| {});
        let coordinator = std::sync::Arc::new(Coordinator::new(config));

        // Register one Ready worker.
        let worker_id = WorkerId::from("w-only".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();

        // Stage an ELF for setup_program.
        let elf_bytes = b"race-test-elf".to_vec();
        let hash_id = coordinator.register_guest_program(elf_bytes).unwrap();
        assert!(fs::metadata(zisk_common::ZiskPaths::global().elf_cache(&hash_id)).is_ok());

        let coord_a = coordinator.clone();
        let coord_b = coordinator.clone();
        let hash_id_a = hash_id.clone();
        let prove_req = LaunchProofRequestDto {
            data_id: "data".to_string().into(),
            hash_id,
            compute_capacity: Some(1),
            minimal_compute_capacity: Some(1),
            inputs_mode: zisk_cluster_common::InputsModeDto::InputsNone,
            hints_mode: zisk_cluster_common::HintsModeDto::HintsNone,
            simulated_node: None,
            metadata: std::collections::BTreeMap::new(),
            execution_only: false,
            proof_type: zisk_cluster_common::ProofKind::VadcopFinal,
        };

        let setup_task = tokio::spawn(async move {
            coord_a.setup_program(&hash_id_a, "race-program".to_string(), false, false).await
        });
        let prove_task = tokio::spawn(async move { coord_b.launch_proof(prove_req).await });
        let (setup_res, prove_res) = tokio::join!(setup_task, prove_task);
        let setup_res = setup_res.unwrap();
        let prove_res = prove_res.unwrap();

        match (&setup_res, &prove_res) {
            // Setup won → worker is SettingUp; launch_proof returns either
            // `InsufficientCapacity` (no Ready capacity at all) or
            // `WorkersSettingUp` (some SettingUp capacity, not enough Ready).
            // Either is fine — both indicate the dispatcher correctly refused.
            (
                Ok(_),
                Err(CoordinatorError::InsufficientCapacity | CoordinatorError::WorkersSettingUp),
            ) => {
                assert_eq!(
                    coordinator.workers_pool.worker_state(&worker_id).await,
                    Some(WorkerState::SettingUp)
                );
            }
            // Launch_proof won → worker is Computing; setup returns InvalidRequest.
            (Err(CoordinatorError::InvalidRequest(_)), Ok(_)) => {
                assert!(matches!(
                    coordinator.workers_pool.worker_state(&worker_id).await,
                    Some(WorkerState::Computing((_, JobPhase::Contributions)))
                ));
            }
            (Ok(_), Ok(_)) => {
                panic!("both setup_program and launch_proof succeeded — double-booking")
            }
            (Err(a), Err(b)) => panic!("both failed: setup={a:?} prove={b:?}"),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => {
                panic!("loser got unexpected error: {e:?}")
            }
        }
    }

    /// `setup_program` must also refuse while a recovery is in flight. A
    /// worker in `pending_recovery` is running `spawn_post_failure_recovery`
    /// on the worker side, which uses the same prover (MPI broadcast +
    /// cluster_barrier) that `run_setup` would call. Concurrent setup would
    /// race against the recovery handshake.
    #[tokio::test]
    async fn test_setup_program_refused_while_workers_recovering() {
        use std::fs;

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        // Park the worker via `fail_job` → SettingUp + pending_recovery.
        coordinator.fail_job(&job_id, "intentional").await.unwrap();
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        // Stage a fake ELF.
        let elf_bytes = b"fake-elf-recovery".to_vec();
        let hash_id = coordinator.register_guest_program(elf_bytes).unwrap();
        assert!(fs::metadata(zisk_common::ZiskPaths::global().elf_cache(&hash_id)).is_ok());

        let err = coordinator
            .setup_program(&hash_id, "test-program".to_string(), false, false)
            .await
            .expect_err("setup_program must refuse while a worker is in pending_recovery");
        assert!(
            matches!(err, CoordinatorError::InvalidRequest(_)),
            "expected InvalidRequest, got {err:?}"
        );

        // Worker state must be untouched.
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));
    }

    /// Regression: completion handlers must not flip worker state to `Ready`
    /// when they observe a resolved job. The original "if Failed → mark Ready"
    /// pattern in `handle_execution_completion` raced with `fail_job`'s
    /// `pending_recovery` parking — the handler would overwrite `SettingUp`
    /// with `Ready` and re-open the dispatcher to a worker that still owed
    /// `WorkerRecoveryComplete`. Direct-call this handler with a Failed job
    /// to lock in the fix.
    #[tokio::test]
    async fn test_execution_completion_on_failed_job_preserves_settingup() {
        use zisk_cluster_common::{
            ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, ExecutionResultDataDto,
            ZiskExecutorTimeDto,
        };

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Execution, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "racing fail").await.unwrap();
        // Sanity: fail_job parked the worker and added to pending_recovery.
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        // Simulate the race by-passing the late-arrival branch and
        // delivering an Execution completion directly to the handler.
        let response = ExecuteTaskResponseDto {
            job_id: job_id.clone(),
            worker_id: w0_id.clone(),
            success: true,
            error_message: None,
            result_data: Some(ExecuteTaskResponseResultDataDto::Execution(
                ExecutionResultDataDto {
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
                    cost_per_type: StatsCostPerType::default(),
                    plan: Vec::new(),
                },
            )),
            worker_in_recovery: false,
        };
        coordinator.handle_execution_completion(response).await.unwrap();

        // Worker MUST stay SettingUp; only `WorkerRecoveryComplete` flips it.
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp),
            "handler must not overwrite SettingUp on resolved job"
        );
        assert!(
            coordinator.pending_recovery.read().await.contains_key(&w0_id),
            "pending_recovery entry must survive a racing completion"
        );
    }

    /// Regression: `handle_wrap_completion` must not flip the worker
    /// `Ready` (overwriting `SettingUp`) when the job has been resolved
    /// concurrently — recovery owns the worker via `pending_recovery`.
    #[tokio::test]
    async fn test_wrap_completion_on_failed_job_preserves_settingup() {
        use zisk_cluster_common::{
            ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, WrapResultDto,
        };

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Aggregate, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "racing fail").await.unwrap();
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        let response = ExecuteTaskResponseDto {
            job_id: job_id.clone(),
            worker_id: w0_id.clone(),
            success: true,
            error_message: None,
            result_data: Some(ExecuteTaskResponseResultDataDto::WrapResult(WrapResultDto {
                proof_data: vec![],
            })),
            worker_in_recovery: false,
        };
        coordinator.handle_wrap_completion(response).await.unwrap();

        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp),
            "handle_wrap_completion must not overwrite SettingUp on resolved job"
        );
        assert!(
            coordinator.pending_recovery.read().await.contains_key(&w0_id),
            "pending_recovery entry must survive racing completion"
        );
    }

    /// Regression: `handle_aggregation_completion`'s final-proof path
    /// must not flip the aggregator `Ready` (overwriting `SettingUp`)
    /// when the job has been resolved concurrently. Previously the
    /// `is_resolved` check was on a separate read lock that was released
    /// before the write lock + `mark_worker_with_state(_, Ready)` —
    /// allowing a racing `fail_job` to park the worker `SettingUp` in
    /// the window, only to have this handler overwrite it.
    #[tokio::test]
    async fn test_aggregation_completion_on_failed_job_preserves_settingup() {
        use zisk_cluster_common::{
            ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, FinalProofDto,
        };

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Aggregate, |_| {}).await;
        let w0_id = workers[0].0.clone();

        // The aggregator path requires `agg_worker_id` to be set on the
        // job. Set it manually to match the worker we'll deliver a result
        // for.
        {
            let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
            entry.write().await.agg_worker_id = Some(w0_id.clone());
        }

        coordinator.fail_job(&job_id, "racing fail").await.unwrap();
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        let response = ExecuteTaskResponseDto {
            job_id: job_id.clone(),
            worker_id: w0_id.clone(),
            success: true,
            error_message: None,
            result_data: Some(ExecuteTaskResponseResultDataDto::FinalProof(FinalProofDto {
                proof_data: vec![1, 2, 3],
                executed_steps: 0,
                instances: 0,
            })),
            worker_in_recovery: false,
        };
        // The handler must short-circuit on `is_resolved` and leave state alone.
        let _ = coordinator.handle_aggregation_completion(response).await;

        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp),
            "aggregator must stay SettingUp on resolved job"
        );
        assert!(
            coordinator.pending_recovery.read().await.contains_key(&w0_id),
            "pending_recovery entry must survive racing aggregation completion"
        );
    }

    /// Regression: a fresh `Register` for a worker that has a stale
    /// `pending_recovery` entry (from a previous incarnation) must clear
    /// the entry — the new worker process won't send `WorkerRecoveryComplete`
    /// for a recovery it doesn't know about, and leaving the entry would
    /// stall dispatch via the setup-ack `pending_recovery` guard until the
    /// stuck-recovery sweep eventually evicted (default 600s).
    #[tokio::test]
    async fn test_fresh_register_clears_stale_pending_recovery() {
        use zisk_cluster_common::WorkerRegisterRequestDto;

        let config = test_config_with(|_| {});
        let coordinator = Coordinator::new(config);

        let worker_id = WorkerId::from("w-restart".to_string());

        // Simulate the previous incarnation: register, then fail_job parks
        // the worker in pending_recovery.
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();
        coordinator.pending_recovery.write().await.insert(worker_id.clone(), Utc::now());
        assert!(coordinator.pending_recovery.read().await.contains_key(&worker_id));

        // Now the worker process restarts and re-registers fresh.
        let (sender, _msgs) = MockMessageSender::new();
        let req = WorkerRegisterRequestDto {
            worker_id: worker_id.clone(),
            compute_capacity: 1u32.into(),
        };
        let (accepted, _msg, _setup) =
            coordinator.handle_stream_registration(req, Box::new(sender)).await;
        assert!(accepted, "fresh register must succeed");

        // The stale pending_recovery entry must be gone — otherwise the
        // setup-ack handler would refuse to flip the worker to Ready and
        // dispatch would stall for the stuck-recovery threshold.
        assert!(
            !coordinator.pending_recovery.read().await.contains_key(&worker_id),
            "stale pending_recovery entry must be cleared on fresh register"
        );
    }

    /// A worker that's Disconnected mid-recovery (network blip) must NOT
    /// have its `pending_recovery` entry drained by the stuck-recovery
    /// sweep — that would let it reconnect as eligible-for-dispatch even
    /// though it never confirmed recovery. The entry is preserved so a
    /// reconnect re-parks the worker `SettingUp`; the stale-disconnected
    /// sweep eventually reaps both the pool entry and the
    /// `pending_recovery` entry together.
    #[tokio::test]
    async fn test_cleanup_stuck_recovery_preserves_disconnected_workers() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |c| {
                c.coordinator.stuck_recovery_threshold_seconds = 1;
            })
            .await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "intentional").await.unwrap();
        {
            let mut pending = coordinator.pending_recovery.write().await;
            let entry = pending.get_mut(&w0_id).expect("worker in pending_recovery");
            *entry = Utc::now() - chrono::Duration::seconds(60);
        }
        // Simulate a mid-recovery disconnect.
        coordinator
            .workers_pool
            .mark_worker_with_state(&w0_id, WorkerState::Disconnected)
            .await
            .unwrap();

        coordinator.cleanup_stuck_recovery_workers().await;

        // pending_recovery entry MUST survive so a reconnect re-parks the
        // worker `SettingUp` (preventing dispatch while still wedged).
        assert!(
            coordinator.pending_recovery.read().await.contains_key(&w0_id),
            "pending_recovery entry must be preserved while worker is Disconnected"
        );
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::Disconnected),
            "worker must remain in pool (Disconnected) for the stale-disconnected sweep to handle"
        );
    }

    /// `stuck_recovery_threshold_seconds = 0` disables the sweep — stuck
    /// workers are NOT unregistered (operator-action recovery mode).
    #[tokio::test]
    async fn test_cleanup_stuck_recovery_disabled_with_zero_threshold() {
        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |c| {
                c.coordinator.stuck_recovery_threshold_seconds = 0;
            })
            .await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "intentional").await.unwrap();
        {
            let mut pending = coordinator.pending_recovery.write().await;
            let entry = pending.get_mut(&w0_id).expect("worker in pending_recovery");
            *entry = Utc::now() - chrono::Duration::seconds(86_400);
        }

        coordinator.cleanup_stuck_recovery_workers().await;

        // Sweep is disabled — worker stays in pending_recovery and in the pool.
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));
        assert_eq!(
            coordinator.workers_pool.worker_state(&w0_id).await,
            Some(WorkerState::SettingUp)
        );
    }

    /// `create_job` must refuse to dispatch a worker that's still in
    /// `pending_recovery`, even if (somehow) the pool's `state == Ready`
    /// filter let it through.
    #[tokio::test]
    async fn test_create_job_refuses_pending_recovery_worker() {
        use zisk_cluster_common::ComputeCapacity;

        let config = test_config_with(|_| {});
        let coordinator = Coordinator::new(config);

        let worker_id = WorkerId::from("w-pending".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();

        // Simulate a stale `pending_recovery` entry while the worker's
        // pool state is (incorrectly) Ready — the bug shape the defense
        // is meant to catch.
        coordinator.pending_recovery.write().await.insert(worker_id.clone(), Utc::now());

        let err = coordinator
            .create_job(
                JobId::new(),
                "data".to_string().into(),
                "hash".to_string(),
                ComputeCapacity::from(1u32),
                ComputeCapacity::from(1u32),
                zisk_cluster_common::InputsModeDto::InputsNone,
                zisk_cluster_common::HintsModeDto::HintsNone,
                None,
                std::collections::BTreeMap::new(),
                false,
                zisk_cluster_common::ProofKind::VadcopFinal,
            )
            .await
            .expect_err("create_job must refuse a pending-recovery worker");
        assert!(
            matches!(err, CoordinatorError::InsufficientCapacity),
            "expected InsufficientCapacity, got {err:?}"
        );
    }

    /// Concurrent dispatch race regression: two `create_job` calls racing
    /// for a pool with capacity for exactly one job. With the
    /// atomic-reserve-at-selection design, one wins (workers Computing) and
    /// the other returns `InsufficientCapacity`. Under the previous
    /// non-atomic design, both could end up selecting the same workers,
    /// silently overwriting each other's `Computing(job_id, _)` state.
    #[tokio::test]
    async fn test_concurrent_create_job_does_not_double_book_workers() {
        use zisk_cluster_common::ComputeCapacity;

        let config = test_config_with(|_| {});
        let coordinator = std::sync::Arc::new(Coordinator::new(config));

        // Register exactly one Ready worker with capacity = 1.
        let worker_id = WorkerId::from("w-only".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();

        let coord_a = coordinator.clone();
        let coord_b = coordinator.clone();
        let task_a = tokio::spawn(async move {
            coord_a
                .create_job(
                    JobId::new(),
                    "data-a".to_string().into(),
                    "hash".to_string(),
                    ComputeCapacity::from(1u32),
                    ComputeCapacity::from(1u32),
                    zisk_cluster_common::InputsModeDto::InputsNone,
                    zisk_cluster_common::HintsModeDto::HintsNone,
                    None,
                    std::collections::BTreeMap::new(),
                    false,
                    zisk_cluster_common::ProofKind::VadcopFinal,
                )
                .await
        });
        let task_b = tokio::spawn(async move {
            coord_b
                .create_job(
                    JobId::new(),
                    "data-b".to_string().into(),
                    "hash".to_string(),
                    ComputeCapacity::from(1u32),
                    ComputeCapacity::from(1u32),
                    zisk_cluster_common::InputsModeDto::InputsNone,
                    zisk_cluster_common::HintsModeDto::HintsNone,
                    None,
                    std::collections::BTreeMap::new(),
                    false,
                    zisk_cluster_common::ProofKind::VadcopFinal,
                )
                .await
        });
        let (ra, rb) = tokio::join!(task_a, task_b);
        let ra = ra.unwrap();
        let rb = rb.unwrap();

        // Exactly one must succeed, exactly one must fail with InsufficientCapacity.
        let (ok, err) = match (&ra, &rb) {
            (Ok(_), Err(_)) => (&ra, &rb),
            (Err(_), Ok(_)) => (&rb, &ra),
            (Ok(_), Ok(_)) => panic!("both create_job calls succeeded — double-booking"),
            (Err(_), Err(_)) => panic!("both create_job calls failed: {:?} / {:?}", ra, rb),
        };
        let job = ok.as_ref().unwrap();
        assert_eq!(job.workers, vec![worker_id.clone()]);
        assert!(
            matches!(err.as_ref().unwrap_err(), CoordinatorError::InsufficientCapacity),
            "loser must get InsufficientCapacity, got {:?}",
            err.as_ref().unwrap_err()
        );

        // The winner's reservation is reflected in the pool: worker is
        // Computing for the winner's job_id.
        match coordinator.workers_pool.worker_state(&worker_id).await {
            Some(WorkerState::Computing((reserved_for, JobPhase::Contributions))) => {
                assert_eq!(reserved_for, job.job_id);
            }
            other => panic!("expected Computing(_, Contributions), got {:?}", other),
        }
    }

    /// Same race-closing property for the wrap path: two concurrent
    /// `launch_wrap` calls racing for a pool with one Ready worker must
    /// produce exactly one success and one `InsufficientCapacity`. Under
    /// the previous non-atomic wrap path (state set Computing AFTER the
    /// manual Ready-search), both could pick the same worker.
    #[tokio::test]
    async fn test_concurrent_launch_wrap_does_not_double_book_workers() {
        use zisk_cluster_common::LaunchWrapRequestDto;

        let config = test_config_with(|_| {});
        let coordinator = std::sync::Arc::new(Coordinator::new(config));

        let worker_id = WorkerId::from("w-only".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();

        let req_a = LaunchWrapRequestDto { proof_data: vec![], proof_dest: 0 };
        let req_b = LaunchWrapRequestDto { proof_data: vec![], proof_dest: 0 };
        let coord_a = coordinator.clone();
        let coord_b = coordinator.clone();
        let task_a = tokio::spawn(async move { coord_a.launch_wrap(req_a).await });
        let task_b = tokio::spawn(async move { coord_b.launch_wrap(req_b).await });
        let (ra, rb) = tokio::join!(task_a, task_b);
        let ra = ra.unwrap();
        let rb = rb.unwrap();

        let winning_job_id = match (ra, rb) {
            (Ok(resp), Err(CoordinatorError::InsufficientCapacity)) => resp.job_id,
            (Err(CoordinatorError::InsufficientCapacity), Ok(resp)) => resp.job_id,
            (Ok(_), Ok(_)) => panic!("both launch_wrap calls succeeded — double-booking"),
            (Err(a), Err(b)) => panic!("both launch_wrap calls failed: {a:?} / {b:?}"),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => {
                panic!("loser must get InsufficientCapacity, got {e:?}")
            }
        };

        // The winner's reservation is reflected in the pool: worker is
        // Computing for the winner's job_id in the Aggregate phase.
        match coordinator.workers_pool.worker_state(&worker_id).await {
            Some(WorkerState::Computing((reserved_for, JobPhase::Aggregate))) => {
                assert_eq!(reserved_for, winning_job_id);
            }
            other => panic!("expected Computing(_, Aggregate), got {:?}", other),
        }
    }

    /// Regression: when a worker disconnects mid-setup, the setup must NOT
    /// hang waiting for an ack that can never arrive (the worker's gRPC
    /// stream is dead; a reconnected process gets a fresh job_id from
    /// `active_setups`). Without this cleanup, watchers on the setup
    /// `job_id` would never receive a terminal event and the
    /// `setup_pending` entry would leak.
    #[tokio::test]
    async fn test_disconnect_mid_setup_finalizes_setup_pending() {
        use std::collections::HashSet;

        let config = test_config_with(|_| {});
        let coordinator = Coordinator::new(config);

        let w0 = WorkerId::from("w0".to_string());
        let w1 = WorkerId::from("w1".to_string());
        let (s0, _m0) = MockMessageSender::new();
        let (s1, _m1) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(w0.clone(), 1u32, Box::new(s0), WorkerState::SettingUp)
            .await
            .unwrap();
        coordinator
            .workers_pool
            .register_worker(w1.clone(), 1u32, Box::new(s1), WorkerState::SettingUp)
            .await
            .unwrap();

        // Seed an in-flight setup waiting on both workers.
        let setup_job = JobId::new();
        let mut pending: HashSet<WorkerId> = HashSet::new();
        pending.insert(w0.clone());
        pending.insert(w1.clone());
        coordinator.setup_pending.write().await.insert(
            setup_job.clone(),
            SetupPendingState {
                pending,
                vks: Vec::new(),
                hash_id: "h".into(),
                program_name: "p".into(),
                with_hints: false,
                emulator_only: false,
            },
        );
        coordinator.alloc_job_events(&setup_job).await;
        let mut rx = coordinator.subscribe_job_events(&setup_job).await.unwrap();

        // w0 successfully acks before its peer is lost.
        coordinator
            .handle_stream_setup_program_ack(zisk_cluster_common::SetupProgramAckDto {
                job_id: setup_job.as_string(),
                worker_id: w0.clone(),
                hash_id: "h".into(),
                success: true,
                error_message: None,
                vk: vec![1, 2, 3],
            })
            .await
            .unwrap();

        // Sanity: setup still pending on w1.
        assert!(coordinator.setup_pending.read().await.contains_key(&setup_job));

        // w1's stream dies — disconnect_worker must drain it from setup_pending.
        coordinator.disconnect_worker(&w1).await.unwrap();

        // setup_pending entry must be gone.
        assert!(
            !coordinator.setup_pending.read().await.contains_key(&setup_job),
            "setup_pending entry must be finalized when its last pending worker is lost"
        );

        // Watcher must receive a terminal event (Completed with w0's VK,
        // since w0 was the only successful ACK).
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("terminal event must fire within timeout")
            .expect("event channel must deliver");
        assert!(
            matches!(event, CoordinatorJobEvent::Completed(_)),
            "setup must complete with w0's VK, got {event:?}"
        );
    }

    /// `unregister_worker` (stuck-recovery sweep) must also drain
    /// `setup_pending` — otherwise the evicted worker holds the entry
    /// hostage forever.
    #[tokio::test]
    async fn test_unregister_worker_drains_setup_pending() {
        use std::collections::HashSet;

        let config = test_config_with(|_| {});
        let coordinator = Coordinator::new(config);

        let w0 = WorkerId::from("w0".to_string());
        let (s0, _m0) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(w0.clone(), 1u32, Box::new(s0), WorkerState::SettingUp)
            .await
            .unwrap();

        let setup_job = JobId::new();
        let mut pending: HashSet<WorkerId> = HashSet::new();
        pending.insert(w0.clone());
        coordinator.setup_pending.write().await.insert(
            setup_job.clone(),
            SetupPendingState {
                pending,
                vks: Vec::new(),
                hash_id: "h".into(),
                program_name: "p".into(),
                with_hints: false,
                emulator_only: false,
            },
        );
        coordinator.alloc_job_events(&setup_job).await;

        coordinator.unregister_worker(&w0).await.unwrap();

        assert!(
            !coordinator.setup_pending.read().await.contains_key(&setup_job),
            "setup_pending entry must be drained when its only worker is unregistered"
        );
    }

    /// Regression: `fail_job(A)` must NOT park workers that were freed from
    /// Job A (e.g. via `resolve_aggregator_assignment` after Phase 2) and
    /// subsequently reassigned to a different live Job B. Under the previous
    /// `mark_computing_workers_settingup` that ignored job_id, terminating
    /// Job A would clobber `Computing(B, _)` → `SettingUp` and add the
    /// worker to `pending_recovery`, wedging Job B (the worker side
    /// filters `JobCancelled` by its own current_job and would never
    /// emit a matching `WorkerRecoveryComplete` for Job A).
    #[tokio::test]
    async fn test_fail_job_does_not_clobber_workers_reassigned_to_other_job() {
        let (coordinator, workers, job_a) =
            setup_coordinator_with_job(2, JobPhase::Prove, |_| {}).await;
        let w0 = workers[0].0.clone();
        let w1 = workers[1].0.clone();

        // Simulate w1 having been freed from Job A's Phase 2 (the
        // non-aggregator path in `resolve_aggregator_assignment` marks the
        // worker Ready) and then picked up by Job B's reservation.
        let job_b = JobId::new();
        coordinator
            .workers_pool
            .mark_worker_with_state(
                &w1,
                WorkerState::Computing((job_b.clone(), JobPhase::Contributions)),
            )
            .await
            .unwrap();

        coordinator.fail_job(&job_a, "aggregator died").await.unwrap();

        // w0 was the still-computing worker for Job A — must be parked.
        assert_eq!(coordinator.workers_pool.worker_state(&w0).await, Some(WorkerState::SettingUp));
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0));

        // w1 belongs to Job B now — Job A's failure must NOT touch it.
        assert_eq!(
            coordinator.workers_pool.worker_state(&w1).await,
            Some(WorkerState::Computing((job_b, JobPhase::Contributions))),
            "worker reassigned to a different live job must keep its Computing state"
        );
        assert!(
            !coordinator.pending_recovery.read().await.contains_key(&w1),
            "worker computing for another live job must not be in pending_recovery"
        );
    }

    /// Defensive: a worker must not be able to inject results into / fail a
    /// job it is not assigned to.
    #[tokio::test]
    async fn test_execute_task_response_rejected_for_non_assigned_worker() {
        use zisk_cluster_common::{
            ExecuteTaskResponseDto, ExecuteTaskResponseResultDataDto, ExecutionResultDataDto,
            ZiskExecutorTimeDto,
        };

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Execution, |_| {}).await;
        // Register a second worker that is NOT in the job.
        let intruder = WorkerId::from("w-intruder".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(intruder.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();

        let response = ExecuteTaskResponseDto {
            job_id: job_id.clone(),
            worker_id: intruder.clone(),
            success: true,
            error_message: None,
            result_data: Some(ExecuteTaskResponseResultDataDto::Execution(
                ExecutionResultDataDto {
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
                    cost_per_type: StatsCostPerType::default(),
                    plan: Vec::new(),
                },
            )),
            worker_in_recovery: false,
        };
        let err = coordinator
            .handle_stream_execute_task_response(response)
            .await
            .expect_err("response from non-assigned worker must be rejected");
        assert!(
            matches!(err, CoordinatorError::InvalidRequest(_)),
            "expected InvalidRequest, got {err:?}"
        );

        // Job must still be running — the spurious response must NOT inject
        // a result into the job's results map.
        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        let job = entry.read().await;
        assert!(matches!(job.state, JobState::Running(_)));
        // Worker 0 (legitimate) was not affected.
        assert_eq!(
            coordinator.workers_pool.worker_state(&workers[0].0).await,
            Some(WorkerState::Computing((job_id, JobPhase::Execution)))
        );
    }

    /// Defensive: `WorkerError` from a worker not assigned to the job must
    /// not fail the job.
    #[tokio::test]
    async fn test_worker_error_rejected_for_non_assigned_worker() {
        use zisk_cluster_common::WorkerErrorDto;

        let (coordinator, _workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let intruder = WorkerId::from("w-intruder".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(intruder.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();

        let err = coordinator
            .handle_stream_error(WorkerErrorDto {
                worker_id: intruder,
                job_id: job_id.clone(),
                error_message: "fake".into(),
            })
            .await
            .expect_err("WorkerError from non-assigned worker must be rejected");
        assert!(matches!(err, CoordinatorError::InvalidRequest(_)));

        // Job must still be running.
        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        assert!(matches!(entry.read().await.state, JobState::Running(_)));
    }

    /// Setup ack from a worker not in the pending set must not contribute a
    /// VK to validation (otherwise a stray worker's VK could fail consensus).
    #[tokio::test]
    async fn test_setup_ack_from_non_pending_worker_ignored() {
        use std::collections::HashSet;
        use zisk_cluster_common::SetupProgramAckDto;

        let config = test_config_with(|_| {});
        let coordinator = Coordinator::new(config);

        let w_pending = WorkerId::from("w-pending".to_string());
        let w_intruder = WorkerId::from("w-intruder".to_string());
        for wid in [&w_pending, &w_intruder] {
            let (s, _m) = MockMessageSender::new();
            coordinator
                .workers_pool
                .register_worker(wid.clone(), 1u32, Box::new(s), WorkerState::SettingUp)
                .await
                .unwrap();
        }

        let setup_job = JobId::new();
        let mut pending: HashSet<WorkerId> = HashSet::new();
        pending.insert(w_pending.clone());
        coordinator.setup_pending.write().await.insert(
            setup_job.clone(),
            SetupPendingState {
                pending,
                vks: Vec::new(),
                hash_id: "h".into(),
                program_name: "p".into(),
                with_hints: false,
                emulator_only: false,
            },
        );
        coordinator.alloc_job_events(&setup_job).await;

        // Intruder sends an ack with a DIFFERENT VK. It must not be
        // accumulated — otherwise it would corrupt VK consensus when the
        // real worker acks.
        coordinator
            .handle_stream_setup_program_ack(SetupProgramAckDto {
                job_id: setup_job.as_string(),
                worker_id: w_intruder.clone(),
                hash_id: "h".into(),
                success: true,
                error_message: None,
                vk: vec![0xFF, 0xFF],
            })
            .await
            .unwrap();

        // The real worker acks with the canonical VK.
        coordinator
            .handle_stream_setup_program_ack(SetupProgramAckDto {
                job_id: setup_job.as_string(),
                worker_id: w_pending.clone(),
                hash_id: "h".into(),
                success: true,
                error_message: None,
                vk: vec![1, 2, 3],
            })
            .await
            .unwrap();

        // Setup must complete successfully (the intruder's VK was discarded).
        let terminal = coordinator
            .get_terminal_event(&setup_job)
            .await
            .expect("terminal event must be stashed");
        match terminal {
            CoordinatorJobEvent::Completed(crate::job_events::CoordinatorJobResult::Setup {
                vk,
            }) => {
                assert_eq!(vk, vec![1, 2, 3]);
            }
            other => panic!("expected Completed(Setup), got {other:?}"),
        }
    }

    /// Wrap jobs only record a phase start for `Aggregate`. Without falling
    /// back across all phases, `Job.duration_ms` stays None and webhooks
    /// report a 0-ms duration.
    #[tokio::test]
    async fn test_duration_ms_set_for_wrap_terminal_state() {
        use zisk_cluster_common::LaunchWrapRequestDto;

        let config = test_config_with(|_| {});
        let coordinator = std::sync::Arc::new(Coordinator::new(config));

        let worker_id = WorkerId::from("w-wrap".to_string());
        let (sender, _msgs) = MockMessageSender::new();
        coordinator
            .workers_pool
            .register_worker(worker_id.clone(), 1u32, Box::new(sender), WorkerState::Ready)
            .await
            .unwrap();

        let response = coordinator
            .launch_wrap(LaunchWrapRequestDto { proof_data: vec![], proof_dest: 0 })
            .await
            .unwrap();
        let job_id = response.job_id;

        // Force the wrap's Aggregate-phase start to 1s ago so duration is non-zero.
        {
            let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
            let mut job = entry.write().await;
            job.phase_timings.insert(
                JobPhase::Aggregate,
                PhaseTimings {
                    start_time: Utc::now() - chrono::Duration::seconds(1),
                    end_time: None,
                },
            );
        }

        // Fail the wrap to trigger change_state (duration_ms is set on
        // terminal transition).
        coordinator.fail_job(&job_id, "test").await.unwrap();

        let entry = coordinator.jobs.read().await.get(&job_id).cloned().unwrap();
        let job = entry.read().await;
        let duration_ms = job.duration_ms.expect("wrap must record duration_ms on terminal");
        assert!(
            duration_ms >= 1000,
            "duration_ms must reflect Aggregate phase time, got {duration_ms}"
        );
    }

    /// Setup-only jobs (no entry in `self.jobs`) must still be evicted by
    /// `cleanup_expired_jobs` — their `job_events` channel entries
    /// previously leaked because the sweep only iterated `self.jobs`.
    #[tokio::test]
    async fn test_cleanup_expired_jobs_evicts_setup_event_channels() {
        let config = test_config_with(|c| {
            c.coordinator.job_ttl_seconds = 1;
        });
        let coordinator = Coordinator::new(config);

        let setup_job = JobId::new();
        coordinator.alloc_job_events(&setup_job).await;
        coordinator
            .fire_job_event(
                &setup_job,
                CoordinatorJobEvent::Completed(crate::job_events::CoordinatorJobResult::Setup {
                    vk: vec![0xAB],
                }),
            )
            .await;

        // Backdate the channel's terminated_at past the TTL.
        {
            let mut events = coordinator.job_events.write().await;
            let chan = events.get_mut(&setup_job).unwrap();
            chan.terminated_at = Some(Utc::now() - chrono::Duration::seconds(60));
        }
        assert!(coordinator.job_events.read().await.contains_key(&setup_job));

        coordinator.cleanup_expired_jobs().await;

        assert!(
            !coordinator.job_events.read().await.contains_key(&setup_job),
            "expired setup event channel must be evicted"
        );
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

    #[tokio::test]
    async fn test_setup_ack_drops_stale_pending_recovery() {
        use zisk_cluster_common::SetupProgramAckDto;

        let (coordinator, workers, _) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator
            .workers_pool
            .mark_worker_with_state(&w0_id, WorkerState::SettingUp)
            .await
            .unwrap();
        coordinator.pending_recovery.write().await.insert(w0_id.clone(), Utc::now());

        coordinator
            .handle_stream_setup_program_ack(SetupProgramAckDto {
                job_id: JobId::new().as_string(),
                worker_id: w0_id.clone(),
                hash_id: "h".into(),
                success: true,
                error_message: None,
                vk: vec![1, 2, 3],
            })
            .await
            .unwrap();

        assert_eq!(coordinator.workers_pool.worker_state(&w0_id).await, Some(WorkerState::Ready));
        assert!(!coordinator.pending_recovery.read().await.contains_key(&w0_id));
    }

    #[tokio::test]
    async fn test_reconnect_cancelstalejob_clears_pending_recovery() {
        use zisk_cluster_common::{ReconnectionDirectiveDto, WorkerReconnectRequestDto};

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "simulated").await.unwrap();
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));

        coordinator.workers_pool.disconnect_worker(&w0_id).await.unwrap();

        let (sender, _msgs) = MockMessageSender::new();
        let (accepted, _msg, directive, _setup) = coordinator
            .handle_stream_reconnection(
                WorkerReconnectRequestDto {
                    worker_id: w0_id.clone(),
                    compute_capacity: ComputeCapacity::from(1u32),
                    last_known_job_id: Some(job_id),
                },
                Box::new(sender),
            )
            .await;

        assert!(accepted);
        assert_eq!(directive, Some(ReconnectionDirectiveDto::CancelStaleJob));
        assert!(!coordinator.pending_recovery.read().await.contains_key(&w0_id));
    }

    #[tokio::test]
    async fn test_reconnect_keepcomputing_preserves_pending_recovery() {
        use zisk_cluster_common::{ReconnectionDirectiveDto, WorkerReconnectRequestDto};

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.pending_recovery.write().await.insert(w0_id.clone(), Utc::now());

        let (sender, _msgs) = MockMessageSender::new();
        let (accepted, _msg, directive, _setup) = coordinator
            .handle_stream_reconnection(
                WorkerReconnectRequestDto {
                    worker_id: w0_id.clone(),
                    compute_capacity: ComputeCapacity::from(1u32),
                    last_known_job_id: Some(job_id),
                },
                Box::new(sender),
            )
            .await;

        assert!(accepted);
        assert_eq!(directive, Some(ReconnectionDirectiveDto::KeepComputing));
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));
    }

    /// Integration of Fix 1 + Fix 3 across the full OOM-restart sequence.
    /// fail_job parks the worker; reconnect clears pending_recovery (Fix 1);
    /// a subsequent SetupProgramAck flips it Ready (the path that was wedged
    /// in production). active_setups is left empty here so we manually mark
    /// SettingUp post-reconnect to mimic what `read_all_setup_dtos` does in
    /// production with a seeded setup.
    #[tokio::test]
    async fn test_fail_job_then_reconnect_then_setup_ack_yields_ready() {
        use zisk_cluster_common::{SetupProgramAckDto, WorkerReconnectRequestDto};

        let (coordinator, workers, job_id) =
            setup_coordinator_with_job(1, JobPhase::Contributions, |_| {}).await;
        let w0_id = workers[0].0.clone();

        coordinator.fail_job(&job_id, "simulated OOM").await.unwrap();
        assert!(coordinator.pending_recovery.read().await.contains_key(&w0_id));
        coordinator.workers_pool.disconnect_worker(&w0_id).await.unwrap();

        let (sender, _msgs) = MockMessageSender::new();
        let (accepted, _msg, _directive, _setup) = coordinator
            .handle_stream_reconnection(
                WorkerReconnectRequestDto {
                    worker_id: w0_id.clone(),
                    compute_capacity: ComputeCapacity::from(1u32),
                    last_known_job_id: Some(job_id),
                },
                Box::new(sender),
            )
            .await;
        assert!(accepted);
        assert!(!coordinator.pending_recovery.read().await.contains_key(&w0_id));

        coordinator
            .workers_pool
            .mark_worker_with_state(&w0_id, WorkerState::SettingUp)
            .await
            .unwrap();

        coordinator
            .handle_stream_setup_program_ack(SetupProgramAckDto {
                job_id: JobId::new().as_string(),
                worker_id: w0_id.clone(),
                hash_id: "h".into(),
                success: true,
                error_message: None,
                vk: vec![1, 2, 3],
            })
            .await
            .unwrap();

        assert_eq!(coordinator.workers_pool.worker_state(&w0_id).await, Some(WorkerState::Ready));
    }
}
