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
/// Path-related utilities and types.
pub mod paths;
mod planner_helpers;
mod profiling;
mod proof;
mod proof_log;
mod regular_counters;
mod regular_planner;
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
pub use paths::*;
pub use planner_helpers::*;
pub use profiling::*;
pub use proof::*;
pub use proof_log::*;
pub use regular_counters::*;
pub use regular_planner::*;
pub use types::*;
pub use utils::*;
pub use zisk_precompile::*;

/// Whether the current block's RAM memory witness rows come from the GPU planner. Set by the
/// memory-ops runner once the device fill succeeded, cleared when a block starts; while it is
/// false the Mem instances are collected and filled on the CPU.
pub static MEM_RAM_ROWS_ON_DEVICE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
