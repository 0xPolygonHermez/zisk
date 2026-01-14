//! Finite field Fp operations for BLS12-381

use crate::{
    syscalls::{syscall_arith384_mod, SyscallArith384ModParams},
    zisklib::{eq, fcall_bls12_381_fp_inv, fcall_bls12_381_fp_sqrt},
};

use super::constants::{NQR_FP, P, P_MINUS_ONE};

/// Addition in Fp
#[inline]
pub fn add_fp_bls12_381(
    x: &[u64; 6],
    y: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    // x·1 + y
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &[1, 0, 0, 0, 0, 0],
        c: y,
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Doubling in Fp
#[inline]
pub fn dbl_fp_bls12_381(x: &[u64; 6], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 6] {
    // 2·x + 0 or x·1 + x
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &[2, 0, 0, 0, 0, 0],
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Subtraction in Fp
#[inline]
pub fn sub_fp_bls12_381(
    x: &[u64; 6],
    y: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    // y·(-1) + x
    let mut params = SyscallArith384ModParams {
        a: y,
        b: &P_MINUS_ONE,
        c: x,
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Negation in Fp
#[inline]
pub fn neg_fp_bls12_381(x: &[u64; 6], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 6] {
    // x·(-1) + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Multiplication in Fp
#[inline]
pub fn mul_fp_bls12_381(
    x: &[u64; 6],
    y: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    // x·y + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: y,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Squaring in Fp
#[inline]
pub fn square_fp_bls12_381(
    x: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 6] {
    // x·x + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: x,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Square root in Fp
#[inline]
pub fn sqrt_fp_bls12_381(
    x: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 6], bool) {
    // Hint the sqrt
    let hint = fcall_bls12_381_fp_sqrt(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let is_qr = hint[0] == 1;
    let sqrt = hint[1..7].try_into().unwrap();

    // Compute sqrt * sqrt
    let mut params = SyscallArith384ModParams {
        a: &sqrt,
        b: &sqrt,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    if is_qr {
        // Check that sqrt * sqrt == x
        assert_eq!(*params.d, *x);
        (sqrt, true)
    } else {
        // Check that sqrt * sqrt == x * NQR
        let nqr = mul_fp_bls12_381(
            x,
            &NQR_FP,
            #[cfg(feature = "hints")]
            hints,
        );
        assert_eq!(*params.d, nqr);
        (sqrt, false)
    }
}

/// Inversion of a non-zero element in Fp
#[inline]
pub fn inv_fp_bls12_381(x: &[u64; 6], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 6] {
    // if x == 0, return 0
    if eq(x, &[0; 6]) {
        return *x;
    }

    // if x != 0, return 1 / x

    // Remember that an element y ∈ Fp is the inverse of x ∈ Fp if and only if x·y = 1 in Fp
    // We will therefore hint the inverse y and check the product with x is 1
    let inv = fcall_bls12_381_fp_inv(
        x,
        #[cfg(feature = "hints")]
        hints,
    );

    // x·y + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &inv,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    assert_eq!(*params.d, [1, 0, 0, 0, 0, 0]);

    inv
}

// ========== Pointer-based API ==========

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes).
/// - `b` must point to a valid `[u64; 6]` (48 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_fp_bls12_381_c")]
pub unsafe extern "C" fn add_fp_bls12_381_c(
    a: *mut u64,
    b: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 6]);
    let b_ref = &*(b as *const [u64; 6]);

    let mut params = SyscallArith384ModParams {
        a: a_ref,
        b: &[1, 0, 0, 0, 0, 0],
        c: b_ref,
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 6);
}

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes), used as both input and output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_dbl_fp_bls12_381_c")]
pub unsafe extern "C" fn dbl_fp_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 6]);

    let mut params = SyscallArith384ModParams {
        a: a_ref,
        b: &[2, 0, 0, 0, 0, 0],
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 6);
}

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes), used as both input and output.
/// - `b` must point to a valid `[u64; 6]` (48 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sub_fp_bls12_381_c")]
pub unsafe extern "C" fn sub_fp_bls12_381_c(
    a: *mut u64,
    b: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 6]);
    let b_ref = &*(b as *const [u64; 6]);

    let mut params = SyscallArith384ModParams {
        a: b_ref,
        b: &P_MINUS_ONE,
        c: a_ref,
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 6);
}

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes), used as both input and output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_neg_fp_bls12_381_c")]
pub unsafe extern "C" fn neg_fp_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 6]);

    let mut params = SyscallArith384ModParams {
        a: a_ref,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 6);
}

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes), used as both input and output.
/// - `b` must point to a valid `[u64; 6]` (48 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mul_fp_bls12_381_c")]
pub unsafe extern "C" fn mul_fp_bls12_381_c(
    a: *mut u64,
    b: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 6]);
    let b_ref = &*(b as *const [u64; 6]);

    let mut params = SyscallArith384ModParams {
        a: a_ref,
        b: b_ref,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 6);
}

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes), used as both input and output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_square_fp_bls12_381_c")]
pub unsafe extern "C" fn square_fp_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 6]);

    let mut params = SyscallArith384ModParams {
        a: a_ref,
        b: a_ref,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 6);
}

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes), used as both input and output.
/// - `is_qr` must point to a valid `u8`.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sqrt_fp_bls12_381_c")]
pub unsafe extern "C" fn sqrt_fp_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let a_ref = &*(a as *const [u64; 6]);
    let (result, qr) = sqrt_fp_bls12_381(
        a_ref,
        #[cfg(feature = "hints")]
        hints,
    );
    *(a as *mut [u64; 6]) = result;
    qr
}

/// # Safety
/// - `a` must point to a valid `[u64; 6]` (48 bytes), used as both input and output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_inv_fp_bls12_381_c")]
pub unsafe extern "C" fn inv_fp_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 6]);
    let result = inv_fp_bls12_381(
        a_ref,
        #[cfg(feature = "hints")]
        hints,
    );
    *(a as *mut [u64; 6]) = result;
}
