//! Finite field Fr operations for BLS12-381

use crate::syscalls::{syscall_arith256_mod, SyscallArith256ModParams};

use super::constants::{R, R_MINUS_ONE};

/// Addition in Fr
#[inline]
pub fn add_fr_bls12_381(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Doubling in Fr
#[inline]
pub fn dbl_fr_bls12_381(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // 2·x + 0 or x·1 + x
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[2, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Subtraction in Fr
#[inline]
pub fn sub_fr_bls12_381(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // y·(-1) + x
    let mut params =
        SyscallArith256ModParams { a: y, b: &R_MINUS_ONE, c: x, module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Negation in Fr
#[inline]
pub fn neg_fr_bls12_381(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &R_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Multiplication in Fr
#[inline]
pub fn mul_fr_bls12_381(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// Squaring in Fr
#[inline]
pub fn square_fr_bls12_381(
    x: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

// ========== Pointer-based API ==========

/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes), used as both input and output.
/// - `b` must point to a valid `[u64; 4]` (32 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_add_fr_bls12_381_c")]
pub unsafe extern "C" fn add_fr_bls12_381_c(
    a: *mut u64,
    b: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);
    let b_ref = &*(b as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: a_ref,
        b: &[1, 0, 0, 0],
        c: b_ref,
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 4);
}

/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes), used as both input and output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_dbl_fr_bls12_381_c")]
pub unsafe extern "C" fn dbl_fr_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: a_ref,
        b: &[2, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 4);
}

/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes), used as both input and output.
/// - `b` must point to a valid `[u64; 4]` (32 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_sub_fr_bls12_381_c")]
pub unsafe extern "C" fn sub_fr_bls12_381_c(
    a: *mut u64,
    b: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);
    let b_ref = &*(b as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: b_ref,
        b: &R_MINUS_ONE,
        c: a_ref,
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 4);
}

/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes), used as both input and output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_neg_fr_bls12_381_c")]
pub unsafe extern "C" fn neg_fr_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: a_ref,
        b: &R_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 4);
}

/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes), used as both input and output.
/// - `b` must point to a valid `[u64; 4]` (32 bytes).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_mul_fr_bls12_381_c")]
pub unsafe extern "C" fn mul_fr_bls12_381_c(
    a: *mut u64,
    b: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);
    let b_ref = &*(b as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: a_ref,
        b: b_ref,
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 4);
}

/// # Safety
/// - `a` must point to a valid `[u64; 4]` (32 bytes), used as both input and output.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_square_fr_bls12_381_c")]
pub unsafe extern "C" fn square_fr_bls12_381_c(
    a: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let a_ref = &*(a as *const [u64; 4]);

    let mut params = SyscallArith256ModParams {
        a: a_ref,
        b: a_ref,
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );

    core::ptr::copy_nonoverlapping(params.d.as_ptr(), a, 4);
}
