//! The Zisk emulator executes the Zisk program rom with the provided input data and generates
//! the corresponding output data, according to the configured options.
//!
//! ```text
//! ELF file --> riscv2zisk --> ZiskRom    \
//!                                         |
//! ZiskRom ------------------> ZiskInst's  |
//!     \--> RO data                         > Emu --> Output data, statistics, metrics, logs...
//!             \                           |
//! Input file ---------------> Mem         |
//!                                         |
//! User configuration -------> EmuOptions /
//! ```

mod elf_symbol_reader;
mod emu;
mod emu_context;
pub mod emu_costs;
mod emu_full_trace;
pub mod emu_options;
mod emu_par_options;
mod emu_reg_trace;
mod emu_segment;
mod emulator;
mod emulator_errors;
pub mod mem_operations_stats;
mod regions_of_interest;
pub mod stats;
mod stats_cost_mark;
mod stats_costs;
pub mod stats_coverage_report;
pub mod stats_report;

pub use elf_symbol_reader::*;
pub use emu::*;
pub use emu_context::*;
pub use emu_costs::*;
pub use emu_full_trace::*;
pub use emu_options::*;
pub use emu_par_options::*;
pub use emu_reg_trace::*;
pub use emu_segment::*;
pub use emulator::*;
pub use emulator_errors::*;
pub use mem_operations_stats::*;
pub use regions_of_interest::*;
pub use stats::*;
pub use stats_cost_mark::*;
pub use stats_costs::*;
pub use stats_coverage_report::*;
pub use stats_report::*;
