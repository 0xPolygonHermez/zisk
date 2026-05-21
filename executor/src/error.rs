//! Crate-level error type for the executor.
//!
//! [`ExecutorError`] is the crate's unified error enum. Each fallible
//! subsystem contributes its own variants; the in-crate code uses
//! [`ExecutorResult`] end-to-end, with explicit conversion to
//! `anyhow::Result` or `ProofmanResult` only at the two external seams
//! (asm-runner callbacks via [`Self::asm_backend`], and the
//! [`proofman::WitnessComponent`] trait via stringification into
//! `ProofmanError::InvalidSetup` in `executor.rs`).

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

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

    /// The parsed ZisK ROM has not been installed yet via
    /// `ZiskExecutor::set_rom`.
    #[error("ROM not initialized")]
    RomNotInitialized,

    /// A plan keyed by `global_id` is missing during the assignment or
    /// populate phase of secondary instance handling — invariant
    /// violation.
    #[error("secn plan missing global_id during {phase}")]
    SecnPlanMissing {
        /// The phase that detected the mismatch ("assignment", "populate").
        phase: &'static str,
    },

    /// A `global_id` we expected to be in an in-memory index built
    /// earlier in the same pipeline is missing — invariant violation.
    #[error("invariant violation: global_id {global_id} not in {index}")]
    MissingIndexEntry {
        /// The global instance id that was missing.
        global_id: usize,
        /// Name of the index that should have contained it.
        index: &'static str,
    },

    /// Forwarded error from `proofman-common` (pctx / dctx / setup
    /// operations). Implements `From<ProofmanError>` so `?` auto-converts.
    #[error(transparent)]
    Proofman(#[from] ProofmanError),

    /// Forwarded error from `sm-main` (main planner / witness).
    #[error(transparent)]
    MainSm(#[from] sm_main::MainSmError),

    /// Forwarded error from `sm-rom` (ROM state-machine setup / planning).
    #[error(transparent)]
    Rom(#[from] sm_rom::RomError),

    /// Forwarded error from `ziskemu` (Rust emulator).
    #[error(transparent)]
    Emulator(#[from] ziskemu::ZiskEmulatorErr),

    /// The minimal-trace buffer in `ExecutionState` has not been
    /// populated yet — `ExecutionPhase::run` must precede any phase
    /// that reads it.
    #[error("min_traces not set")]
    MinTracesNotSet,

    /// Internal invariant violation — typically a missing key in an
    /// in-memory index built earlier in the same pipeline. If this
    /// fires, the pipeline has a bug.
    #[error("internal invariant violation: {0}")]
    Internal(String),

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

    /// An ASM runner thread's `JoinHandle` was already consumed by a
    /// previous `await_*` call.
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

    /// An ASM runner thread returned an error from its body. `message`
    /// is the stringified inner error chain.
    #[error("{name} runner failed: {message}")]
    RunnerFailed {
        /// Short label for the runner (e.g. "MO", "RH").
        name: &'static str,
        /// Stringified runner error.
        message: String,
    },

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

/// Extension trait for `Mutex<T>` that converts poison errors into
/// [`ExecutorError::MutexPoisoned`]. Replaces the boilerplate
/// `.lock_or_poison("name")` with
/// `.lock_or_poison("name")`.
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

/// Extension trait for `RwLock<T>` that converts poison errors into
/// [`ExecutorError::MutexPoisoned`]. Replaces the boilerplate
/// `.read().map_err(|_| …)` / `.write().map_err(|_| …)` with
/// `.read_or_poison("name")` / `.write_or_poison("name")`.
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
