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
mod planner;
mod registry;
mod rom_executor;
mod sm_static_bundle;
mod state;
mod static_data_bus;
mod static_data_bus_collect;
mod utils;

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
use planner::*;
use registry::*;
use rom_executor::*;
pub use sm_static_bundle::*;
pub use state::*;
pub use static_data_bus::*;
pub use static_data_bus_collect::*;
pub use utils::*;
use witness_generator::*;
use witness_orchestrator::*;
use zisk_core::ZiskRom;

pub type DeviceMetricsList = Vec<DeviceMetricsByChunk>;
pub type NestedDeviceMetricsList = HashMap<usize, DeviceMetricsList>;

use asm_runner::AsmRunnerMO;
use fields::PrimeField64;
use std::{collections::HashMap, sync::Mutex, thread::JoinHandle};
use zisk_common::{io::ZiskStdin, EmuTrace, ExecutorStatsHandle, StatsScope};

/// Trait for unified execution across different emulator backends
#[allow(clippy::too_many_arguments)]
pub trait Emulator<F: PrimeField64>: Send + Sync {
    /// Execute the emulator
    fn execute(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &Mutex<ZiskStdin>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        u64,
    );
}

/// Enum wrapper for different emulator backends (no heap allocation)
pub enum EmulatorKind {
    Asm(EmulatorAsm),
    Rust(EmulatorRust),
}

impl EmulatorKind {
    /// Check if this is an ASM emulator (non-generic, can be called without F)
    pub fn is_asm_emulator(&self) -> bool {
        matches!(self, Self::Asm(_))
    }

    pub fn get_chunk_size(&self) -> u64 {
        match self {
            Self::Asm(e) => e.get_chunk_size(),
            Self::Rust(e) => e.get_chunk_size(),
        }
    }

    pub fn set_asm_resources(&self, asm_resources: AsmResources) {
        match self {
            Self::Asm(e) => e.set_asm_resources(asm_resources),
            Self::Rust(_) => (), // No ASM resources in Rust emulator
        };
    }

    pub fn reset_hints_stream(&self) {
        match self {
            Self::Asm(e) => e.reset_hints_stream(),
            Self::Rust(_) => (), // No hints stream in Rust emulator
        }
    }
}

impl<F: PrimeField64> Emulator<F> for EmulatorKind {
    fn execute(
        &self,
        zisk_rom: &ZiskRom,
        stdin: &Mutex<ZiskStdin>,
        sm_bundle: &StaticSMBundle<F>,
        use_hints: bool,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        u64,
    ) {
        match self {
            Self::Asm(e) => {
                e.execute(zisk_rom, stdin, sm_bundle, use_hints, stats, caller_stats_scope)
            }
            Self::Rust(e) => e.execute(zisk_rom, stdin, sm_bundle),
        }
    }
}
