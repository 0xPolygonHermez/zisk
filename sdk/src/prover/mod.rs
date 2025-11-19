mod asm;
mod backend;
mod emu;

pub use asm::*;
use backend::*;
pub use emu::*;
use proofman::{AggProofs, ProvePhase, ProvePhaseInputs, ProvePhaseResult};
use proofman_common::ProofOptions;

use crate::Proof;
use anyhow::Result;
use std::{path::PathBuf, time::Duration};
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

pub struct ZiskProveResult {
    pub execution: ZiskExecutionResult,
    pub duration: Duration,
    pub stats: ExecutorStats,
    pub proof: Proof,
}

pub type ZiskPhaseResult = ProvePhaseResult;

pub struct ZiskAggPhaseResult {
    pub agg_proofs: Vec<AggProofs>,
}

pub trait ProverEngine {
    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin);

    fn executed_steps(&self) -> u64;

    fn execute(&self, stdin: ZiskStdin, output_path: Option<PathBuf>) -> Result<ZiskExecuteResult>;

    fn stats(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)>;

    fn verify_constraints_debug(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<ZiskVerifyConstraintsResult>;

    fn verify_constraints(&self, stdin: ZiskStdin) -> Result<ZiskVerifyConstraintsResult>;

    fn prove(&self, stdin: ZiskStdin) -> Result<ZiskProveResult>;

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

    fn mpi_broadcast(&self, data: &mut Vec<u8>);
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

    /// Set the standard input for the current proof.
    pub fn set_stdin(&self, stdin: ZiskStdin) {
        self.prover.set_stdin(stdin);
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
        mpi_node: Option<u32>,
    ) -> Result<(i32, i32, Option<ExecutorStats>)> {
        self.prover.stats(stdin, debug_info, mpi_node)
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

    /// Generate a proof with the given standard input.
    pub fn prove(&self, stdin: ZiskStdin) -> Result<ZiskProveResult> {
        self.prover.prove(stdin)
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
    pub fn mpi_broadcast(&self, data: &mut Vec<u8>) {
        self.prover.mpi_broadcast(data);
    }
}
