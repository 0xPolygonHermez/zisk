use anyhow::Result;
use asm_runner::HintsShmem;
use cargo_zisk::commands::{get_proving_key, get_witness_computation_lib};
use precompiles_hints::HintsProcessor;
use proofman::{AggProofs, ContributionsInfo};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor_and_arity,
    DEFAULT_CACHE_PATH,
};
use std::collections::hash_map::Entry;
use std::fs;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use zisk_common::io::{StreamSource, ZiskStdin};
use zisk_common::reinterpret_vec;
use zisk_distributed_common::{
    AggregationParams, DataCtx, HintsSourceDto, InputSourceDto, JobPhase, StreamDataDto,
    StreamMessageKind, StreamPayloadDto, WorkerState,
};
use zisk_distributed_common::{ComputeCapacity, JobId, WorkerId};
use zisk_sdk::{Asm, Emu, ProverClient, ZiskBackend, ZiskProver};

use proofman::ProofInfo;
use proofman::ProvePhaseInputs;
use proofman_common::ParamsGPU;
use proofman_common::ProofOptions;
use proofman_common::{json_to_debug_instances_map, DebugInfo};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, info};

use crate::config::ProverServiceConfigDto;

/// Result from computation tasks
#[derive(Debug)]
pub enum ComputationResult {
    Challenge {
        job_id: JobId,
        success: bool,
        result: Result<Vec<ContributionsInfo>>,
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
    },
}

pub struct ProverConfig {
    /// Path to the ELF file
    pub elf: PathBuf,

    /// Path to the witness computation dynamic library
    pub witness_lib: PathBuf,

    /// Path to the ASM file (optional)
    pub asm: Option<PathBuf>,

    /// Path to the ASM ROM file (optional)
    pub asm_rom: Option<PathBuf>,

    /// Map of custom commits
    pub custom_commits_map: HashMap<String, PathBuf>,

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

    /// Flag to verify constraints
    pub verify_constraints: bool,

    /// Flag to enable aggregation
    pub aggregation: bool,

    /// Flag to enable final SNARK
    pub final_snark: bool,

    /// Preallocate resources
    pub gpu_params: ParamsGPU,

    /// Whether to use shared tables in the witness library
    pub shared_tables: bool,

    /// Whether to use RMA for communication
    pub rma: bool,

    /// Whether to use minimal memory mode
    pub minimal_memory: bool,
}

impl ProverConfig {
    pub fn load(mut prover_service_config: ProverServiceConfigDto) -> Result<Self> {
        if !prover_service_config.elf.exists() {
            return Err(anyhow::anyhow!(
                "ELF file '{}' not found.",
                prover_service_config.elf.display()
            ));
        }
        let proving_key = get_proving_key(prover_service_config.proving_key.as_ref());
        let debug_info = match &prover_service_config.debug {
            None => DebugInfo::default(),
            Some(None) => DebugInfo::new_debug(),
            Some(Some(debug_value)) => {
                json_to_debug_instances_map(proving_key.clone(), debug_value.clone())?
            }
        };

        let home = std::env::var("HOME").map(PathBuf::from).map_err(|_| {
            anyhow::anyhow!(
                "HOME environment variable not set, cannot determine default cache path"
            )
        })?;

        let default_cache_path = home.join(DEFAULT_CACHE_PATH);
        if !default_cache_path.exists() {
            if let Err(e) = fs::create_dir_all(default_cache_path.clone()) {
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(anyhow::anyhow!("Failed to create the cache directory: {e:?}"));
                }
            }
        }

