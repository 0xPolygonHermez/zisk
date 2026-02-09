//! ROM executor
//!
//! This module handles the execution of a ZisK ROM program, coordinating
//! the emulator backend and hints stream processing.

use crate::{DeviceMetricsList, Emulator, EmulatorKind, NestedDeviceMetricsList, StaticSMBundle};
use anyhow::Result;
use asm_runner::AsmRunnerMO;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::{sync::Mutex, thread::JoinHandle};
use zisk_common::{
    io::{StreamSource, ZiskStdin, ZiskStream},
    EmuTrace, ExecutorStatsHandle, StatsScope, ZiskExecutionResult,
};
use zisk_core::ZiskRom;

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
    /// Execution result with step counts.
    pub execution_result: ZiskExecutionResult,
}

pub struct RomExecutor {
    /// The emulator backend used for execution.
    emulator: EmulatorKind,

    /// Standard input for the ZisK program execution.
    stdin: Mutex<ZiskStdin>,

    /// Pipeline for handling precompile hints.
    hints_stream: Mutex<Option<ZiskStream>>,
}

impl RomExecutor {
    /// Creates a new `RomExecutor`.
    ///
    /// # Arguments
    /// * `emulator` - The emulator backend to use.
    /// * `hints_stream` - Optional hints stream for precompile processing.
    pub fn new(emulator: EmulatorKind, hints_stream: Option<ZiskStream>) -> Self {
        Self {
            emulator,
            stdin: Mutex::new(ZiskStdin::null()),
            hints_stream: Mutex::new(hints_stream),
        }
    }

    /// Sets the standard input for execution.
    pub fn set_stdin(&self, stdin: ZiskStdin) {
        *self.stdin.lock().unwrap() = stdin;
    }

    /// Sets the hints stream source.
    pub fn set_hints_stream_src(&self, stream: StreamSource) -> Result<()> {
        if let Some(hints_stream) = self.hints_stream.lock().unwrap().as_mut() {
            hints_stream.set_hints_stream_src(stream)
        } else {
            Err(anyhow::anyhow!("No hints stream configured"))
        }
    }

    /// Starts the hints stream if configured.
    pub fn start_hints_stream(&self) {
        if let Ok(mut hints_stream_guard) = self.hints_stream.lock() {
            if let Some(hints_stream) = hints_stream_guard.as_mut() {
                let _ = hints_stream.start_stream();
            }
        }
    }

    /// Resets the hints stream if configured.
    pub fn reset_hints_stream(&self) {
        if let Ok(mut hints_stream_guard) = self.hints_stream.lock() {
            if let Some(hints_stream) = hints_stream_guard.as_mut() {
                hints_stream.reset();
            }
        }
    }

    /// Executes the ROM program and collects minimal traces.
    ///
    /// # Arguments
    /// * `zisk_rom` - The ROM to execute.
    /// * `pctx` - Proof context.
    /// * `sm_bundle` - State machine bundle.
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
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> RomExecutionOutput {
        let (min_traces, main_count, secn_count, handle_mo, execution_result) = self
            .emulator
            .execute(zisk_rom, &self.stdin, pctx, sm_bundle, stats, caller_stats_scope);

        RomExecutionOutput { min_traces, main_count, secn_count, handle_mo, execution_result }
    }
}
