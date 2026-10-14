//! Common utilities and types for Zisk.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

/// Global allocator selection.
pub mod allocator;
#[macro_use]
pub mod witness_timers;
mod bus;
mod component;
mod emu_minimal_trace;
mod error;
mod executor_stats;
mod hash_mode;
mod hints;
mod instance_context;
/// I/O utilities and types.
pub mod io;
mod late_value;
/// Path-related utilities and types.
pub mod paths;
mod planner_helpers;
mod profiling;
mod proof;
mod proof_log;
mod regular_counters;
mod regular_planner;
mod seed;
mod types;
mod utils;
mod zisk_precompile;

pub use bus::*;
pub use component::*;
pub use emu_minimal_trace::*;
// Named (not glob) so the `Result` alias isn't exported into `zisk_common::*`,
// where it would shadow `std::result::Result` for downstream consumers.
pub use error::CommonError;
pub use executor_stats::*;
pub use hash_mode::*;
pub use hints::*;
pub use instance_context::*;
pub use late_value::*;
pub use paths::*;
pub use planner_helpers::*;
pub use profiling::*;
pub use proof::*;
pub use proof_log::*;
pub use regular_counters::*;
pub use regular_planner::*;
pub use seed::*;
pub use types::*;
pub use utils::*;
pub use zisk_precompile::*;

/// Whether the memory airs' witness rows (Mem, RomData, InputData, MemAlign) come from the GPU
/// planner for the current block. Set by the memory-ops runner once the block's preparation
/// started, cleared when a block starts, when the last owned instance took its rows, and when the
/// preparation or an instance fails: from then on the block's memory instances are collected and
/// filled on the CPU.
pub static MEM_ROWS_ON_DEVICE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
/// Whether this block's rows of the memory airs come from the GPU planner (see
/// [`MEM_ROWS_ON_DEVICE`]).
pub fn mem_rows_on_device() -> bool {
    MEM_ROWS_ON_DEVICE.load(std::sync::atomic::Ordering::Acquire)
}

/// Whether the memory witness is built on the GPU planner when the assembly emulator runs with
/// it: every owned memory instance takes its rows from the planner's staged image (or is built on
/// demand) into its host trace buffer. `ZISK_MEM_GPU_FILL=off` (or `cpu`) keeps the CPU witness;
/// unset, empty or `on` means on, as does an unknown value (with a warning). Read once.
pub fn device_mem_witness() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| match std::env::var("ZISK_MEM_GPU_FILL").as_deref() {
        Ok("off") | Ok("cpu") => false,
        Ok("") | Ok("on") | Err(_) => true,
        Ok(other) => {
            tracing::warn!("ZISK_MEM_GPU_FILL={other}: unknown value, the device witness is on");
            true
        }
    })
}
