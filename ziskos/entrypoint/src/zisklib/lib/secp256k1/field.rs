//! Operations in the base field Fp of the secp256k1 curve

use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::fcall_secp256k1_fp_sqrt,
};

use super::constants::{NQR, P};

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
    let sqrt = hint[1..5].try_into().unwrap();

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
        assert_eq!(*params.d, *x);
        (sqrt, true)
    } else {
        // Check that sqrt * sqrt == x * NQR
        let nqr = mul_fp_secp256k1(
            x,
            &NQR,
            #[cfg(feature = "hints")]
            hints,
        );
        assert_eq!(*params.d, nqr);
        (sqrt, false)
    }
}
