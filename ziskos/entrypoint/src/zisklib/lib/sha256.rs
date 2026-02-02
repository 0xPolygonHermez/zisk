use crate::syscalls::{syscall_sha256_f, SyscallSha256Params};

/// SHA-256 initial hash values
const SHA256_INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

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

pub fn sha256(input: &[u8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u8; 32] {
    let mut state = SHA256_INIT;
    let input_len = input.len();

    // Process complete 64-byte blocks
    let mut offset = 0;
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

    // Handle final block(s) with padding
    let remaining = input_len - offset;
    let bit_len = (input_len as u64) * 8;

    // We need: remaining bytes + 1 (0x80) + padding + 8 (length)
    // If remaining + 9 > 64, we need 2 blocks
    let mut final_block = [0u8; 64];

    // Copy remaining bytes
    final_block[..remaining].copy_from_slice(&input[offset..]);

    // Append 0x80
    final_block[remaining] = 0x80;

    if remaining + 9 > 64 {
        // Need two blocks: process first block, then second with length
        compress_block(
            &mut state,
            &final_block,
            #[cfg(feature = "hints")]
            hints,
        );

        // Second block: all zeros except length at the end
        final_block = [0u8; 64];
        final_block[56..64].copy_from_slice(&bit_len.to_be_bytes());
        compress_block(
            &mut state,
            &final_block,
            #[cfg(feature = "hints")]
            hints,
        );
    } else {
        // Single block: append length at the end
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

/// C-compatible wrapper for full SHA-256 hash
///
/// # Safety
/// - `input` must point to at least `input_len` bytes
/// - `output` must point to a writable buffer of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sha256_c")]
pub unsafe extern "C" fn sha256_c(
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
