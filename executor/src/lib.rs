mod dummy_counter;
mod executor;
mod runner_asm;
mod runner_emu;
mod sm_static_bundle;
mod static_data_bus;
mod static_data_bus_collect;

pub use dummy_counter::*;
pub use executor::*;
use fields::PrimeField64;
use proofman_common::ProofCtx;
pub use runner_asm::*;
pub use runner_emu::*;
pub use sm_static_bundle::*;
pub use static_data_bus::*;
pub use static_data_bus_collect::*;

use asm_runner::{AsmRunnerMO, AsmRunnerRH, MinimalTraces};
use std::{path::PathBuf, thread::JoinHandle};
use zisk_common::ExecutorStatsHandle;
use zisk_core::ZiskRom;

/// The maximum number of steps to execute in the emulator or assembly runner.
pub const MAX_NUM_STEPS: u64 = 1 << 32;

pub struct ExecutionResult {
    minimal_traces: MinimalTraces,
    main: DeviceMetricsList,
    secn: NestedDeviceMetricsList,
    steps: u64,
}

pub enum ExecutionResultEnum {
    FinalResult(ExecutionResult),
    PartialResult(ExecutionResult),
}

impl ExecutionResultEnum {
    pub fn minimal_traces(&self) -> &MinimalTraces {
        match self {
            ExecutionResultEnum::FinalResult(res) => &res.minimal_traces,
            ExecutionResultEnum::PartialResult(res) => &res.minimal_traces,
        }
    }

    pub fn main(&self) -> &DeviceMetricsList {
        match self {
            ExecutionResultEnum::FinalResult(res) => &res.main,
            ExecutionResultEnum::PartialResult(res) => &res.main,
        }
    }

    pub fn secn(&self) -> &NestedDeviceMetricsList {
        match self {
            ExecutionResultEnum::FinalResult(res) => &res.secn,
            ExecutionResultEnum::PartialResult(res) => &res.secn,
        }
    }

    pub fn steps(&self) -> u64 {
        match self {
            ExecutionResultEnum::FinalResult(res) => res.steps,
            ExecutionResultEnum::PartialResult(res) => res.steps,
        }
    }

    pub fn into_inner(self) -> (MinimalTraces, DeviceMetricsList, NestedDeviceMetricsList, u64) {
        match self {
            ExecutionResultEnum::FinalResult(res) => {
                (res.minimal_traces, res.main, res.secn, res.steps)
            }
            ExecutionResultEnum::PartialResult(res) => {
                (res.minimal_traces, res.main, res.secn, res.steps)
            }
        }
    }
}

pub trait ExecutorRunner<F: PrimeField64>: Send + Sync {
    /// Runs the executor with the given ZisK ROM, input data path, and chunk size.
    /// Returns a tuple containing minimal traces, device metrics, nested device metrics, and the number of steps executed.
    /// # Arguments
    /// * `zisk_rom` - The ZisK ROM to be executed.
    /// * `input_data_path` - Optional path to the input data file.
    /// * `chunk_size` - The size of each chunk for processing.
    /// # Returns
    /// A tuple containing:
    /// - `MinimalTraces`: The minimal traces generated during execution.
    /// - `DeviceMetricsList`: Metrics related to device performance.
    /// - `NestedDeviceMetricsList`: Nested metrics for more detailed performance analysis.
    /// - `u64`: The total number of steps executed.
    fn run(
        &mut self,
        pctx: &ProofCtx<F>,
        zisk_rom: &ZiskRom,
        input_data_path: Option<PathBuf>,
        chunk_size: u64,
        sm_bundle: &StaticSMBundle<F>,
        stats: &ExecutorStatsHandle,
        #[cfg(feature = "stats")] _caller_stats_id: u64,
    ) -> ExecutionResultEnum;

    fn finalize(&mut self) -> Option<(JoinHandle<AsmRunnerRH>, JoinHandle<AsmRunnerMO>)> {
        None
    }
}
