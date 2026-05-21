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

    /// The ASM emulator was driven before its worker-supplied
    /// `AsmResources` handle was installed via `set_asm_resources`.
    #[error("AsmResources not initialized")]
    AsmResourcesNotInitialized,

    /// A hints-only operation was invoked on a program that wasn't
    /// set up with the hints pipeline (no `HintsShmem` mapped).
    #[error("program was not set up with hints")]
    HintsNotConfigured,

    /// A `Mutex`/`RwLock` we hold internally was poisoned by a panic
    /// in another thread. `name` identifies the lock for diagnosis.
    #[error("mutex poisoned: {name}")]
    MutexPoisoned {
        /// Static identifier of the lock that was poisoned.
        name: &'static str,
    },

    /// An `Arc` we tried to unwrap still had additional owners after
    /// the scope that produced it ended — invariant violation.
    #[error("arc still has multiple owners after scope: {what}")]
    ArcStillReferenced {
        /// Description of the value that was still shared.
        what: &'static str,
    },

    /// Catch-all for failures bubbling up from the external ASM
    /// backend (asm-runner / shmem / precompiles-hints).
    #[error("ASM backend operation failed: {0}")]
    AsmBackend(String),

    /// Aggregated failure across one or more MT-assembly chunks.
    /// `count` is the number of failures; `message` is the
    /// newline-joined `Display` chain of each one.
    #[error("MT assembly chunk processing failed ({count} errors):\n{message}")]
    MtChunkProcessing {
        /// Number of chunk failures aggregated into `message`.
        count: usize,
        /// Joined `Display` chains of the underlying errors.
        message: String,
    },
}

/// Convenience [`Result`] alias for fallible operations in this crate.
pub type ExecutorResult<T> = Result<T, ExecutorError>;

impl ExecutorError {
    /// Build a [`Self::MutexPoisoned`] with the given lock name. Intended
    /// for use as `.map_err(|_| ExecutorError::mutex_poisoned("foo"))?`.
    #[inline]
    pub fn mutex_poisoned(name: &'static str) -> Self {
        Self::MutexPoisoned { name }
    }

    /// Wrap an upstream `anyhow::Error` (from asm-runner, precompiles-hints,
    /// shmem, etc.) into [`Self::AsmBackend`] with the formatted `Display`
    /// chain. Intended for use as `.map_err(ExecutorError::asm_backend)?`.
    #[inline]
    pub fn asm_backend(e: anyhow::Error) -> Self {
        Self::AsmBackend(format!("{e:#}"))
    }
}
