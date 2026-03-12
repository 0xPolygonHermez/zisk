use sha2::compress256;

#[allow(deprecated)]
use sha2::digest::generic_array::{typenum::U64, GenericArray};

use precompiles_helpers::blake2b_round;

#[allow(deprecated)]
pub fn sha256f(state: &mut [u64; 4], input: &[u64; 8]) {
    let state_u32: &mut [u32; 8] = unsafe { &mut *(state.as_mut_ptr() as *mut [u32; 8]) };
    let input_u8: &[GenericArray<u8, U64>; 1] =
        unsafe { &*(input.as_ptr() as *const [GenericArray<u8, U64>; 1]) };
    compress256(state_u32, input_u8);
}

#[allow(deprecated)]
pub fn blake2br(index: u64, state: &mut [u64; 16], input: &[u64; 16]) {
    blake2b_round(state, input, index as u32);
}
