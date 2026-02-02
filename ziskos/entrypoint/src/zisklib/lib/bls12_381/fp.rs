//! Finite field Fp operations for BLS12-381

use crate::{
    syscalls::{syscall_arith384_mod, SyscallArith384ModParams},
    zisklib::{eq, fcall_bls12_381_fp_inv, fcall_bls12_381_fp_sqrt},
};

use super::constants::{NQR_FP, P, P_MINUS_ONE};

/// Sign function in Fp
#[inline]
pub fn sgn0_fp_bls12_381(x: &[u64; 6]) -> u64 {
    x[0] & 1
}

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

/// Convert 48-byte big-endian field element to [u64; 6] little-endian
pub fn bytes_be_to_u64_le_fp_bls12_381(bytes: &[u8; 48]) -> [u64; 6] {
    let mut result = [0u64; 6];
    for i in 0..6 {
        for j in 0..8 {
            result[5 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }
    result
}
