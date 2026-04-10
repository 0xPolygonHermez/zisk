use anyhow::Result;
use cargo_zisk::common::get_proving_key;
use proofman::{AggProofs, AggProofsRegister, ContributionsInfo};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::ZiskExecutorTime;
use zisk_distributed_common::{AggregationParams, DataCtx, InputSourceDto, JobPhase, WorkerState};
use zisk_distributed_common::{ComputeCapacity, JobId, PartitionInfo, WorkerId};
use zisk_distributed_common::{ContributionsMessage, ProveMessage};
use zisk_distributed_common::{HintsSourceDto, StreamDataDto, StreamMessageKind};
use zisk_prover_backend::GuestProgram;
use zisk_prover_backend::{Asm, Emu, ProgramId, ProverClientBuilder, ZiskBackend, ZiskProver};

use crate::stream_ordering::StreamOrderingActor;

use proofman::ProvePhaseInputs;
use proofman::WitnessInfo;
use proofman_common::ParamsGPU;
use proofman_common::ProofOptions;
use proofman_common::{json_to_debug_instances_map, DebugInfo};
use std::path::PathBuf;
use tracing::{error, info, warn};

use crate::config::ProverServiceConfigDto;

/// Timeout for awaiting cancellation of blocking computation tasks.
/// If a spawn_blocking task doesn't promptly observe the cancel signal,
/// we'll detach it after this duration to keep the worker event loop responsive.
const CANCELLATION_TIMEOUT: Duration = Duration::from_secs(60);

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
        minimal: bool,
        instances: u64,
    },
}

pub struct ProverConfig {
    /// GuestProgram
    pub guest_program: Arc<GuestProgram>,

    /// Flag indicating whether to use the prebuilt emulator
    pub emulator: bool,

    /// Path to the proving key
    pub proving_key: PathBuf,

    /// Verbosity level for logging
    pub verbose: u8,

    /// Debug information
    pub debug_info: DebugInfo,

    /// Additional options for the ASM runner
    // pub asm_runner_options: AsmRunnerOptions,

    /// Base port for ASM services
    pub asm_port: Option<u16>,

    /// Flag to unlock mapped memory
    pub unlock_mapped_memory: bool,

    /// Flag to redirect ASM emulator output to file
    pub asm_out_file: bool,

    /// Flag to verify constraints
    pub verify_constraints: bool,

    /// Flag to enable aggregation
    pub aggregation: bool,

    /// Preallocate resources
    pub gpu_params: Option<ParamsGPU>,

    /// Whether to use shared tables in the witness library
    pub shared_tables: bool,

    /// Whether to use RMA for communication
    pub rma: bool,

    /// Whether to use minimal memory mode
    pub minimal_memory: bool,

    /// Whether to include precompile hints in the assembly generation
    pub hints: bool,
}

impl ProverConfig {
    pub fn load(prover_service_config: ProverServiceConfigDto) -> Result<Self> {
        if !prover_service_config.elf.exists() {
            return Err(anyhow::anyhow!(
                "ELF file '{}' not found.",
                prover_service_config.elf.display()
            ));
        }
        let proving_key = get_proving_key(prover_service_config.proving_key.as_ref())?;
        let debug_info = match &prover_service_config.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(proving_key.clone(), debug_value.clone())?
            }
        };

        let emulator =
            if cfg!(target_os = "macos") { true } else { prover_service_config.emulator };

        let guest_program =
            Arc::new(GuestProgram::from_uri(prover_service_config.elf.to_str().unwrap())?);

        let mut gpu_params = None;
        if prover_service_config.preallocate
            || prover_service_config.max_streams.is_some()
            || prover_service_config.number_threads_witness.is_some()
            || prover_service_config.max_witness_stored.is_some()
        {
            let mut gpu_params_new = ParamsGPU::new(prover_service_config.preallocate);
            if let Some(max_streams) = prover_service_config.max_streams {
                gpu_params_new.with_max_number_streams(max_streams);
            }
            if let Some(number_threads_witness) = prover_service_config.number_threads_witness {
                gpu_params_new.with_number_threads_pools_witness(number_threads_witness);
            }
            if let Some(max_witness_stored) = prover_service_config.max_witness_stored {
                gpu_params_new.with_max_witness_stored(max_witness_stored);
            }
            gpu_params = Some(gpu_params_new);
        }

        Ok(ProverConfig {
            guest_program,
            emulator,
            proving_key,
            verbose: prover_service_config.verbose,
            debug_info,
            asm_port: prover_service_config.asm_port,
            unlock_mapped_memory: prover_service_config.unlock_mapped_memory,
            asm_out_file: prover_service_config.asm_out_file,
            verify_constraints: prover_service_config.verify_constraints,
            aggregation: prover_service_config.aggregation,
            gpu_params,
            shared_tables: prover_service_config.shared_tables,
            rma: prover_service_config.rma,
            minimal_memory: prover_service_config.minimal_memory,
            hints: prover_service_config.hints,
        })
    }
}

