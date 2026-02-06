mod dummy_counter;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod emu_asm;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod emu_asm_stub;
mod emu_rust;
mod executor;
mod sm_static_bundle;
mod static_data_bus;
mod static_data_bus_collect;

pub use dummy_counter::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use emu_asm::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use emu_asm_stub::*;
pub use emu_rust::*;
pub use executor::*;
pub use sm_static_bundle::*;
pub use static_data_bus::*;
pub use static_data_bus_collect::*;

pub type DeviceMetricsList = Vec<DeviceMetricsByChunk>;
pub type NestedDeviceMetricsList = HashMap<usize, DeviceMetricsList>;

use asm_runner::AsmRunnerMO;
use fields::PrimeField64;
use proofman_common::ProofCtx;
use std::{collections::HashMap, sync::Mutex, thread::JoinHandle};
use zisk_common::{io::ZiskStdin, EmuTrace, ExecutorStatsHandle, StatsScope, ZiskExecutionResult};

/// Trait for unified execution across different emulator backends
pub trait Emulator<F: PrimeField64>: Send + Sync {
    /// Execute the emulator
    fn execute(
        &self,
        stdin: &Mutex<ZiskStdin>,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        ZiskExecutionResult,
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
}

impl<F: PrimeField64> Emulator<F> for EmulatorKind {
    fn execute(
        &self,
        stdin: &Mutex<ZiskStdin>,
        pctx: &ProofCtx<F>,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        caller_stats_scope: &StatsScope,
    ) -> (
        Vec<EmuTrace>,
        DeviceMetricsList,
        NestedDeviceMetricsList,
        Option<JoinHandle<AsmRunnerMO>>,
        ZiskExecutionResult,
    ) {
        match self {
            Self::Asm(e) => e.execute(stdin, pctx, sm_bundle, stats, caller_stats_scope),
            Self::Rust(e) => e.execute(stdin, sm_bundle),
        }
    }
}
