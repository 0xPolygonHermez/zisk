mod builder;
mod client;
mod public_prover;

pub use builder::*;
pub use client::ProverClient;
pub use public_prover::PublicZiskProver;

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
