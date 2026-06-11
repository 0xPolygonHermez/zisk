//! Operations in the degree 2 extension Fp2 of the BN254 curve

use crate::{
    syscalls::{
        syscall_bn254_complex_add, syscall_bn254_complex_mul, syscall_bn254_complex_sub,
        SyscallBn254ComplexAddParams, SyscallBn254ComplexMulParams, SyscallBn254ComplexSubParams,
        SyscallComplex256,
    },
    zisklib::{eq, fcall_bn254_fp2_inv, is_one, is_zero, lt},
};

use super::constants::{P, P_MINUS_ONE};

/// Helper to convert from array representation to syscall representation
#[inline]
fn to_syscall_complex(limbs: &[u64; 8]) -> SyscallComplex256 {
    SyscallComplex256 { x: limbs[0..4].try_into().unwrap(), y: limbs[4..8].try_into().unwrap() }
}

#[inline]
fn to_syscall_complex_x(limbs: &[u64; 4]) -> SyscallComplex256 {
    SyscallComplex256 { x: *limbs, y: [0u64; 4] }
}

/// Helper to convert from syscall representation to array representation
#[inline]
fn from_syscall_complex(complex: &SyscallComplex256) -> [u64; 8] {
    let mut result = [0u64; 8];
    result[0..4].copy_from_slice(&complex.x);
    result[4..8].copy_from_slice(&complex.y);
    result
}

/// Addition in the degree 2 extension of the BN254 curve
#[inline]
pub fn add_fp2_bn254(
    a: &[u64; 8],
    b: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);

    let mut params = SyscallBn254ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_add(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Doubling in the degree 2 extension of the BN254 curve
#[inline]
pub fn dbl_fp2_bn254(a: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 8] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(a);

    let mut params = SyscallBn254ComplexAddParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_add(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Negation in the degree 2 extension of the BN254 curve
#[inline]
pub fn neg_fp2_bn254(a: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 8] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex_x(&P_MINUS_ONE);

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Subtraction in the degree 2 extension of the BN254 curve
#[inline]
pub fn sub_fp2_bn254(
    a: &[u64; 8],
    b: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);

    let mut params = SyscallBn254ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_sub(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Multiplication in the degree 2 extension of the BN254 curve
#[inline]
pub fn mul_fp2_bn254(
    a: &[u64; 8],
    b: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(b);

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Scalar multiplication in the degree 2 extension of the BN254 curve
#[inline]
pub fn scalar_mul_fp2_bn254(
    a: &[u64; 8],
    b: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex_x(b);

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Squaring in the degree 2 extension of the BN254 curve
#[inline]
pub fn square_fp2_bn254(a: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 8] {
    let mut f1 = to_syscall_complex(a);
    let f2 = to_syscall_complex(a);

    let mut params = SyscallBn254ComplexMulParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_mul(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}

/// Inversion in the degree 2 extension of the BN254 curve
#[inline]
pub fn inv_fp2_bn254(a: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 8] {
    // if a == 0, return 0
    if is_zero(a) {
        return *a;
    }

    // if a != 0, return 1 / a

    // Remember that an element b ∈ Fp2 is the inverse of a ∈ Fp2 if and only if a·b = 1 in Fp2
    // We will therefore hint the inverse b and check the product with a is 1
    let inv = fcall_bn254_fp2_inv(
        a,
        #[cfg(feature = "hints")]
        hints,
    );

    // Check that the inverse is canonical
    assert!(lt(&inv[0..4], &P) && lt(&inv[4..8], &P), "Inverse is not canonical");

    let product = mul_fp2_bn254(
        a,
        &inv,
        #[cfg(feature = "hints")]
        hints,
    );
    assert!(is_one(&product), "Inverse verification failed");

    inv
}

/// Conjugation in the degree 2 extension of the BN254 curve
#[inline]
pub fn conjugate_fp2_bn254(
    a: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let mut f1 = to_syscall_complex_x(&a[0..4].try_into().unwrap());
    let f2 = to_syscall_complex_x(&a[4..8].try_into().unwrap());

    let mut params = SyscallBn254ComplexSubParams { f1: &mut f1, f2: &f2 };
    syscall_bn254_complex_sub(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    from_syscall_complex(&f1)
}
