mod client;
pub mod core;
mod execute;
mod hints;
mod program;
mod proof;
mod prove;
mod setup;
mod stdin;
mod types;
mod upload;

pub use client::{EmbeddedClientBuilder, EmbeddedOptions, ProverClient, ProverClientBuilder};
pub use execute::{ExecuteRequest, ExecuteResult, Tracing};
pub use hints::ZiskHints;
// pub use program::{Elf, GuestProgram, ProgramId};
pub use proof::Proof;
pub use prove::ProveRequest;
pub use setup::SetupRequest;
pub use stdin::ZiskStdin;
pub use types::{ClientConfig, Executor, WatchEvent};
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