        let emulator =
            if cfg!(target_os = "macos") { true } else { prover_service_config.emulator };
        let mut asm_rom = None;
        if emulator {
            prover_service_config.asm = None;
        } else if prover_service_config.asm.is_none() {
            let stem = prover_service_config
                .elf
                .file_stem()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "ELF path '{}' does not have a file stem.",
                        prover_service_config.elf.display()
                    )
                })?
                .to_str()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "ELF file stem for '{}' is not valid UTF-8.",
                        prover_service_config.elf.display()
                    )
                })?;

            let hash = get_elf_data_hash(&prover_service_config.elf)
                .map_err(|e| anyhow::anyhow!("Error computing ELF hash: {}", e))?;
            let new_filename = format!("{stem}-{hash}-mt.bin");
            let asm_rom_filename = format!("{stem}-{hash}-rh.bin");
            asm_rom = Some(default_cache_path.join(asm_rom_filename));
            prover_service_config.asm = Some(default_cache_path.join(new_filename));
        }
        if let Some(asm_path) = &prover_service_config.asm {
            if !asm_path.exists() {
                return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_path.display()));
            }
        }

        if let Some(asm_rom) = &asm_rom {
            if !asm_rom.exists() {
                return Err(anyhow::anyhow!("ASM file not found at {:?}", asm_rom.display()));
            }
        }
        let (blowup_factor, merkle_tree_arity) = get_rom_blowup_factor_and_arity(&proving_key);
        let rom_bin_path = get_elf_bin_file_path(
            &prover_service_config.elf.to_path_buf(),
            &default_cache_path,
            blowup_factor,
            merkle_tree_arity,
        )?;
        if !rom_bin_path.exists() {
            let _ = gen_elf_hash(
                &prover_service_config.elf.clone(),
                rom_bin_path.as_path(),
                blowup_factor,
                merkle_tree_arity,
                false,
            )
            .map_err(|e| anyhow::anyhow!("Error generating elf hash: {}", e));
        }
        let mut custom_commits_map: HashMap<String, PathBuf> = HashMap::new();
        custom_commits_map.insert("rom".to_string(), rom_bin_path);
        let mut gpu_params = ParamsGPU::new(prover_service_config.preallocate);
        if prover_service_config.max_streams.is_some() {
            gpu_params.with_max_number_streams(prover_service_config.max_streams.unwrap());
        }
        if prover_service_config.number_threads_witness.is_some() {
            gpu_params.with_number_threads_pools_witness(
                prover_service_config.number_threads_witness.unwrap(),
            );
        }
        if prover_service_config.max_witness_stored.is_some() {
            gpu_params.with_max_witness_stored(prover_service_config.max_witness_stored.unwrap());
        }

        Ok(ProverConfig {
            elf: prover_service_config.elf.clone(),
            witness_lib: get_witness_computation_lib(prover_service_config.witness_lib.as_ref()),
            asm: prover_service_config.asm.clone(),
            asm_rom,
            custom_commits_map,
            emulator,
            proving_key,
            verbose: prover_service_config.verbose,
            debug_info,
            asm_port: prover_service_config.asm_port,
            unlock_mapped_memory: prover_service_config.unlock_mapped_memory,
            verify_constraints: prover_service_config.verify_constraints,
            aggregation: prover_service_config.aggregation,
            final_snark: prover_service_config.final_snark,
            gpu_params,
            shared_tables: prover_service_config.shared_tables,
            rma: prover_service_config.rma,
            minimal_memory: prover_service_config.minimal_memory,
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
}

pub struct Worker<T: ZiskBackend + 'static> {
    _worker_id: WorkerId,
    _compute_capacity: ComputeCapacity,
    state: WorkerState,
    current_job: Option<Arc<Mutex<JobContext>>>,
    current_computation: Option<JoinHandle<()>>,

    prover: Arc<ZiskProver<T>>,
    prover_config: ProverConfig,

    stream_buffers: HashMap<JobId, (u32, HashMap<u32, Vec<u8>>)>, // (job_id, (next_seq, (seq_number, data)))
    hints_processor: Option<HintsProcessor<HintsShmem>>,
}

