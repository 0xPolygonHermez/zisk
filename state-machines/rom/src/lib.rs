//! ROM state machine for the ZisK proving pipeline.
//!
//! - [`RomSM`] is the `ComponentBuilder` that wires the parsed ROM and the per-instruction
//!   execution counters into a [`RomInstance`].
//! - [`RomInstance`] computes the ROM multiplicity witness, dispatching to the Rust- or
//!   ASM-emulator path depending on which inputs `RomSM` was fed.
//! - [`CustomRom`] builds the static ROM-ROM trace from ELF bytes (used at setup time,
//!   not during proving).
//! - [`RomError`] / [`RomResult`] are the crate-local error types; the boundary with
//!   `proofman_common` is bridged at the call site.

#![warn(missing_docs)]
#![deny(rustdoc::all)]

mod custom_rom;
mod error;
mod rom;
mod rom_counter;
mod rom_instance;
mod rom_planner;

pub use custom_rom::CustomRom;
pub use error::*;
pub use rom::*;
pub use rom_instance::*;
use rom_planner::*;
