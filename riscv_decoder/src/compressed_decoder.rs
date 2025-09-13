mod error;
mod instruction;
mod opcode;

use crate::compressed_decoder::opcode::Opcode;
pub use error::Error;
pub use instruction::Instruction;

/// IALIGN_BITS refers to the instruction alignment constraint in bits.
///
/// This number is always either 16 or 32. It is 16 when the compressed
/// extension is enabled and 32 otherwise.
///
/// Note: The program counter will always increment by multiples of IALIGN
/// and not bytes.
pub const IALIGN_BITS: usize = 16;
/// IALIGN_BYTES refers to the instruction alignment constraint in bytes.
pub const IALIGN_BYTES: usize = IALIGN_BITS / 8;

const MASK2: u16 = 0b11; // 2-bit mask

#[inline(always)]
/// Compressed instructions can be identified by checking that the
/// last two bits in the instruction are not `0b11`
pub fn is_compressed(bits: u16) -> bool {
    (bits & MASK2) != 0x3
}

/// Decode a 16-bit compressed RISC-V instruction
pub fn decode_compressed_instruction(bits: u16) -> Result<Instruction, Error> {
    let encoded = EncodedInstruction::new(bits);

    let opcode = Opcode::from_bits(encoded.opcode)
        .ok_or(Error::UnsupportedOpcode { opcode_bits: encoded.opcode })?;

    match opcode {
        Opcode::Quadrant0 => todo!(),
        Opcode::Quadrant1 => todo!(),
        Opcode::Quadrant2 => todo!(),
    }
}

// TODO:
pub struct EncodedInstruction {
    opcode: u8,
}

impl EncodedInstruction {
    pub fn new(bits: u16) -> Self {
        todo!()
    }
}
