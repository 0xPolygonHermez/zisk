use crate::syscalls::{syscall_arith256, SyscallArith256Params};
use crate::zisklib::fcall_uint256_inv;
use crate::zisklib::lib::{
    constants::{MAX_256 as MAX, ONE_256 as ONE, ZERO_256 as ZERO},
    utils::is_one,
};

/// Given 256-bit integers `a,b`, it computes `a * b (mod 2^256)`.
/// Returns `None` if overflow occurs.
pub fn checked_mul256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    match overflowing_mul256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => Some(res),
        _ => None,
    }
}

/// Given 256-bit integers `a,b`, it computes `a^2 (mod 2^256)`.
/// Returns `None` if overflow occurs.
pub fn checked_square256(
    a: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    match overflowing_square256(
        a,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => Some(res),
        _ => None,
    }
}

/// Given 256-bit integers `a,b`, it computes `a * b (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates overflow (high 256 bits are non-zero).
pub fn overflowing_mul256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    let mut dl = ZERO;
    let mut dh = ZERO;
    let mut params = SyscallArith256Params { a, b, c: &ZERO, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (dl, dh != ZERO)
}

/// Given 256-bit integers `a,b`, it computes `a^2 (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates overflow (high 256 bits are non-zero).
pub fn overflowing_square256(
    a: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    let mut dl = ZERO;
    let mut dh = ZERO;
    let mut params = SyscallArith256Params { a, b: a, c: &ZERO, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (dl, dh != ZERO)
}

/// Given 256-bit integers `a,b`, it computes `a * b (mod 2^256)`.
/// Saturates to the maximum value if overflow occurs.
pub fn saturating_mul256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    match overflowing_mul256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => res,
        _ => MAX,
    }
}

/// Given 256-bit integers `a,b`, it computes `a^2 (mod 2^256)`.
/// Saturates to the maximum value if overflow occurs.
pub fn saturating_square256(
    a: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    match overflowing_square256(
        a,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => res,
        _ => MAX,
    }
}

/// Given 256-bit integers `a,b`, it computes `a * b (mod 2^256)`.
pub fn wrapping_mul256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    overflowing_mul256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given a 256-bit integer `a`, it computes `a^2 (mod 2^256)`.
pub fn wrapping_square256(
    a: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    overflowing_mul256(
        a,
        a,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given a 256-bit integer `a`, it computes `a^(-1) (mod 2^256)`, if it exists.
/// Returns `None` if `a` is not invertible.
pub fn inv256(a: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> Option<[u64; 4]> {
    // Hint the inverse
    match fcall_uint256_inv(
        a,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(inv) => {
            // Verify: a * inv ≡ 1 (mod 2^256)
            let result = wrapping_mul256(
                a,
                &inv,
                #[cfg(feature = "hints")]
                hints,
            );
            assert!(is_one(&result), "Hinted inverse is incorrect");

            Some(inv)
        }
        None => {
            // Modulo 2^256, an element is invertible iff it is odd
            // Therefore a non-invertible element must be even
            assert!(a[0] & 1 == 0, "a must be even if it is not invertible");
            None
        }
    }
}

// ==================== C FFI Functions ====================

/// 256-bit checked multiplication. Returns 1 if successful, 0 if overflow occurs.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_mul256_c")]
pub unsafe extern "C" fn checked_mul256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    match checked_mul256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(res) => {
            let result = &mut *(result_ptr as *mut [u64; 4]);
            *result = res;
            1 // Success
        }
        None => 0, // Overflow
    }
}

/// 256-bit checked square. Returns 1 if successful, 0 if overflow occurs.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_square256_c")]
pub unsafe extern "C" fn checked_square256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);

    match checked_square256(
        a,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(res) => {
            let result = &mut *(result_ptr as *mut [u64; 4]);
            *result = res;
            1 // Success
        }
        None => 0, // Overflow
    }
}

/// 256-bit overflowing multiplication. Returns 1 if overflow occurred, 0 otherwise.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_overflowing_mul256_c")]
pub unsafe extern "C" fn overflowing_mul256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let (res, overflow) = overflowing_mul256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;

    overflow as u8
}

/// 256-bit overflowing square. Returns 1 if overflow occurred, 0 otherwise.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_overflowing_square256_c")]
pub unsafe extern "C" fn overflowing_square256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);

    let (res, overflow) = overflowing_square256(
        a,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;

    overflow as u8
}

/// 256-bit saturating multiplication.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_saturating_mul256_c")]
pub unsafe extern "C" fn saturating_mul256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = saturating_mul256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// 256-bit saturating square.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_saturating_square256_c")]
pub unsafe extern "C" fn saturating_square256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = saturating_square256(
        a,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// 256-bit wrapping multiplication.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_mul256_c")]
pub unsafe extern "C" fn wrapping_mul256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = wrapping_mul256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// 256-bit wrapping squaring.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_square256_c")]
pub unsafe extern "C" fn wrapping_square256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = wrapping_square256(
        a,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// 256-bit modular inverse (mod 2^256). Returns 1 if inverse exists, 0 otherwise.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_inv256_c")]
pub unsafe extern "C" fn inv256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);

    match inv256(
        a,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Some(res) => {
            let result = &mut *(result_ptr as *mut [u64; 4]);
            *result = res;
            1
        }
        None => 0,
    }
}
