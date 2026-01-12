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
        let input_u64: &[u64; 8] = &*(blocks_ptr.add(i * 64) as *const [u64; 8]);
        let mut sha256_params = SyscallSha256Params { state: state_64, input: input_u64 };
        syscall_sha256_f(&mut sha256_params);
    }
}
