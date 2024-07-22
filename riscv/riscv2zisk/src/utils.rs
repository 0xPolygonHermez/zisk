use crate::{SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_SP, SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE};

/// Read a u64 value from the u8 vector at the specified position in little endian order
pub fn read_u64_le(data: &[u8], index: usize) -> u64 {
    assert!(data.len() > (index + 7));
    let mut aux: u64;
    aux = data[index] as u64;
    aux += (data[index + 1] as u64).rotate_left(8);
    aux += (data[index + 2] as u64).rotate_left(16);
    aux += (data[index + 3] as u64).rotate_left(24);
    aux += (data[index + 4] as u64).rotate_left(32);
    aux += (data[index + 5] as u64).rotate_left(40);
    aux += (data[index + 6] as u64).rotate_left(48);
    aux += (data[index + 7] as u64).rotate_left(56);
    aux
}

/// Read a u32 value from the u8 vector at the specified position in little endian order
pub fn read_u32_le(data: &[u8], index: usize) -> u32 {
    assert!(data.len() > (index + 3));
    let mut aux: u32;
    aux = data[index] as u32;
    aux += (data[index + 1] as u32).rotate_left(8);
    aux += (data[index + 2] as u32).rotate_left(16);
    aux += (data[index + 3] as u32).rotate_left(24);
    aux
}

/// Read a u16 value from the u8 vector at the specified position in little endian order
pub fn read_u16_le(data: &[u8], index: usize) -> u16 {
    assert!(data.len() > (index + 1));
    let mut aux: u16;
    aux = data[index] as u16;
    aux += (data[index + 1] as u16).rotate_left(8);
    aux
}

/// Write a u64 value to the u8 vector at the specified position in little endian order
pub fn write_u64_le(data: &mut [u8], index: usize, value: u64) {
    assert!(data.len() > (index + 7));
    let mut aux: u64 = value;
    data[index] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 1] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 2] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 3] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 4] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 5] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 6] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 7] = aux as u8;
}

/// Write a u32 value to the u8 vector at the specified position in little endian order
pub fn write_u32_le(data: &mut [u8], index: usize, value: u32) {
    assert!(data.len() > (index + 3));
    let mut aux: u32 = value;
    data[index] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 1] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 2] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 3] = aux as u8;
}

/// Write a u16 value to the u8 vector at the specified position in little endian order
pub fn write_u16_le(data: &mut [u8], index: usize, value: u16) {
    assert!(data.len() > (index + 1));
    let mut aux: u16 = value;
    data[index] = aux as u8;
    aux = aux.rotate_right(8);
    data[index + 1] = aux as u8;
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
        SRC_SP => "SRC_SP",
        SRC_IND => "SRC_IND",
        _ => panic!("Unknown source({})", source),
    }
}

pub fn store_to_str(store: u64) -> &'static str {
    match store {
        STORE_NONE => "STORE_NONE",
        STORE_MEM => "STORE_MEM",
        STORE_IND => "STORE_IND",
        _ => panic!("Unknown store({})", store),
    }
}
