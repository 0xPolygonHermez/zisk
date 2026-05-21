//! Crate-level error type for the executor.
//!
//! [`ExecutorError`] is the crate's unified error enum. Each fallible
//! subsystem contributes its own variants as it migrates off `anyhow`.
//! The current variants cover collector-phase data-bus construction in
//! [`crate::StaticDataBusCollect::for_chunk`]; more will be added here
//! as other phases follow.

use proofman_common::ProofmanError;
use thiserror::Error;

/// Crate-wide error type for the executor. Variants are organised by
/// failure site; today they cover collector-phase bus construction and
/// will grow as other phases adopt typed errors.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// The `global_id` referenced by the chunk's plan is missing from
    /// the supplied `instances` map.
    #[error("instance not found for global_id={global_id}")]
    InstanceNotFound {
        /// The missing global instance id.
        global_id: usize,
    },

    /// `ProofCtx::dctx_get_instance_info` failed for this `global_id`.
    #[error("failed to get instance info for global_id={global_id}")]
    InstanceInfo {
        /// The global instance id that failed lookup.
        global_id: usize,
        /// Underlying proofman error.
        #[source]
        source: ProofmanError,
    },

    /// No built-in or registered precompile state machine matches
    /// `air_id` — bundle-construction invariant violation.
    #[error("state machine not found: airgroup_id={airgroup_id}, air_id={air_id}")]
    StateMachineNotFound {
        /// The expected airgroup id.
        airgroup_id: usize,
        /// The air id with no matching SM.
        air_id: usize,
    },

    /// A required input generator is missing from the bundle when
    /// building the per-chunk `BuiltinCollectors` /
    /// `PrecompileCollectors`. Bundle-construction invariant violation.
    #[error("input generator not found: {kind}")]
    InputGeneratorNotFound {
        /// The state-machine / precompile whose input generator is missing.
        kind: &'static str,
    },

    /// `try_push_collector` matched an air-id but the corresponding
    /// `Instance` downcast did not yield the expected concrete type —
    /// bundle-construction invariant violation.
    #[error(
        "instance type mismatch for global_id={global_id}, air_id={air_id}: expected {expected}"
    )]
    InstanceTypeMismatch {
        /// The global instance id whose downcast failed.
        global_id: usize,
        /// The air id whose dispatch was attempted.
        air_id: usize,
        /// The concrete `*Instance<F>` type that was expected.
        expected: &'static str,
    },
}

/// Convenience [`Result`] alias for fallible operations in this crate.
pub type ExecutorResult<T> = Result<T, ExecutorError>;
