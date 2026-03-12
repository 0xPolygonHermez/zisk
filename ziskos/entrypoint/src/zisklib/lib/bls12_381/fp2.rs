//! Finite field Fp2 operations for BLS12-381

use crate::{
    syscalls::{
        syscall_bls12_381_complex_add, syscall_bls12_381_complex_mul,
        syscall_bls12_381_complex_sub, SyscallBls12_381ComplexAddParams,
        SyscallBls12_381ComplexMulParams, SyscallBls12_381ComplexSubParams, SyscallComplex384,
    },
    zisklib::{eq, fcall_bls12_381_fp2_inv, fcall_bls12_381_fp2_sqrt, is_zero},
};

use super::constants::{NQR_FP2, P_MINUS_ONE};

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

/// Sign function in Fp2
pub fn sgn0_fp2_bls12_381(x: &[u64; 12]) -> u64 {
    let sign_0 = x[0] & 1;
    let zero_0 = is_zero(&x[0..6]) as u64;
    let sign_1 = x[6] & 1;
    sign_0 | (zero_0 & sign_1)
}

/// Addition in Fp2
#[inline]
pub fn add_fp2_bls12_381(
    a: &[u64; 12],
    b: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);
    let mut params = SyscallBls12_381ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_add(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Doubling in Fp2
#[inline]
pub fn dbl_fp2_bls12_381(
    a: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(a);
    let mut params = SyscallBls12_381ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_add(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Negation in Fp2
#[inline]
pub fn neg_fp2_bls12_381(
    a: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex_x(&P_MINUS_ONE);
    let mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Subtraction in Fp2
#[inline]
pub fn sub_fp2_bls12_381(
    a: &[u64; 12],
    b: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);
    let mut params = SyscallBls12_381ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_sub(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Multiplication in Fp2
#[inline]
pub fn mul_fp2_bls12_381(
    a: &[u64; 12],
    b: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);
    let mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Scalar multiplication in Fp2
#[inline]
pub fn scalar_mul_fp2_bls12_381(
    a: &[u64; 12],
    b: &[u64; 6],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 =
        SyscallComplex384 { x: a[0..6].try_into().unwrap(), y: a[6..12].try_into().unwrap() };
    let f2 = SyscallComplex384 { x: b[0..6].try_into().unwrap(), y: [0, 0, 0, 0, 0, 0] };

    let mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Squaring in Fp2
#[inline]
pub fn square_fp2_bls12_381(
    a: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(a);
    let mut params = SyscallBls12_381ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Square root in Fp2
#[inline]
pub fn sqrt_fp2_bls12_381(
    x: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> ([u64; 12], bool) {
    // Hint the sqrt
    let hint = fcall_bls12_381_fp2_sqrt(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let is_qr = hint[0] == 1;
    let sqrt = hint[1..13].try_into().unwrap();

    // Compute sqrt * sqrt
    let mul = mul_fp2_bls12_381(
        &sqrt,
        &sqrt,
        #[cfg(feature = "hints")]
        hints,
    );

    if is_qr {
        // Check that sqrt * sqrt == x
        assert!(eq(&mul, x));
        (sqrt, true)
    } else {
        // Check that sqrt * sqrt == x * NQR
        let nqr = mul_fp2_bls12_381(
            x,
            &NQR_FP2,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(eq(&mul, &nqr));
        (sqrt, false)
    }
}

/// Inversion in Fp2: returns a⁻¹
#[inline]
pub fn inv_fp2_bls12_381(
    a: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    // if a == 0, return 0
    if eq(a, &[0; 12]) {
        return *a;
    }

    // if a != 0, return 1 / a

    // Remember that an element b ∈ Fp2 is the inverse of a ∈ Fp2 if and only if a·b = 1 in Fp2
    // We will therefore hint the inverse b and check the product with a is 1
    let inv = fcall_bls12_381_fp2_inv(
        a,
        #[cfg(feature = "hints")]
        hints,
    );

    let product = mul_fp2_bls12_381(
        a,
        &inv,
        #[cfg(feature = "hints")]
        hints,
    );
    assert_eq!(&product[0..6], &[1, 0, 0, 0, 0, 0]);
    assert_eq!(&product[6..12], &[0, 0, 0, 0, 0, 0]);

    inv
}

/// Conjugation in Fp2
#[inline]
pub fn conjugate_fp2_bls12_381(
    a: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 12] {
    let mut f1 = SyscallComplex384 { x: a[0..6].try_into().unwrap(), y: [0, 0, 0, 0, 0, 0] };
    let f2 = SyscallComplex384 { x: [0, 0, 0, 0, 0, 0], y: a[6..12].try_into().unwrap() };

    let mut params = SyscallBls12_381ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bls12_381_complex_sub(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Convert 96-byte big-endian Fp2 element to [u64; 12] little-endian
/// Format: fp2 = (c0, c1) where c0 is real, c1 is imaginary
/// Bytes: c0 (48 bytes) || c1 (48 bytes)
pub fn bytes_be_to_u64_le_fp2_bls12_381(bytes: &[u8; 96]) -> [u64; 12] {
    let mut result = [0u64; 12];

    // c0 (real part, bytes 0-47) -> result[0..6]
    for i in 0..6 {
        for j in 0..8 {
            result[5 - i] |= (bytes[i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    // c1 (imaginary part, bytes 48-95) -> result[6..12]
    for i in 0..6 {
        for j in 0..8 {
            result[11 - i] |= (bytes[48 + i * 8 + j] as u64) << (8 * (7 - j));
        }
    }

    result
}
