use anyhow::Result;
use asm_runner::{AsmRunnerOptions, AsmServices};
use consensus_api::ProverAllocation;
use consensus_core::{BlockContext, ComputeCapacity, JobId, JobPhase, ProverId, ProverState};
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProofMan;
use proofman_common::{DebugInfo, ParamsGPU, ProofOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{error, info};
use witness::WitnessLibrary;

use zisk_common::{MpiContext, ZiskLibInitFn};

use crate::prover_grpc_endpoint::ComputationResult;

/// Current job context
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: JobId,
    pub block: BlockContext,
    pub rank_id: u32,
    pub total_provers: u32,
    pub allocation: Vec<u32>, // Prover allocation for this job, vector of all computed units assigned
    pub total_compute_units: u32, // Total compute units for the whole job
    pub phase: JobPhase,
}

pub struct ProverServiceConfig {
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
    pub debug_info: Arc<DebugInfo>,

    /// Size of the chunks in bits
    pub chunk_size_bits: Option<u64>,

    /// Additional options for the ASM runner
    pub asm_runner_options: AsmRunnerOptions,

    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,

    pub gpu_params: ParamsGPU,
}

#[allow(clippy::too_many_arguments)]
impl ProverServiceConfig {
    pub fn new(
        elf: PathBuf,
        witness_lib: PathBuf,
        asm: Option<PathBuf>,
        asm_rom: Option<PathBuf>,
        custom_commits_map: HashMap<String, PathBuf>,
        emulator: bool,
        proving_key: PathBuf,
        verbose: u8,
        debug: DebugInfo,
        chunk_size_bits: Option<u64>,
        asm_runner_options: AsmRunnerOptions,
        verify_constraints: bool,
        aggregation: bool,
        final_snark: bool,
        gpu_params: ParamsGPU,
    ) -> Self {
        Self {
            elf,
            witness_lib,
            asm,
            asm_rom,
            custom_commits_map,
            emulator,
            proving_key,
            verbose,
            debug_info: Arc::new(debug),
            chunk_size_bits,
            asm_runner_options,
            verify_constraints,
            aggregation,
            final_snark,
            gpu_params,
        }
    }
}

pub struct ProverService {
    prover_id: ProverId,
    compute_capacity: ComputeCapacity,
    state: ProverState,
    current_job: Option<Arc<Mutex<JobContext>>>,
    current_computation: Option<JoinHandle<()>>,
    config: ProverServiceConfig,
    // It is important to keep the witness_lib declaration before the proofman declaration
    // to ensure that the witness library is dropped before the proofman.
    witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync>,
    proofman: Arc<ProofMan<Goldilocks>>,
    mpi_context: MpiContext,
    asm_services: Option<AsmServices>,
    is_busy: AtomicBool,
    pending_handles: Vec<std::thread::JoinHandle<()>>,
}

impl ProverService {
    pub fn new(
        prover_id: ProverId,
        compute_capacity: ComputeCapacity,
        config: ProverServiceConfig,
        mpi_context: MpiContext,
    ) -> Result<Self> {
        info!("Starting asm microservices...");

        let world_rank = config.asm_runner_options.world_rank;
        let local_rank = config.asm_runner_options.local_rank;
        let base_port = config.asm_runner_options.base_port;
        let unlock_mapped_memory = config.asm_runner_options.unlock_mapped_memory;

        let asm_services = if config.emulator {
            None
        } else {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            asm_services.start_asm_services(
                config.asm.as_ref().unwrap(),
                config.asm_runner_options.clone(),
            )?;
            Some(asm_services)
        };

        let library =
            unsafe { Library::new(config.witness_lib.clone()).expect("Failed to load library") };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library").expect("Failed to get symbol") };

        let mut witness_lib = witness_lib_constructor(
            config.verbose.into(),
            config.elf.clone(),
            config.asm.clone(),
            config.asm_rom.clone(),
            config.chunk_size_bits,
            Some(world_rank),
            Some(local_rank),
            base_port,
            unlock_mapped_memory,
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
        )
        .expect("Failed to initialize proofman");

        proofman.register_witness(witness_lib.as_mut(), library);

        let witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync> = Arc::from(witness_lib);

        Ok(Self {
            prover_id,
            compute_capacity,
            state: ProverState::Disconnected,
            current_job: None,
            current_computation: None,
            config,
            witness_lib,
            proofman: Arc::new(proofman),
            mpi_context,
            asm_services,
            is_busy: AtomicBool::new(false),
            pending_handles: Vec::new(),
        })
    }

    pub fn get_state(&self) -> &ProverState {
        &self.state
    }

    pub fn set_state(&mut self, state: ProverState) {
        self.state = state;
    }

