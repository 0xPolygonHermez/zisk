use crate::{
    bls12_381_complex_add::{syscall_bls12_381_complex_add, SyscallBls12_381ComplexAddParams},
    bls12_381_complex_mul::{syscall_bls12_381_complex_mul, SyscallBls12_381ComplexMulParams},
    bls12_381_complex_sub::{syscall_bls12_381_complex_sub, SyscallBls12_381ComplexSubParams},
    complex::SyscallComplex384,
    fcall_bls12_381_fp2_inv,
};

use super::constants::P_MINUS_ONE;

// ========== Core Implementation (Array-based, Safe) ==========

/// Helper to convert from array representation to syscall representation
#[inline]
fn to_syscall_complex(limbs: &[u64; 12]) -> SyscallComplex384 {
    SyscallComplex384 { x: limbs[0..6].try_into().unwrap(), y: limbs[6..12].try_into().unwrap() }
}

#[inline]
fn to_syscall_complex_x(limbs: &[u64; 6]) -> SyscallComplex384 {
    SyscallComplex384 { x: *limbs, y: [0u64; 6] }
}

/// Helper to convert from syscall representation to array representation
#[inline]
fn from_syscall_complex(complex: &SyscallComplex384) -> [u64; 12] {
    let mut result = [0u64; 12];
    result[0..6].copy_from_slice(&complex.x);
    result[6..12].copy_from_slice(&complex.y);
    result
}

/// Addition in Fp2
#[inline]
pub(crate) fn add_fp2_bls12_381_core(a: &[u64; 12], b: &[u64; 12]) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);
    let mut params = SyscallBls12_381ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_add(&mut params);
    from_syscall_complex(&f1)
}

/// Doubling in Fp2
#[inline]
pub(crate) fn dbl_fp2_bls12_381_core(a: &[u64; 12]) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(a);
    let mut params = SyscallBls12_381ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_add(&mut params);
    from_syscall_complex(&f1)
}

/// Negation in Fp2
#[inline]
pub(crate) fn neg_fp2_bls12_381_core(a: &[u64; 12]) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex_x(&P_MINUS_ONE);
    let mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_mul(&mut params);
    from_syscall_complex(&f1)
}

/// Subtraction in Fp2
#[inline]
pub(crate) fn sub_fp2_bls12_381_core(a: &[u64; 12], b: &[u64; 12]) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);
    let mut params = SyscallBls12_381ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_sub(&mut params);
    from_syscall_complex(&f1)
}

/// Multiplication in Fp2
#[inline]
pub(crate) fn mul_fp2_bls12_381_core(a: &[u64; 12], b: &[u64; 12]) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);
    let mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_mul(&mut params);
    from_syscall_complex(&f1)
}

/// Squaring in Fp2
#[inline]
pub(crate) fn square_fp2_bls12_381_core(a: &[u64; 12]) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = SyscallComplex384 { x: f1.x, y: f1.y };
    let mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_mul(&mut params);
    from_syscall_complex(&f1)
}

/// Inversion in Fp2: returns a⁻¹
#[inline]
pub(crate) fn inv_fp2_bls12_381_core(a: &[u64; 12]) -> [u64; 12] {
    // Remember that an element b ∈ Fp2 is the inverse of a ∈ Fp2 if and only if a·b = 1 in Fp2
    // We will therefore hint the inverse b and check the product with a is 1
    let inv = fcall_bls12_381_fp2_inv(a);

    let product = mul_fp2_bls12_381_core(a, &inv);
    assert_eq!(&product[0..6], &[1, 0, 0, 0, 0, 0]);
    assert_eq!(&product[6..12], &[0, 0, 0, 0, 0, 0]);

    inv
}

// ========== Pointer-based API (Thin Wrappers) ==========

#[inline]
pub unsafe fn add_fp2_bls12_381(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 12);
    let b_in = core::slice::from_raw_parts(b, 12);

    let result = add_fp2_bls12_381_core(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 12);
    out.copy_from_slice(&result);
}

#[inline]
pub unsafe fn dbl_fp2_bls12_381(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 12);

    let result = dbl_fp2_bls12_381_core(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 12);
    out.copy_from_slice(&result);
}

/// Negation in the degree 2 extension of the BLS12-381 curve
#[inline]
pub unsafe fn neg_fp2_bls12_381(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 12);

    let result = neg_fp2_bls12_381_core(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 12);
    out.copy_from_slice(&result);
}

/// Subtraction in the degree 2 extension of the BLS12-381 curve
#[inline]
pub unsafe fn sub_fp2_bls12_381(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 12);
    let b_in = core::slice::from_raw_parts(b, 12);

    let result = sub_fp2_bls12_381_core(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 12);
    out.copy_from_slice(&result);
}

/// Multiplication in the degree 2 extension of the BLS12-381 curve
#[inline]
pub unsafe fn mul_fp2_bls12_381(a: *mut u64, b: *const u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 12);
    let b_in = core::slice::from_raw_parts(b, 12);

    let result = mul_fp2_bls12_381_core(a_in.try_into().unwrap(), b_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 12);
    out.copy_from_slice(&result);
}

/// Squaring in the degree 2 extension of the BLS12-381 curve
#[inline]
pub unsafe fn square_fp2_bls12_381(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 12);

    let result = square_fp2_bls12_381_core(a_in.try_into().unwrap());

    let out = core::slice::from_raw_parts_mut(a, 12);
    out.copy_from_slice(&result);
}

/// Inversion of a non-zero element in the degree 2 extension of the BLS12-381 curve
#[inline]
pub unsafe fn inv_fp2_bls12_381(a: *mut u64) {
    let a_in = core::slice::from_raw_parts(a as *const u64, 12).try_into().unwrap();

    let result = inv_fp2_bls12_381_core(&a_in);

    let out = core::slice::from_raw_parts_mut(a, 12);
    out.copy_from_slice(&result);
}
