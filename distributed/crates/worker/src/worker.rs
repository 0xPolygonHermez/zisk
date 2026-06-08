use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use proofman::{AggProofs, AggProofsRegister, ContributionsInfo};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use zisk_cluster_common::{AggregationParams, DataCtx, InputSourceDto, JobPhase, WorkerState};
use zisk_cluster_common::{ContributionsMessage, ProveMessage};
use zisk_cluster_common::{HintsSourceDto, StreamDataDto, StreamMessageKind};
use zisk_cluster_common::{JobId, PartitionInfo};
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::{ProgramVK, Proof, ProofKind, SetupKey, ZiskExecutorTime, ZiskPaths};
use zisk_prover_backend::GuestProgram;
use zisk_prover_backend::{
    Asm, AsmOptions, BackendProverOpts, Emu, ProverClientBuilder, ProverEngine, ZiskBackend,
    ZiskProver,
};

use crate::stream_ordering::StreamOrderingActor;
use crate::worker_node::run_recovery;

use proofman::ProvePhaseInputs;
use proofman::WitnessInfo;
use proofman_common::ProofOptions;
use proofman_common::{json_to_debug_instances_map, DebugInfo};
use std::path::PathBuf;
use tracing::{error, info, warn};

use crate::config::ProverServiceConfigDto;

#[derive(BorshSerialize, BorshDeserialize)]
struct SetupMessage {
    hash_id: String,
    program_name: String,
    elf_bytes: Vec<u8>,
    with_hints: bool,
    emulator_only: bool,
}

/// Tag byte used as the first byte of every MPI broadcast message.
///
/// Variants must stay in this order (Borsh encodes variant index, not the repr value).
/// The first six entries intentionally mirror `JobPhase` so that existing messages
/// remain wire-compatible; `Setup` is only used for the worker-internal setup broadcast
/// and has no meaning in the coordinator's `JobPhase`.
#[repr(u8)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq)]
pub(crate) enum WorkerMpiTag {
    Execution,
    Contributions,
    Prove,
    Aggregate,
    ContributionsInputsStream,
    ContributionsHintsStream,
    Setup,
}

/// Timeout for waiting for the stream-ordering actor to finish its current
/// `process_hints` call when shutting it down between proves.
const STREAM_ACTOR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

/// Run `body` inside `catch_unwind`; on unwind, log and invoke `on_panic`.
/// Each `spawn_blocking` compute body uses this so a guest panic surfaces as
/// a failure `LoopEvent` instead of silently killing the worker thread.
fn run_panic_guarded<B, P>(label: &str, job_id: &JobId, body: B, on_panic: P)
where
    B: FnOnce(),
    P: FnOnce(),
{
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
    if outcome.is_err() {
        error!("{label} task panicked for {job_id}; emitting failure result");
        on_panic();
    }
}

/// Result from computation tasks
#[derive(Debug)]
pub enum ComputationResult {
    /// Execution-only task (no proof generation)
    Execution {
        job_id: JobId,
        success: bool,
        result: Result<(WitnessInfo, ZiskExecutorTime, u64, u64)>, // (witness_info, exec_time, instances, executed_steps)
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    },
    /// Partial contribution with challenges
    Contribution {
        job_id: JobId,
        success: bool,
        result: Result<(WitnessInfo, ZiskExecutorTime, Vec<ContributionsInfo>, u64)>,
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    },
    Proofs {
        job_id: JobId,
        success: bool,
        result: Result<Vec<AggProofs>>,
    },
    AggProof {
        job_id: JobId,
        success: bool,
        result: Result<Option<Vec<Vec<u64>>>>,
        executed_steps: u64,
        proof_type: ProofKind,
        instances: u64,
    },
}

/// Events driving the worker event loop. Compute results and recovery
/// completions share one channel — same lifetime, single source of truth.
#[derive(Debug)]
pub enum LoopEvent {
    Computation(ComputationResult),
    RecoveryComplete(zisk_cluster_api::WorkerRecoveryComplete),
}

/// Typed sender so call sites read `send_computation` / `send_recovery_complete`
/// rather than wrapping variants by hand.
#[derive(Clone)]
pub struct LoopEventSender(mpsc::UnboundedSender<LoopEvent>);

/// Zero-sized send error: callers discard the payload, so we don't carry the
/// 600-byte `LoopEvent` around just to retrieve it.
#[derive(Debug)]
pub struct LoopChannelClosed;

impl std::fmt::Display for LoopChannelClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("worker event loop channel closed")
    }
}

impl std::error::Error for LoopChannelClosed {}

impl LoopEventSender {
    pub fn new(tx: mpsc::UnboundedSender<LoopEvent>) -> Self {
        Self(tx)
    }

    pub fn send_computation(&self, result: ComputationResult) -> Result<(), LoopChannelClosed> {
        self.0.send(LoopEvent::Computation(result)).map_err(|_| LoopChannelClosed)
    }

    pub fn send_recovery_complete(
        &self,
        rc: zisk_cluster_api::WorkerRecoveryComplete,
    ) -> Result<(), LoopChannelClosed> {
        self.0.send(LoopEvent::RecoveryComplete(rc)).map_err(|_| LoopChannelClosed)
    }
}

pub struct ProverConfig {
    /// Flag indicating whether to use the prebuilt emulator
    pub emulator: bool,

    /// Path to the proving key
    pub proving_key: PathBuf,

    /// Path to the PLONK proving key
    pub proving_key_snark: Option<PathBuf>,

    /// Verbosity level for logging
    pub verbose: u8,

    /// Debug information
    pub debug_info: DebugInfo,

    /// Additional options for the ASM runner
    // pub asm_runner_options: AsmRunnerOptions,

    /// Flag to unlock mapped memory
    pub unlock_mapped_memory: bool,

    /// Flag to redirect ASM emulator output to file
    pub asm_out_file: bool,

