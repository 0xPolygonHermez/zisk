pub mod database;
pub mod error;
pub mod prover_manager;
pub mod shutdown;
pub mod tracing;

// Re-export types for backward compatibility
pub use database::*;
pub use error::{Error, Result};
pub use prover_manager::{
    CoordinatorConfig, FinalProofResult, Job, JobId, JobStartResult, JobStatus, Phase1Result,
    ProvePhase1Result, ProverConnection, ProverManager, ProverRegistrationResult, ProverStatus,
};
