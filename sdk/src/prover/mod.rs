mod asm;
mod backend;
mod emu;

pub use asm::*;
use backend::*;
pub use emu::*;
use proofman::{AggProofs, ProvePhase, ProvePhaseInputs, ProvePhaseResult, SnarkProof};
use proofman_common::ProofOptions;
use proofman_util::VadcopFinalProof;

use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use zisk_common::{io::ZiskStdin, ExecutorStats, ZiskExecutionResult};

pub struct ZiskExecuteResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
}

pub struct ZiskVerifyConstraintsResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
    pub stats: ExecutorStats,
}

pub struct ZiskProgramVK {
    pub vk: Vec<u8>,
    pub starting_pos_publics_program_vk: u64,
}

#[derive(Debug, Clone)]
pub struct ProofOpts {
    pub aggregation: bool,
    pub verify_proofs: bool,
    pub rma: bool,
    pub minimal_memory: bool,
    pub output_dir_path: Option<PathBuf>,
    pub save_proofs: bool,
}

impl Default for ProofOpts {
    fn default() -> Self {
        Self {
            aggregation: true,
            verify_proofs: false,
            rma: false,
            minimal_memory: false,
            output_dir_path: None,
            save_proofs: false,
        }
    }
}

impl ProofOpts {
    pub fn output_dir(mut self, path: PathBuf) -> Self {
        self.output_dir_path = Some(path);
        self
    }

    pub fn save_proofs(mut self) -> Self {
        self.save_proofs = true;
        self
    }

    pub fn verify_proofs(mut self) -> Self {
        self.verify_proofs = true;
        self
    }

    pub fn minimal_memory(mut self) -> Self {
        self.minimal_memory = true;
        self
    }

    pub fn no_aggregation(mut self) -> Self {
        self.aggregation = false;
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProofMode {
    VadcopFinal,
    VadcopFinalCompressed,
    Plonk,
}

pub enum Proof {
    Null(),
    VadcopFinal(VadcopFinalProof),
    Plonk(SnarkProof),
}

pub struct ZiskProveResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
    pub stats: ExecutorStats,
    pub proof_id: Option<String>,
    pub proof: Proof,
}

impl ZiskProveResult {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        match &self.proof {
            Proof::Null() => Err(anyhow::anyhow!("No proof to save")),
            Proof::VadcopFinal(vadcop_proof) => {
                vadcop_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
            Proof::Plonk(snark_proof) => {
                snark_proof.save(path).map_err(|e| anyhow::anyhow!("{}", e))
            }
        }
    }
}

pub type ZiskPhaseResult = ProvePhaseResult;

pub struct ZiskAggPhaseResult {
    pub agg_proofs: Vec<AggProofs>,
}

pub trait ProverEngine {
    fn setup(&self, elf_path: PathBuf) -> Result<ZiskProgramVK>;

    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin) -> Result<()>;

    fn executed_steps(&self) -> u64;

    fn execute(&self, stdin: ZiskStdin, output_path: Option<PathBuf>) -> Result<ZiskExecuteResult>;

    fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)>;

    fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult>;

    fn verify_constraints(&self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult>;

    fn vk(&self, elf_path: PathBuf) -> Result<ZiskProgramVK>;

    fn verify(&self, proof: &ZiskProveResult, vk: &ZiskProgramVK) -> Result<()>;

    fn prove_debug(&self, stdin: ZiskStdin, proof_options: ProofOpts) -> Result<ZiskProveResult>;

    fn prove(
        &self,
        stdin: ZiskStdin,
        mode: ProofMode,
        proof_options: ProofOpts,
    ) -> Result<ZiskProveResult>;

    fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult>;

    fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>>;

    fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()>;
}

pub trait ZiskBackend: Send + Sync {
    type Prover: ProverEngine + Send + Sync;
}

pub struct ZiskProver<C: ZiskBackend> {
    pub prover: C::Prover,
}

impl<C: ZiskBackend> ZiskProver<C> {
    /// Create a new ZiskProver with the given prover engine.
    pub fn new(prover: C::Prover) -> Self {
        Self { prover }
    }

