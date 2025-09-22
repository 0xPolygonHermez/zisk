use crate::{
    arith384_mod::{syscall_arith384_mod, SyscallArith384ModParams},
    fcall_bls12_381_fp_inv, fcall_bls12_381_fp_sqrt,
    zisklib::lib::utils::eq,
};

use super::constants::{NQR, P, P_MINUS_ONE};

/// Addition in Fp
#[inline]
pub fn add_fp_bls12_381(x: &[u64; 6], y: &[u64; 6]) -> [u64; 6] {
    // x·1 + y
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &[1, 0, 0, 0, 0, 0],
        c: y,
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(&mut params);
    *params.d
}

/// Doubling in Fp
#[inline]
pub fn dbl_fp_bls12_381(x: &[u64; 6]) -> [u64; 6] {
    // 2·x + 0 or x·1 + x
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &[2, 0, 0, 0, 0, 0],
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(&mut params);
    *params.d
}

/// Subtraction in Fp
#[inline]
pub fn sub_fp_bls12_381(x: &[u64; 6], y: &[u64; 6]) -> [u64; 6] {
    // y·(-1) + x
    let mut params = SyscallArith384ModParams {
        a: y,
        b: &P_MINUS_ONE,
        c: x,
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(&mut params);
    *params.d
}

/// Negation in Fp
#[inline]
pub fn neg_fp_bls12_381(x: &[u64; 6]) -> [u64; 6] {
    // x·(-1) + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(&mut params);
    *params.d
}

/// Multiplication in Fp
#[inline]
pub fn mul_fp_bls12_381(x: &[u64; 6], y: &[u64; 6]) -> [u64; 6] {
    // x·y + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: y,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(&mut params);
    *params.d
}

/// Squaring in Fp
#[inline]
pub fn square_fp_bls12_381(x: &[u64; 6]) -> [u64; 6] {
    // x·x + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: x,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(&mut params);
    *params.d
}

/// Square root in Fp
#[inline]
pub fn sqrt_fp_bls12_381(x: &[u64; 6]) -> ([u64; 6], bool) {
    // Hint the sqrt
    let hint = fcall_bls12_381_fp_sqrt(x);
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
    syscall_arith384_mod(&mut params);

    if is_qr {
        // Check that sqrt * sqrt == x
        assert_eq!(*params.d, *x);
        (sqrt, true)
    } else {
        // Check that sqrt * sqrt == x * NQR
        let nqr = mul_fp_bls12_381(x, &NQR);
        assert_eq!(*params.d, nqr);
        (sqrt, false)
    }
}

/// Inversion of a non-zero element in Fp
#[inline]
pub fn inv_fp_bls12_381(x: &[u64; 6]) -> [u64; 6] {
    // if x == 0, return 0
    if eq(x, &[0; 6]) {
        return *x;
    }

    // if x != 0, return 1 / x

    // Remember that an element y ∈ Fp is the inverse of x ∈ Fp if and only if x·y = 1 in Fp
    // We will therefore hint the inverse y and check the product with x is 1
    let inv = fcall_bls12_381_fp_inv(x);

    // x·y + 0
    let mut params = SyscallArith384ModParams {
        a: x,
        b: &inv,
        c: &[0, 0, 0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0, 0, 0],
    };
    syscall_arith384_mod(&mut params);
    assert_eq!(*params.d, [1, 0, 0, 0, 0, 0]);

    inv
}

// ========== Pointer-based API ==========

/// # Safety
///
/// Addition in Fp
#[inline]
pub unsafe fn add_fp_bls12_381_ptr(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);
    let b_in = core::slice::from_raw_parts(b, 6);

    let result = add_fp_bls12_381(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Doubling in Fp
#[inline]
pub unsafe fn dbl_fp_bls12_381_ptr(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);

    let result = dbl_fp_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Subtraction in Fp
#[inline]
pub unsafe fn sub_fp_bls12_381_ptr(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);
    let b_in = core::slice::from_raw_parts(b, 6);

    let result = sub_fp_bls12_381(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Negation in Fp
#[inline]
pub unsafe fn neg_fp_bls12_381_ptr(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);

    let result = neg_fp_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Multiplication in Fp
#[inline]
pub unsafe fn mul_fp_bls12_381_ptr(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);
    let b_in = core::slice::from_raw_parts(b, 6);

    let result = mul_fp_bls12_381(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Squaring in Fp
#[inline]
pub unsafe fn square_fp_bls12_381_ptr(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);

    let result = square_fp_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
}

/// # Safety
///
/// Square root in Fp
#[inline]
pub unsafe fn sqrt_fp_bls12_381_ptr(a: *mut u64, is_qr: *mut u8) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);

    let (result, qr) = sqrt_fp_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
    *is_qr = if qr { 1 } else { 0 };
}

/// # Safety
///
/// Inversion of a non-zero element in Fp
#[inline]
pub unsafe fn inv_fp_bls12_381_ptr(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 6);

    let result = inv_fp_bls12_381(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 6);
    out.copy_from_slice(&result);
}