    /// Whether to use minimal memory mode
    pub minimal_memory: bool,

    /// Enable GPU acceleration
    pub gpu: bool,

    /// Enable PLONK proofs
    pub plonk: bool,

    /// Whether to preload PLONK proving key and verification key into the prover service on startup (only applies if `plonk` is true)
    pub preload_plonk: bool,

    /// Maximum number of GPU streams
    pub max_streams: Option<usize>,

    /// Number of threads for witness computation
    pub number_threads_witness: Option<usize>,

    /// Maximum witness buffers stored in memory
    pub max_witness_stored: Option<usize>,
}

impl ProverConfig {
    pub fn load(prover_service_config: ProverServiceConfigDto) -> Result<Self> {
        let proving_key = ZiskPaths::get_proving_key(prover_service_config.proving_key.as_ref());
        let proving_key_snark = if prover_service_config.plonk {
            Some(ZiskPaths::get_proving_key_snark(prover_service_config.proving_key_snark.as_ref()))
        } else {
            None
        };
        let debug_info = match &prover_service_config.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(proving_key.clone(), debug_value.clone())?
            }
        };

        let preload_plonk = prover_service_config.plonk && prover_service_config.preload_plonk;

        let emulator =
            if cfg!(target_os = "macos") { true } else { prover_service_config.emulator };

        Ok(ProverConfig {
            emulator,
            proving_key,
            proving_key_snark,
            verbose: prover_service_config.verbose,
            debug_info,
            unlock_mapped_memory: prover_service_config.unlock_mapped_memory,
            asm_out_file: prover_service_config.asm_out_file,
            minimal_memory: prover_service_config.minimal_memory,
            gpu: prover_service_config.gpu,
            max_streams: prover_service_config.max_streams,
            number_threads_witness: prover_service_config.number_threads_witness,
            max_witness_stored: prover_service_config.max_witness_stored,
            plonk: prover_service_config.plonk,
            preload_plonk,
        })
    }
}

/// Current job context
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: JobId,
    pub hash_id: String,
    pub data_ctx: DataCtx,
    pub rank_id: u32,
    pub total_workers: u32,
    pub allocation: Vec<u32>, // Worker allocation for this job, vector of all computed units assigned
    pub total_compute_units: u32, // Total compute units for the whole job
    pub phase: JobPhase,
    pub executed_steps: u64,
    pub instances: u64,
    pub task_received_time: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct Worker<T: ZiskBackend + 'static> {
    state: WorkerState,
    current_job: Option<Arc<Mutex<JobContext>>>,
    current_computation: Option<JoinHandle<()>>,

    /// MPI peer-rank task handle. Held across `handle_mpi_broadcast_request`
    /// iterations so the loop can keep receiving stream broadcasts (input
    /// data, hints) WHILE the task runs — without this, rank 1 would block
    /// inside `handle_mpi_broadcast_request.await` and never consume the
    /// streamed input its ASM child needs to make progress.
    current_mpi_task: Option<JoinHandle<()>>,

    prover: Arc<ZiskProver<T>>,
    prover_config: ProverConfig,

    stream_actor: Option<StreamOrderingActor>,
    /// All set-up programs, keyed by hash_id. Supports multiple concurrent programs.
    guest_programs: HashMap<String, Arc<GuestProgram>>,
    /// Two setups for the same program (one with hints, one without) coexist independently.
    program_vks: HashMap<SetupKey, ProgramVK>,
}

