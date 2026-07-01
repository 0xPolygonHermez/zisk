use crate::syscalls::{syscall_add256, SyscallAdd256Params};
use crate::zisklib::lib::{
    constants::{MAX_256 as MAX, MINUS_ONE_256 as MINUS_ONE, ONE_256 as ONE, ZERO_256 as ZERO},
    utils::{be_bytes_to_u64_4, u64_4_to_be_bytes},
};

/// Given 256-bit integers `a,b`, it computes `a + b (mod 2^256)`.
/// Returns `None` if overflow occurs.
pub fn checked_add256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    match overflowing_add256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => Some(res),
        _ => None,
    }
}

/// Given 256-bit integers `a,b`, it computes `-a (mod 2^256)`.
/// Returns `None` unless `a == 0`.
pub fn checked_neg256(
    a: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    match overflowing_neg256(
        a,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => Some(res),
        _ => None,
    }
}

/// Given 256-bit integers `a,b`, it computes `a - b (mod 2^256)`.
/// Returns `None` if underflow occurs (a < b).
pub fn checked_sub256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    match overflowing_sub256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => Some(res),
        _ => None,
    }
}

/// Given 256-bit integers `a,b`, it computes `a + b (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates overflow.
pub fn overflowing_add256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    let mut c = ZERO;
    let mut params = SyscallAdd256Params { a, b, cin: 0, c: &mut c };
    let cout = syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (c, cout != 0)
}

/// Given 256-bit integers `a,b`, it computes `-a (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates overflow.
pub fn overflowing_neg256(
    a: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    overflowing_sub256(
        &ZERO,
        a,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Given 256-bit integers `a,b`, it computes `a - b (mod 2^256)`.
/// Returns `(c, flag)` where `c` is the result and `flag` indicates underflow (a < b).
pub fn overflowing_sub256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    // a - b = a + (~b + 1); cout=0 means borrow (a < b)
    let not_b = [!b[0], !b[1], !b[2], !b[3]];
    let mut c = ZERO;
    let mut params = SyscallAdd256Params { a, b: &not_b, cin: 1, c: &mut c };
    let cout = syscall_add256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    (c, cout == 0)
}

/// Given 256-bit integers `a,b`, it computes `a + b (mod 2^256)`.
/// Saturates at the numeric bounds on overflow.
pub fn saturating_add256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    match overflowing_add256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => res,
        _ => MAX,
    }
}

/// Given 256-bit integers `a,b`, it computes `a - b (mod 2^256)`.
/// Saturates at the numeric bounds on underflow.
pub fn saturating_sub256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    match overflowing_sub256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    ) {
        (res, false) => res,
        _ => ZERO,
    }
}

/// Given 256-bit integers `a,b`, it computes `a + b (mod 2^256)`.
pub fn wrapping_add256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    overflowing_add256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given a 256-bit integer `a`, it computes `-a (mod 2^256)`.
pub fn wrapping_neg256(a: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    overflowing_sub256(
        &ZERO,
        a,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given 256-bit integers `a,b`, it computes `a - b (mod 2^256)`.
pub fn wrapping_sub256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    overflowing_sub256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

// ==================== C FFI Functions ====================

/// 256-bit checked addition. Writes the result and returns 1 on success, 0 on overflow.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_add256_c")]
pub unsafe extern "C" fn checked_add256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    match checked_add256(
        a,
        b,
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

/// 256-bit checked negation. Writes the result and returns 1 on success, 0 on overflow.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_neg256_c")]
pub unsafe extern "C" fn checked_neg256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);

    match checked_neg256(
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

/// 256-bit checked subtraction. Writes the result and returns 1 on success, 0 on underflow.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_sub256_c")]
pub unsafe extern "C" fn checked_sub256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    match checked_sub256(
        a,
        b,
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

/// 256-bit overflowing addition. Writes the result and returns 1 if overflow occurred, 0 otherwise.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_overflowing_add256_c")]
pub unsafe extern "C" fn overflowing_add256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let (res, overflow) = overflowing_add256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;

    overflow as u8
}

/// 256-bit overflowing negation. Writes the result and returns 1 if overflow occurred, 0 otherwise.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_overflowing_neg256_c")]
pub unsafe extern "C" fn overflowing_neg256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);

    let (res, overflow) = overflowing_neg256(
        a,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;

    overflow as u8
}

/// 256-bit overflowing subtraction. Writes the result and returns 1 if overflow occurred, 0 otherwise.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_overflowing_sub256_c")]
pub unsafe extern "C" fn overflowing_sub256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let (res, overflow) = overflowing_sub256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;

    overflow as u8
}

/// 256-bit saturating addition.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_saturating_add256_c")]
pub unsafe extern "C" fn saturating_add256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let res = saturating_add256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit saturating subtraction.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_saturating_sub256_c")]
pub unsafe extern "C" fn saturating_sub256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let res = saturating_sub256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit wrapping addition.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_add256_c")]
pub unsafe extern "C" fn wrapping_add256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let res = wrapping_add256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit wrapping subtraction.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_sub256_c")]
pub unsafe extern "C" fn wrapping_sub256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let res = wrapping_sub256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}

/// 256-bit wrapping negation.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_neg256_c")]
pub unsafe extern "C" fn wrapping_neg256_c(
    a_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);

    let res = wrapping_neg256(
        a,
        #[cfg(feature = "hints")]
        hints,
    );

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = res;
}
