//! [`AsmRunnerSupervisor`] — lifecycle owner for the MO + (optional) RH
//! ASM-emulator runner threads.
//!
//! Previously [`crate::EmulatorAsm::execute`] inlined ~70 lines of
//! spawn / join / MT-failure-cleanup logic. This module extracts that
//! lifecycle so it can be tested in isolation (without spinning up
//! shmem) and so the cleanup contract is named in one place.
//!
//! Design:
//!   * The supervisor stores the two `JoinHandle`s only — it does
//!     *not* own [`crate::AsmResources`]. The cancellation hook on the
//!     MT-failure path is a caller-supplied closure, decoupling the
//!     supervisor from the resource type and making it trivial to
//!     fake in tests.
//!   * Construction has two flavours: `AsmRunnerSupervisor::new` takes
//!     pre-spawned handles (the testing seam);
//!     [`AsmRunnerSupervisor::spawn_on`] is the production convenience
//!     that spawns both runners against an [`crate::AsmResources`].
//!   * On MT success the caller asks the supervisor for its handles
//!     via [`AsmRunnerSupervisor::into_handles`] and embeds them in
//!     `BackendArtifacts::Asm`.
//!   * On MT failure the caller calls
//!     [`AsmRunnerSupervisor::cleanup_after_mt_failure`] with a
//!     cancellation closure; the supervisor signals cancel,
//!     joins the handles, and logs any runner panic or runner-side
//!     error so observability isn't silently lost.
//!
//! See `.claude/executor_refactor_plan.md` step 2.3 for context.

#![cfg_attr(not(all(target_os = "linux", target_arch = "x86_64")), allow(dead_code))]

use std::thread::JoinHandle;

use asm_runner::{AsmRunnerMO, AsmRunnerRH};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zisk_common::ExecutorStatsHandle;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::error::{ExecutorError, MutexExt};

use crate::error::ExecutorResult;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::{AsmResources, MAX_NUM_STEPS};

/// Owns the MO + (optionally) RH runner threads spawned at the start
/// of an ASM execution. See module-level docs.
pub struct AsmRunnerSupervisor {
    handle_mo: JoinHandle<ExecutorResult<AsmRunnerMO>>,
    handle_rh: Option<JoinHandle<ExecutorResult<AsmRunnerRH>>>,
}

impl AsmRunnerSupervisor {
    /// Construct from already-spawned handles. The testing seam — tests
    /// pass canned `std::thread::spawn(|| Ok(canned_runner))` handles
    /// without needing real shmem.
    #[cfg(test)]
    pub fn new(
        handle_mo: JoinHandle<ExecutorResult<AsmRunnerMO>>,
        handle_rh: Option<JoinHandle<ExecutorResult<AsmRunnerRH>>>,
    ) -> Self {
        Self { handle_mo, handle_rh }
    }

    /// Production convenience: spawn MO (always) and RH (only when
    /// `has_rom_sm`) against the supplied resources.
    ///
    /// `has_rom_sm` mirrors today's `pctx.dctx_is_first_process()`
    /// check — only the first rank computes the ROM histogram.
    ///
    /// `stats` is cloned per thread, so the underlying handle is
    /// shared without being moved into either spawn.
    ///
    /// Linux x86_64 only — `AsmResources::readers()` (which sources the
    /// shmem readers for both spawns) is gated to that target. On other
    /// platforms construct the supervisor directly via `Self::new`.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub fn spawn_on(
        resources: &AsmResources,
        chunk_size: u64,
        has_rom_sm: bool,
        stats: &ExecutorStatsHandle,
    ) -> Self {
        let handle_mo = std::thread::spawn({
            let asm_shmem_mo = resources.readers().mo.clone();
            let asm_services = resources.asm_services().clone();
            let stats_mo = stats.clone();
            move || -> ExecutorResult<AsmRunnerMO> {
                let mut guard = asm_shmem_mo.lock_or_poison("mo_shmem")?;
                AsmRunnerMO::run(&mut guard, MAX_NUM_STEPS, chunk_size, asm_services, stats_mo)
                    .map_err(ExecutorError::asm_backend)
            }
        });

        let handle_rh = has_rom_sm.then(|| {
            let asm_shmem_rh = resources.readers().rh.clone();
            let asm_services = resources.asm_services().clone();
            let unlock_mapped_memory = resources.config().unlock_mapped_memory;
            let stats_rh = stats.clone();
            std::thread::spawn(move || -> ExecutorResult<AsmRunnerRH> {
                let mut guard = asm_shmem_rh.lock_or_poison("rh_shmem")?;

                AsmRunnerRH::run(
                    &mut guard,
                    MAX_NUM_STEPS,
                    asm_services,
                    unlock_mapped_memory,
                    stats_rh,
                )
                .map_err(ExecutorError::asm_backend)
            })
        });

        Self { handle_mo, handle_rh }
    }

    /// Hand the supervisor's handles back to the caller. Used on the
    /// MT-success path: caller wraps them in
    /// [`crate::BackendArtifacts::Asm`] for [`crate::ExecutionOutput`].
    pub fn into_handles(
        self,
    ) -> (JoinHandle<ExecutorResult<AsmRunnerMO>>, Option<JoinHandle<ExecutorResult<AsmRunnerRH>>>)
    {
        (self.handle_mo, self.handle_rh)
    }

    /// MT-failure cleanup: signal cancellation, then join both runner
    /// handles, logging any runner panic or runner-side error.
    ///
    /// The caller supplies the cancellation closure because the
    /// cancellation surface lives on `AsmResources`; the supervisor
    /// doesn't need to know about it. The closure's `Err` is logged
    /// (not propagated) — the original MT error is what the caller
    /// will return, and a cancellation that itself fails is a
    /// secondary observability signal, not a new error mode.
    pub fn cleanup_after_mt_failure(
        self,
        signal_cancellation: impl FnOnce() -> ExecutorResult<()>,
    ) {
        if let Err(reset_err) = signal_cancellation() {
            tracing::error!("AsmRunnerSupervisor: signal_cancellation failed: {reset_err}");
        }
        join_runner_during_cleanup("MO", self.handle_mo);
        if let Some(h) = self.handle_rh {
            join_runner_during_cleanup("RH", h);
        }
    }
}

