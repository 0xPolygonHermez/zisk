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

pub type DeviceMetricsList = Vec<DeviceMetricsByChunk>;
pub type NestedDeviceMetricsList = HashMap<usize, DeviceMetricsList>;

use asm_runner::{AsmRunnerMO, AsmRunnerRH};
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::{collections::HashMap, thread::JoinHandle};
use zisk_common::{io::ZiskStdin, EmuTrace, ExecutorStatsHandle, StatsScope};

use anyhow::Result;

pub type EmulatorResult = (
    Vec<EmuTrace>,
    DeviceMetricsList,
    NestedDeviceMetricsList,
    Option<JoinHandle<Result<AsmRunnerMO>>>,
    Option<JoinHandle<Result<AsmRunnerRH>>>,
    u64,
);

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
