//! Program memory map. Shared by Rust, the C emulator, PIL, and the hand-written asm.

use zisk_definitions_macros::constants;

#[constants(group = "memory", to(rust, c, pil, asm), hex, fits = 32)]
pub mod memory {
    /// First global RW memory address.
    pub const RAM_ADDR: u64 = 0xa000_0000;

    /// Program stack size — derives `SYS_ADDR`; itself emitted nowhere.
    #[emit(internal)]
    pub const STACK_SIZE: u64 = 0x40_0000;

    /// First system RW memory address.
    pub const SYS_ADDR: u64 = RAM_ADDR + STACK_SIZE;

    /// Extra precompile parameters (256 B → 32 params). Rust + PIL only.
    #[emit(skip(c))]
    pub const EXTRA_PARAMS_ADDR: u64 = SYS_ADDR + 0x0F00;
}

pub use memory::{EXPORTS, GROUP};
