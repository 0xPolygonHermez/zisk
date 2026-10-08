//! Operation codes. Shared by Rust and PIL; PIL wants an `OP_` prefix.

use zisk_definitions_macros::constants;

#[constants(group = "opcodes", to(rust, pil), hex, pil_prefix = "OP_")]
pub mod opcodes {
    /// Addition.
    pub const ADD: u8 = 0x0a;
    /// Subtraction.
    pub const SUB: u8 = 0x0b;
}

pub use opcodes::{EXPORTS, GROUP};
