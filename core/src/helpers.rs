use sha2::compress256;
#[allow(deprecated)]
use sha2::digest::generic_array::{typenum::U64, GenericArray};

pub fn sha256f(state: &mut [u64; 4], input: &[u64; 8]) {
    // Convert both the state and the input to appropriate types
    let mut state_u32: [u32; 8] = convert_u64_to_u32(state).try_into().unwrap();
    let block = convert_u64_to_generic_array_bytes(input);
    compress256(&mut state_u32, &[block]);

    // Convert the state back to u64 and write it to the memory address
    *state = convert_u32_to_u64(&state_u32);
}

pub fn convert_u64_to_u32(input: &[u64]) -> Vec<u32> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &word in input {
        out.push((word >> 32) as u32);
        out.push((word & 0xFFFFFFFF) as u32);
    }
    out
}

#[allow(deprecated)]
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
