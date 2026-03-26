mod client;
pub mod core;
mod execute;
mod hints;
mod proof;
mod prove;
mod prover;
mod setup;
mod stdin;
mod upload;

pub use client::ProverClient;
pub use execute::{ExecuteRequest, ExecuteResult, Tracing};
pub use hints::ZiskHints;
pub use prover::{EmbeddedClientBuilder, EmbeddedOptions};
// pub use program::{Elf, GuestProgram, ProgramId};
pub use proof::Proof;
pub use prove::ProveRequest;
pub use setup::SetupRequest;
pub use stdin::ZiskStdin;
pub use upload::UploadRequest;

// Re-export guest types from backend (public API for loading programs)
pub use zisk_prover_backend::{load_program, Elf, EmuOptions, GuestProgram, ProgramId};

// Re-export result and data types from backend (public outputs)
pub use zisk_prover_backend::{
    PlonkBuilder, ProofOpts, ProveBuilder, ReduceBuilder, ZiskExecuteResult, ZiskProveResult,
    ZiskVerifyConstraintsResult,
};

// Re-export common types
pub use proofman_common::VerboseMode;

// Re-export types from zisk_common
pub use zisk_common::{
    io::*, PlonkVkey, ProofMode, ZiskProgramVK, ZiskProof, ZiskProofWithPublicValues, ZiskPublics,
    ZiskVK,
};

pub use zisk_build::*;

use anyhow::Result;

/// Executor backend for running programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutorKind {
    /// Emulator: always available.
    #[default]
    Emulator,
    /// Assembly: must be explicitly enabled on the client builder.
    Assembly,
}

/// Events emitted during proof generation.
///
/// `WatchEvent::All` is a subscription filter meaning "receive all events".
/// It is never emitted as a concrete event in callbacks.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// Subscribe to all events (filter only; never emitted to callbacks).
    All,
    /// Job accepted and execution started.
    Started,
    /// Proof generation progress (0–100).
    Progress(u8),
    /// Proof completed successfully.
    Completed,
    /// Proof generation failed.
    Failed(String),
}

/// Core client trait implemented by all prover backends.
pub trait Client: Send + Sync {
    /// Run a prove operation with the given executor.
    fn run_prove(
        &self,
        program: &GuestProgram,
        stdin: zisk_common::io::ZiskStdin,
        executor: ExecutorKind,
        opts: ProofOpts,
    ) -> Result<Proof>;

    /// Run an execute operation (dry-run, no proof) with the given executor.
    fn run_execute(
        &self,
        program: &GuestProgram,
        stdin: zisk_common::io::ZiskStdin,
        executor: ExecutorKind,
    ) -> Result<ExecuteResult>;
}
