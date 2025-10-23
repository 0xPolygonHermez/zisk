use anyhow::Result;
use cargo_zisk::commands::{get_proving_key, get_witness_computation_lib};
use proofman::{AggProofs, ContributionsInfo};
use rom_setup::{
    gen_elf_hash, get_elf_bin_file_path, get_elf_data_hash, get_rom_blowup_factor,
    DEFAULT_CACHE_PATH,
};
use std::fs;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use zisk_distributed_common::{AggregationParams, BlockContext, JobPhase, WorkerState};
use zisk_distributed_common::{ComputeCapacity, JobId, WorkerId};

use asm_runner::AsmRunnerOptions;
use asm_runner::AsmServices;
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProvePhaseInputs;
use proofman::{ProofInfo, ProofMan};
use proofman_common::ParamsGPU;
use proofman_common::ProofOptions;
use proofman_common::{json_to_debug_instances_map, DebugInfo};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, info};

use zisk_common::{ZiskLib, ZiskLibInitFn};

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
                json_to_debug_instances_map(proving_key.clone(), debug_value.clone())
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
        let blowup_factor = get_rom_blowup_factor(&proving_key);
        let rom_bin_path = get_elf_bin_file_path(
            &prover_service_config.elf.to_path_buf(),
            &default_cache_path,
            blowup_factor,
        )?;
        if !rom_bin_path.exists() {
            let _ = gen_elf_hash(
                &prover_service_config.elf.clone(),
                rom_bin_path.as_path(),
                blowup_factor,
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
        })
    }
}

/// Current job context
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: JobId,
    pub block: BlockContext,
    pub rank_id: u32,
    pub total_workers: u32,
    pub allocation: Vec<u32>, // Worker allocation for this job, vector of all computed units assigned
    pub total_compute_units: u32, // Total compute units for the whole job
    pub phase: JobPhase,
}

pub struct Worker {
    _worker_id: WorkerId,
    _compute_capacity: ComputeCapacity,
    state: WorkerState,
    current_job: Option<Arc<Mutex<JobContext>>>,
    current_computation: Option<JoinHandle<()>>,
    prover_config: ProverConfig,

    // It is important to keep the witness_lib declaration before the proofman declaration
    // to ensure that the witness library is dropped before the proofman.
    _witness_lib: Arc<Box<dyn ZiskLib<Goldilocks>>>,
    _asm_services: Option<AsmServices>,

    proofman: Arc<ProofMan<Goldilocks>>,
    local_rank: i32,
}

