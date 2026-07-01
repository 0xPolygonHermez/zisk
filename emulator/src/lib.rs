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

mod disasm;
mod elf_symbol_reader;
mod emu;
mod emu_context;
pub mod emu_costs;
pub mod emu_options;
mod emu_par_options;
mod emu_reg_trace;
mod emu_segment;
mod emulator;
mod emulator_errors;
pub mod stats;

pub use disasm::*;
pub use elf_symbol_reader::*;
pub use emu::*;
pub use emu_context::*;
pub use emu_costs::*;
pub use emu_options::*;
pub use emu_par_options::*;
pub use emu_reg_trace::*;
pub use emu_segment::*;
pub use emulator::*;
pub use emulator_errors::*;
pub use stats::*;
pub use zisk_common::ProfilingMode;
