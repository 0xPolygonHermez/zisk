use crate::syscalls::syscall_keccak_f;

/// Keccak-256 rate in bytes (1600 - 2*256) / 8 = 136 bytes
const KECCAK256_RATE: usize = 136;

/// Computes the Keccak-256 hash of the input data.
///
/// This implements the Keccak sponge construction with:
/// - Rate: 1088 bits (136 bytes)
/// - Capacity: 512 bits (64 bytes)  
/// - Output: 256 bits (32 bytes)
/// - Padding: Keccak padding (0x01...0x80)
pub fn keccak256(input: &[u8]) -> [u8; 32] {
    let mut state = [0u64; 25];
    let input_len = input.len();

    // Absorb phase: process complete rate-sized blocks
    let mut offset = 0;
    while offset + KECCAK256_RATE <= input_len {
        // XOR block into state
        xor_block_into_state(&mut state, &input[offset..offset + KECCAK256_RATE]);
        // Apply Keccak-f permutation
        syscall_keccak_f(&mut state);
        offset += KECCAK256_RATE;
    }

    // Handle final block with padding
    let remaining = input_len - offset;
    let mut final_block = [0u8; KECCAK256_RATE];

    // Copy remaining bytes
    final_block[..remaining].copy_from_slice(&input[offset..]);

    // Keccak padding: append 0x01, then zeros, then 0x80 at the end of the rate
    // For Keccak-256: domain separator is 0x01
    final_block[remaining] = 0x01;
    final_block[KECCAK256_RATE - 1] |= 0x80;

    // XOR final padded block into state
    xor_block_into_state(&mut state, &final_block);

    // Final permutation
    syscall_keccak_f(&mut state);

    // Squeeze phase: extract first 32 bytes (256 bits) from state
    let mut result = [0u8; 32];
    let state_bytes: &[u8; 200] = unsafe { &*(&state as *const [u64; 25] as *const [u8; 200]) };
    result.copy_from_slice(&state_bytes[..32]);

    result
}

/// XOR a rate-sized block into the state (first 136 bytes = 17 u64 words)
#[inline]
fn xor_block_into_state(state: &mut [u64; 25], block: &[u8]) {
    // XOR block bytes into state, interpreting as little-endian u64s
    for i in 0..KECCAK256_RATE / 8 {
        let word = u64::from_le_bytes(block[i * 8..(i + 1) * 8].try_into().unwrap());
        state[i] ^= word;
    }
}

/// C-compatible wrapper for Keccak-256 hash
///
/// This is the function that `alloy-primitives` will call when the `native-keccak` feature is enabled.
///
/// # Safety
/// - `input` must point to at least `input_len` bytes
/// - `output` must point to a writable buffer of at least 32 bytes
#[no_mangle]
pub unsafe extern "C" fn keccak256_c(input: *const u8, input_len: usize, output: *mut u8) {
    let input_slice = core::slice::from_raw_parts(input, input_len);
    let hash = keccak256(input_slice);
    let output_slice = core::slice::from_raw_parts_mut(output, 32);
    output_slice.copy_from_slice(&hash);
}

/// Native keccak256 implementation for external callers
///
/// # Safety
/// - `bytes` must point to at least `len` bytes
/// - `output` must point to a writable buffer of at least 32 bytes
#[no_mangle]
pub unsafe extern "C" fn native_keccak256(bytes: *const u8, len: usize, output: *mut u8) {
    keccak256_c(bytes, len, output);
}
