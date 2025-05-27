//! Operations in the base field Fp of the BN254 curve

use crate::{
    arith256_mod::{syscall_arith256_mod, SyscallArith256ModParams},
    fcall_bn254_fp_inv,
    zisklib::lib::utils::eq,
};

use super::constants::{P, P_MINUS_ONE};

/// Addition in the base field of the BN254 curve
#[inline]
pub fn add_fp_bn254(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·1 + y
    let mut params =
        SyscallArith256ModParams { a: x, b: &[1, 0, 0, 0], c: y, module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Negation in the base field of the BN254 curve
#[inline]
pub fn neg_fp_bn254(x: &[u64; 4]) -> [u64; 4] {
    // x·(-1) + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &P_MINUS_ONE,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Multiplication in the base field of the BN254 curve
#[inline]
pub fn mul_fp_bn254(x: &[u64; 4], y: &[u64; 4]) -> [u64; 4] {
    // x·y + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: y, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Squaring in the base field of the BN254 curve
#[inline]
pub fn square_fp_bn254(x: &[u64; 4]) -> [u64; 4] {
    // x·x + 0
    let mut params =
        SyscallArith256ModParams { a: x, b: x, c: &[0, 0, 0, 0], module: &P, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(&mut params);
    *params.d
}

/// Inversion in the base field of the BN254 curve
#[inline]
pub fn inv_fp_bn254(x: &[u64; 4]) -> [u64; 4] {
    // if x == 0, return 0
    if eq(x, &[0, 0, 0, 0]) {
        return [0, 0, 0, 0];
    }

    // if x != 0, return 1 / x

    // Remember that an element y ∈ Fp is the inverse of x ∈ Fp if and only if x·y = 1 in Fp
    // We will therefore hint the inverse y and check the product with x is 1
    let inv = fcall_bn254_fp_inv(x);

    // x·y + 0
    let mut params = SyscallArith256ModParams {
        a: x,
        b: &inv,
        c: &[0, 0, 0, 0],
        module: &P,
        d: &mut [0, 0, 0, 0],
    };
    syscall_arith256_mod(&mut params);
    assert_eq!(*params.d, [1, 0, 0, 0]);

    inv
}