impl Worker {
    pub fn new(
        worker_id: WorkerId,
        compute_capacity: ComputeCapacity,
        config: ProverConfig,
    ) -> Result<Self> {
        info!("Starting asm microservices...");

        let library =
            unsafe { Library::new(config.witness_lib.clone()).expect("Failed to load library") };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library").expect("Failed to get symbol") };

        let base_port = config.asm_port;
        let unlock_mapped_memory = config.unlock_mapped_memory;

        let mut witness_lib = witness_lib_constructor(
            config.verbose.into(),
            config.elf.clone(),
            config.asm.clone(),
            config.asm_rom.clone(),
            base_port,
            unlock_mapped_memory,
            config.shared_tables,
        )
        .expect("Failed to initialize witness library");

        let proofman = ProofMan::<Goldilocks>::new(
            config.proving_key.clone(),
            config.custom_commits_map.clone(),
            config.verify_constraints,
            config.aggregation,
            config.final_snark,
            config.gpu_params.clone(),
            config.verbose.into(),
            witness_lib.get_packed_info(),
        )
        .expect("Failed to initialize proofman");

        let world_rank = proofman.get_world_rank();
        let local_rank = proofman.get_local_rank();

        let asm_runner_options = AsmRunnerOptions::new()
            .with_verbose(config.verbose > 0)
            .with_base_port(config.asm_port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_unlock_mapped_memory(config.unlock_mapped_memory);

        let asm_services = if config.emulator {
            None
        } else {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            asm_services
                .start_asm_services(config.asm.as_ref().unwrap(), asm_runner_options.clone())?;
            Some(asm_services)
        };

        proofman.register_witness(&mut *witness_lib, library);

        let witness_lib = Arc::from(witness_lib);

        Ok(Self {
            _worker_id: worker_id,
            _compute_capacity: compute_capacity,
            state: WorkerState::Disconnected,
            current_job: None,
            current_computation: None,
            prover_config: config,
            _witness_lib: witness_lib,
            proofman: Arc::new(proofman),
            local_rank,
            _asm_services: asm_services,
        })
    }

    pub fn local_rank(&self) -> i32 {
        self.local_rank
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
        block: BlockContext,
        rank_id: u32,
        total_workers: u32,
        allocation: Vec<u32>,
        total_compute_units: u32,
    ) -> Arc<Mutex<JobContext>> {
        let current_job = Arc::new(Mutex::new(JobContext {
            job_id: job_id.clone(),
            block,
            rank_id,
            total_workers,
            allocation,
            total_compute_units,
            phase: JobPhase::Contributions,
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
            Some(job.block.input_path.clone()),
            job.total_compute_units as usize,
            job.allocation.clone(),
            job.rank_id as usize,
        );
        let phase_inputs = proofman::ProvePhaseInputs::Contributions(proof_info);

        let options = Self::get_proof_options_partial_contribution();

        let mut serialized =
            borsh::to_vec(&(JobPhase::Contributions, job_id, phase_inputs, options)).unwrap();

        self.proofman.mpi_broadcast(&mut serialized);
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

        let options = Self::get_proof_options_prove();

        let mut serialized =
            borsh::to_vec(&(JobPhase::Prove, job_id, phase_inputs, options)).unwrap();

        self.proofman.mpi_broadcast(&mut serialized);
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
        let proofman = self.proofman.clone();

        tokio::spawn(async move {
            let job = job.lock().await;
            let job_id = job.job_id.clone();

            info!("Computing Contribution for {job_id}");

            let proof_info = ProofInfo::new(
                Some(job.block.input_path.clone()),
                job.total_compute_units as usize,
                job.allocation.clone(),
                job.rank_id as usize,
            );
            let phase_inputs = proofman::ProvePhaseInputs::Contributions(proof_info);

            let options = Self::get_proof_options_partial_contribution();

            let result =
                Self::execute_contribution_task(job_id.clone(), proofman, phase_inputs, options)
                    .await;

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
        proofman: Arc<ProofMan<Goldilocks>>,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
    ) -> Result<Vec<ContributionsInfo>> {
        let phase = proofman::ProvePhase::Contributions;

        // Handle the result immediately without holding it across await
        let challenge = match proofman.generate_proof_from_lib(phase_inputs, options, phase) {
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

    pub async fn prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        let proofman = self.proofman.clone();

        tokio::spawn(async move {
            let job = job.lock().await;
            let job_id = job.job_id.clone();

            info!("Computing Prove for {job_id}");

            let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

            let options = Self::get_proof_options_prove();

            let result =
                Self::execute_prove_task(job_id.clone(), proofman, phase_inputs, options).await;
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
        proofman: Arc<ProofMan<Goldilocks>>,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
    ) -> Result<Vec<AggProofs>> {
        let world_rank = proofman.rank().unwrap_or(0);

        let proof = match proofman.generate_proof_from_lib(
            phase_inputs,
            options,
            proofman::ProvePhase::Internal,
        ) {
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
        let proofman = self.proofman.clone();
        let witness_lib = self._witness_lib.clone();

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

            let result: Vec<Vec<u64>> = proofman
                .receive_aggregated_proofs(
                    agg_proofs,
                    agg_params.last_proof,
                    agg_params.final_proof,
                    &options,
                )
                .map(|proof| proof.into_iter().map(|p| p.proof).collect())
                .unwrap_or_default();

            let executed_steps = match witness_lib.get_execution_result() {
                Some((exec_result, _)) => exec_result.executed_steps,
                None => {
                    error!("Failed to get execution result from witness library for {job_id}");
                    0
                }
            };

            let _ = tx.send(ComputationResult::AggProof {
                job_id,
                success: true,
                result: Ok(Some(result)),
                executed_steps,
            });
        })
    }

    fn get_proof_options_partial_contribution() -> ProofOptions {
        ProofOptions {
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            verify_proofs: true,
            save_proofs: true,
            test_mode: false,
            output_dir_path: PathBuf::from("."),
            minimal_memory: false,
        }
    }

    fn get_proof_options_prove() -> ProofOptions {
        ProofOptions {
            verify_constraints: false,
            aggregation: true,
            final_snark: false,
            verify_proofs: false,
            save_proofs: false,
            test_mode: false,
            output_dir_path: PathBuf::default(),
            minimal_memory: false,
        }
    }

    fn get_proof_options_aggregation(agg_params: &AggregationParams) -> ProofOptions {
        ProofOptions {
            verify_constraints: agg_params.verify_constraints,
            aggregation: agg_params.aggregation,
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

        self.proofman.mpi_broadcast(&mut bytes);

        // extract byte 0 to decide the option
        let phase = borsh::from_slice(&bytes[0..1]).unwrap();

        match phase {
            JobPhase::Contributions => {
                let (job_id, phase_inputs, options): (JobId, ProvePhaseInputs, ProofOptions) =
                    borsh::from_slice(&bytes[1..]).unwrap();

                Self::execute_contribution_task(
                    job_id,
                    self.proofman.clone(),
                    phase_inputs,
                    options,
                )
                .await?;
            }
            JobPhase::Prove => {
                let (job_id, phase_inputs, options): (JobId, ProvePhaseInputs, ProofOptions) =
                    borsh::from_slice(&bytes[1..]).unwrap();

                Self::execute_prove_task(job_id, self.proofman.clone(), phase_inputs, options)
                    .await?;
            }
            JobPhase::Aggregate => {
                unreachable!("Aggregate phase is not supported in MPI broadcast");
            }
        }
        Ok(())
    }
}
