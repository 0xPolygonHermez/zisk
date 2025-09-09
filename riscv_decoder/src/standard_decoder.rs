//! 32-bit instruction decoder. This decoder assumes a specific set of instructions are enabled.
//! See `crate::target::Extension` for what these extensions are.
mod error;
mod instruction;
mod opcode;

pub use error::Error;
pub use instruction::Instruction;

/// Decode a u32 into a standard RISC-V instruction
///
/// Note: The instruction should not be compressed.
pub fn decode_standard_instruction(bits: u32) -> Result<Instruction, Error> {
    todo!()
}