/// Current job context
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: JobId,
    pub data_ctx: DataCtx,
    pub rank_id: u32,
    pub total_workers: u32,
    pub allocation: Vec<u32>, // Worker allocation for this job, vector of all computed units assigned
    pub total_compute_units: u32, // Total compute units for the whole job
    pub phase: JobPhase,
    pub executed_steps: u64,
    pub instances: u64,
    pub task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    pub guest_program: Arc<GuestProgram>,
}

pub struct Worker<T: ZiskBackend + 'static> {
    _worker_id: WorkerId,
    _compute_capacity: ComputeCapacity,
    state: WorkerState,
    current_job: Option<Arc<Mutex<JobContext>>>,
    current_computation: Option<JoinHandle<()>>,

    prover: Arc<ZiskProver<T>>,
    prover_config: ProverConfig,

    stream_actor: Option<StreamOrderingActor>,
    guest_program: Arc<GuestProgram>,
}

impl<T: ZiskBackend + 'static> Worker<T> {
    pub fn new_emu(
        worker_id: WorkerId,
        compute_capacity: ComputeCapacity,
        prover_config: ProverConfig,
    ) -> Result<Worker<Emu>> {
        let prover = Arc::new(
            ProverClientBuilder::new()
                .emu()
                .prove()
                .aggregation(true)
                .proving_key_path(prover_config.proving_key.clone())
                .verbose(prover_config.verbose)
                .shared_tables(prover_config.shared_tables)
                .gpu(prover_config.gpu_params.clone())
                .build()?,
        );

        let guest_program = prover_config.guest_program.clone();
        prover.setup(&guest_program).run()?;

        Ok(Worker::<Emu> {
            _worker_id: worker_id,
            _compute_capacity: compute_capacity,
            state: WorkerState::Disconnected,
            current_job: None,
            current_computation: None,
            prover,
            prover_config,
            guest_program,
            stream_actor: None,
        })
    }

    pub fn new_asm(
        worker_id: WorkerId,
        compute_capacity: ComputeCapacity,
        prover_config: ProverConfig,
    ) -> Result<Worker<Asm>> {
        let prover = Arc::new(
            ProverClientBuilder::new()
                .asm()
                .prove()
                .aggregation(true)
                .proving_key_path(prover_config.proving_key.clone())
                .verbose(prover_config.verbose)
                .shared_tables(prover_config.shared_tables)
                .base_port_opt(prover_config.asm_port)
                .unlock_mapped_memory(prover_config.unlock_mapped_memory)
                .asm_out_file(prover_config.asm_out_file)
                .gpu(prover_config.gpu_params.clone())
                .is_distributed(true)
                .build()?,
        );

        let guest_program = prover_config.guest_program.clone();

        if prover_config.hints {
            prover.setup(&guest_program).with_hints().run()?;
        } else {
            prover.setup(&guest_program).run()?;
        }

        Ok(Worker::<Asm> {
            _worker_id: worker_id,
            _compute_capacity: compute_capacity,
            state: WorkerState::Disconnected,
            current_job: None,
            current_computation: None,
            prover,
            prover_config,
            guest_program,
            stream_actor: None,
        })
    }

    pub fn local_rank(&self) -> i32 {
        self.prover.local_rank()
    }

    pub fn world_rank(&self) -> i32 {
        self.prover.world_rank()
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

    pub fn get_vadcop_vk(&self, minimal: bool) -> Result<Vec<u8>> {
        let vk = self.prover.get_vadcop_vk(minimal)?;
        Ok(vk.vk)
    }

    pub async fn cancel_current_computation(&mut self) {
        self.prover.cancel();

        if let Some(handle) = self.current_computation.take() {
            match tokio::time::timeout(CANCELLATION_TIMEOUT, handle).await {
                Ok(_) => {}
                Err(_) => {
                    warn!(
                        "Cancellation timeout ({:?}) expired; detaching computation task (it may complete in background)",
                        CANCELLATION_TIMEOUT
                    );
                }
            }
        }

        // Drop the actor on a blocking thread: closes the channel, which signals the ordering
        // thread to exit, without blocking the Tokio runtime worker thread.
        if let Some(stream_actor) = self.stream_actor.take() {
            tokio::task::spawn_blocking(move || {
                drop(stream_actor);
            });
        }
    }

    /// Cancels any in-flight computation and clears the current job context.
    /// Use this when the worker should become fully idle (e.g., job cancelled,
    /// stale job cleared on reconnection).
    pub async fn clear_current_job(&mut self) {
        self.cancel_current_computation().await;
        self.current_job = None;
    }

    #[allow(clippy::type_complexity)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_job(
        &mut self,
        job_id: JobId,
        data_ctx: DataCtx,
        rank_id: u32,
        total_workers: u32,
        allocation: Vec<u32>,
        total_compute_units: u32,
        task_received_time: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Arc<Mutex<JobContext>> {
        let current_job = Arc::new(Mutex::new(JobContext {
            job_id: job_id.clone(),
            data_ctx,
            rank_id,
            total_workers,
            allocation,
            total_compute_units,
            phase: JobPhase::Contributions,
            executed_steps: 0,
            task_received_time,
            guest_program: self.guest_program.clone(),
            instances: 0,
        }));
        self.current_job = Some(current_job.clone());

        self.state = WorkerState::Computing((job_id, JobPhase::Contributions));

        current_job
    }

    pub async fn handle_partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> Result<JoinHandle<()>> {
        self.partial_contribution_mpi_broadcast(&job).await?;
        Ok(self.partial_contribution(job, tx))
    }

    pub async fn partial_contribution_mpi_broadcast(&self, job: &Mutex<JobContext>) -> Result<()> {
        let mut serialized = {
            let job = job.lock().await;

            let phase_inputs = ProvePhaseInputs::Contributions();

            let options = self.get_proof_options(false);

            let message = ContributionsMessage {
                job_id: job.job_id.clone(),
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

            borsh::to_vec(&(JobPhase::Contributions, message)).map_err(|e| {
                anyhow::anyhow!("Failed to serialize Contributions MPI broadcast: {}", e)
            })?
        };

        self.prover.mpi_broadcast(&mut serialized)?;
        Ok(())
    }

    pub async fn handle_execution_only(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> Result<JoinHandle<()>> {
        self.execution_only_mpi_broadcast(&job).await?;
        Ok(self.execution_only(job, tx))
    }

    pub async fn execution_only_mpi_broadcast(&self, job: &Mutex<JobContext>) -> Result<()> {
        let mut serialized = {
            let job = job.lock().await;

            let phase_inputs = ProvePhaseInputs::Contributions();

            let options = self.get_proof_options(false);

            let message = ContributionsMessage {
                job_id: job.job_id.clone(),
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

            borsh::to_vec(&(JobPhase::Execution, message)).map_err(|e| {
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
        tx: mpsc::UnboundedSender<ComputationResult>,
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

            let options = self.get_proof_options(false);

            let message = ProveMessage { job_id: job.job_id.clone(), phase_inputs, options };

            borsh::to_vec(&(JobPhase::Prove, message))
                .map_err(|e| anyhow::anyhow!("Failed to serialize Prove MPI broadcast: {}", e))?
        };

        self.prover.mpi_broadcast(&mut serialized)?;
        Ok(())
    }

    pub fn handle_aggregate(
        &self,
        job: Arc<Mutex<JobContext>>,
        agg_params: AggregationParams,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        self.aggregate(job, agg_params, tx)
    }

    pub fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();
        let options = self.get_proof_options(false);

        tokio::task::spawn_blocking(move || {
            let guard = job.blocking_lock();
            let job_id = guard.job_id.clone();

            let program_id = guard.guest_program.program_id.clone();

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
                &program_id,
                options,
            );

            let (witness_info, zisk_execution_time) = prover
                .get_execution_info()
                .unwrap_or_else(|_| (WitnessInfo::default(), ZiskExecutorTime::default()));

            let instances = witness_info.total_instances as u64;

            let mut guard = job.blocking_lock();
            guard.instances = instances;
            guard.executed_steps = prover.executed_steps();
            let task_received_time = guard.task_received_time;
            drop(guard);

            match result {
                Ok(data) => {
                    if tx
                        .send(ComputationResult::Contribution {
                            job_id,
                            success: true,
                            result: Ok((witness_info, zisk_execution_time, data, instances)),
                            task_received_time,
                        })
                        .is_err()
                    {
                        warn!("Failed to send contribution result: event loop channel closed");
                    }
                }
                Err(error) => {
                    error!("Contribution computation failed for {}: {}", job_id, error);
                    if tx
                        .send(ComputationResult::Contribution {
                            job_id,
                            success: false,
                            result: Err(error),
                            task_received_time,
                        })
                        .is_err()
                    {
                        warn!("Failed to send contribution error: event loop channel closed");
                    }
                }
            }
        })
    }

    pub fn execution_only(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();

        tokio::task::spawn_blocking(move || {
            let guard = job.blocking_lock();
            let job_id = guard.job_id.clone();

            info!("Computing Execution (execution-only) for {job_id}");

            let inputs_source = guard.data_ctx.input_source.clone();
            let hints_source = guard.data_ctx.hints_source.clone();
            let partition_info = PartitionInfo {
                total_compute_units: guard.total_compute_units as usize,
                allocation: guard.allocation.clone(),
                worker_idx: guard.rank_id as usize,
            };
            let guest_program = guard.guest_program.clone();
            drop(guard);

            // Execute the program (same as contribution) but without generating challenges
            let result = Self::execute_execution_task(
                &prover,
                inputs_source,
                hints_source,
                partition_info,
                &guest_program,
            );

            let mut guard = job.blocking_lock();
            guard.executed_steps = prover.executed_steps();
            let task_received_time = guard.task_received_time;
            drop(guard);

            let (witness_info, zisk_execution_time) = prover
                .get_execution_info()
                .unwrap_or_else(|_| (WitnessInfo::default(), ZiskExecutorTime::default()));

            match result {
                Ok(num_instances) => {
                    let instances = num_instances as u64;
                    let executed_steps = prover.executed_steps();
                    guard = job.blocking_lock();
                    guard.instances = instances;
                    drop(guard);

                    if tx
                        .send(ComputationResult::Execution {
                            job_id,
                            success: true,
                            result: Ok((
                                witness_info,
                                zisk_execution_time,
                                instances,
                                executed_steps,
                            )),
                            task_received_time,
                        })
                        .is_err()
                    {
                        warn!("Failed to send execution result: event loop channel closed");
                    }
                }
                Err(error) => {
                    error!("Execution-only computation failed for {}: {}", job_id, error);
                    if tx
                        .send(ComputationResult::Execution {
                            job_id,
                            success: false,
                            result: Err(error),
                            task_received_time,
                        })
                        .is_err()
                    {
                        warn!("Failed to send execution error: event loop channel closed");
                    }
                }
            }
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
        program_id: &ProgramId,
        options: ProofOptions,
    ) -> Result<Vec<ContributionsInfo>> {
        let phase = proofman::ProvePhase::Contributions;

        let stdin = match input_source {
            InputSourceDto::InputPath(inputs_uri) => ZiskStdin::from_file(inputs_uri)?,
            InputSourceDto::InputData(input_data) => ZiskStdin::from_vec(input_data),
            InputSourceDto::InputNull => ZiskStdin::new(),
        };

        match hints_source {
            HintsSourceDto::HintsPath(hints_uri) => {
                let hints_stream = StreamSource::from_uri(hints_uri)?;
                prover.register_hints_stream(hints_stream)?;
            }
            HintsSourceDto::HintsStream(_hints_uri) => {
                // For HintsStream, the worker will receive hint data via StreamData gRPC messages
                // routed through the stream ordering actor into the hints processor.
                // No need to set hints_stream on prover for this case
            }
            HintsSourceDto::HintsNull => {
                // No hints to set
            }
        }

        prover.set_stdin(stdin)?;

        prover.register_program(program_id)?;

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
                return Err(anyhow::anyhow!("Failed to generate proof"));
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
    ) -> Result<usize> {
        let stdin = match input_source {
            InputSourceDto::InputPath(inputs_uri) => ZiskStdin::from_file(inputs_uri)?,
            InputSourceDto::InputData(input_data) => ZiskStdin::from_vec(input_data),
            InputSourceDto::InputNull => ZiskStdin::new(),
        };

        match hints_source {
            HintsSourceDto::HintsPath(hints_uri) => {
                let hints_stream = StreamSource::from_uri(hints_uri)?;
                prover.register_hints_stream(hints_stream)?;
            }
            HintsSourceDto::HintsStream(_hints_uri) => {
                // For HintsStream, the worker will receive hint data via StreamData gRPC messages
                // routed through the stream ordering actor into the hints processor.
                // No need to set hints_stream on prover for this case
            }
            HintsSourceDto::HintsNull => {
                // No hints to set
            }
        }

        prover.set_stdin(stdin.clone())?;

        prover.register_program(&guest_program.program_id)?;

        prover.set_partition(
            partition_info.total_compute_units,
            partition_info.allocation.clone(),
            partition_info.worker_idx,
        )?;

        let result = prover.execute(guest_program, stdin)?;

        let num_instances = result.planning_info.num_instances;

        Ok(num_instances)
    }

    /// Routes an incoming `StreamData` message to the per-job ordering actor.
    ///
    /// - `Start`: initialises the `HintsProcessor` (if needed), resets it, and spawns the actor.
    /// - `Data` / `End`: enqueues the message into the actor's channel — O(1), non-blocking.
    ///
    /// The actor thread owns the reorder buffer and calls `process_hints` in sequence order.
    pub async fn route_stream_data(
        &mut self,
        stream_data: StreamDataDto,
        is_first_partition: bool,
    ) -> Result<()> {
        match &stream_data.stream_type {
            StreamMessageKind::Start => {
                let job_id = stream_data.job_id.clone();

                self.prover.reset_resources()?;

                let processor = self.prover.get_hints_processor()?.ok_or_else(|| {
                    anyhow::anyhow!("HintsProcessor not found for job {}", job_id)
                })?;

                self.prover.set_active_services(is_first_partition)?;

                // Replace any existing actor (handles reconnect / job restart)
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
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();
        let options = self.get_proof_options(false);

        tokio::task::spawn_blocking(move || {
            let job_id = job.blocking_lock().job_id.clone();

            info!("Computing Prove for {job_id}");

            let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);
            let result = Self::execute_prove_task(job_id.clone(), &prover, phase_inputs, options);

            match result {
                Ok(data) => {
                    if tx
                        .send(ComputationResult::Proofs { job_id, success: true, result: Ok(data) })
                        .is_err()
                    {
                        warn!("Failed to send prove result: event loop channel closed");
                    }
                }
                Err(error) => {
                    error!("Prove computation failed for {}: {}", job_id, error);
                    if tx
                        .send(ComputationResult::Proofs {
                            job_id,
                            success: false,
                            result: Err(error),
                        })
                        .is_err()
                    {
                        warn!("Failed to send prove error: event loop channel closed");
                    }
                }
            }
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
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();
        let options = self.get_proof_options(agg_params.minimal);

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
                .send(ComputationResult::AggProof {
                    job_id,
                    success: false,
                    result: Err(error),
                    executed_steps,
                    minimal: agg_params.minimal,
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
                    let proof = data
                        .map(|proof| proof.agg_proofs.into_iter().map(|p| p.proof).collect())
                        .unwrap_or_default();
                    if tx
                        .send(ComputationResult::AggProof {
                            job_id,
                            success: true,
                            result: Ok(Some(proof)),
                            executed_steps,
                            minimal: agg_params.minimal,
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
                        .send(ComputationResult::AggProof {
                            job_id,
                            success: false,
                            result: Err(error),
                            executed_steps,
                            minimal: agg_params.minimal,
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

    fn get_proof_options(&self, minimal: bool) -> ProofOptions {
        ProofOptions {
            verify_constraints: self.prover_config.verify_constraints,
            aggregation: self.prover_config.aggregation,
            verify_proofs: false,
            save_proofs: false,
            test_mode: false,
            output_dir_path: None,
            rma: self.prover_config.rma,
            minimal_memory: self.prover_config.minimal_memory,
            compressed: minimal,
        }
    }

    // --------------------------------------------------------------------------
    // MPI Broadcast handlers for receiving and executing tasks
    // --------------------------------------------------------------------------

    pub async fn handle_mpi_broadcast_request(&self) -> Result<()> {
        let mut bytes: Vec<u8> = Vec::new();

        self.prover.mpi_broadcast(&mut bytes)?;

        if bytes.is_empty() {
            return Err(anyhow::anyhow!("Empty MPI broadcast received"));
        }

        let phase: JobPhase = borsh::from_slice(&bytes[0..1])
            .map_err(|e| anyhow::anyhow!("Failed to deserialize MPI broadcast phase: {}", e))?;

        let prover = self.prover.clone();
        let program_id = self.guest_program.program_id.clone();
        let guest_program = self.guest_program.clone();
        let options = self.get_proof_options(false);

        if phase == JobPhase::ContributionsHintsStream {
            prover.submit_hint(&bytes)?;
        } else if phase == JobPhase::ContributionsInputsStream {
            prover.submit_input(&bytes)?;
        } else {
            tokio::task::spawn_blocking(move || {
                let deserialize_and_run = || -> Result<()> {
                    match phase {
                        JobPhase::Execution => {
                            let message: ContributionsMessage = borsh::from_slice(&bytes[1..])
                                .map_err(|e| {
                                    anyhow::anyhow!(
                                        "Failed to deserialize Execution MPI broadcast: {}",
                                        e
                                    )
                                })?;

                            Self::execute_execution_task(
                                &prover,
                                message.input_source,
                                message.hints_source,
                                message.partition_info,
                                &guest_program,
                            )?;
                        }
                        JobPhase::Contributions => {
                            let message: ContributionsMessage = borsh::from_slice(&bytes[1..])
                                .map_err(|e| {
                                    anyhow::anyhow!(
                                        "Failed to deserialize Contributions MPI broadcast: {}",
                                        e
                                    )
                                })?;

                            Self::execute_contribution_task(
                                message.job_id,
                                &prover,
                                message.phase_inputs,
                                message.input_source,
                                message.hints_source,
                                message.partition_info,
                                &program_id,
                                message.options,
                            )?;
                        }
                        JobPhase::Prove => {
                            let message: ProveMessage =
                                borsh::from_slice(&bytes[1..]).map_err(|e| {
                                    anyhow::anyhow!(
                                        "Failed to deserialize Prove MPI broadcast: {}",
                                        e
                                    )
                                })?;

                            Self::execute_prove_task(
                                message.job_id,
                                &prover,
                                message.phase_inputs,
                                options,
                            )?;
                        }
                        JobPhase::Aggregate => {
                            return Err(anyhow::anyhow!(
                                "Aggregate phase is not supported in MPI broadcast"
                            ));
                        }
                        JobPhase::ContributionsHintsStream
                        | JobPhase::ContributionsInputsStream => {
                            return Err(anyhow::anyhow!(
                                "Stream phases should be handled separately and not reach this point"
                            ));
                        }
                    }
                    Ok(())
                };

                if let Err(e) = deserialize_and_run() {
                    error!("MPI broadcast task failed: {}. Waiting for new job...", e);
                }
            });
        }
        Ok(())
    }
}
