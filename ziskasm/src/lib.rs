//! ZisK assembly (`.zisk`) tooling.
//!
//! `parser` turns `.zisk` source text into instructions; `assembler` turns those
//! into a `ZiskRom` (the same ROM type the RISC-V transpiler produces), ready to
//! run on the emulator. See `ziskasm/ziskasm.md` for the language.

pub mod assembler;
pub mod parser;
pub mod utils;

pub use assembler::{assemble, assemble_files};
pub use utils::collect_zisk_files;
