mod asm;
mod client;
mod emu;
mod utils;
mod zisk_lib_loader;

use std::path::PathBuf;

pub use client::{ProverClient, ProverClientBuilder};
pub use utils::*;
use zisk_common::{ExecutorStats, ZiskExecutionResult};
pub use zisk_lib_loader::*;

pub use anyhow::Result;

pub struct RankInfo {
    pub world_rank: i32,
    pub local_rank: i32,
}

pub struct Proof;

pub trait ProverEngine {
    fn verify_constraints(&self, input: Option<PathBuf>) -> Result<()>;
    fn generate_proof(&self, input: Option<PathBuf>) -> Result<Proof>;
    fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)>; // TODO parametrize these types
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

    pub fn verify_constraints(&self, input: Option<PathBuf>) -> Result<()> {
        self.prover.verify_constraints(input)
    }

    pub fn generate_proof(&self, input: Option<PathBuf>) -> Result<Proof> {
        self.prover.generate_proof(input)
    }

    pub fn execution_result(&self) -> Option<(ZiskExecutionResult, ExecutorStats)> {
        self.prover.execution_result()
    }
}
