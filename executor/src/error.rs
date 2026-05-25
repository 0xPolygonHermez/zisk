//! Crate-level error type for the executor.

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use proofman_common::ProofmanError;
use thiserror::Error;

/// Crate-wide error type for the executor.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// The `global_id` referenced by the chunk's plan is missing from the supplied `instances` map.
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

    /// An expected built-in SM or precompile is missing from the bundle.
    #[error("bundle component missing: {kind}")]
    BundleComponentMissing {
        /// The built-in / precompile whose counter or input generator could not be found.
        kind: &'static str,
    },

    /// An expected built-in SM or precompile is duplicated in the bundle.
    #[error("bundle component duplicated: {kind}")]
    BundleComponentDuplicate {
        /// The built-in / precompile whose counter or input generator
        /// was found more than once while walking the bundle.
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

    /// The parsed ZisK ROM has not been installed yet via `ZiskExecutor::set_rom`.
    #[error("ROM not initialized")]
    RomNotInitialized,

    /// A plan keyed by `global_id` is missing during the assignment or populate phase of secondary instance handling.
    #[error("secn plan missing global_id during {phase}")]
    SecnPlanMissing {
        /// The phase that detected the mismatch ("assignment", "populate").
        phase: &'static str,
    },

    /// A `global_id` we expected to be in an in-memory index built earlier in the same pipeline is missing.
    #[error("invariant violation: global_id {global_id} not in {index}")]
    MissingIndexEntry {
        /// The global instance id that was missing.
        global_id: usize,
        /// Name of the index that should have contained it.
        index: &'static str,
    },

    /// Forwarded error from `proofman-common`.
    #[error(transparent)]
    Proofman(#[from] ProofmanError),

    /// Forwarded error from `sm-main`.
    #[error(transparent)]
    MainSm(#[from] sm_main::MainSmError),

    /// Forwarded error from `sm-rom` (ROM state-machine setup / planning).
    #[error(transparent)]
    Rom(#[from] sm_rom::RomError),

    /// Forwarded error from `ziskemu` (Rust emulator).
    #[error(transparent)]
    Emulator(#[from] ziskemu::ZiskEmulatorErr),

    /// The minimal-trace buffer in `ExecutionState` has not been populated yet.
    #[error("min_traces not set")]
    MinTracesNotSet,

    /// Internal invariant violation.
    #[error("internal invariant violation: {0}")]
    Internal(String),

    /// The ASM emulator was driven before its worker-supplied
    /// `AsmResources` handle was installed via `set_asm_resources`.
    #[error("AsmResources not initialized")]
    AsmResourcesNotInitialized,

    /// A hints-only operation was invoked on a program that wasn't set up with the hints pipeline.
    #[error("program was not set up with hints")]
    HintsNotConfigured,

    /// A `Mutex`/`RwLock` we hold internally was poisoned by a panic in another thread.
    #[error("mutex poisoned: {name}")]
    MutexPoisoned {
        /// Static identifier of the lock that was poisoned.
        name: &'static str,
    },

    /// An `Arc` we tried to unwrap still had additional owners.
    #[error("arc still has multiple owners after scope: {what}")]
    ArcStillReferenced {
        /// Description of the value that was still shared.
        what: &'static str,
    },

    /// Catch-all for failures bubbling up from the external ASM backend.
    #[error("ASM backend operation failed: {0}")]
    AsmBackend(String),

    /// An ASM runner thread's `JoinHandle` was already consumed by a previous `await_*` call.
    #[error("{name} runner handle already consumed")]
    RunnerHandleConsumed {
        /// Short label for the runner (e.g. "MO", "RH").
        name: &'static str,
    },

    /// An ASM runner thread panicked rather than returning normally.
    #[error("{name} runner thread panicked")]
    RunnerThreadPanicked {
        /// Short label for the runner (e.g. "MO", "RH").
        name: &'static str,
    },

    /// An ASM runner thread returned an error from its body.
    #[error("{name} runner failed: {message}")]
    RunnerFailed {
        /// Short label for the runner (e.g. "MO", "RH").
        name: &'static str,
        /// Stringified runner error.
        message: String,
    },

    /// Aggregated failure across one or more MT-assembly chunks.
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
    /// Build a [`Self::MutexPoisoned`] with the given lock name. Most
    /// sites should prefer [`MutexExt::lock_or_poison`] / [`RwLockExt::read_or_poison`] /
    /// [`RwLockExt::write_or_poison`]; this constructor remains for the
    /// `.into_inner()` and `.lock().map(…).map_err(…)` shapes those
    /// traits don't cover.
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

/// Extension trait for `Mutex<T>` that converts poison errors into [`ExecutorError::MutexPoisoned`].
pub trait MutexExt<T> {
    /// Acquire the lock or return a typed poison error.
    fn lock_or_poison(&self, name: &'static str) -> ExecutorResult<MutexGuard<'_, T>>;
}

impl<T> MutexExt<T> for Mutex<T> {
    #[inline]
    fn lock_or_poison(&self, name: &'static str) -> ExecutorResult<MutexGuard<'_, T>> {
        self.lock().map_err(|_| ExecutorError::mutex_poisoned(name))
    }
}

/// Extension trait for `RwLock<T>` that converts poison errors into [`ExecutorError::MutexPoisoned`].
pub trait RwLockExt<T> {
    /// Acquire the read lock or return a typed poison error.
    fn read_or_poison(&self, name: &'static str) -> ExecutorResult<RwLockReadGuard<'_, T>>;
    /// Acquire the write lock or return a typed poison error.
    fn write_or_poison(&self, name: &'static str) -> ExecutorResult<RwLockWriteGuard<'_, T>>;
}

impl<T> RwLockExt<T> for RwLock<T> {
    #[inline]
    fn read_or_poison(&self, name: &'static str) -> ExecutorResult<RwLockReadGuard<'_, T>> {
        self.read().map_err(|_| ExecutorError::mutex_poisoned(name))
    }

    #[inline]
    fn write_or_poison(&self, name: &'static str) -> ExecutorResult<RwLockWriteGuard<'_, T>> {
        self.write().map_err(|_| ExecutorError::mutex_poisoned(name))
    }
}
