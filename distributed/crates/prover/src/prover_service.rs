use anyhow::Result;
use distributed_common::{AggregationParams, BlockContext, JobPhase, ProverState};
use distributed_common::{ComputeCapacity, JobId, ProverId};
use proofman::{AggProofs, ContributionsInfo};
use proofman_common::{DebugInfo, ParamsGPU};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::proof_generator::ProofGenerator;

/// Result from computation tasks
#[derive(Debug)]
pub enum ComputationResult {
    Challenge { job_id: JobId, success: bool, result: Result<Vec<ContributionsInfo>> },
    Proofs { job_id: JobId, success: bool, result: Result<Vec<AggProofs>> },
    AggProof { job_id: JobId, success: bool, result: Result<Option<Vec<u64>>> },
}

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
    pub total_tables: u32,
    pub table_id_start: u32,
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
    // pub asm_runner_options: AsmRunnerOptions,
    pub asm_port: Option<u16>,

    pub unlock_mapped_memory: bool,

    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,

    pub gpu_params: ParamsGPU,

    pub shared_tables: bool,
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
        // asm_runner_options: AsmRunnerOptions,
        asm_port: Option<u16>,
        unlock_mapped_memory: bool,
        verify_constraints: bool,
        aggregation: bool,
        final_snark: bool,
        gpu_params: ParamsGPU,
        shared_tables: bool,
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
            asm_port,
            unlock_mapped_memory,
            verify_constraints,
            aggregation,
            final_snark,
            gpu_params,
            shared_tables,
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
    proof_generator: ProofGenerator,
}

impl ProverService {
    pub fn new(
        prover_id: ProverId,
        compute_capacity: ComputeCapacity,
        config: ProverServiceConfig,
    ) -> Result<Self> {
        let proof_generator = ProofGenerator::new(&config)?;

        Ok(Self {
            prover_id,
            compute_capacity,
            state: ProverState::Disconnected,
            current_job: None,
            current_computation: None,
            config,
            proof_generator,
        })
    }

    pub fn local_rank(&self) -> i32 {
        self.proof_generator.local_rank()
    }

    pub async fn receive_mpi_request(&self) -> Result<()> {
        self.proof_generator.receive_mpi_request().await
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

    pub fn set_current_computation(&mut self, handle: JoinHandle<()>) {
        self.current_computation = Some(handle);
    }

    pub fn cancel_current_computation(&mut self) {
        if let Some(handle) = self.current_computation.take() {
            handle.abort();
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_job(
        &mut self,
        job_id: JobId,
        block: BlockContext,
        rank_id: u32,
        total_provers: u32,
        allocation: Vec<u32>,
        total_compute_units: u32,
        total_tables: u32,
        table_id_start: u32,
    ) -> Arc<Mutex<JobContext>> {
        let current_job = Arc::new(Mutex::new(JobContext {
            job_id,
            block,
            rank_id,
            total_provers,
            allocation,
            total_compute_units,
            phase: JobPhase::Contributions,
            total_tables,
            table_id_start,
        }));
        self.current_job = Some(current_job.clone());

        self.state = ProverState::Computing(JobPhase::Contributions);

        current_job
    }

    pub async fn partial_contribution(
        &self,
        job: Arc<Mutex<JobContext>>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        self.proof_generator.partial_contribution_broadcast(job.clone()).await;
        self.proof_generator.partial_contribution(job, tx).await
    }

    pub async fn prove(
        &self,
        job: Arc<Mutex<JobContext>>,
        challenges: Vec<ContributionsInfo>,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        self.proof_generator.prove_broadcast(job.clone(), challenges.clone()).await;
        self.proof_generator.prove(job, challenges, tx).await
    }

    pub async fn aggregate(
        &self,
        job: Arc<Mutex<JobContext>>,
        agg_params: AggregationParams,
        tx: mpsc::UnboundedSender<ComputationResult>,
    ) -> JoinHandle<()> {
        self.proof_generator.aggregate(job, agg_params, tx).await
    }
}
