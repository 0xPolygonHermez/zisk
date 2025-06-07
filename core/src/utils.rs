use crate::{
    SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG, SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE,
    STORE_REG,
};

use generic_array::{typenum::U64, GenericArray};

// #[cfg(feature = "sp")]
// use crate::SRC_SP;

/// Converts a u8 vector into a u32 vector
/// The length of the input vector must be a multiple of 4
pub fn convert_vector(input: &[u8]) -> Vec<u32> {
    // Check that the input length is a multiple of 4
    let input_len = input.len();
    if (input_len & 0x03) != 0 {
        panic!("convert_vector() found input length={} not a multiple of 4", input.len());
    }

    // Calculate the output length
    let output_len = input_len >> 2;

    // Create an empty u32 vector
    let mut output: Vec<u32> = Vec::<u32>::new();

    // For every output u32 data, calculate it based on input u8 data, in little endian order
    for i in 0..output_len {
        output.push(u32::from_le_bytes(input[4 * i..4 * i + 4].try_into().unwrap()));
    }

    // Return the output u32 vector
    output
}

/// Returns a human-readable text that describes an a or b registers source
pub fn source_to_str(source: u64) -> &'static str {
    match source {
        SRC_C => "SRC_C",
        SRC_REG => "SRC_REG",
        SRC_MEM => "SRC_MEM",
        SRC_IMM => "SRC_IMM",
        SRC_STEP => "SRC_STEP",
        // #[cfg(feature = "sp")]
        // SRC_SP => "SRC_SP",
        SRC_IND => "SRC_IND",
        _ => panic!("source_to_str() unknown source({})", source),
    }
}

/// Returns a human-readable text that describes a c register store destination
pub fn store_to_str(store: u64) -> &'static str {
    match store {
        STORE_NONE => "STORE_NONE",
        STORE_MEM => "STORE_MEM",
        STORE_REG => "STORE_REG",
        STORE_IND => "STORE_IND",
        _ => panic!("store_to_str() unknown store({})", store),
    }
}

pub fn is_elf_file(file_data: &[u8]) -> std::io::Result<bool> {
    if file_data.len() < 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "File data is too short to be a valid ELF file",
        ));
    }

    // Check if the first 4 bytes match the ELF magic number
    Ok(file_data[0..4] == [0x7F, b'E', b'L', b'F'])
}

pub fn convert_u64_to_u32(input: &[u64]) -> Vec<u32> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &word in input {
        out.push((word >> 32) as u32);
        out.push((word & 0xFFFFFFFF) as u32);
    }
    out
}

pub fn convert_u64_to_generic_array_bytes(input: &[u64; 8]) -> GenericArray<u8, U64> {
    let mut out = [0u8; 64];
    for (i, word) in input.iter().enumerate() {
        for j in 0..8 {
            out[i * 8 + j] = (word >> (56 - j * 8)) as u8;
        }
    }
    GenericArray::<u8, U64>::clone_from_slice(&out)
}

pub fn convert_u32_to_u64(words: &[u32; 8]) -> [u64; 4] {
    let mut out = [0u64; 4];
    for i in 0..4 {
        out[i] = ((words[2 * i] as u64) << 32) | (words[2 * i + 1] as u64);
    }
    out
}
