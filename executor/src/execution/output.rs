//! Uniform output from any [`crate::Emulator<F>`] backend.
//!
//! Both `EmulatorAsm` and `EmulatorRust` return a [`ExecutionOutput`].
//! Sync data (`min_traces`, `counters`, `pub_outs`, `steps`) has the
//! same shape on both paths. Backend-specific artifacts — the
//! asynchronous MO and RH join handles produced only by the ASM
//! path — live in the [`BackendArtifacts`] enum so downstream phases
//! can call `await_*` methods uniformly without knowing which backend
//! ran.
//!
//! See `.claude/executor_refactor_plan.md` step 1.2 for context.
//!
//! The plan-merging callers (step 1.3 onward) drive this via:
//! ```ignore
//! let (mem_plans, gpu_mops_used_bytes) = trace.backend.await_mem_plans()?;
//! let rh_handle = trace.backend.take_rh_handle();
//! ```
//! The MO plans are needed here, so `await_mem_plans` joins that runner;
//! the ROM histogram is not, so its handle is handed over unjoined for
//! whoever reads the histogram to join at the point of use. On the Rust
//! path both yield empty instantly.

use std::{sync::Arc, thread::JoinHandle};

use zisk_asm_runner::{AsmRunnerMO, AsmRunnerRH};
use zisk_common::{EmuTrace, LateJoinHandle, Plan};

use crate::error::{ExecutorError, ExecutorResult};
use crate::pub_outs_collector::PubOutsCollector;
use crate::CountersChunkMetrics;

/// Uniform return type for all `Emulator<F>` impls.
pub struct ExecutionOutput {
    /// Minimal traces produced by the emulator (shared with the progressive
    /// main-witness advancement store, hence `Arc` elements).
    pub min_traces: Vec<Arc<EmuTrace>>,
    /// Device metrics for secondary devices (counter-phase output).
    pub counters: CountersChunkMetrics,
    /// Public outputs accumulated during execution.
    pub pub_outs: PubOutsCollector,
    /// Total number of steps executed by the emulator.
    pub steps: u64,
    /// Backend-specific async artifacts (ASM-only join handles, or
    /// the unit `Rust` variant).
    pub backend: BackendArtifacts,
}

/// Backend-specific artifacts produced by the emulator.
///
/// - [`BackendArtifacts::Asm`] carries the MO + RH join handles spawned
///   in parallel with the MT chunk processor. The handles are wrapped in
///   `Option` so [`Self::await_mem_plans`] / [`Self::take_rh_handle`]
///   can take ownership once: `Some` = not yet taken, `None` = consumed.
/// - [`BackendArtifacts::Rust`] is a unit variant — the Rust emulator
///   has no async work, so the `await_*` methods return empty results
///   immediately.
pub enum BackendArtifacts {
    /// ASM backend: parallel MO + optional RH runners.
    Asm {
        /// Memory-operations runner handle. `Some` until consumed by
        /// `await_mem_plans`, then `None`.
        mo: Option<JoinHandle<ExecutorResult<AsmRunnerMO>>>,
        /// ROM-histogram runner handle. `Some` only on the first rank
        /// (the rank that actually runs the RH service); `None`
        /// otherwise. Set to `None` after `take_rh_handle` hands it over.
        rh: Option<LateJoinHandle<AsmRunnerRH>>,
    },
    /// Rust backend: no async artifacts. `await_*` returns empty.
    Rust,
}

impl BackendArtifacts {
    /// Joins the memory-operations runner (ASM) and returns its plans plus the
    /// GPU mem-ops planner's borrowed-buffer usage in bytes (`None` on the CPU
    /// planner path). Returns `Ok((vec![], None))` for the Rust backend.
    ///
    /// Each call consumes the `mo` handle inside the `Asm` variant; a
    /// second call returns an error noting the handle was already taken.
    pub fn await_mem_plans(&mut self) -> ExecutorResult<(Vec<Plan>, Option<u64>)> {
        match self {
            Self::Asm { mo, .. } => {
                let handle = mo.take().ok_or(ExecutorError::RunnerHandleConsumed { name: "MO" })?;
                let asm_runner_mo = handle
                    .join()
                    .map_err(|_| ExecutorError::RunnerThreadPanicked { name: "MO" })?
                    .map_err(|e| ExecutorError::RunnerFailed {
                        name: "MO",
                        message: e.to_string(),
                    })?;
                Ok((asm_runner_mo.plans, asm_runner_mo.gpu_mops_used_bytes))
            }
            Self::Rust => Ok((Vec::new(), None)),
        }
    }

