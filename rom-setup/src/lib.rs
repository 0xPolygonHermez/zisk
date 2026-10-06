#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]

//! ROM setup for the ZisK zkVM.
//!
//! Given a guest program's RISC-V ELF, this crate produces the on-disk
//! artifacts the prover needs before it can run or prove that program:
//!
//! - **ASM emulator binaries** — the native minimal-trace, ROM-histogram, and
//!   memory-op binaries used by the ASM backend, generated from the ELF via the
//!   `emulator-asm` toolchain. See [`generate_assembly`] and
//!   [`get_assembly_file_paths`].
//! - **ROM Merkle setup** — the ROM custom commit and the program verification
//!   key (verkey) derived from it. See [`rom_merkle_setup`] and
//!   [`rom_merkle_setup_verkey`].
//!
//! All artifacts are content-addressed by the blake3 hash of the ELF (see
//! [`get_elf_data_hash`]) and cached under the ZisK cache directory, so a given
//! program is set up only once. The [`HashMode`] selects the Merkle hash family
//! and must match the one the proving key was generated with.

mod asm_setup;
mod rom_merkle;
mod utils;

pub use asm_setup::*;
pub use rom_merkle::*;
pub use utils::*;
