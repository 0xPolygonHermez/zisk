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
pub use types::*;
pub use utils::*;
pub use zisk_precompile::*;

/// Which memory airs' witness rows come from the GPU planner for the current block, as a mask of
/// `MEM_ROWS_*` bits. Set by the memory-ops runner once each device fill succeeded, cleared when a
/// block starts; an air whose bit is clear is collected and filled on the CPU.
pub static MEM_ROWS_ON_DEVICE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// `MEM_ROWS_ON_DEVICE` bit: the `Mem` (RAM) rows.
pub const MEM_ROWS_RAM: u32 = 1;
/// `MEM_ROWS_ON_DEVICE` bit: the `RomData` rows.
pub const MEM_ROWS_ROM: u32 = 2;
/// `MEM_ROWS_ON_DEVICE` bit: the `InputData` rows.
pub const MEM_ROWS_INPUT: u32 = 4;
