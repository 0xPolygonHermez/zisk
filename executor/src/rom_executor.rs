//! ROM executor
//!
//! This module handles the execution of a ZisK ROM program, coordinating
//! the emulator backend and hints stream processing.

use crate::{AsmResources, EmulatorAsm, EmulatorRust, NestedDeviceMetricsList, StaticSMBundle};
use arc_swap::ArcSwap;
use asm_runner::{AsmRunnerMO, AsmRunnerRH};
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use zisk_common::{io::ZiskStdin, AsmExecutionInfo, EmuTrace, ExecutorStatsHandle, StatsScope};
use zisk_core::ZiskRom;

use anyhow::Result;

/// Output from ROM execution.
pub struct RomExecutionOutput {
    /// Minimal traces collected during execution.
    pub min_traces: Vec<EmuTrace>,
    /// Device metrics for secondary state machines.
    pub counters: NestedDeviceMetricsList,
    /// Handle to memory operations thread (for ASM emulator).
    pub handle_mo: Option<JoinHandle<Result<AsmRunnerMO>>>,
    /// Handle to hints runner thread (for ASM emulator).
    pub handle_rh: Option<JoinHandle<Result<AsmRunnerRH>>>,
    /// Execution result with step counts.
    pub steps: u64,
    /// Public outputs accumulated during execution (low/high 32-bit halves).
    pub pub_outs: Vec<(u64, u32)>,
}

pub struct RomExecutor {
    // Emulator backend for executing the ROM program in ASM.
    emulator_asm: EmulatorAsm,

    // Emulator backend for executing the ROM program in native.
    emulator_rust: EmulatorRust,

    is_asm_execution: AtomicBool,

    /// Standard input for the ZisK program execution.
    stdin: ArcSwap<ZiskStdin>,
}

impl RomExecutor {
    /// Creates a new `RomExecutor`.
    ///
    /// # Arguments
    /// * `chunk_size` - Chunk size for processing.
    pub fn new(chunk_size: u64) -> Self {
        Self {
            emulator_asm: EmulatorAsm::new(chunk_size),
            emulator_rust: EmulatorRust::new(chunk_size),
            is_asm_execution: AtomicBool::new(false),
            stdin: ArcSwap::from_pointee(ZiskStdin::new()),
        }
    }

    pub fn is_asm_emulator(&self) -> bool {
        self.is_asm_execution.load(Ordering::SeqCst)
    }

    /// Sets the standard input for execution.
    pub fn set_stdin(&self, stdin: ZiskStdin) -> Result<()> {
        self.stdin.store(Arc::new(stdin));
        Ok(())
    }

    pub fn set_asm_resources(&self, asm_resources: Arc<AsmResources>) -> Result<()> {
        self.is_asm_execution.store(true, Ordering::SeqCst);
        self.emulator_asm.set_asm_resources(asm_resources)
    }

    /// Clears the ASM-execution flag so subsequent `execute` calls route through the
    /// Rust emulator. Used when switching to a program that was set up emulator-only.
    pub fn clear_asm_resources(&self) {
        self.is_asm_execution.store(false, Ordering::SeqCst);
    }

    /// Returns a reference to the ASM emulator if ASM execution is active.
    pub fn asm_emulator(&self) -> Option<&EmulatorAsm> {
        self.is_asm_execution.load(Ordering::SeqCst).then_some(&self.emulator_asm)
    }

    /// Resets the ASM pipeline for the next job.
    pub fn reset(&self) -> Result<()> {
        if let Some(asm) = self.asm_emulator() {
            asm.reset()?;
        }
        Ok(())
    }

    pub fn get_asm_execution_info(&self) -> Result<Option<AsmExecutionInfo>> {
        if self.is_asm_execution.load(Ordering::SeqCst) {
            self.emulator_asm.get_asm_execution_info()
        } else {
            Ok(None)
        }
    }

    /// Executes the ROM program and collects minimal traces.
    ///
    /// # Arguments
    /// * `zisk_rom` - The ROM to execute.
    /// * `pctx` - Proof context.
    /// * `sm_bundle` - State machine bundle.
    /// * `use_hints` - Flag to indicate whether to use hints.
    /// * `stats` - Statistics handle.
    /// * `caller_stats_scope` - Parent statistics scope.
    ///
    /// # Returns
    /// Execution output containing traces, metrics, and results.
    pub fn execute<F: PrimeField64>(
        &self,
        zisk_rom: &ZiskRom,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> Result<RomExecutionOutput> {
        let stdin = self.stdin.load_full();
        let (min_traces, counters, handle_mo, handle_rh, steps, pub_outs) =
            match self.is_asm_execution.load(Ordering::SeqCst) {
                true => self.emulator_asm.execute(
                    zisk_rom,
                    &stdin,
                    pctx,
                    sm_bundle,
                    use_hints,
                    stats,
                    caller_stats_scope,
                )?,
                false => self.emulator_rust.execute(zisk_rom, &stdin, sm_bundle)?,
            };

        Ok(RomExecutionOutput { min_traces, counters, handle_mo, handle_rh, steps, pub_outs })
    }
}
