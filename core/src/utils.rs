use crate::{SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE};

// #[cfg(feature = "sp")]
// use crate::SRC_SP;

/// Read a u64 value from the u8 vector at the specified position in little endian order
#[inline(always)]
pub fn read_u64_le(data: &[u8], index: usize) -> u64 {
    u64::from_le_bytes(data[index..index + 8].try_into().unwrap())
}

/// Read a u32 value from the u8 vector at the specified position in little endian order
#[inline(always)]
pub fn read_u32_le(data: &[u8], index: usize) -> u32 {
    u32::from_le_bytes(data[index..index + 4].try_into().unwrap())
}

/// Read a u16 value from the u8 vector at the specified position in little endian order
#[inline(always)]
pub fn read_u16_le(data: &[u8], index: usize) -> u16 {
    u16::from_le_bytes(data[index..index + 2].try_into().unwrap())
}

/// Write a u64 value to the u8 vector at the specified position in little endian order
#[inline(always)]
pub fn write_u64_le(data: &mut [u8], index: usize, value: u64) {
    data[index..index + 8].copy_from_slice(&value.to_le_bytes());
}

/// Write a u32 value to the u8 vector at the specified position in little endian order
#[inline(always)]
pub fn write_u32_le(data: &mut [u8], index: usize, value: u32) {
    data[index..index + 4].copy_from_slice(&value.to_le_bytes());
}

/// Write a u16 value to the u8 vector at the specified position in little endian order
#[inline(always)]
pub fn write_u16_le(data: &mut [u8], index: usize, value: u16) {
    data[index..index + 2].copy_from_slice(&value.to_le_bytes());
}

/// Converts a u8 vector into a u32 vector
pub fn convert_vector(input: &[u8]) -> Vec<u32> {
    // Check that the input length is a multiple of 4
    let input_len = input.len();
    if (input_len % 4) != 0 {
        panic!("convert_vector() found input length={} not a multiple of 4", input.len());
    }

    // Calculate the output length
    let output_len = input_len >> 2;

    // Create an empty u32 vector
    let mut output: Vec<u32> = Vec::<u32>::new();

    // For every output u32 data, calculate it based on input u8 data, in little endian order
    for i in 0..output_len {
        output.push(read_u32_le(input, 4 * i));
    }

    // Return the output u32 vector
    output
}

pub fn source_to_str(source: u64) -> &'static str {
    match source {
        SRC_C => "SRC_C",
        SRC_MEM => "SRC_MEM",
        SRC_IMM => "SRC_IMM",
        SRC_STEP => "SRC_STEP",
        // #[cfg(feature = "sp")]
        // SRC_SP => "SRC_SP",
        SRC_IND => "SRC_IND",
        _ => panic!("source_to_str() unknown source({})", source),
    }
}

pub fn store_to_str(store: u64) -> &'static str {
    match store {
        STORE_NONE => "STORE_NONE",
        STORE_MEM => "STORE_MEM",
        STORE_IND => "STORE_IND",
        _ => panic!("store_to_str() unknown store({})", store),
    }
}
