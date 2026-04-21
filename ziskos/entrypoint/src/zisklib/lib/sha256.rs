//! SHA2-256 hash function (FIPS 180-4).

use crate::syscalls::{syscall_sha256_f, SyscallSha256Params};

use super::is_aligned_8;

/// SHA-256 initial hash values
const SHA256_INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// SHA-256 hash function. For reference: https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf
pub fn sha256(input: &[u8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u8; 32] {
    let mut state = SHA256_INIT;
    let input_len = input.len();

    // Process complete 64-byte blocks
    let mut offset = 0;
    if is_aligned_8(input.as_ptr()) {
        // Fast path: input is aligned, use directly
        while offset + 64 <= input_len {
            let block: &[u8; 64] = input[offset..offset + 64].try_into().unwrap();
            compress_block(
                &mut state,
                block,
                #[cfg(feature = "hints")]
                hints,
            );
            offset += 64;
        }
    } else {
        // Slow path: input is unaligned, copy each block
        let mut aligned_block = [0u8; 64];
        while offset + 64 <= input_len {
            aligned_block.copy_from_slice(&input[offset..offset + 64]);
            compress_block(
                &mut state,
                &aligned_block,
                #[cfg(feature = "hints")]
                hints,
            );
            offset += 64;
        }
    }

    // Handle final block(s) with padding
    let remaining = input_len - offset;
    let bit_len = (input_len as u64) * 8;

    // We need: remaining bytes + 1 (0x80) + padding + 8 (length)
    let mut final_block = [0u8; 64];

    // Copy remaining bytes
    final_block[..remaining].copy_from_slice(&input[offset..]);

    // Append 0x80
    final_block[remaining] = 0x80;

    // If remaining + 9 > 64, we need 2 blocks
    if remaining + 9 > 64 {
        // First block
        compress_block(
            &mut state,
            &final_block,
            #[cfg(feature = "hints")]
            hints,
        );

        // Second block
        final_block = [0u8; 64];
        final_block[56..64].copy_from_slice(&bit_len.to_be_bytes());
        compress_block(
            &mut state,
            &final_block,
            #[cfg(feature = "hints")]
            hints,
        );
    } else {
        // Single final block
        final_block[56..64].copy_from_slice(&bit_len.to_be_bytes());
        compress_block(
            &mut state,
            &final_block,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // Convert state to big-endian bytes
    let mut result = [0u8; 32];
    for (i, &word) in state.iter().enumerate() {
        result[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }

    result
}

/// Compress a single 64-byte block into the state
#[inline]
fn compress_block(
    state: &mut [u32; 8],
    block: &[u8; 64],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let state_64: &mut [u64; 4] = unsafe { &mut *(state.as_mut_ptr() as *mut [u64; 4]) };
    let input_u64: &[u64; 8] = unsafe { &*(block.as_ptr() as *const [u64; 8]) };
    let mut sha256_params = SyscallSha256Params { state: state_64, input: input_u64 };
    syscall_sha256_f(
        &mut sha256_params,
        #[cfg(feature = "hints")]
        hints,
    );
}

// ==================== C FFI Functions ====================

/// SHA-256 compression function: applies `num_blocks` 512-bit blocks to the 256-bit state in-place.
///
/// # Safety
/// - `state_ptr` must point to a writable `[u32; 8]`
/// - `blocks_ptr` must point to `num_blocks * 64` readable bytes, 8-byte aligned
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sha256f_compress_c")]
pub unsafe extern "C" fn sha256f_compress_c(
    state_ptr: *mut u32,
    blocks_ptr: *const u8,
    num_blocks: usize,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let state: &mut [u32; 8] = &mut *(state_ptr as *mut [u32; 8]);
    let mut state_64 = convert_u32_to_u64(state);

    for i in 0..num_blocks {
        let block: &[u8; 64] = &*(blocks_ptr.add(i * 64) as *const [u8; 64]);
        let input_u64 = convert_bytes_to_u64(block);

        let mut sha256_params = SyscallSha256Params { state: &mut state_64, input: &input_u64 };
        syscall_sha256_f(
            &mut sha256_params,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    *state = convert_u64_to_u32(&state_64);
}

#[inline]
fn convert_u32_to_u64(state: &[u32; 8]) -> [u64; 4] {
    unsafe { *(state.as_ptr() as *const [u64; 4]) }
}

#[inline]
fn convert_u64_to_u32(state: &[u64; 4]) -> [u32; 8] {
    unsafe { *(state.as_ptr() as *const [u32; 8]) }
}

#[inline]
fn convert_bytes_to_u64(block: &[u8; 64]) -> [u64; 8] {
    unsafe { *(block.as_ptr() as *const [u64; 8]) }
}

/// C-compatible wrapper for full SHA-256 hash
///
/// # Safety
/// - `input` must point to at least `input_len` bytes
/// - `output` must point to a writable buffer of at least 32 bytes
#[inline]
pub(crate) unsafe fn sha256_c(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let input_slice = core::slice::from_raw_parts(input, input_len);
    let hash = sha256(
        input_slice,
        #[cfg(feature = "hints")]
        hints,
    );
    let output_slice = core::slice::from_raw_parts_mut(output, 32);
    output_slice.copy_from_slice(&hash);
}
