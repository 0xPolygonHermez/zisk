//! Uniform output from any [`crate::Emulator<F>`] backend.
//!
//! Both `EmulatorAsm` and `EmulatorRust` return a [`TraceOutput`].
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
//! let mem_plans = trace.backend.await_mem_plans()?;
//! let rh_data   = trace.backend.await_rom_histogram()?;
//! ```
//! On the Rust path the calls yield `vec![]` / `None` instantly; on
//! the ASM path they join the corresponding runner thread.

use std::thread::JoinHandle;

use anyhow::Result;
use asm_runner::{AsmRunnerMO, AsmRunnerRH};
use zisk_common::{EmuTrace, Plan};

use crate::pub_outs_collector::PubOutsCollector;
use crate::CountersChunkMetrics;

/// Uniform return type for all `Emulator<F>` impls.
///
/// Sync fields are identical across backends; backend-specific
/// async work is encapsulated in [`Self::backend`].
pub struct TraceOutput {
    /// Minimal traces produced by the emulator.
    pub min_traces: Vec<EmuTrace>,
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
///   `Option` so [`Self::await_mem_plans`] / [`Self::await_rom_histogram`]
///   can take ownership once: `Some` = not yet joined, `None` = consumed.
/// - [`BackendArtifacts::Rust`] is a unit variant — the Rust emulator
///   has no async work, so the `await_*` methods return empty results
///   immediately.
pub enum BackendArtifacts {
    /// ASM backend: parallel MO + optional RH runners.
    Asm {
        /// Memory-operations runner handle. `Some` until consumed by
        /// `await_mem_plans`, then `None`.
        mo: Option<JoinHandle<Result<AsmRunnerMO>>>,
        /// ROM-histogram runner handle. `Some` only on the first rank
        /// (the rank that actually runs the RH service); `None`
        /// otherwise. Set to `None` after `await_rom_histogram` consumes
        /// it.
        rh: Option<JoinHandle<Result<AsmRunnerRH>>>,
    },
    /// Rust backend: no async artifacts. `await_*` returns empty.
    Rust,
}

impl BackendArtifacts {
    /// Joins the memory-operations runner (ASM) and returns its plans.
    /// Returns `Ok(vec![])` for the Rust backend.
    ///
    /// Each call consumes the `mo` handle inside the `Asm` variant; a
    /// second call returns an error noting the handle was already taken.
    pub fn await_mem_plans(&mut self) -> Result<Vec<Plan>> {
        match self {
            Self::Asm { mo, .. } => {
                let handle = mo.take().ok_or_else(|| {
                    anyhow::anyhow!("Assembly Memory Operations handle already consumed")
                })?;
                let asm_runner_mo = handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("Assembly Memory Operations thread panicked"))?
                    .map_err(|e| {
                        anyhow::anyhow!("Assembly Memory Operations execution failed: {e}")
                    })?;
                Ok(asm_runner_mo.plans)
            }
            Self::Rust => Ok(Vec::new()),
        }
    }

    /// Joins the ROM-histogram runner (ASM, first rank only) and returns
    /// its output. Returns `Ok(None)` for the Rust backend, or for ASM
    /// ranks that don't run RH.
    ///
    /// Each call consumes the `rh` handle inside the `Asm` variant; a
    /// second call returns `Ok(None)`.
    pub fn await_rom_histogram(&mut self) -> Result<Option<AsmRunnerRH>> {
        match self {
            Self::Asm { rh, .. } => {
                let Some(handle) = rh.take() else {
                    return Ok(None);
                };
                let rh_data = handle
                    .join()
                    .map_err(|_| anyhow::anyhow!("ROM Histogram thread panicked"))?
                    .map_err(|e| anyhow::anyhow!("ROM Histogram execution failed: {e}"))?;
                Ok(Some(rh_data))
            }
            Self::Rust => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asm_runner::AsmRHData;

    #[test]
    fn rust_await_mem_plans_yields_empty() {
        let mut backend = BackendArtifacts::Rust;
        let plans = backend.await_mem_plans().expect("await_mem_plans on Rust");
        assert!(plans.is_empty());
    }

    #[test]
    fn rust_await_rom_histogram_yields_none() {
        let mut backend = BackendArtifacts::Rust;
        let rh = backend.await_rom_histogram().expect("await_rom_histogram on Rust");
        assert!(rh.is_none());
    }

    #[test]
    fn asm_await_mem_plans_returns_canned_plans_after_thread_join() {
        // The Vec<Plan> is opaque to this test; we just need its length
        // to match what we passed in.
        let canned = Vec::<Plan>::new();
        let expected_len = canned.len();
        let mo_handle = std::thread::spawn(move || Ok(AsmRunnerMO::new(canned)));
        let mut backend = BackendArtifacts::Asm { mo: Some(mo_handle), rh: None };

        let plans = backend.await_mem_plans().expect("await_mem_plans on Asm");
        assert_eq!(plans.len(), expected_len);
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
        let mo_handle =
            std::thread::spawn(|| -> Result<AsmRunnerMO> { Err(anyhow::anyhow!("boom")) });
        let mut backend = BackendArtifacts::Asm { mo: Some(mo_handle), rh: None };
        let err = backend.await_mem_plans().expect_err("runner Err must propagate");
        assert!(err.to_string().contains("Assembly Memory Operations execution failed"));
        assert!(err.to_string().contains("boom"));
    }

    #[test]
    fn asm_await_rom_histogram_returns_some_after_join() {
        let rh_handle = std::thread::spawn(|| {
            Ok(AsmRunnerRH::new(AsmRHData::new(0, Vec::new(), Vec::new())))
        });
        let mut backend = BackendArtifacts::Asm { mo: None, rh: Some(rh_handle) };
        // `mo` is None to assert that await_rom_histogram doesn't touch the mo slot.
        let rh = backend.await_rom_histogram().expect("await_rom_histogram on Asm");
        assert!(rh.is_some());
    }

    #[test]
    fn asm_await_rom_histogram_none_when_not_present() {
        // ASM variant with rh = None mirrors a non-first-rank execution
        // (the RH service only runs on rank 0).
        let mut backend = BackendArtifacts::Asm { mo: None, rh: None };
        let rh = backend.await_rom_histogram().expect("await_rom_histogram with rh=None");
        assert!(rh.is_none());
    }

    #[test]
    fn asm_await_rom_histogram_double_take_yields_none() {
        let rh_handle = std::thread::spawn(|| {
            Ok(AsmRunnerRH::new(AsmRHData::new(0, Vec::new(), Vec::new())))
        });
        let mut backend = BackendArtifacts::Asm { mo: None, rh: Some(rh_handle) };
        backend.await_rom_histogram().expect("first call OK");
        let second = backend.await_rom_histogram().expect("second call OK (None)");
        assert!(second.is_none());
    }
}
