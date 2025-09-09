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

pub struct CompressedInstruction;

impl From<CompressedInstruction> for crate::Instruction {
    fn from(value: CompressedInstruction) -> Self {
        todo!()
    }
}
