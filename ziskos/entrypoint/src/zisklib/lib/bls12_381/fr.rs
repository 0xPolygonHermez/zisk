//! Finite field Fr operations for BLS12-381

use crate::arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams};

use super::constants::{R, R_MINUS_ONE};

/// Addition in Fr
#[inline]
pub fn add_fr_bls12_381(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Doubling in Fr
#[inline]
pub fn dbl_fr_bls12_381(x: &[u64; 4]) -> [u64; 4] {
    // 2·x + 0 or x·1 + x
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &[2, 0, 0, 0],
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Subtraction in Fr
#[inline]
pub fn sub_fr_bls12_381(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // y·(-1) + x
    let mut params =
        SyscallArith256ModParams { a: y, b: &R_MINUS_ONE, c: x, module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Negation in Fr
#[inline]
pub fn neg_fr_bls12_381(x: &[u64; 4]) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &R_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &R,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Multiplication in Fr
#[inline]
pub fn mul_fr_bls12_381(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Squaring in Fr
#[inline]
pub fn square_fr_bls12_381(x: &[u64; 4]) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &R, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

// ========== Pointer-based API ==========

/// # Safety
///
/// Addition in Fr
#[inline]
pub unsafe fn add_fr_bls12_381_ptr(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 4);
    let b_in = core::slice::from_raw_parts(b, 4);

    let result = add_fr_bls12_381(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 4);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Doubling in Fr
#[inline]
pub unsafe fn dbl_fr_bls12_381_ptr(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 4);

    let result = dbl_fr_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 4);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Subtraction in Fr
#[inline]
pub unsafe fn sub_fr_bls12_381_ptr(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 4);
    let b_in = core::slice::from_raw_parts(b, 4);

    let result = sub_fr_bls12_381(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 4);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Negation in Fr
#[inline]
pub unsafe fn neg_fr_bls12_381_ptr(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 4);

    let result = neg_fr_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 4);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Multiplication in Fr
#[inline]
pub unsafe fn mul_fr_bls12_381_ptr(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 4);
    let b_in = core::slice::from_raw_parts(b, 4);

    let result = mul_fr_bls12_381(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 4);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Squaring in Fr
#[inline]
pub unsafe fn square_fr_bls12_381_ptr(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 4);

    let result = square_fr_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 4);
    out.copy_from_slice(&result);
}
