//! Operations in the base field Fp of the Ed25519 curve, p = 2^255 - 19

use crate::{
    syscalls::{syscall_arith256_mod, SyscallArith256ModParams},
    zisklib::{eq, fcall_ed25519_fp_sqrt_ratio, is_zero, lt},
};

use super::constants::P;

/// `(a·b + c) mod m` through the `arith256_mod` precompile (output canonical).
#[inline]
pub(super) fn arith_mod(
    a: &[u64; 4],
    b: &[u64; 4],
    c: &[u64; 4],
    module: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    let mut params = SyscallArith256ModParams { a, b, c, module, d: &mut [0, 0, 0, 0] };
    syscall_arith256_mod(
        &mut params,
        #[cfg(feature = "hints")]
        hints,
    );
    *params.d
}

/// `a - b` on 256-bit integers, assuming `a >= b`.
#[inline]
pub(super) fn sub_256(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut out = [0u64; 4];
    let mut borrow = false;
    for i in 0..4 {
        let (d1, b1) = a[i].overflowing_sub(b[i]);
        let (d2, b2) = d1.overflowing_sub(borrow as u64);
        out[i] = d2;
        borrow = b1 || b2;
    }
    debug_assert!(!borrow, "sub_256 underflow");
    out
}

/// Reduction of a value `< 2^255` in the base field of Ed25519 (at most one subtraction of p).
pub fn reduce_fp_ed25519(x: &[u64; 4]) -> [u64; 4] {
    debug_assert!(x[3] >> 63 == 0, "reduce_fp_ed25519 expects x < 2^255");
    if lt(x, &P) {
        *x
    } else {
        sub_256(x, &P)
    }
}

/// Negation in the base field of Ed25519, for a canonical `x`.
pub fn neg_fp_ed25519(x: &[u64; 4]) -> [u64; 4] {
    if is_zero(x) {
        *x
    } else {
        sub_256(&P, x)
    }
}

/// Multiplication in the base field of Ed25519.
pub fn mul_fp_ed25519(
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 4] {
    arith_mod(
        x,
        y,
        &[0, 0, 0, 0],
        &P,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Squaring in the base field of Ed25519.
pub fn square_fp_ed25519(x: &[u64; 4], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 4] {
    mul_fp_ed25519(
        x,
        x,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// Square root of `u / v` in the base field of Ed25519, for canonical `u`, `v` with `v != 0`.
///
/// Returns `Some(r)` with `v·r² = u` if `u / v` is a square (`r` is either root; the caller picks
/// the sign), or `None` if it is not. Both answers are checked, so the hint is not trusted:
/// - square: `v·r² = u`;
/// - non-square: `v·r² = 2u` with `u != 0`. Since 2 is a non-residue mod p, a root of `2u/v`
///   exists only if `u/v` is not a square. `u != 0` is required because `u = 0` is a square
///   that would also satisfy `v·0² = 2·0`.
pub fn sqrt_ratio_fp_ed25519(
    u: &[u64; 4],
    v: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 4]> {
    // With v = 0 any r would satisfy v·r² = u = 0. Unreachable for Ed25519 decompression
    // (v = d·y² + 1 = 0 needs y² = -1/d, and -1/d is a non-residue), but cheap to rule out.
    assert!(!is_zero(v), "sqrt_ratio_fp_ed25519: v = 0");

    let hint = fcall_ed25519_fp_sqrt_ratio(
        u,
        v,
        #[cfg(feature = "hints")]
        hints,
    );
    let is_square = hint[0] == 1;
    let r: [u64; 4] = hint[1..5].try_into().unwrap();
    assert!(lt(&r, &P), "Square root is not canonical");

    // v·r²
    let r2 = square_fp_ed25519(
        &r,
        #[cfg(feature = "hints")]
        hints,
    );
    let vr2 = mul_fp_ed25519(
        v,
        &r2,
        #[cfg(feature = "hints")]
        hints,
    );

    if is_square {
        assert!(eq(&vr2, u), "Square root verification failed");
        Some(r)
    } else {
        assert!(!is_zero(u), "0 is a square");
        // 2u = u·1 + u
        let two_u = arith_mod(
            u,
            &[1, 0, 0, 0],
            u,
            &P,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(eq(&vr2, &two_u), "Non-square verification failed");
        None
    }
}
