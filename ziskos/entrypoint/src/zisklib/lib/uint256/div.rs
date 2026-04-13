use crate::syscalls::{
    syscall_add256, syscall_arith256, syscall_arith256_mod, SyscallAdd256Params,
    SyscallArith256ModParams, SyscallArith256Params,
};
use crate::zisklib::fcall_uint256_div;
use crate::zisklib::lib::{
    constants::{MINUS_ONE_256 as MINUS_ONE, ONE_256 as ONE, ZERO_256 as ZERO},
    utils::{is_zero, lt},
};

/// Given 256-bit integers `a,b`, it computes `a / b`.
/// Returns `None` if `b == 0`.
pub fn checked_div256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    if is_zero(b) {
        None
    } else {
        Some(wrapping_div256(
            a,
            b,
            #[cfg(feature = "hints")]
            hints,
        ))
    }
}

/// Given 256-bit integers `a,b`, it computes `a % b`.
/// Returns `None` if `b == 0`.
pub fn checked_rem256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    if is_zero(b) {
        None
    } else {
        Some(wrapping_rem256(
            a,
            b,
            #[cfg(feature = "hints")]
            hints,
        ))
    }
}

/// Given 256-bit integers `a,b`, it computes `a / b` and `a % b`.
///
/// Panics if `b == 0`.
pub fn div_rem256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], [u64; 4]) {
    // Strategy: Hint the division result and then verify it satisfies Euclid's division lemma

    // Hint the quotient and remainder
    let (quo, rem) = fcall_uint256_div(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check the division lemma: a = q·b + r
    let mut dl = ZERO;
    let mut dh = ZERO;
    let mut params = SyscallArith256Params { a: &quo, b, c: &rem, dl: &mut dl, dh: &mut dh };
    syscall_arith256(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert_eq!(dl, *a);
    assert_eq!(dh, ZERO);

    // Check that: r < b
    assert!(lt(&rem, b), "Remainder must be less than the divisor");

    (quo, rem)
}

/// Given 256-bit integers `a,b`, it computes the ceiling of `a / b`.
///
/// Panics if `b == 0`.
pub fn div_ceil256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    let (quo, rem) = div_rem256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );
    if is_zero(&rem) {
        quo
    } else {
        let mut res = ZERO;
        let mut params = SyscallAdd256Params { a: &quo, b: &ZERO, cin: 1, c: &mut res };
        syscall_add256(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        res
    }
}

/// Given 256-bit integers `a,b`, it computes `a / b`.
///
/// Panics if `b == 0`.
pub fn wrapping_div256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    div_rem256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    )
    .0
}

/// Given 256-bit integers `a,b`, it computes `a % b`.
///
/// Panics if `b == 0`.
pub fn wrapping_rem256(
    a: &[u64; 4],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if is_zero(b) {
        panic!("Division by zero");
    }

    let mut d = ZERO;
    let mut params = SyscallArith256ModParams { a, b: &ONE, c: &ZERO, module: b, d: &mut d };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    d
}

// ==================== C FFI Functions ====================

/// 256-bit checked division. Writes the result and returns 1 if division succeeded (b != 0), 0 if b == 0.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_div256_c")]
pub unsafe extern "C" fn checked_div256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    match checked_div256(
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
        None => 0, // Division by zero
    }
}

/// 256-bit checked remainder. Writes the result and returns 1 if remainder succeeded (b != 0), 0 if b == 0.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_checked_rem256_c")]
pub unsafe extern "C" fn checked_rem256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    match checked_rem256(
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
        None => 0, // Division by zero
    }
}

/// 256-bit division returning both quotient and remainder.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `quo_ptr` must point to a valid `[u64; 4]` array
/// - `rem_ptr` must point to a valid `[u64; 4]` array
///
/// Panics if `b == 0`.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_div_rem256_c")]
pub unsafe extern "C" fn div_rem256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    quo_ptr: *mut u64,
    rem_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let (q, r) = div_rem256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );

    let quo = &mut *(quo_ptr as *mut [u64; 4]);
    *quo = q;
    let rem = &mut *(rem_ptr as *mut [u64; 4]);
    *rem = r;
}

/// 256-bit ceiling division.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
///
/// Panics if `b == 0`.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_div_ceil256_c")]
pub unsafe extern "C" fn div_ceil256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = div_ceil256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// 256-bit division.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
///
/// Panics if `b == 0`.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_div256_c")]
pub unsafe extern "C" fn wrapping_div256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = wrapping_div256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// 256-bit remainder.
///
/// # Safety
/// - `a_ptr` must point to a valid `[u64; 4]` array
/// - `b_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a valid `[u64; 4]` array
///
/// Panics if `b == 0`.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_wrapping_rem256_c")]
pub unsafe extern "C" fn wrapping_rem256_c(
    a_ptr: *const u64,
    b_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a = &*(a_ptr as *const [u64; 4]);
    let b = &*(b_ptr as *const [u64; 4]);

    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = wrapping_rem256(
        a,
        b,
        #[cfg(feature = "hints")]
        hints,
    );
}
