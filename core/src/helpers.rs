use sha2::compress256;

#[allow(deprecated)]
use sha2::digest::generic_array::{typenum::U64, GenericArray};

use zisk_precomp_helpers::{blake2b_round, blake2s_round, BLAKE2S_ROUNDS};

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

/// BLAKE2s round over 32-bit words. Each word occupies its own 64-bit slot
/// with a zero high half, matching the memory layout the Blake2sr AIR
/// enforces via its `value: [mem_lo, 0]` memory argument.
#[allow(deprecated)]
pub fn blake2sr(index: u64, state: &mut [u64; 16], input: &[u64; 16]) {
    let mut v = [0u32; 16];
    let mut m = [0u32; 16];
    for i in 0..16 {
        debug_assert_eq!(state[i] >> 32, 0, "blake2s state word {i} is not 32-bit");
        debug_assert_eq!(input[i] >> 32, 0, "blake2s input word {i} is not 32-bit");
        v[i] = state[i] as u32;
        m[i] = input[i] as u32;
    }
    // Check the u64 before narrowing: `index as u32` would turn 2^32 into a
    // valid-looking round 0, which executes and only fails later at the trace
    // gate. BLAKE2S_ROUNDS is the same bound the AIR's one-hot round_idx has.
    assert!(
        index < BLAKE2S_ROUNDS as u64,
        "blake2s round index {index} exceeds SIGMA ({BLAKE2S_ROUNDS}); reduce before calling"
    );
    blake2s_round(&mut v, &m, index as u32);
    for i in 0..16 {
        state[i] = v[i] as u64;
    }
}
