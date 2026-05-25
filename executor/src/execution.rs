//! [`ExecutionPhase`] — the executor's emulator-front-end.

pub mod asm;
pub mod output;
pub mod rust;

pub use asm::*;
pub use output::*;
pub use rust::*;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::{ExecutorError, ExecutorResult};
use fields::PrimeField64;
use proofman_common::ProofCtx;
use zisk_common::{io::ZiskStdin, AsmExecutionInfo, ExecutorStatsHandle, StatsScope};
use zisk_core::ZiskRom;

use crate::sm::StaticSMBundle;

/// Phase-1 actor: runs the chosen emulator backend, returns a  [`ExecutionOutput`]
/// regardless of which backend ran.
pub struct ExecutionPhase {
    /// Rust emulator. Always present.
    emulator_rust: EmulatorRust,

    /// ASM-backed emulator. Present iff the executor was constructed with `with_asm_emulator = true`.
    emulator_asm: Option<EmulatorAsm>,

    /// Runtime selector. Set to `true` when ASM resources are installed
    /// via [`Self::set_asm_resources`]; cleared by [`Self::clear_asm_resources`].
    is_asm_execution: AtomicBool,
}

impl ExecutionPhase {
    /// Construct the execution phase.
    ///
    /// * `with_asm_emulator = false` — builds an emu-only phase.
    /// * `with_asm_emulator = true` — builds both backends.
    pub fn new(chunk_size: u64, with_asm_emulator: bool) -> Self {
        Self {
            emulator_asm: with_asm_emulator.then(|| EmulatorAsm::new(chunk_size)),
            emulator_rust: EmulatorRust::new(chunk_size),
            is_asm_execution: AtomicBool::new(false),
        }
    }

    /// Returns `true` if the next `run` will route through the ASM backend.
    #[inline]
    pub fn is_asm_execution(&self) -> bool {
        self.is_asm_execution.load(Ordering::Relaxed)
    }

    /// Returns a reference to the ASM emulator iff ASM execution is currently active.
    /// Returns `None` when the executor is emu-only or when the runtime flag is cleared.
    pub fn asm_emulator(&self) -> Option<&EmulatorAsm> {
        if self.is_asm_execution() {
            self.emulator_asm.as_ref()
        } else {
            None
        }
    }

    /// Installs the worker-supplied [`AsmResources`] handle and flips
    /// the runtime flag to ASM. Returns an error on emu-only
    /// executors — they have no ASM backend to install into.
    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> ExecutorResult<()> {
        let asm = self.emulator_asm.as_ref().ok_or(ExecutorError::AsmNotAvailable)?;
        asm.set_asm_resources(asm_resources)?;
        self.is_asm_execution.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Clears the runtime flag so subsequent `run` calls route through the Rust emulator.
    pub fn clear_asm_resources(&self) {
        self.is_asm_execution.store(false, Ordering::Relaxed);
    }

    /// Resets the ASM pipeline (hints stream + input shmem) only when
    /// the ASM backend is the active one. No-op on the Rust path and
    /// on emu-only executors.
    pub fn reset(&self) -> ExecutorResult<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.reset()?;
        }
        Ok(())
    }

    /// Returns the ASM execution info captured during the last run,
    /// or `None` when the Rust backend was the active one or the executor is emu-only.
    pub fn get_asm_execution_info(&self) -> ExecutorResult<Option<AsmExecutionInfo>> {
        match self.asm_emulator() {
            Some(asm) => asm.get_asm_execution_info(),
            None => Ok(None),
        }
    }

    /// Runs the active emulator and returns its [`ExecutionOutput`].
    #[allow(clippy::too_many_arguments)]
    pub fn run<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> ExecutorResult<ExecutionOutput> {
        match self.asm_emulator() {
            Some(asm) => {
                let has_rom_sm = pctx.dctx_is_first_process();
                asm.execute(
                    zisk_rom,
                    stdin,
                    sm_bundle,
                    has_rom_sm,
                    use_hints,
                    stats,
                    caller_stats_scope,
                )
            }
            None => self.emulator_rust.execute(zisk_rom, stdin, sm_bundle),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emu_only_starts_in_rust_mode() {
        let phase = ExecutionPhase::new(1024, false);
        assert!(!phase.is_asm_execution());
        assert!(phase.asm_emulator().is_none());
    }

    #[test]
    fn asm_capable_starts_in_rust_mode() {
        let phase = ExecutionPhase::new(1024, true);
        assert!(!phase.is_asm_execution(), "runtime flag defaults to false");
        assert!(phase.asm_emulator().is_none(), "no asm dispatch until set_asm_resources");
    }

    #[test]
    fn rust_reset_is_noop() {
        let phase = ExecutionPhase::new(1024, false);
        assert!(phase.reset().is_ok());
    }

    #[test]
    fn rust_asm_execution_info_is_none() {
        let phase = ExecutionPhase::new(1024, false);
        let info = phase.get_asm_execution_info().expect("ok");
        assert!(info.is_none());
    }

    #[test]
    fn clear_without_set_is_safe() {
        let phase = ExecutionPhase::new(1024, true);
        phase.clear_asm_resources();
        assert!(!phase.is_asm_execution());
    }

    // Note: The ASM path is exercised via integration tests, since
    // `set_asm_resources` requires real shmem-backed `AsmResources`.
}