impl<T: ZiskBackend + 'static> Worker<T> {
    pub fn new_emu(
        worker_id: WorkerId,
        compute_capacity: ComputeCapacity,
        prover_config: ProverConfig,
    ) -> Result<Worker<Emu>> {
        let prover = Arc::new(
            ProverClient::builder()
                .emu()
                .prove()
                .aggregation(true)
                .rma(true)
                .witness_lib_path(prover_config.witness_lib.clone())
                .proving_key_path(prover_config.proving_key.clone())
                .elf_path(prover_config.elf.clone())
                .verbose(prover_config.verbose)
                .shared_tables(prover_config.shared_tables)
                .gpu(prover_config.gpu_params.clone())
                .build()?,
        );

        Ok(Worker::<Emu> {
            _worker_id: worker_id,
            _compute_capacity: compute_capacity,
            state: WorkerState::Disconnected,
            current_job: None,
            current_computation: None,
            prover,
            prover_config,
            stream_buffers: HashMap::new(),
            hints_processor: None,
        })
    }

    pub fn new_asm(
        worker_id: WorkerId,
        compute_capacity: ComputeCapacity,
        prover_config: ProverConfig,
    ) -> Result<Worker<Asm>> {
        let prover = Arc::new(
            ProverClient::builder()
                .asm()
                .prove()
                .aggregation(true)
                .rma(true)
                .witness_lib_path(prover_config.witness_lib.clone())
                .proving_key_path(prover_config.proving_key.clone())
                .elf_path(prover_config.elf.clone())
                .verbose(prover_config.verbose)
                .shared_tables(prover_config.shared_tables)
                .asm_path_opt(prover_config.asm.clone())
                .base_port_opt(prover_config.asm_port)
                .unlock_mapped_memory(prover_config.unlock_mapped_memory)
                .gpu(prover_config.gpu_params.clone())
                .build()?,
        );

        Ok(Worker::<Asm> {
            _worker_id: worker_id,
            _compute_capacity: compute_capacity,
            state: WorkerState::Disconnected,
            current_job: None,
            current_computation: None,
            prover,
            prover_config,
            stream_buffers: HashMap::new(),
            hints_processor: None,
        })
    }

    pub fn local_rank(&self) -> i32 {
        self.prover.local_rank()
    }

    pub fn world_rank(&self) -> i32 {
        self.prover.world_rank()
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

    pub fn cancel_current_computation(&mut self) {
        if let Some(handle) = self.current_computation.take() {
            handle.abort();
        }
    }

    pub fn new_job(
        &mut self,
        job_id: JobId,
        data_ctx: DataCtx,
        rank_id: u32,
        total_workers: u32,
        allocation: Vec<u32>,
        total_compute_units: u32,
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
        }));
        self.current_job = Some(current_job.clone());

        self.state = WorkerState::Computing((job_id, JobPhase::Contributions));

        current_job
    }

    pub async fn handle_partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        self.partial_contribution_mpi_broadcast(&job).await;
        self.partial_contribution(job, tx).await
    }

    pub async fn partial_contribution_mpi_broadcast(&self, job: &Mutex<JobContext>) {
        let job = job.lock().await;
        let job_id = job.job_id.clone();

        let proof_info = ProofInfo::new(
            None,
            job.total_compute_units as usize,
            job.allocation.clone(),
            job.rank_id as usize,
        );
        let phase_inputs = proofman::ProvePhaseInputs::Contributions(proof_info);

        let options = self.get_proof_options_partial_contribution();

        let mut serialized = borsh::to_vec(&(
            JobPhase::Contributions,
            job_id,
            phase_inputs,
            options,
            job.data_ctx.input_source.clone(),
        ))
        .unwrap();

        self.prover.mpi_broadcast(&mut serialized);
    }

    pub async fn handle_prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        self.prove_mpi_broadcast(&job, challenges.clone()).await;
        self.prove(job, challenges, tx).await
    }

    pub async fn prove_mpi_broadcast(
        &self,
        job: &Mutex<JobContext>,
        challenges: Vec<ContributionsInfo>,
    ) {
        let job = job.lock().await;
        let job_id = job.job_id.clone();

        let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

        let options = self.get_proof_options_prove();

        let mut serialized =
            borsh::to_vec(&(JobPhase::Prove, job_id, phase_inputs, options)).unwrap();

        self.prover.mpi_broadcast(&mut serialized);
    }

    pub async fn handle_aggregate(
        &self,
        job: Arc<Mutex<JobContext>>,
        agg_params: AggregationParams,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        self.aggregate(job, agg_params, tx).await
    }

    pub async fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();

        let options = self.get_proof_options_partial_contribution();

        tokio::spawn(async move {
            let guard = job.lock().await;
            let job_id = guard.job_id.clone();

            info!("Computing Contribution for {job_id}");

            let proof_info = ProofInfo::new(
                None,
                guard.total_compute_units as usize,
                guard.allocation.clone(),
                guard.rank_id as usize,
            );

            let phase_inputs = proofman::ProvePhaseInputs::Contributions(proof_info);

            let inputs_source = guard.data_ctx.input_source.clone();
            let hints_source = guard.data_ctx.hints_source.clone();

            drop(guard);

            let result = Self::execute_contribution_task(
                job_id.clone(),
                prover.as_ref(),
                phase_inputs,
                inputs_source,
                hints_source,
                options,
            )
            .await;

            let mut guard = job.lock().await;

            guard.executed_steps = prover.executed_steps();

            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Challenge {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Contribution computation failed for {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Challenge {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn execute_contribution_task(
        job_id: JobId,
        prover: &ZiskProver<T>,
        phase_inputs: ProvePhaseInputs,
        input_source: InputSourceDto,
        hints_source: HintsSourceDto,
        options: ProofOptions,
    ) -> Result<Vec<ContributionsInfo>> {
        let phase = proofman::ProvePhase::Contributions;

        match input_source {
            InputSourceDto::InputPath(inputs_uri) => {
                let stdin = ZiskStdin::from_file(inputs_uri)?;

                prover.set_stdin(stdin);
            }
            InputSourceDto::InputData(input_data) => {
                let stdin = ZiskStdin::from_vec(input_data);
                prover.set_stdin(stdin);
            }
            InputSourceDto::InputNull => {
                let stdin = ZiskStdin::null();
                prover.set_stdin(stdin);
            }
        }

        match hints_source {
            HintsSourceDto::HintsPath(hints_uri) => {
                let hints_stream = StreamSource::from_uri(hints_uri.into())?;
                prover.set_hints_stream(hints_stream)?;
            }
            HintsSourceDto::HintsStream(_hints_uri) => {
                // let hints_stream = StreamSource::from_uri(hints_uri.into())?;
                // prover.set_hints_stream(hints_stream)?;
            }
            HintsSourceDto::HintsNull => {
                // No hints to set
            }
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

    pub async fn process_stream_data(&mut self, stream_data: StreamDataDto) -> Result<()> {
        let job_id = stream_data.job_id;
        let stream_type = stream_data.stream_type;

        if self.hints_processor.is_none() {
            let base_port = self.prover_config.asm_port;
            let local_rank = self.prover.local_rank();
            let unlock_mapped_memory = self.prover_config.unlock_mapped_memory;
            let hints_shmem = HintsShmem::new(base_port, local_rank, unlock_mapped_memory)?;
            self.hints_processor = Some(
                HintsProcessor::builder(hints_shmem)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to initialize hints processor: {}", e))?,
            );
        }

        // Check the existence of stream buffer based on stream type
        if stream_type == StreamMessageKind::Start {
            // Check if buffer already exists
            match self.stream_buffers.entry(job_id.clone()) {
                Entry::Occupied(_) => {
                    return Err(anyhow::anyhow!("Received duplicate START for job {}", job_id));
                }
                Entry::Vacant(entry) => {
                    entry.insert((1, HashMap::new()));
                }
            }

            return Ok(());
        } else if stream_type == StreamMessageKind::End {
            // Ensure buffer exists
            if !self.stream_buffers.contains_key(&job_id) {
                return Err(anyhow::anyhow!(
                    "Received {:?} without START for job {}",
                    stream_type,
                    job_id,
                ));
            }

            return Ok(());
        }

        let element = self.stream_buffers.get_mut(&job_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Received stream data without START for job {} stream type {:?}",
                job_id,
                stream_type
            )
        })?;

        let next_seq = &mut element.0;
        let stream_buffer = &mut element.1;

        let StreamPayloadDto { sequence_number: current_seq, mut payload } =
            stream_data.stream_payload.ok_or_else(|| {
                anyhow::anyhow!(
                    "Missing stream payload for job {} stream type {:?}",
                    job_id,
                    stream_type
                )
            })?;

        // Check if this is the expected sequence number
        // If not, buffer it for later processing
        if current_seq != *next_seq {
            stream_buffer.insert(current_seq, payload);
            return Ok(());
        }

        // Process the current payload (which has the expected sequence number)
        // and increment next_seq to expect the following sequence
        *next_seq += 1;

        // Check if we have any buffered subsequent payloads waiting
        // If so, append them to the current payload in order
        while let Some(buffered_data) = stream_buffer.remove(next_seq) {
            payload.extend(buffered_data);
            *next_seq += 1;
        }

        // Process the hints
        let payload = reinterpret_vec(payload)?;
        self.hints_processor.as_mut().unwrap().process_hints(&payload, current_seq == 1)?;

        Ok(())
    }

    pub async fn prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();

        let options = self.get_proof_options_prove();

        tokio::spawn(async move {
            let job = job.lock().await;
            let job_id = job.job_id.clone();

            info!("Computing Prove for {job_id}");

            let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

            let result =
                Self::execute_prove_task(job_id.clone(), prover.as_ref(), phase_inputs, options)
                    .await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Proofs {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Prove computation failed for {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Proofs {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        })
    }

    pub async fn execute_prove_task(
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

    pub async fn aggregate(
        &self,
        job: Arc<Mutex<JobContext>>,
        agg_params: AggregationParams,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let prover = self.prover.clone();

        tokio::spawn(async move {
            let job = job.lock().await;
            let job_id = job.job_id.clone();

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

            let options = Self::get_proof_options_aggregation(&agg_params);

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
                    let _ = tx.send(ComputationResult::AggProof {
                        job_id,
                        success: true,
                        result: Ok(Some(proof)),
                        executed_steps: job.executed_steps,
                    });
                }
                Err(error) => {
                    tracing::error!("Aggregation failed for {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::AggProof {
                        job_id,
                        success: false,
                        result: Err(error),
                        executed_steps: job.executed_steps,
                    });
                }
            }
        })
    }

    fn get_proof_options_partial_contribution(&self) -> ProofOptions {
        ProofOptions {
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            verify_proofs: true,
            save_proofs: true,
            test_mode: false,
            output_dir_path: PathBuf::from("."),
            rma: self.prover_config.rma,
            minimal_memory: self.prover_config.minimal_memory,
        }
    }

    fn get_proof_options_prove(&self) -> ProofOptions {
        ProofOptions {
            verify_constraints: false,
            aggregation: true,
            final_snark: false,
            verify_proofs: false,
            save_proofs: false,
            test_mode: false,
            output_dir_path: PathBuf::default(),
            rma: self.prover_config.rma,
            minimal_memory: self.prover_config.minimal_memory,
        }
    }

    fn get_proof_options_aggregation(agg_params: &AggregationParams) -> ProofOptions {
        ProofOptions {
            verify_constraints: agg_params.verify_constraints,
            aggregation: agg_params.aggregation,
            rma: agg_params.rma,
            final_snark: agg_params.final_snark,
            verify_proofs: agg_params.verify_proofs,
            save_proofs: agg_params.save_proofs,
            test_mode: agg_params.test_mode,
            output_dir_path: agg_params.output_dir_path.clone(),
            minimal_memory: agg_params.minimal_memory,
        }
    }

    // --------------------------------------------------------------------------
    // MPI Broadcast handlers for receiving and executing tasks
    // --------------------------------------------------------------------------

    pub async fn handle_mpi_broadcast_request(&self) -> Result<()> {
        let mut bytes: Vec<u8> = Vec::new();

        self.prover.mpi_broadcast(&mut bytes);

        // extract byte 0 to decide the option
        let phase = borsh::from_slice(&bytes[0..1]).unwrap();

        match phase {
            JobPhase::Contributions => {
                let (job_id, phase_inputs, options, input_source_dto, hints_source_dto): (
                    JobId,
                    ProvePhaseInputs,
                    ProofOptions,
                    InputSourceDto,
                    HintsSourceDto,
                ) = borsh::from_slice(&bytes[1..]).unwrap();

                let result = Self::execute_contribution_task(
                    job_id,
                    self.prover.as_ref(),
                    phase_inputs,
                    input_source_dto,
                    hints_source_dto,
                    options,
                )
                .await;
                if let Err(e) = result {
                    error!("Error during Contributions MPI broadcast execution: {}. Waiting for new job...", e);
                }
            }
            JobPhase::Prove => {
                let (job_id, phase_inputs, options): (JobId, ProvePhaseInputs, ProofOptions) =
                    borsh::from_slice(&bytes[1..]).unwrap();

                let result =
                    Self::execute_prove_task(job_id, self.prover.as_ref(), phase_inputs, options)
                        .await;
                if let Err(e) = result {
                    error!(
                        "Error during Prove MPI broadcast execution: {}. Waiting for new job...",
                        e
                    );
                }
            }
            JobPhase::Aggregate => {
                unreachable!("Aggregate phase is not supported in MPI broadcast");
            }
        }
        Ok(())
    }
}