    pub fn get_current_job(&self) -> Option<Arc<Mutex<JobContext>>> {
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

    pub fn set_current_computation(&mut self, handle: Option<JoinHandle<()>>) {
        self.current_computation = handle;
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
        total_provers: u32,
        allocation: Vec<ProverAllocation>,
        total_compute_units: u32,
    ) -> Arc<Mutex<JobContext>> {
        let current_job = Arc::new(Mutex::new(JobContext {
            job_id,
            block,
            rank_id,
            total_provers,
            allocation: allocation
                .iter()
                .flat_map(|alloc| alloc.range_start..alloc.range_end)
                .collect(),
            total_compute_units,
            phase: JobPhase::Phase1,
        }));
        self.current_job = Some(current_job.clone());

        self.state = ProverState::Computing(JobPhase::Phase1);

        current_job
    }

    pub async fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> Option<JoinHandle<()>> {
        let proofman = self.proofman.clone();

        proofman.set_mpi_ctx2(1, 0);

        let job_id = job.lock().await.job_id.clone();

        Some(tokio::spawn(async move {
            let result = Self::compute_phase1_task(job_id.clone(), job, proofman).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Phase1 {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Phase 1 computation failed for job {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Phase1 {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        }))
    }

    pub async fn compute_phase1_task(
        job_id: JobId,
        job: Arc<Mutex<JobContext>>,
        proofman: Arc<ProofMan<Goldilocks>>,
    ) -> Result<Vec<u64>> {
        info!("Computing Phase 1 for job {}", job_id);

        // Prepare parameters
        let phase_inputs = proofman::ProvePhaseInputs::Contributions(Some(
            job.lock().await.block.input_path.clone(),
        ));

        let options = ProofOptions {
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            verify_proofs: true,
            save_proofs: true,
            test_mode: false,
            output_dir_path: PathBuf::from("."),
            minimal_memory: true,
        };
        let phase = proofman::ProvePhase::Contributions;

        // Handle the result immediately without holding it across await
        let challenge = match proofman.generate_proof_from_lib(phase_inputs, options, phase) {
            Ok(proofman::ProvePhaseResult::Contributions(challenge)) => {
                info!("Phase 1 computation successful for job {}", job_id);
                challenge
            }
            Ok(_) => {
                error!("Error during Phase 1 computation for job {}", job_id);
                return Err(anyhow::anyhow!("Unexpected result type during Phase 1 computation"));
            }
            Err(err) => {
                error!("Failed to generate proof for job {}: {:?}", job_id, err);
                return Err(anyhow::anyhow!("Failed to generate proof"));
            }
        };

        println!("Phase 1 challenge: {:?}", challenge);

        Ok(challenge.to_vec())
    }

    pub async fn prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<Vec<u64>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> Option<JoinHandle<()>> {
        let proofman = self.proofman.clone();

        // TODO!!!!!!! Challenges must arrive in Vec<[u64;10]>
        let challenges: Vec<[u64; 10]> = challenges
            .into_iter()
            .map(|v| v.try_into().expect("Each challenge must have exactly 10 elements"))
            .collect();

        let job_id = job.lock().await.job_id.clone();

        Some(tokio::spawn(async move {
            let result = Self::execute_phase2(job, proofman, challenges).await;
            match result {
                Ok(data) => {
                    let _ = tx.send(ComputationResult::Phase2 {
                        job_id,
                        success: true,
                        result: Ok(data),
                    });
                }
                Err(error) => {
                    error!("Phase 2 computation failed for job {}: {}", job_id, error);
                    let _ = tx.send(ComputationResult::Phase2 {
                        job_id,
                        success: false,
                        result: Err(error),
                    });
                }
            }
        }))
    }

    pub async fn execute_phase2(
        job: Arc<Mutex<JobContext>>,
        proofman: Arc<ProofMan<Goldilocks>>,
        challenges: Vec<[u64; 10]>,
    ) -> Result<Vec<Vec<u64>>> {
        let job = job.lock().await;
        let job_id = job.job_id.clone();
        let input_path = job.block.input_path.clone();

        info!("Computing Phase 2 for job {}", job_id);

        // Prepare parameters
        let phase_inputs = proofman::ProvePhaseInputs::Internal(challenges);

        let options = ProofOptions {
            verify_constraints: false,
            aggregation: false,
            final_snark: false,
            verify_proofs: true,
            save_proofs: true,
            test_mode: false,
            output_dir_path: PathBuf::from("."),
            minimal_memory: true,
        };
        let phase = proofman::ProvePhase::Internal;

        // Handle the result immediately without holding it across await
        let proof = match proofman.generate_proof_from_lib(phase_inputs, options, phase) {
            Ok(proofman::ProvePhaseResult::Internal(proof)) => {
                info!("Phase 1 computation successful for job {}", job_id);
                proof.unwrap()
            }
            Ok(_) => {
                error!("Error during Phase 1 computation for job {}", job_id);
                return Err(anyhow::anyhow!("Unexpected result type during Phase 1 computation"));
            }
            Err(err) => {
                error!("Failed to generate proof for job {}: {:?}", job_id, err);
                return Err(anyhow::anyhow!("Failed to generate proof"));
            }
        };

        info!("Phase 2 computation completed for job {}", job.job_id);

        Ok(proof)
    }
}
