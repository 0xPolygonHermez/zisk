use super::utils::{be_bytes_to_u64_4, u64_4_to_be_bytes};
use crate::syscalls::{syscall_arith256_mod, SyscallArith256ModParams};

pub fn mulmod256(
    a: &[u64; 4],
    b: &[u64; 4],
    m: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    let mut params = SyscallArith256ModParams { a, b, c: &[0u64; 4], module: m, d: &mut [0u64; 4] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

// ========== Pointer-based API ==========

/// Modular multiplication of 256-bit integers
///
/// # Safety
/// - `a` must point to a valid array of 32 bytes (big endian)
/// - `b` must point to a valid array of 32 bytes (big endian)
/// - `m` must point to a valid array of 32 bytes (big endian)
/// - `result` must point to a valid array of at least 32 bytes
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mulmod256_c")]
pub unsafe extern "C" fn mulmod256_c(
    a_ptr: *const u8,
    b_ptr: *const u8,
    m_ptr: *const u8,
    result_ptr: *mut u8,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_bytes = std::slice::from_raw_parts(a_ptr, 32);
    let b_bytes = std::slice::from_raw_parts(b_ptr, 32);
    let m_bytes = std::slice::from_raw_parts(m_ptr, 32);

    // Convert from big-endian bytes to little-endian
    let a = be_bytes_to_u64_4(a_bytes.try_into().unwrap());
    let b = be_bytes_to_u64_4(b_bytes.try_into().unwrap());
    let m = be_bytes_to_u64_4(m_bytes.try_into().unwrap());

    let result = mulmod256(
        &a,
        &b,
        &m,
        #[cfg(feature = "hints")]
        hints,
    );

    // Convert result back to big-endian bytes
    let result_bytes = std::slice::from_raw_parts_mut(result_ptr, 32);
    let result_be = u64_4_to_be_bytes(&result);
    result_bytes.copy_from_slice(&result_be);
}
