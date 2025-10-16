mod asm;
mod backend;
mod emu;

pub(crate) use asm::*;
use backend::*;
pub(crate) use emu::*;

use crate::Proof;
use anyhow::Result;
use std::{path::PathBuf, time::Duration};
use zisk_common::{ExecutorStats, ZiskExecutionResult};

pub trait ProverEngine {
    fn verify_constraints(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)>;

    fn debug_verify_constraints(
        &self,
        input: Option<PathBuf>,
        debug_info: Option<Option<String>>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)>;

    fn prove(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats, Proof)>;
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

    pub fn debug_verify_constraints(
        &self,
        input: Option<PathBuf>,
        debug_info: Option<Option<String>>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        self.prover.debug_verify_constraints(input, debug_info)
    }

    pub fn verify_constraints(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats)> {
        self.prover.verify_constraints(input)
    }

    pub fn generate_proof(
        &self,
        input: Option<PathBuf>,
    ) -> Result<(ZiskExecutionResult, Duration, ExecutorStats, Proof)> {
        self.prover.prove(input)
    }
}
