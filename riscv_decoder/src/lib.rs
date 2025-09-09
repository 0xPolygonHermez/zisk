mod compressed_decoder;
mod standard_decoder;

mod target;

use crate::compressed_decoder::{is_compressed, CompressedInstruction};

pub use standard_decoder::Instruction;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Tried to read past end of file")]
    ReadingPastEOF,
}

/// Indicates whether an instruction was compressed or not
///
/// This is needed because compressed instructions are converted to
/// their 32-bit counterpart, and we want to preserve this information.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WasCompressed {
    /// 16-bit compressed instruction
    Yes,
    /// 32-bit standard instruction
    No,
}

/// InstructionDecoder decodes bytes into RISC-V instructions
pub struct InstructionDecoder;

impl InstructionDecoder {
    /// Decodes a sequence of bytes into multiple instructions.
    ///
    /// The decoder supports `RV64IMAC`
    pub fn decode_bytes(bytes: &[u8]) -> Result<Vec<(Instruction, WasCompressed)>, Error> {
        let expected_code_alignment = code_alignment();
        assert!(
            bytes.len().is_multiple_of(expected_code_alignment),
            "code length = {} which is not a multiple of {}",
            bytes.len(),
            expected_code_alignment
        );

        let mut instructions = Vec::with_capacity(bytes.len() / 2);
        let mut i = 0;

        while i + 2 <= bytes.len() {
            // Read first 16-bit half
            let first_half = u16::from_le_bytes([bytes[i], bytes[i + 1]]);

            // Check if this is a 32-bit instruction
            if is_compressed(first_half) {
                // 16-bit compressed instruction
                let compressed_instruction = Self::decode_compressed(first_half)?;
                // Convert from Compressed to Standard
                let instruction = Instruction::from(compressed_instruction);
                instructions.push((instruction, WasCompressed::Yes));
                i += 2;
            } else {
                // 32-bit instruction - need second half
                if i + 4 > bytes.len() {
                    return Err(Error::ReadingPastEOF);
                }

                let second_half = u16::from_le_bytes([bytes[i + 2], bytes[i + 3]]);
                let bits = (first_half as u32) | ((second_half as u32) << 16);

                let instruction = Self::decode_standard(bits)?;
                instructions.push((instruction, WasCompressed::No));
                i += 4;
            }
        }

        Ok(instructions)
    }

    /// Decode a single 32-bit instruction
    fn decode_standard(_bits: u32) -> Result<Instruction, Error> {
        todo!()
    }

    /// Decode a single 16-bit compressed instruction
    fn decode_compressed(_bits: u16) -> Result<CompressedInstruction, Error> {
        todo!()
    }
}

/// Returns the code alignment in bytes
///
/// The code alignment should either be a multiple of 2 and or 4.
/// The specs note that if the compression extension is enabled
/// then the code alignment is 2 and if not, then it is 4.
///
/// The cases where it will not be a multiple of 2 or 4, is if
/// the code was written with assembly and data was manually
/// added to the assembly file. However, one can force alignment in
/// assembly or in a linker script by adding .align 2
const fn code_alignment() -> usize {
    compressed_decoder::IALIGN_BYTES
}
