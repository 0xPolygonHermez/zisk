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

pub trait ProverEngine {
    fn world_rank(&self) -> i32;

    fn local_rank(&self) -> i32;

    fn set_stdin(&self, stdin: ZiskStdin);

    fn executed_steps(&self) -> u64;

    fn execute(
        &self,
        stdin: ZiskStdin,
        output_path: PathBuf,
    ) -> Result<(ZiskExecutionResult, Duration)>;

    fn debug_verify_constraints(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)>;

    fn verify_constraints(
        &self,
        stdin: ZiskStdin,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)>;

    fn prove(
        &self,
        stdin: ZiskStdin,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats, Proof)>;

    fn generate_proof_from_lib(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ProvePhaseResult, Box<dyn std::error::Error>>;

    fn receive_aggregated_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Option<Vec<AggProofs>>;

    fn mpi_broadcast(&self, data: &mut Vec<u8>);
}

pub trait ZiskBackend: Send + Sync {
    type Prover: ProverEngine + Send + Sync;
}

pub struct ZiskProver<C: ZiskBackend> {
    pub prover: C::Prover,
}

impl<C: ZiskBackend> ZiskProver<C> {
    pub fn new(prover: C::Prover) -> Self {
        Self { prover }
    }

    pub fn set_stdin(&self, stdin: ZiskStdin) {
        self.prover.set_stdin(stdin);
    }

    pub fn world_rank(&self) -> i32 {
        self.prover.world_rank()
    }

    pub fn local_rank(&self) -> i32 {
        self.prover.local_rank()
    }

    pub fn executed_steps(&self) -> u64 {
        self.prover.executed_steps()
    }

    pub fn execute(
        &self,
        stdin: ZiskStdin,
        output_path: PathBuf,
    ) -> Result<(ZiskExecutionResult, Duration)> {
        self.prover.execute(stdin, output_path)
    }

    pub fn debug_verify_constraints(
        &self,
        stdin: ZiskStdin,
        debug_info: Option<Option<String>>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        self.prover.debug_verify_constraints(stdin, debug_info)
    }

    pub fn verify_constraints(
        &self,
        stdin: ZiskStdin,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        self.prover.verify_constraints(stdin)
    }

    pub fn generate_proof(
        &self,
        stdin: ZiskStdin,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats, Proof)> {
        self.prover.prove(stdin)
    }

    pub fn generate_proof_from_lib(
        &self,
        phase_inputs: ProvePhaseInputs,
        options: ProofOptions,
        phase: ProvePhase,
    ) -> Result<ProvePhaseResult, Box<dyn std::error::Error>> {
        self.prover.generate_proof_from_lib(phase_inputs, options, phase)
    }

    pub fn receive_aggregated_proofs(
        &self,
        agg_proofs: Vec<AggProofs>,
        last_proof: bool,
        final_proof: bool,
        options: &ProofOptions,
    ) -> Option<Vec<AggProofs>> {
        self.prover.receive_aggregated_proofs(agg_proofs, last_proof, final_proof, options)
    }

    pub fn mpi_broadcast(&self, data: &mut Vec<u8>) {
        self.prover.mpi_broadcast(data);
    }
}
