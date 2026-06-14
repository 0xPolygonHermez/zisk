//! Operations in the base field Fp of the secp256k1 curve

use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::{eq, fcall_secp256k1_fp_inv, fcall_secp256k1_fp_sqrt, is_one, is_zero, lt},
};

use super::constants::{NQR, P, P_MINUS_ONE};

/// Reduction in the base field of the secp256k1 curve
pub fn reduce_fp_secp256k1(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    if lt(x, &P) {
        return *x;
    }

    // x mod P
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[1, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Addition in the base field of the secp256k1 curve
pub fn add_fp_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Negation in the base field of the secp256k1 curve
pub fn neg_fp_secp256k1(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Subtraction in the base field of the secp256k1 curve
pub fn sub_fp_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // y·(-1) + x
    let mut params =
        SyscallArith256ModParams { a: y, b: &P_MINUS_ONE, c: x, module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Multiplication in the base field of the secp256k1 curve
pub fn mul_fp_secp256k1(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Squaring in the base field of the secp256k1 curve
pub fn square_fp_secp256k1(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Square root in the base field of the secp256k1 curve.
///
/// Returns `(sqrt, is_qr)` where `is_qr` indicates whether `x` is a quadratic residue.
/// When `is_qr` is true, `sqrt` satisfies `sqrt² ≡ x (mod P)` with the given parity.
/// When `is_qr` is false, `sqrt` satisfies `sqrt² ≡ x·NQR (mod P)`.
pub fn sqrt_fp_secp256k1(
    x: &[u64; 4],
    parity: u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 4], bool) {
    // Hint the sqrt
    let hint = fcall_secp256k1_fp_sqrt(
        x,
        parity,
        #[cfg(feature = "hints")]
        hints,
    );
    let is_qr = hint[0] == 1;
    let sqrt: [u64; 4] = hint[1..5].try_into().unwrap();

    // Check the sqrt is canonical
    assert!(lt(&sqrt, &P), "Square root is not canonical");

    // Compute sqrt * sqrt
    let mut params = SyscallArith256ModParams {
        a: &sqrt,
        b: &sqrt,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    if is_qr {
        // Check that sqrt * sqrt == x
        assert!(eq(params.d, x), "Square root verification failed");
        (sqrt, true)
    } else {
        // Check that sqrt * sqrt == x * NQR
        let nqr = mul_fp_secp256k1(
            x,
            &NQR,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(eq(params.d, &nqr), "Square root verification failed");
        (sqrt, false)
    }
}

/// Inversion in the base field of the secp256k1 curve
pub fn inv_fp_secp256k1(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // if x == 0, return 0
    if is_zero(x) {
        return *x;
    }

    // if x != 0, return 1 / x

    // Remember that an element y ∈ Fp is the inverse of x ∈ Fp if and only if x·y = 1 in Fp
    // We will therefore hint the inverse y and check the product with x is 1
    let inv = fcall_secp256k1_fp_inv(
        x,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check the inverse is canonical
    assert!(lt(&inv, &P));

    // x·y + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &inv,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert!(is_one(params.d), "Inverse verification failed");

    inv
}

// ==================== C FFI Functions ====================

/// Reduction modulo Fp of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_reduce_fp_secp256k1_c")]
pub unsafe extern "C" fn reduce_fp_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = reduce_fp_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Addition in Fp of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `y_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_fp_secp256k1_c")]
pub unsafe extern "C" fn add_fp_secp256k1_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let y = &*(y_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = add_fp_secp256k1(
        x,
        y,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Negation in Fp of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_neg_fp_secp256k1_c")]
pub unsafe extern "C" fn neg_fp_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = neg_fp_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Subtraction in Fp of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `y_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sub_fp_secp256k1_c")]
pub unsafe extern "C" fn sub_fp_secp256k1_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let y = &*(y_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = sub_fp_secp256k1(
        x,
        y,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Multiplication in Fp of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `y_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mul_fp_secp256k1_c")]
pub unsafe extern "C" fn mul_fp_secp256k1_c(
    x_ptr: *const u64,
    y_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let y = &*(y_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = mul_fp_secp256k1(
        x,
        y,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Squaring in Fp of the secp256k1 curve.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_square_fp_secp256k1_c")]
pub unsafe extern "C" fn square_fp_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = square_fp_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Square root in Fp of the secp256k1 curve. Writes the square root to `result_ptr`.
/// Returns 1 if `x` is a quadratic residue, 0 otherwise.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sqrt_fp_secp256k1_c")]
pub unsafe extern "C" fn sqrt_fp_secp256k1_c(
    x_ptr: *const u64,
    parity: u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    let (sqrt, is_qr) = sqrt_fp_secp256k1(
        x,
        parity,
        #[cfg(feature = "hints")]
        hints,
    );
    *result = sqrt;
    is_qr as u8
}

/// Inversion in Fp of the secp256k1 curve. Returns 0 for input 0.
///
/// # Safety
/// - `x_ptr` must point to a valid `[u64; 4]` array
/// - `result_ptr` must point to a writable `[u64; 4]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_inv_fp_secp256k1_c")]
pub unsafe extern "C" fn inv_fp_secp256k1_c(
    x_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let x = &*(x_ptr as *const [u64; 4]);
    let result = &mut *(result_ptr as *mut [u64; 4]);
    *result = inv_fp_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
}
