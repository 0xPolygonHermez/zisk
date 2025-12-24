use crate::syscalls::{syscall_sha256_f, SyscallSha256Params};

/// C-compatible wrapper for sha256f_compress
///
/// # Safety
/// - `state_ptr` must point to at least 8 u32s (will be read and written)
/// - `blocks_ptr` must point to at least `num_blocks * 64` bytes
#[no_mangle]
pub unsafe extern "C" fn sha256f_compress_c(
    state_ptr: *mut u32,
    blocks_ptr: *const u8,
    num_blocks: usize,
) {
    let state_64: &mut [u64; 4] = &mut *(state_ptr as *mut [u64; 4]);

    for i in 0..num_blocks {
        let block: &[u8; 64] = &*(blocks_ptr.add(i * 64) as *const [u8; 64]);
        let input_u64 = convert_bytes_to_u64(block);
        let mut sha256_params = SyscallSha256Params { state: state_64, input: &input_u64 };
        syscall_sha256_f(&mut sha256_params);
    }
}

#[inline(always)]
fn convert_bytes_to_u64(input: &[u8; 64]) -> [u64; 8] {
    [
        u64::from_be_bytes(input[0..8].try_into().unwrap()),
        u64::from_be_bytes(input[8..16].try_into().unwrap()),
        u64::from_be_bytes(input[16..24].try_into().unwrap()),
        u64::from_be_bytes(input[24..32].try_into().unwrap()),
        u64::from_be_bytes(input[32..40].try_into().unwrap()),
        u64::from_be_bytes(input[40..48].try_into().unwrap()),
        u64::from_be_bytes(input[48..56].try_into().unwrap()),
        u64::from_be_bytes(input[56..64].try_into().unwrap()),
    ]
}
