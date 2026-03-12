//! ROM executor
//!
//! This module handles the execution of a ZisK ROM program, coordinating
//! the emulator backend and hints stream processing.

use crate::{
    AsmResources, DeviceMetricsList, Emulator, EmulatorKind, NestedDeviceMetricsList,
    StaticSMBundle,
};
use asm_runner::{AsmRunnerMO, AsmRunnerRH};
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::{sync::Mutex, thread::JoinHandle};
use zisk_common::{io::ZiskStdin, AsmExecutionInfo, EmuTrace, ExecutorStatsHandle, StatsScope};
use zisk_core::ZiskRom;

use anyhow::Result;

/// Output from ROM execution.
pub struct RomExecutionOutput {
    /// Minimal traces collected during execution.
    pub min_traces: Vec<EmuTrace>,
    /// Device metrics for main state machines.
    pub main_count: DeviceMetricsList,
    /// Device metrics for secondary state machines.
    pub secn_count: NestedDeviceMetricsList,
    /// Handle to memory operations thread (for ASM emulator).
    pub handle_mo: Option<JoinHandle<AsmRunnerMO>>,
    /// Handle to hints runner thread (for ASM emulator).
    pub handle_rh: Option<JoinHandle<AsmRunnerRH>>,
    /// Execution result with step counts.
    pub steps: u64,
}

pub struct RomExecutor {
    /// The emulator backend used for execution.
    emulator: EmulatorKind,

    /// Standard input for the ZisK program execution.
    stdin: Mutex<ZiskStdin>,
}

impl RomExecutor {
    /// Creates a new `RomExecutor`.
    ///
    /// # Arguments
    /// * `emulator` - The emulator backend to use.
    /// * `hints_stream` - Optional hints stream for precompile processing.
    pub fn new(emulator: EmulatorKind) -> Self {
        Self { emulator, stdin: Mutex::new(ZiskStdin::null()) }
    }

    /// Sets the standard input for execution.
    pub fn set_stdin(&self, stdin: ZiskStdin) {
        *self.stdin.lock().unwrap() = stdin;
    }

    pub fn set_asm_resources(&self, asm_resources: AsmResources) {
        self.emulator.set_asm_resources(asm_resources);
    }

    /// Resets the hints stream if configured.
    pub fn reset_hints_stream(&self) {
        self.emulator.reset_hints_stream()
    }

    pub fn get_asm_execution_info(&self) -> Option<AsmExecutionInfo> {
        self.emulator.get_asm_execution_info()
    }

    pub fn set_rh_data(&self, rh_data: AsmRunnerRH) {
        self.emulator.set_rh_data(rh_data);
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
        let (min_traces, main_count, secn_count, handle_mo, handle_rh, steps) =
            self.emulator.execute(
                zisk_rom,
                &self.stdin,
                pctx,
                sm_bundle,
                use_hints,
                stats,
                caller_stats_scope,
            )?;

        Ok(RomExecutionOutput { min_traces, main_count, secn_count, handle_mo, handle_rh, steps })
    }
}