/// Join an MO/RH runner thread on the MT-failure cleanup path, logging
/// any thread panic or runner error so observability isn't silently
/// lost. The caller has already issued `signal_cancellation`, so a
/// healthy runner will observe the reset flag and exit `Ok(_)`.
fn join_runner_during_cleanup<T>(label: &str, handle: JoinHandle<ExecutorResult<T>>) {
    match handle.join() {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => {
            tracing::warn!("{label} runner returned error during MT-failure cleanup: {err}");
        }
        Err(_) => {
            tracing::warn!("{label} runner thread panicked during MT-failure cleanup")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asm_runner::AsmRHData;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// Spawn a no-op MO runner that returns an empty `AsmRunnerMO`.
    fn spawn_canned_mo() -> JoinHandle<ExecutorResult<AsmRunnerMO>> {
        std::thread::spawn(|| Ok(AsmRunnerMO::new(Vec::new())))
    }

    /// Spawn a no-op RH runner that returns an empty `AsmRunnerRH`.
    fn spawn_canned_rh() -> JoinHandle<ExecutorResult<AsmRunnerRH>> {
        std::thread::spawn(|| Ok(AsmRunnerRH::new(AsmRHData::new(0, Vec::new()))))
    }

    #[test]
    fn happy_path_returns_handles_for_both_runners() {
        let sup = AsmRunnerSupervisor::new(spawn_canned_mo(), Some(spawn_canned_rh()));

        let (mo, rh) = sup.into_handles();
        let mo_result = mo.join().expect("MO thread joined").expect("MO runner Ok");
        assert!(mo_result.plans.is_empty(), "canned MO returns empty plans");

        let rh_handle = rh.expect("RH handle present");
        rh_handle.join().expect("RH thread joined").expect("RH runner Ok");
    }

    #[test]
    fn happy_path_without_rh_returns_none() {
        let sup = AsmRunnerSupervisor::new(spawn_canned_mo(), None);
        let (_mo, rh) = sup.into_handles();
        assert!(rh.is_none(), "supervisor preserves rh=None");
    }

    #[test]
    fn mt_failure_invokes_cancellation_and_joins() {
        let sup = AsmRunnerSupervisor::new(spawn_canned_mo(), Some(spawn_canned_rh()));
        let cancelled = Arc::new(AtomicBool::new(false));

        let cancelled_for_closure = cancelled.clone();
        sup.cleanup_after_mt_failure(move || {
            cancelled_for_closure.store(true, Ordering::SeqCst);
            Ok(())
        });

        assert!(cancelled.load(Ordering::SeqCst), "cancellation closure must run");
    }

    #[test]
    fn mt_failure_with_failing_cancellation_does_not_panic() {
        // The cancellation closure's Err is logged, not propagated.
        // We assert the supervisor swallows it and still joins handles.
        let sup = AsmRunnerSupervisor::new(spawn_canned_mo(), Some(spawn_canned_rh()));
        sup.cleanup_after_mt_failure(|| Err(ExecutorError::AsmBackend("cancel boom".to_string())));
    }

    #[test]
    fn mt_failure_with_panicking_runner_does_not_propagate_panic() {
        // The MO runner thread panics. cleanup_after_mt_failure must
        // join it (observing the JoinHandle's Err(panic)) and log a
        // warning, but must NOT itself panic.
        let panicking_mo = std::thread::spawn(|| -> ExecutorResult<AsmRunnerMO> {
            panic!("simulated runner panic")
        });
        let sup = AsmRunnerSupervisor::new(panicking_mo, None);
        sup.cleanup_after_mt_failure(|| Ok(()));
    }

    #[test]
    fn mt_failure_with_runner_returning_err_does_not_propagate() {
        // The MO runner thread returns `Err`; the supervisor logs the
        // error and continues. Mirrors the runner-side-error branch
        // of join_runner_during_cleanup.
        let erroring_mo: JoinHandle<ExecutorResult<AsmRunnerMO>> = std::thread::spawn(|| {
            Err(ExecutorError::AsmBackend("simulated runner error".to_string()))
        });
        let sup = AsmRunnerSupervisor::new(erroring_mo, None);
        sup.cleanup_after_mt_failure(|| Ok(()));
    }
}
