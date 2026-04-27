//! Operations in the scalar field Fn of the secp256k1 curve

use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::{fcall_secp256k1_fn_inv, is_zero, lt},
};

use super::constants::{N, N_MINUS_ONE};

/// Reduces a 256-bit value modulo the secp256k1 curve order N
pub fn reduce_fn_secp256k1(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if lt(x, &N) {
        return *x;
    }

    // x·1 + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Addition in the scalar field of the secp256k1 curve
pub fn add_fn_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Negation in the scalar field of the secp256k1 curve
pub fn neg_fn_secp256k1(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &N_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Subtraction in the scalar field of the secp256k1 curve
pub fn sub_fn_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // y·(-1) + x
    let mut params =
        SyscallArith256ModParams { a: y, b: &N_MINUS_ONE, c: x, module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Multiplication in the scalar field of the secp256k1 curve
pub fn mul_fn_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Squaring in the scalar field of the secp256k1 curve
pub fn square_fn_secp256k1(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &N, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    *params.d
}

/// Inversion in the scalar field of the secp256k1 curve
pub fn inv_fn_secp256k1(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // if x == 0, return 0
    if is_zero(x) {
        return *x;
    }

    // if x != 0, return 1 / x

    // Remember that an element y is the inverse of x if and only if x·y = 1
    // We will therefore hint the inverse y and check the product with x is 1
    let inv = fcall_secp256k1_fn_inv(
        x,
        #[cfg(feature = "hints")]
        hints,
    );

    // x·y + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &inv,
        c: &[0, 0, 0, 0],
        module: &N,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert_eq!(*params.d, [1, 0, 0, 0]);

    inv
}

// ==================== C FFI Functions ====================

/// Reduction modulo Fn of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_reduce_fn_secp256k1_c")]
pub unsafe extern "C" fn reduce_fn_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = reduce_fn_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Addition in Fn of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `y_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_fn_secp256k1_c")]
pub unsafe extern "C" fn add_fn_secp256k1_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let y = &*(y_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = add_fn_secp256k1(
        x,
        y,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Negation in Fn of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_neg_fn_secp256k1_c")]
pub unsafe extern "C" fn neg_fn_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = neg_fn_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Subtraction in Fn of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `y_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sub_fn_secp256k1_c")]
pub unsafe extern "C" fn sub_fn_secp256k1_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let y = &*(y_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = sub_fn_secp256k1(
        x,
        y,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Multiplication in Fn of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `y_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mul_fn_secp256k1_c")]
pub unsafe extern "C" fn mul_fn_secp256k1_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let y = &*(y_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = mul_fn_secp256k1(
        x,
        y,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Squaring in Fn of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_square_fn_secp256k1_c")]
pub unsafe extern "C" fn square_fn_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = square_fn_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Inversion in Fn of the secp256k1 curve. Returns 0 for input 0.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_inv_fn_secp256k1_c")]
pub unsafe extern "C" fn inv_fn_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = inv_fn_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}
