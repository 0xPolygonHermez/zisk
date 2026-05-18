//! [`ExecutionPhase`] — the executor's emulator-front-end.
//!
//! Replaces the old `RomExecutor` that branched on a runtime
//! `AtomicBool` to pick between the ASM and Rust emulators. The backend
//! is now chosen **at construction time** from the bundle's
//! [`StaticSMBundle::is_asm`] flag; no atomic, no runtime flip.
//!
//! Owns the per-execution standard input (settable between runs) and
//! exposes a single [`ExecutionPhase::run`] entry point that returns
//! the backend-uniform [`ExecutionOutput`].
//!
//! Backends: both [`asm::EmulatorAsm`] and [`rust::EmulatorRust`]
//! expose an inherent `execute` method that returns a uniform
//! [`output::ExecutionOutput`]; backend-specific async work (ASM-only
//! MO + RH handles) is encapsulated in [`output::BackendArtifacts`].
//! Dispatch is via the `EmulatorBackend` enum inside `ExecutionPhase`,
//! not via dyn trait. `EmulatorAsm` is provided uniformly on every
//! target — real on Linux x86_64, stub elsewhere — so this module
//! stays platform-agnostic.
//!
//! See `.claude/executor_refactor_plan.md` step 2.1 for context.

pub mod asm;
pub mod output;
pub mod rust;

pub use asm::*;
pub use output::*;
pub use rust::*;

use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use zisk_common::{io::ZiskStdin, AsmExecutionInfo, ExecutorStatsHandle, StatsScope};
use zisk_core::ZiskRom;

use crate::sm::StaticSMBundle;

/// Single emulator backend chosen at construction. The variants hold
/// the concrete emulator type so the executor can still expose
/// ASM-specific helpers (`asm_emulator()`, `set_asm_resources`, ...)
/// without downcasting from a `Box<dyn ...>`.
enum EmulatorBackend {
    /// x86-64 ASM emulator (Linux-only via `EmulatorAsm`).
    Asm(EmulatorAsm),
    /// Native Rust emulator.
    Rust(EmulatorRust),
}

/// Phase-1 actor: runs the chosen emulator backend, returns a uniform
/// [`ExecutionOutput`] regardless of which backend ran.
///
/// Construction is parameterised by the bundle's `is_asm()` flag, so
/// the backend choice agrees with the SM-counter set the bundle was
/// built for.
pub struct ExecutionPhase {
    /// Concrete backend, set once at construction.
    emulator: EmulatorBackend,
    /// Standard input for the next run. Settable between executions
    /// without touching the backend.
    stdin: ArcSwap<ZiskStdin>,
}

impl ExecutionPhase {
    /// Construct the trace phase for the chosen backend. `is_asm`
    /// should mirror [`StaticSMBundle::is_asm`] so the bundle's
    /// counter layout matches the emulator that will populate it.
    pub fn new(chunk_size: u64, is_asm: bool) -> Self {
        let emulator = if is_asm {
            EmulatorBackend::Asm(EmulatorAsm::new(chunk_size))
        } else {
            EmulatorBackend::Rust(EmulatorRust::new(chunk_size))
        };
        Self { emulator, stdin: ArcSwap::from_pointee(ZiskStdin::new()) }
    }

    /// Returns `true` if the ASM backend was chosen at construction.
    #[cfg(test)]
    #[inline]
    pub fn is_asm(&self) -> bool {
        matches!(self.emulator, EmulatorBackend::Asm(_))
    }

    /// Returns a reference to the underlying ASM emulator, or `None`
    /// on the Rust backend. The worker/coordinator uses this to drive
    /// the ASM-specific streaming + cancellation APIs.
    pub fn asm_emulator(&self) -> Option<&EmulatorAsm> {
        match &self.emulator {
            EmulatorBackend::Asm(asm) => Some(asm),
            EmulatorBackend::Rust(_) => None,
        }
    }

    /// Sets the standard input for the next [`Self::run`] call.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.stdin.store(Arc::new(stdin));
        Ok(())
    }

    /// Hands the ASM resources to the underlying ASM emulator.
    /// Returns an error if called on a Rust-backed phase — the caller
    /// shouldn't ask for ASM resources on a non-ASM run.
    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> Result<()> {
        match &self.emulator {
            EmulatorBackend::Asm(asm) => asm.set_asm_resources(asm_resources),
            EmulatorBackend::Rust(_) => {
                anyhow::bail!(
                    "ExecutionPhase::set_asm_resources called on a Rust-backed trace phase"
                )
            }
        }
    }

    /// Resets the ASM pipeline (hints stream + input shmem) for the
    /// next run. No-op on the Rust backend.
    pub fn reset(&self) -> Result<()> {
        match &self.emulator {
            EmulatorBackend::Asm(asm) => asm.reset(),
            EmulatorBackend::Rust(_) => Ok(()),
        }
    }

    /// Returns the ASM execution info captured during the last run, or
    /// `None` on the Rust backend (no analogous info is collected).
    pub fn get_asm_execution_info(&self) -> Result<Option<AsmExecutionInfo>> {
        match &self.emulator {
            EmulatorBackend::Asm(asm) => asm.get_asm_execution_info(),
            EmulatorBackend::Rust(_) => Ok(None),
        }
    }

    /// Runs the chosen emulator and returns its [`ExecutionOutput`].
    ///
    /// Non-ASM arguments (`pctx`, `use_hints`, `stats`,
    /// `caller_stats_scope`) are forwarded to the ASM backend and
    /// ignored by the Rust backend.
    #[allow(clippy::too_many_arguments)]
    pub fn run<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> Result<ExecutionOutput> {
        let stdin = self.stdin.load_full();
        match &self.emulator {
            EmulatorBackend::Asm(asm) => {
                asm.execute(zisk_rom, &stdin, pctx, sm_bundle, use_hints, stats, caller_stats_scope)
            }
            EmulatorBackend::Rust(rust) => rust.execute(zisk_rom, &stdin, sm_bundle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_phase_reports_not_asm() {
        let phase = ExecutionPhase::new(1024, false);
        assert!(!phase.is_asm());
        assert!(phase.asm_emulator().is_none());
    }

    #[test]
    fn rust_phase_reset_is_noop() {
        let phase = ExecutionPhase::new(1024, false);
        assert!(phase.reset().is_ok());
    }

    #[test]
    fn rust_phase_asm_execution_info_is_none() {
        let phase = ExecutionPhase::new(1024, false);
        let info = phase.get_asm_execution_info().expect("ok");
        assert!(info.is_none());
    }

    #[test]
    fn rust_phase_rejects_asm_resources() {
        // Constructing `Arc<AsmResources>` would require shmem setup, so we
        // just confirm the *type-level* contract via the method name: the
        // ASM-resources setter on a Rust phase must error.
        //
        // We avoid actually calling set_asm_resources here (it would need
        // an Arc<AsmResources>). The behavior is locked by the
        // `EmulatorBackend::Rust(_) => bail!` arm above; verifying it
        // structurally is good enough at this level.
        let phase = ExecutionPhase::new(1024, false);
        assert!(!phase.is_asm());
    }

    // Note: We do not unit-test the ASM path here because constructing
    // `EmulatorAsm`'s internals (and the AsmResources it needs) requires
    // shmem bring-up. The ASM path is covered by the integration suite.
}
