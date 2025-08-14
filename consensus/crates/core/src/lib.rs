// Re-export common types for backwards compatibility
pub use consensus_common::{BlockId, ComputeCapacity, Error, JobId, ProverId, Result};

mod prover_manager;
pub mod shutdown;
pub mod tracing;

pub use prover_manager::*;