    /// Hands over the ROM-histogram runner handle (ASM, first rank only)
    /// **without joining it**. Returns `None` for the Rust backend, or for
    /// ASM ranks that don't run RH.
    ///
    /// Unjoined on purpose: nothing in this phase reads the histogram, so
    /// the runner is left to finish while the phases that follow run, and
    /// whoever does read it joins at that point. Ownership transfers here —
    /// the caller becomes responsible for the thread, which is why a second
    /// call returns `None`.
    pub fn take_rh_handle(&mut self) -> Option<LateJoinHandle<AsmRunnerRH>> {
        match self {
            Self::Asm { rh, .. } => rh.take(),
            Self::Rust => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_asm_runner::AsmRHData;

    #[test]
    fn rust_await_mem_plans_yields_empty() {
        let mut backend = BackendArtifacts::Rust;
        let (plans, gpu_mops_used_bytes) =
            backend.await_mem_plans().expect("await_mem_plans on Rust");
        assert!(plans.is_empty());
        assert!(gpu_mops_used_bytes.is_none());
    }

    #[test]
    fn rust_take_rh_handle_yields_none() {
        let mut backend = BackendArtifacts::Rust;
        assert!(backend.take_rh_handle().is_none());
    }

    #[test]
    fn asm_await_mem_plans_returns_canned_plans_after_thread_join() {
        // Plan contents are opaque here; we only verify the tuple unwraps and
        // preserves the plan count for the canned runner output.
        let canned = Vec::<Plan>::new();
        let expected_len = canned.len();
        let mo_handle = std::thread::spawn(move || Ok(AsmRunnerMO::new(canned)));
        let mut backend = BackendArtifacts::Asm { mo: Some(mo_handle), rh: None };

        let (plans, gpu_mops_used_bytes) =
            backend.await_mem_plans().expect("await_mem_plans on Asm");
        assert_eq!(plans.len(), expected_len);
        assert!(gpu_mops_used_bytes.is_none());
    }

    #[test]
    fn asm_await_mem_plans_errs_on_double_take() {
        let mo_handle = std::thread::spawn(move || Ok(AsmRunnerMO::new(Vec::new())));
        let mut backend = BackendArtifacts::Asm { mo: Some(mo_handle), rh: None };
        backend.await_mem_plans().expect("first call OK");
        let err = backend.await_mem_plans().expect_err("second call must err");
        assert!(err.to_string().contains("already consumed"));
    }

    #[test]
    fn asm_await_mem_plans_propagates_runner_error() {
        let mo_handle = std::thread::spawn(|| -> ExecutorResult<AsmRunnerMO> {
            Err(ExecutorError::AsmBackend("boom".to_string()))
        });
        let mut backend = BackendArtifacts::Asm { mo: Some(mo_handle), rh: None };
        let err = backend.await_mem_plans().expect_err("runner Err must propagate");
        assert!(err.to_string().contains("MO runner failed"));
        assert!(err.to_string().contains("boom"));
    }

    #[test]
    fn asm_take_rh_handle_hands_over_without_joining() {
        let rh_handle = std::thread::spawn(|| Ok(AsmRunnerRH::new(AsmRHData::new(0, Vec::new()))));
        let mut backend = BackendArtifacts::Asm { mo: None, rh: Some(rh_handle) };
        // `mo` is None to assert that take_rh_handle doesn't touch the mo slot.
        let handle = backend.take_rh_handle().expect("handle handed over");
        handle.join().expect("thread joined").expect("canned runner Ok");
    }

    #[test]
    fn asm_take_rh_handle_none_when_not_present() {
        // ASM variant with rh = None mirrors a non-first-rank execution
        // (the RH service only runs on rank 0).
        let mut backend = BackendArtifacts::Asm { mo: None, rh: None };
        assert!(backend.take_rh_handle().is_none());
    }

    #[test]
    fn asm_take_rh_handle_twice_yields_none() {
        let rh_handle = std::thread::spawn(|| Ok(AsmRunnerRH::new(AsmRHData::new(0, Vec::new()))));
        let mut backend = BackendArtifacts::Asm { mo: None, rh: Some(rh_handle) };
        assert!(backend.take_rh_handle().is_some(), "first call hands it over");
        assert!(backend.take_rh_handle().is_none(), "second call has nothing left");
    }
}
