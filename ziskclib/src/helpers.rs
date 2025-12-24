use sha2::compress256;
#[allow(deprecated)]
use sha2::digest::generic_array::{typenum::U64, GenericArray};

pub fn sha256f(state: &mut [u64; 4], input: &[u64; 8]) {
    // Convert both the state and the input to appropriate types
    let state_u32: &mut [u32; 8] = unsafe { &mut *(state.as_mut_ptr() as *mut [u32; 8]) };
    let input_u8 = convert_u64_to_generic_array_bytes(input);

    compress256(state_u32, &[input_u8]);

    // Convert the state back to u64 and write it to the memory address
    *state = unsafe { *(state_u32 as *mut [u32; 8] as *mut [u64; 4]) };
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
