mod bus_device_metrics;
mod bus_device_wrapper;
mod component_builder;
mod component_counter;
mod component_instance;
mod component_planner;
mod dummy_counter;
mod instance_context;
mod planner_helpers;
mod regular_counters;
mod regular_planner;
mod utils;

use asm_runner::AsmRunnerMT;
pub use bus_device_metrics::*;
pub use bus_device_wrapper::*;
pub use component_builder::*;
pub use component_counter::*;
pub use component_instance::*;
pub use component_planner::*;
pub use dummy_counter::*;
pub use instance_context::*;
pub use planner_helpers::*;
pub use regular_counters::*;
pub use regular_planner::*;
pub use utils::*;

use zisk_common::EmuTrace;

#[derive(Debug)]
pub enum MinimalTraces {
    None,
    EmuTrace(Vec<EmuTrace>),
    AsmEmuTrace(AsmRunnerMT),
}
