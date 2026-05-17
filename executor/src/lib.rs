//! Executor crate: responsible for executing Zisk ROMs and collecting execution traces and metrics.
//!
//! This crate provides a unified interface for executing Zisk ROMs across different emulator
//! backends, including an x86-64 assembly emulator and a Rust-based emulator.
//! It collects execution traces, counters, and other relevant data during execution,
//! which is used for proof generation.

#![warn(missing_docs)] // ratchet up to deny once clean
#![warn(rustdoc::all)] // broken intra-doc links, invalid HTML, bare URLs
#![deny(rustdoc::all)]

mod adapters;
mod air_classifier;
mod asm_resources;
mod collector;
mod dummy_counter;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod emu_asm;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod emu_asm_stub;
mod emu_rust;
mod executor;
mod init;
mod planner;
mod ports;
mod pub_outs_collector;
mod registry;
mod rom_executor;
mod sm_builtins;
mod sm_precompiles;
mod sm_registry;
mod state;
mod static_data_bus;
mod static_data_bus_collect;

mod witness_generator;
mod witness_orchestrator;

pub use adapters::*;
use air_classifier::*;
pub use asm_resources::*;
use collector::*;
pub use dummy_counter::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use emu_asm::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use emu_asm_stub::*;
pub use emu_rust::*;
pub use executor::*;
pub use init::*;
use planner::*;
pub use ports::*;
use registry::*;
use rom_executor::*;
pub use sm_builtins::*;
pub use sm_precompiles::*;
pub use state::*;
pub use static_data_bus::*;
pub use static_data_bus_collect::*;
use witness_generator::*;
use witness_orchestrator::*;
use zisk_core::ZiskRom;
/// Type alias for chunk counters, mapping SM type ID to a list of device metrics by chunk.
pub type CountersChunkMetrics = HashMap<usize, Vec<DeviceMetricsByChunk>>;

use asm_runner::{AsmRunnerMO, AsmRunnerRH};
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::{collections::HashMap, thread::JoinHandle};
use zisk_common::{io::ZiskStdin, EmuTrace, ExecutorStatsHandle, StatsScope};

use anyhow::Result;

use crate::pub_outs_collector::PubOutsCollector;

/// Result of an emulator execution
pub struct EmulatorResult {
    /// Minimal traces produced by the emulator.
    pub min_traces: Vec<EmuTrace>,
    /// Device metrics for secondary devices.
    pub counters: CountersChunkMetrics,
    /// Join handle for the memory-only ASM runner (only applicable for ASM emulator).
    pub handle_mo: Option<JoinHandle<Result<AsmRunnerMO>>>,
    /// Join handle for the ROM histogram runner (only applicable for ASM emulator).
    pub handle_rh: Option<JoinHandle<Result<AsmRunnerRH>>>,
    /// Total number of steps executed by the emulator.
    pub steps: u64,
    /// Collected public outputs from the emulator execution.
    pub pub_outs: PubOutsCollector,
}

/// Trait for unified execution across different emulator backends
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub trait Emulator<F: PrimeField64>: Send + Sync {
    /// Execute the emulator
    fn execute(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &ZiskStdin,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> Result<EmulatorResult>;
}