impl<T: ZiskBackend + 'static> Worker<T> {
    pub fn new_emu(prover_config: ProverConfig) -> Result<Worker<Emu>> {
        let mut prover_options = BackendProverOpts::default()
            .proving_key(prover_config.proving_key.clone())
            .verbose(prover_config.verbose)
            .aggregation(true);

        if prover_config.plonk {
            if prover_config.proving_key_snark.is_none() {
                return Err(anyhow::anyhow!(
                    "PLONK proving key must be provided when PLONK is enabled"
                ));
            }
            prover_options = prover_options
                .proving_key_plonk(prover_config.proving_key_snark.clone().unwrap())
                .plonk(prover_config.preload_plonk);
        }

        if prover_config.minimal_memory {
            prover_options = prover_options.minimal_memory();
        }
        if prover_config.gpu {
            prover_options = prover_options.gpu();
        }
        if let Some(max_streams) = prover_config.max_streams {
            prover_options = prover_options.max_streams(max_streams);
        }
        if let Some(threads) = prover_config.number_threads_witness {
            prover_options = prover_options.number_threads_witness(threads);
        }
        if let Some(max) = prover_config.max_witness_stored {
            prover_options = prover_options.max_witness_stored(max);
        }

        let prover = Arc::new(
            ProverClientBuilder::new().emu().prove().with_prover_options(prover_options).build()?,
        );

        Ok(Worker::<Emu> {
            state: WorkerState::Disconnected,
            current_job: None,
            current_computation: None,
            current_mpi_task: None,
            guest_programs: HashMap::new(),
            program_vks: HashMap::new(),
            prover,
            prover_config,
            stream_actor: None,
        })
    }

    pub fn new_asm(prover_config: ProverConfig) -> Result<Worker<Asm>> {
        let mut prover_options = BackendProverOpts::default()
            .proving_key(prover_config.proving_key.clone())
            .verbose(prover_config.verbose)
            .aggregation(true);

        if prover_config.plonk {
            if prover_config.proving_key_snark.is_none() {
                return Err(anyhow::anyhow!(
                    "PLONK proving key must be provided when PLONK is enabled"
                ));
            }
            prover_options = prover_options
                .proving_key_plonk(prover_config.proving_key_snark.clone().unwrap())
                .plonk(prover_config.preload_plonk);
        }

        if prover_config.minimal_memory {
            prover_options = prover_options.minimal_memory();
        }
        if prover_config.gpu {
            prover_options = prover_options.gpu();
        }
        if let Some(max_streams) = prover_config.max_streams {
            prover_options = prover_options.max_streams(max_streams);
        }
        if let Some(threads) = prover_config.number_threads_witness {
            prover_options = prover_options.number_threads_witness(threads);
        }
        if let Some(max) = prover_config.max_witness_stored {
            prover_options = prover_options.max_witness_stored(max);
        }

        // ASM-specific options for distributed worker
        let mut asm_options = AsmOptions::default();
        if prover_config.unlock_mapped_memory {
            asm_options = asm_options.unlock_mapped_memory();
        }
        if prover_config.asm_out_file {
            asm_options = asm_options.asm_out_file();
        }
        asm_options = asm_options.is_distributed();
        prover_options = prover_options.with_asm_options(asm_options);

        let prover = Arc::new(
            ProverClientBuilder::new().asm().prove().with_prover_options(prover_options).build()?,
        );

        Ok(Worker::<Asm> {
            state: WorkerState::Disconnected,
            current_job: None,
            current_computation: None,
            current_mpi_task: None,
            prover,
            prover_config,
            stream_actor: None,
            guest_programs: HashMap::new(),
            program_vks: HashMap::new(),
        })
    }

    pub fn local_rank(&self) -> i32 {
        self.prover.local_rank()
    }

    pub fn world_rank(&self) -> i32 {
        self.prover.world_rank()
    }

    /// Run setup for a guest program, storing it in the multi-program map.
    /// Skips setup if this hash_id was already set up.
    pub fn run_setup(
        &mut self,
        hash_id: &str,
        elf_bytes: &[u8],
        with_hints: bool,
        emulator_only: bool,
        new_guest_program: Arc<GuestProgram>,
    ) -> Result<ProgramVK> {
        // Skip if already set up for this (hash_id, with_hints, emulator_only) combination.
        if let Some(vk) = self.program_vks.get(&SetupKey::new(hash_id, with_hints, emulator_only)) {
            info!(
                "Received same guest program for setup (hash_id={}, with_hints={}, emulator_only={}). Skipping setup",
                hash_id, with_hints, emulator_only
            );
            return Ok(vk.clone());
        }

        // Broadcast ELF to secondary MPI ranks before setup (they have no gRPC connection).
        let message = SetupMessage {
            hash_id: hash_id.to_string(),
            program_name: new_guest_program.name().to_string(),
            elf_bytes: elf_bytes.to_vec(),
            with_hints,
            emulator_only,
        };
        let mut serialized = borsh::to_vec(&(WorkerMpiTag::Setup, message))
            .map_err(|e| anyhow::anyhow!("Failed to serialize Setup MPI broadcast: {}", e))?;
        self.prover.mpi_broadcast(&mut serialized)?;

        let vk =
            self.prover.prover.setup_internal(&new_guest_program, with_hints, emulator_only)?;
        self.guest_programs.insert(hash_id.to_string(), new_guest_program);
        self.program_vks.insert(SetupKey::new(hash_id, with_hints, emulator_only), vk.clone());
        Ok(vk)
    }

    pub fn get_executed_steps(&self) -> u64 {
        self.prover.executed_steps()
    }

    pub fn state(&self) -> &WorkerState {
        &self.state
    }

    pub fn connection_config(&self) -> &ProverConfig {
        &self.prover_config
    }

    pub fn set_state(&mut self, state: WorkerState) {
        self.state = state;
    }

    pub fn current_job(&self) -> Option<Arc<Mutex<JobContext>>> {
        self.current_job.clone()
    }

    pub fn set_current_job(&mut self, job: Option<JobContext>) {
        if let Some(job) = job {
            self.current_job = Some(Arc::new(Mutex::new(job)));
        } else {
            self.current_job = None;
        }
    }

    pub fn take_current_computation(&mut self) -> Option<JoinHandle<()>> {
        self.current_computation.take()
    }

    pub fn set_current_computation(&mut self, handle: JoinHandle<()>) {
        self.current_computation = Some(handle);
    }

    pub fn has_current_computation(&self) -> bool {
        self.current_computation.is_some()
    }

    pub fn get_vadcop_vk(&self, minimal: bool) -> Result<Vec<u64>> {
        self.prover.get_vadcop_vk(minimal)
    }

    /// Hash family the loaded proving key was generated with (e.g. "Poseidon1" / "Poseidon2").
    pub fn hash(&self) -> Result<String> {
        self.prover.hash()
    }

    pub fn prover_arc(&self) -> Arc<ZiskProver<T>> {
        self.prover.clone()
    }

    pub fn guest_program(&self, hash_id: &str) -> Option<Arc<GuestProgram>> {
        self.guest_programs.get(hash_id).cloned()
    }

    /// Signals cancellation and pokes the ASM children so the in-flight
    /// `executor::execute` returns Err promptly (its Err arm does the actual
    /// ASM cleanup). The in-flight handle itself is detached — awaiting it
    /// here would block the event loop. Stream-actor shutdown runs in
    /// background.
    pub fn cancel_current_computation(&mut self) {
        if let Err(e) = self.prover.cancel() {
            tracing::warn!("cancel_current_computation: prover.cancel failed: {e:#}");
        }

        if self.current_computation.take().is_some() {
            self.prover.notify_cluster_cancellation();
        }

        if let Some(stream_actor) = self.stream_actor.take() {
            tokio::task::spawn_blocking(move || {
                stream_actor.shutdown_and_join(STREAM_ACTOR_SHUTDOWN_TIMEOUT);
            });
        }
    }

    /// Cancels any in-flight computation (without awaiting) and clears the
    /// current job context. The caller is responsible for kicking off
    /// recovery (`spawn_post_failure_recovery`) so the detached spawn_blocking
    /// task actually unwinds.
    pub fn clear_current_job(&mut self) {
        self.cancel_current_computation();
        self.current_job = None;
    }

    pub fn prepare_for_new_job(
        &self,
        hash_id: &str,
        with_hints: bool,
        is_first_process: bool,
    ) -> Result<()> {
        let program_id = self
            .guest_programs
            .get(hash_id)
            .ok_or_else(|| anyhow::anyhow!("Guest program not found for hash_id={hash_id}"))?
            .program_id
            .clone();

        self.prover.register_program(&program_id, with_hints)?;
        self.prover.reset()?;
        self.prover.set_active_services(is_first_process)?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_job(
        &mut self,
        job_id: JobId,
        hash_id: String,
        data_ctx: DataCtx,
        rank_id: u32,
        total_workers: u32,
        allocation: Vec<u32>,
        total_compute_units: u32,
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Arc<Mutex<JobContext>> {
        let current_job = Arc::new(Mutex::new(JobContext {
            job_id: job_id.clone(),
            hash_id,
            data_ctx,
            rank_id,
            total_workers,
            allocation,
            total_compute_units,
            phase: JobPhase::Contributions,
            executed_steps: 0,
            task_received_time,
            instances: 0,
        }));
        self.current_job = Some(current_job.clone());

        self.state = WorkerState::Computing((job_id, JobPhase::Contributions));

        current_job
    }

    pub async fn handle_partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: LoopEventSender,
    ) -> Result<JoinHandle<()>> {
        self.partial_contribution_mpi_broadcast(&job).await?;
        Ok(self.partial_contribution(job, tx))
    }

    pub async fn partial_contribution_mpi_broadcast(&self, job: &Mutex<JobContext>) -> Result<()> {
        let mut serialized = {
            let job = job.lock().await;

            let phase_inputs = ProvePhaseInputs::Contributions();

            let options = self.get_prove_options(false);

            let message = ContributionsMessage {
                job_id: job.job_id.clone(),
                hash_id: job.hash_id.clone(),
                phase_inputs,
                options,
                input_source: job.data_ctx.input_source.clone(),
                hints_source: job.data_ctx.hints_source.clone(),
                partition_info: PartitionInfo {
                    total_compute_units: job.total_compute_units as usize,
                    allocation: job.allocation.clone(),
                    worker_idx: job.rank_id as usize,
                },
            };

            borsh::to_vec(&(WorkerMpiTag::Contributions, message)).map_err(|e| {
                anyhow::anyhow!("Failed to serialize Contributions MPI broadcast: {}", e)
            })?
        };

        self.prover.mpi_broadcast(&mut serialized)?;
        Ok(())
    }

    pub async fn handle_execution_only(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: LoopEventSender,
    ) -> Result<JoinHandle<()>> {
        self.execution_only_mpi_broadcast(&job).await?;
        let hash_id = job.lock().await.hash_id.clone();
        Ok(self.execution_only(job, hash_id, tx))
    }

    pub async fn execution_only_mpi_broadcast(&self, job: &Mutex<JobContext>) -> Result<()> {
        let mut serialized = {
            let job = job.lock().await;

            let phase_inputs = ProvePhaseInputs::Contributions();

            let options = self.get_execution_options();

            let message = ContributionsMessage {
                job_id: job.job_id.clone(),
                hash_id: job.hash_id.clone(),
                phase_inputs,
                options,
                input_source: job.data_ctx.input_source.clone(),
                hints_source: job.data_ctx.hints_source.clone(),
                partition_info: PartitionInfo {
                    total_compute_units: job.total_compute_units as usize,
                    allocation: job.allocation.clone(),
                    worker_idx: job.rank_id as usize,
                },
            };

            borsh::to_vec(&(WorkerMpiTag::Execution, message)).map_err(|e| {
                anyhow::anyhow!("Failed to serialize Execution MPI broadcast: {}", e)
            })?
        };

        self.prover.mpi_broadcast(&mut serialized)?;
        Ok(())
    }

    pub async fn handle_prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
        tx: LoopEventSender,
    ) -> Result<JoinHandle<()>> {
        self.prove_mpi_broadcast(&job, challenges.clone()).await?;
        Ok(self.prove(job, challenges, tx))
    }

    pub async fn prove_mpi_broadcast(
        &self,
        job: &Mutex<JobContext>,
        challenges: Vec<ContributionsInfo>,
    ) -> Result<()> {
        let mut serialized = {
            let job = job.lock().await;

            let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

            let options = self.get_prove_options(false);

            let message = ProveMessage { job_id: job.job_id.clone(), phase_inputs, options };

            borsh::to_vec(&(WorkerMpiTag::Prove, message))
                .map_err(|e| anyhow::anyhow!("Failed to serialize Prove MPI broadcast: {}", e))?
        };

        self.prover.mpi_broadcast(&mut serialized)?;
        Ok(())
    }

    pub fn handle_aggregate(
        &self,
        job: Arc<Mutex<JobContext>>,
        agg_params: AggregationParams,
        tx: LoopEventSender,
    ) -> JoinHandle<()> {
        self.aggregate(job, agg_params, tx)
    }

    pub fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: LoopEventSender,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();
        let options = self.get_prove_options(false);

        tokio::task::spawn_blocking(move || {
            let (job_id, task_received_time) = {
                let guard = job.blocking_lock();
                (guard.job_id.clone(), guard.task_received_time)
            };
            let tx_panic = tx.clone();
            let job_id_panic = job_id.clone();

            run_panic_guarded(
                "Contribution",
                &job_id,
                || {
                    let guard = job.blocking_lock();
                    info!("Computing Contribution for {job_id}");

                    let phase_inputs = proofman::ProvePhaseInputs::Contributions();
                    let inputs_source = guard.data_ctx.input_source.clone();
                    let hints_source = guard.data_ctx.hints_source.clone();
                    let partition_info = PartitionInfo {
                        total_compute_units: guard.total_compute_units as usize,
                        allocation: guard.allocation.clone(),
                        worker_idx: guard.rank_id as usize,
                    };
                    drop(guard);
                    let result = Self::execute_contribution_task(
                        job_id.clone(),
                        &prover,
                        phase_inputs,
                        inputs_source,
                        hints_source,
                        partition_info,
                        options,
                    );

                    let (witness_info, zisk_execution_time) = prover
                        .get_execution_info()
                        .unwrap_or_else(|_| (WitnessInfo::default(), ZiskExecutorTime::default()));

                    let instances = witness_info.total_instances as u64;

                    let mut guard = job.blocking_lock();
                    guard.instances = instances;
                    guard.executed_steps = prover.executed_steps();
                    drop(guard);

                    let computation = match result {
                        Ok(data) => ComputationResult::Contribution {
                            job_id: job_id.clone(),
                            success: true,
                            result: Ok((witness_info, zisk_execution_time, data, instances)),
                            task_received_time,
                        },
                        Err(error) => {
                            error!("Contribution computation failed for {job_id}: {error}");
                            ComputationResult::Contribution {
                                job_id: job_id.clone(),
                                success: false,
                                result: Err(error),
                                task_received_time,
                            }
                        }
                    };
                    if tx.send_computation(computation).is_err() {
                        warn!("Failed to send contribution result: event loop channel closed");
                    }
                },
                || {
                    let _ = tx_panic.send_computation(ComputationResult::Contribution {
                        job_id: job_id_panic,
                        success: false,
                        result: Err(anyhow::anyhow!("contribution task panicked")),
                        task_received_time,
                    });
                },
            );
        })
    }

    pub fn execution_only(
        &self,
        job: Arc<Mutex<JobContext>>,
        hash_id: String,
        tx: LoopEventSender,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();
        let guest_program = self
            .guest_programs
            .get(&hash_id)
            .unwrap_or_else(|| panic!("Guest program not found for hash_id={hash_id}"))
            .clone();

        tokio::task::spawn_blocking(move || {
            let (job_id, task_received_time) = {
                let guard = job.blocking_lock();
                (guard.job_id.clone(), guard.task_received_time)
            };
            let tx_panic = tx.clone();
            let job_id_panic = job_id.clone();

            run_panic_guarded(
                "Execution",
                &job_id,
                || {
                    let guard = job.blocking_lock();
                    info!("Computing Execution (execution-only) for {job_id}");

                    let inputs_source = guard.data_ctx.input_source.clone();
                    let hints_source = guard.data_ctx.hints_source.clone();
                    let partition_info = PartitionInfo {
                        total_compute_units: guard.total_compute_units as usize,
                        allocation: guard.allocation.clone(),
                        worker_idx: guard.rank_id as usize,
                    };
                    drop(guard);

                    // Execute the program (same as contribution) but without generating challenges
                    let result = Self::execute_execution_task(
                        &prover,
                        inputs_source,
                        hints_source,
                        partition_info,
                        &guest_program,
                    );

                    {
                        let mut guard = job.blocking_lock();
                        guard.executed_steps = prover.executed_steps();
                    }

                    let (witness_info, zisk_execution_time) = prover
                        .get_execution_info()
                        .unwrap_or_else(|_| (WitnessInfo::default(), ZiskExecutorTime::default()));

                    let computation = match result {
                        Ok((num_instances, publics)) => {
                            let instances = num_instances as u64;
                            let executed_steps = prover.executed_steps();
                            job.blocking_lock().instances = instances;

                            // witness_info.publics is empty in execution-only mode (no witness
                            // phase), so override with the publics from ExecuteOutput.
                            let mut wi = witness_info;
                            wi.publics = publics;

                            ComputationResult::Execution {
                                job_id: job_id.clone(),
                                success: true,
                                result: Ok((wi, zisk_execution_time, instances, executed_steps)),
                                task_received_time,
                            }
                        }
                        Err(error) => {
                            error!("Execution-only computation failed for {job_id}: {error}");
                            ComputationResult::Execution {
                                job_id: job_id.clone(),
                                success: false,
                                result: Err(error),
                                task_received_time,
                            }
                        }
                    };
                    if tx.send_computation(computation).is_err() {
                        warn!("Failed to send execution result: event loop channel closed");
                    }
                },
                || {
                    let _ = tx_panic.send_computation(ComputationResult::Execution {
                        job_id: job_id_panic,
                        success: false,
                        result: Err(anyhow::anyhow!("execution task panicked")),
                        task_received_time,
                    });
                },
            );
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_contribution_task(
        job_id: JobId,
        prover: &ZiskProver<T>,
        phase_inputs: ProvePhaseInputs,
        input_source: InputSourceDto,
        hints_source: HintsSourceDto,
        partition_info: PartitionInfo,
        options: ProofOptions,
    ) -> Result<Vec<ContributionsInfo>> {
        let phase = proofman::ProvePhase::Contributions;

        let stdin = match input_source {
            InputSourceDto::InputPath(inputs_uri) => ZiskStdin::from_file(inputs_uri)?,
            InputSourceDto::InputData(input_data) => ZiskStdin::from_vec(input_data),
            InputSourceDto::InputNull => ZiskStdin::new(),
        };

        if prover.world_rank() == 0 {
            match hints_source {
                HintsSourceDto::HintsPath(hints_uri) => {
                    let hints_stream = StreamSource::from_uri(hints_uri)?;
                    prover.register_hints_stream(hints_stream)?;
                }
                HintsSourceDto::HintsData(hints_data) => {
                    let hints_stream = StreamSource::from_vec(hints_data);
                    prover.register_hints_stream(hints_stream)?;
                }
                HintsSourceDto::HintsStream(_) | HintsSourceDto::HintsNull => {
                    // HintsStream: data is delivered via route_stream_data → actor → process_hints.
                    // HintsNull: nothing to register.
                }
            }
        }

        prover.set_stdin(stdin)?;

        if matches!(phase_inputs, ProvePhaseInputs::Contributions()) {
            prover.set_partition(
                partition_info.total_compute_units,
                partition_info.allocation.clone(),
                partition_info.worker_idx,
            )?;
        }

        let challenge = match prover.prove_phase(phase_inputs, options, phase) {
            Ok(proofman::ProvePhaseResult::Contributions(challenge)) => {
                info!("Contribution computation successful for {job_id}");
                challenge
            }
            Ok(_) => {
                error!("Error during Contribution computation for {job_id}");
                return Err(anyhow::anyhow!(
                    "Unexpected result type during Contribution computation"
                ));
            }
            Err(err) => {
                error!("Failed to generate proof for {job_id}: {:?}", err);
                return Err(err.context("Failed to generate proof"));
            }
        };

        Ok(challenge)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_execution_task(
        prover: &ZiskProver<T>,
        input_source: InputSourceDto,
        hints_source: HintsSourceDto,
        partition_info: PartitionInfo,
        guest_program: &GuestProgram,
    ) -> Result<(usize, Vec<u64>)> {
        let stdin = match input_source {
            InputSourceDto::InputPath(inputs_uri) => ZiskStdin::from_file(inputs_uri)?,
            InputSourceDto::InputData(input_data) => ZiskStdin::from_vec(input_data),
            InputSourceDto::InputNull => ZiskStdin::new(),
        };

        if prover.world_rank() == 0 {
            match hints_source {
                HintsSourceDto::HintsPath(hints_uri) => {
                    let hints_stream = StreamSource::from_uri(hints_uri)?;
                    prover.register_hints_stream(hints_stream)?;
                }
                HintsSourceDto::HintsData(hints_data) => {
                    let hints_stream = StreamSource::from_vec(hints_data);
                    prover.register_hints_stream(hints_stream)?;
                }
                HintsSourceDto::HintsStream(_) | HintsSourceDto::HintsNull => {
                    // HintsStream: data is delivered via route_stream_data → actor → process_hints.
                    // HintsNull: nothing to register.
                }
            }
        }

        prover.set_stdin(stdin.clone())?;

        prover.set_partition(
            partition_info.total_compute_units,
            partition_info.allocation.clone(),
            partition_info.worker_idx,
        )?;

        let result = prover.execute(guest_program, stdin)?;

        let num_instances = prover.get_execution_info()?.0.total_instances;

        let publics_u64 = result.get_publics().public_u64();

        // `execute` has no implicit MPI sync (each rank runs its own
        // partition independently). Block until every rank has finished so
        // rank 0 can't report success while peer ranks are still draining
        // stale broadcasts queued behind a previous cancel/failure.
        prover.cluster_barrier();

        Ok((num_instances, publics_u64))
    }

    /// Wrap an existing vadcop proof into a minimal or SNARK proof.
    /// `proof_data` is a bincode-encoded `Proof`.
    /// Returns the bincode-encoded wrapped `Proof`.
    pub fn execute_wrap_task(
        prover: &ZiskProver<T>,
        proof_data: Vec<u8>,
        proof_dest: i32,
    ) -> Result<Vec<u8>> {
        let proof_kind = match proof_dest {
            1 => ProofKind::VadcopFinalMinimal,
            2 => ProofKind::Plonk,
            _ => anyhow::bail!("Unsupported proof_dest for wrap: {}", proof_dest),
        };

        let proof: Proof =
            bincode::serde::decode_from_slice(&proof_data, bincode::config::standard())
                .map(|(v, _)| v)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize proof for wrap: {}", e))?;

        let result = prover.wrap_proof(&proof, proof_kind).run()?;

        let wrapped = result.get_proof();

        let result_bytes = bincode::serde::encode_to_vec(wrapped, bincode::config::standard())
            .map_err(|e| anyhow::anyhow!("Failed to serialize wrapped proof: {}", e))?;

        Ok(result_bytes)
    }

    /// Routes an incoming `StreamData` message to the per-job ordering actor.
    pub fn append_raw_input(&self, data: &[u8]) -> Result<()> {
        self.prover.append_raw_input(data)
    }

    ///
    /// - `Start`: spawns a new `StreamOrderingActor`. Resetting shmem and setting
    ///   active services are NOT done here — they are handled synchronously by
    ///   `prepare_for_new_job` before the contribution task is spawned, so reset
    ///   is guaranteed to happen before the C services start reading and before
    ///   any data is written via `process_hints`.
    /// - `Data` / `End`: enqueues the message into the actor's channel — O(1), non-blocking.
    ///
    /// The actor thread owns the reorder buffer and calls `process_hints` in sequence order.
    pub async fn route_stream_data(&mut self, stream_data: StreamDataDto) -> Result<()> {
        match &stream_data.stream_type {
            StreamMessageKind::Start => {
                let job_id = stream_data.job_id.clone();

                let processor = self.prover.get_hints_processor()?;

                // Replace any existing actor — `prepare_for_new_job` already ran
                // `cancel_current_computation`, which joined the previous actor's
                // worker thread, so this assignment can't race a stale process_hints.
                self.stream_actor = Some(StreamOrderingActor::new(processor, job_id));
            }
            StreamMessageKind::Data | StreamMessageKind::End => match &self.stream_actor {
                Some(actor) => actor.send(stream_data)?,
                None => {
                    return Err(anyhow::anyhow!(
                        "Received stream {:?} without a prior Start for job {}",
                        stream_data.stream_type,
                        stream_data.job_id
                    ));
                }
            },
        }
        Ok(())
    }

    pub fn set_partition(
        &self,
        total_compute_units: usize,
        allocation: Vec<u32>,
        worker_idx: usize,
    ) -> Result<()> {
        self.prover.set_partition(total_compute_units, allocation, worker_idx)
    }

    pub fn prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
        tx: LoopEventSender,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();
        let options = self.get_prove_options(false);

        tokio::task::spawn_blocking(move || {
            let job_id = job.blocking_lock().job_id.clone();
            let tx_panic = tx.clone();
            let job_id_panic = job_id.clone();

            run_panic_guarded(
                "Prove",
                &job_id,
                || {
                    info!("Computing Prove for {job_id}");

                    let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);
                    let result =
                        Self::execute_prove_task(job_id.clone(), &prover, phase_inputs, options);

                    let computation = match result {
                        Ok(data) => ComputationResult::Proofs {
                            job_id: job_id.clone(),
                            success: true,
                            result: Ok(data),
                        },
                        Err(error) => {
                            error!("Prove computation failed for {job_id}: {error}");
                            ComputationResult::Proofs {
                                job_id: job_id.clone(),
                                success: false,
                                result: Err(error),
                            }
                        }
                    };
                    if tx.send_computation(computation).is_err() {
                        warn!("Failed to send prove result: event loop channel closed");
                    }
                },
                || {
                    let _ = tx_panic.send_computation(ComputationResult::Proofs {
                        job_id: job_id_panic,
                        success: false,
                        result: Err(anyhow::anyhow!("prove task panicked")),
                    });
                },
            );
        })
    }

    pub fn execute_prove_task(
        job_id: JobId,
        prover: &ZiskProver<T>,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
    ) -> Result<Vec<AggProofs>> {
        let world_rank = prover.world_rank();

        let proof = match prover.prove_phase(phase_inputs, options, proofman::ProvePhase::Internal)
        {
            Ok(proofman::ProvePhaseResult::Internal(proof)) => {
                if world_rank == 0 {
                    info!("Prove computation successful for {job_id}",);
                }
                proof
            }
            Ok(_) => {
                error!("Error during Prove computation for {job_id}");
                return Err(anyhow::anyhow!("Unexpected result type during Prove computation"));
            }
            Err(err) => {
                error!("Failed to generate proof for {job_id}: {err}");
                return Err(anyhow::anyhow!("Failed to generate proof"));
            }
        };

        Ok(proof)
    }

    pub fn aggregate(
        &self,
        job: Arc<Mutex<JobContext>>,
        agg_params: AggregationParams,
        tx: LoopEventSender,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();
        let options =
            self.get_prove_options(agg_params.proof_type == ProofKind::VadcopFinalMinimal);

        let agg_proofs_register: Vec<AggProofsRegister> = agg_params
            .agg_proofs
            .iter()
            .map(|v| AggProofsRegister {
                airgroup_id: v.airgroup_id,
                worker_indexes: vec![v.worker_idx as usize],
            })
            .collect();

        if let Err(error) = prover.register_aggregated_proofs(agg_proofs_register) {
            let job_guard = job.blocking_lock();
            let job_id = job_guard.job_id.clone();
            let executed_steps = job_guard.executed_steps;
            let instances = job_guard.instances;

            if tx
                .send_computation(ComputationResult::AggProof {
                    job_id,
                    success: false,
                    result: Err(error),
                    executed_steps,
                    proof_type: agg_params.proof_type,
                    instances,
                })
                .is_err()
            {
                warn!("Failed to send aggregation register error: event loop channel closed");
            }

            return tokio::spawn(async {});
        }

        tokio::task::spawn_blocking(move || {
            let (job_id, executed_steps, instances) = {
                let guard = job.blocking_lock();
                (guard.job_id.clone(), guard.executed_steps, guard.instances)
            };

            info!("Starting aggregation step for {job_id}");

            let agg_proofs: Vec<AggProofs> = agg_params
                .agg_proofs
                .iter()
                .map(|v| AggProofs {
                    airgroup_id: v.airgroup_id,
                    proof: v.values.clone(),
                    worker_indexes: vec![v.worker_idx as usize],
                })
                .collect();

            let result = prover.aggregate_proofs(
                agg_proofs,
                agg_params.last_proof,
                agg_params.final_proof,
                &options,
            );

            match result {
                Ok(data) => {
                    let proof: Vec<Vec<u64>> = data
                        .map(|proof| proof.agg_proofs.into_iter().map(|p| p.proof).collect())
                        .unwrap_or_default();

                    if tx
                        .send_computation(ComputationResult::AggProof {
                            job_id,
                            success: true,
                            result: Ok(Some(proof)),
                            executed_steps,
                            proof_type: agg_params.proof_type,
                            instances,
                        })
                        .is_err()
                    {
                        warn!("Failed to send aggregation result: event loop channel closed");
                    }
                }
                Err(error) => {
                    tracing::error!("Aggregation failed for {}: {}", job_id, error);
                    if tx
                        .send_computation(ComputationResult::AggProof {
                            job_id,
                            success: false,
                            result: Err(error),
                            executed_steps,
                            proof_type: agg_params.proof_type,
                            instances,
                        })
                        .is_err()
                    {
                        warn!("Failed to send aggregation error: event loop channel closed");
                    }
                }
            }
        })
    }

    /// Proof options for the prove/contribution/aggregation phases.
    /// Aggregation must always be enabled so proofman returns partial proof data.
    fn get_prove_options(&self, minimal: bool) -> ProofOptions {
        ProofOptions {
            verify_constraints: false,
            aggregation: true,
            verify_proofs: false,
            rma: true,
            minimal_memory: self.prover_config.minimal_memory,
            compressed: minimal,
        }
    }

    /// Proof options for execution-only phase.
    /// No aggregation needed; verify_constraints follows worker config.
    fn get_execution_options(&self) -> ProofOptions {
        ProofOptions {
            verify_constraints: true,
            aggregation: false,
            verify_proofs: false,
            rma: true,
            minimal_memory: self.prover_config.minimal_memory,
            compressed: false,
        }
    }

    // --------------------------------------------------------------------------
    // MPI Broadcast handlers for receiving and executing tasks
    // --------------------------------------------------------------------------

    pub async fn handle_mpi_broadcast_request(&mut self) -> Result<()> {
        let mut bytes: Vec<u8> = Vec::new();

        self.prover.mpi_broadcast(&mut bytes)?;

        if bytes.is_empty() {
            return Err(anyhow::anyhow!("Empty MPI broadcast received"));
        }

        let tag: WorkerMpiTag = borsh::from_slice(&bytes[0..1])
            .map_err(|e| anyhow::anyhow!("Failed to deserialize MPI broadcast tag: {}", e))?;

        let prover = self.prover.clone();
        let options = self.get_prove_options(false);

        match tag {
            // Stream broadcasts must run synchronously and concurrently with
            // the in-flight task: the running ASM child is waiting on the
            // `input_avail`/`hint_avail` semaphores that submit_input /
            // submit_hint post. If we awaited the task here we'd never get
            // around to feeding it.
            WorkerMpiTag::ContributionsHintsStream => {
                prover.submit_hint(&bytes)?;
            }
            WorkerMpiTag::ContributionsInputsStream => {
                prover.submit_input(&bytes)?;
            }
            WorkerMpiTag::Setup => {
                self.await_current_mpi_task().await;
                let message: SetupMessage = borsh::from_slice(&bytes[1..]).map_err(|e| {
                    anyhow::anyhow!("Failed to deserialize Setup MPI broadcast: {}", e)
                })?;

                let guest_program =
                    Arc::new(GuestProgram::from_bytes(message.program_name, message.elf_bytes));
                let gp_clone = guest_program.clone();
                let with_hints = message.with_hints;
                let emulator_only = message.emulator_only;
                tokio::task::spawn_blocking(move || {
                    prover.prover.setup_internal(&gp_clone, with_hints, emulator_only)
                })
                .await
                .map_err(|e| anyhow::anyhow!("Setup spawn_blocking panicked: {}", e))??;

                self.guest_programs.insert(message.hash_id.clone(), guest_program);
            }
            WorkerMpiTag::Execution | WorkerMpiTag::Contributions => {
                self.await_current_mpi_task().await;

                let message: ContributionsMessage =
                    borsh::from_slice(&bytes[1..]).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to deserialize Contributions/Execution MPI broadcast: {}",
                            e
                        )
                    })?;

                let with_hints = !matches!(message.hints_source, HintsSourceDto::HintsNull);
                let is_first_process =
                    message.partition_info.allocation.contains(&0) && self.world_rank() == 0;
                self.prepare_for_new_job(&message.hash_id, with_hints, is_first_process)?;

                let guest_programs = self.guest_programs.clone();
                let is_execution = matches!(tag, WorkerMpiTag::Execution);
                let world_rank = self.world_rank();
                let handle = tokio::task::spawn_blocking(move || {
                    let run = || -> Result<()> {
                        if is_execution {
                            let guest_program = guest_programs
                                .get(&message.hash_id)
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "Guest program not found for hash_id={}",
                                        message.hash_id
                                    )
                                })?
                                .clone();
                            Self::execute_execution_task(
                                &prover,
                                message.input_source,
                                message.hints_source,
                                message.partition_info,
                                &guest_program,
                            )?;
                        } else {
                            Self::execute_contribution_task(
                                message.job_id,
                                &prover,
                                message.phase_inputs,
                                message.input_source,
                                message.hints_source,
                                message.partition_info,
                                message.options,
                            )?;
                        }
                        Ok(())
                    };

                    let task_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
                    let task_failed = match task_result {
                        Ok(Ok(())) => false,
                        Ok(Err(e)) => {
                            error!("MPI broadcast task failed: {}. Waiting for new job...", e);
                            true
                        }
                        Err(_) => {
                            error!(
                                "MPI broadcast task panicked on rank {world_rank}. Waiting for new job..."
                            );
                            true
                        }
                    };

                    if task_failed {
                        if let Err(e) = run_recovery(&*prover) {
                            error!("[Recovery] rank {world_rank}: recovery failed: {e:#}");
                        }
                    }
                });
                self.current_mpi_task = Some(handle);
            }
            WorkerMpiTag::Prove => {
                self.await_current_mpi_task().await;

                let message: ProveMessage = borsh::from_slice(&bytes[1..]).map_err(|e| {
                    anyhow::anyhow!("Failed to deserialize Prove MPI broadcast: {}", e)
                })?;

                let world_rank = self.world_rank();
                let handle = tokio::task::spawn_blocking(move || {
                    let task_result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            Self::execute_prove_task(
                                message.job_id,
                                &prover,
                                message.phase_inputs,
                                options,
                            )
                        }));
                    let task_failed = match task_result {
                        Ok(Ok(_)) => false,
                        Ok(Err(e)) => {
                            error!("MPI Prove task failed: {}. Waiting for new job...", e);
                            true
                        }
                        Err(_) => {
                            error!(
                                "MPI Prove task panicked on rank {world_rank}. Waiting for new job..."
                            );
                            true
                        }
                    };

                    if task_failed {
                        run_recovery(&*prover).unwrap_or_else(|e| {
                            error!("[Recovery] rank {world_rank}: recovery failed: {e:#}");
                        });
                    }
                });
                self.current_mpi_task = Some(handle);
            }
            WorkerMpiTag::Aggregate => {
                return Err(anyhow::anyhow!("Aggregate phase is not supported in MPI broadcast"));
            }
        }
        Ok(())
    }

    /// Joins the previous MPI peer-rank task before starting a new one.
    /// Errors and panics from the joined task are logged here so the loop
    /// keeps running — the task itself already ran `run_recovery` on failure.
    async fn await_current_mpi_task(&mut self) {
        if let Some(handle) = self.current_mpi_task.take() {
            if let Err(e) = handle.await {
                error!("MPI broadcast task join failed: {e}");
            }
        }
    }
}