    pub fn setup(&self, elf_path: PathBuf) -> Result<ZiskProgramVK> {
        self.prover.setup(elf_path)
    }

    /// Set the standard input for the current proof.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.prover.set_stdin(stdin)
    }

    /// Get the world rank of the prover. The world rank is the rank of the prover in the global MPI context.
    /// If MPI is not used, this will always return 0.
    pub fn world_rank(&self) -> i32 {
        self.prover.world_rank()
    }

    /// Get the local rank of the prover. The local rank is the rank of the prover in the local MPI context.
    /// If MPI is not used, this will always return 0.
    pub fn local_rank(&self) -> i32 {
        self.prover.local_rank()
    }

    /// Get the number of executed steps by the prover after a proof generation or execution.
    pub fn executed_steps(&self) -> u64 {
        self.prover.executed_steps()
    }

    /// Execute the prover with the given standard input and output path.
    /// It only runs the execution without generating a proof.
    pub fn execute(&self, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        self.prover.execute(stdin, None)
    }

    /// Get the execution statistics with the given standard input and debug information.
    pub fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        minimal_memory: bool,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
        self.prover.stats(stdin, debug_info, minimal_memory, mpi_node)
    }

    /// Verify the constraints with the given standard input and debug information.
    pub fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult> {
        self.prover.verify_constraints_debug(stdin, debug_info)
    }

    /// Verify the constraints with the given standard input.
    pub fn verify_constraints(&self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult> {
        self.prover.verify_constraints(stdin)
    }

    pub fn vk(&self, elf_path: PathBuf) -> Result<ZiskProgramVK> {
        self.prover.vk(elf_path)
    }

    pub fn verify(&self, proof: &ZiskProveResult, vk: &ZiskProgramVK) -> Result<()> {
        self.prover.verify(proof, vk)
    }

    /// Generate a proof with the given standard input.
    /// Returns a `ProveBuilder` that allows setting per-proof options before running.
    ///
    /// # Example
    /// ```ignore
    /// let result = prover.prove(stdin).compressed().run()?;
    /// ```
    pub fn prove(&self, stdin: ZiskStdin) -> ProveBuilder<'_, C> {
        ProveBuilder::new(&self.prover, stdin)
    }

    pub fn prove_phase(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ZiskPhaseResult> {
        self.prover.prove_phase(phase_inputs, options, phase)
    }

    pub fn aggregate_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Result<Option<ZiskAggPhaseResult>> {
        self.prover.aggregate_proofs(agg_proofs, last_proof, final_proof, options)
    }

    /// Broadcast data to all MPI processes.
    pub fn mpi_broadcast(&self, data: &mut Vec<u8>) -> Result<()> {
        self.prover.mpi_broadcast(data)
    }
}

/// Builder for configuring and running a proof.
///
/// This struct provides a fluent API for setting per-proof options
/// before executing the proof generation.
///
/// # Example
/// ```ignore
/// let result = prover.prove(stdin).compressed().run()?;
/// ```
pub struct ProveBuilder<'a, C: ZiskBackend> {
    prover: &'a C::Prover,
    stdin: ZiskStdin,
    mode: ProofMode,
    proof_options: ProofOpts,
}

impl<'a, C: ZiskBackend> ProveBuilder<'a, C> {
    fn new(prover: &'a C::Prover, stdin: ZiskStdin) -> Self {
        Self { prover, stdin, mode: ProofMode::VadcopFinal, proof_options: ProofOpts::default() }
    }

    /// Enable compressed proof generation.
    pub fn compressed(mut self) -> Self {
        self.mode = ProofMode::VadcopFinalCompressed;
        self
    }

    pub fn plonk(mut self) -> Self {
        self.mode = ProofMode::Plonk;
        self
    }

    pub fn with_proof_options(mut self, options: ProofOpts) -> Self {
        self.proof_options = options;
        self
    }

    /// Execute the proof generation with the configured options.
    pub fn run(self) -> Result<ZiskProveResult> {
        self.prover.prove(self.stdin, self.mode, self.proof_options)
    }

    pub fn run_debug(self) -> Result<ZiskProveResult> {
        self.prover.prove_debug(self.stdin, self.proof_options)
    }
}
