//! Point arithmetic on the Ed25519 (edwards25519) curve, -x² + y² = 1 + d·x²·y² over Fp.
//!
//! Points are affine and canonical, `[u64; 8]` = `x ‖ y` (little-endian limbs). The addition law
//! is complete.

use crate::{
    syscalls::{syscall_ed25519_add, SyscallPoint256},
    zisklib::{eq, le_bytes_to_u64_4, u64_4_to_le_bytes},
};

use super::{
    constants::{B_TABLE, D, IDENTITY, P, P_MINUS_ONE},
    field::{
        arith_mod, neg_fp_ed25519, reduce_fp_ed25519, sqrt_ratio_fp_ed25519, square_fp_ed25519,
    },
};

const IDENTITY_POINT: SyscallPoint256 = point_from_u64_8(&IDENTITY);

const fn point_from_u64_8(p: &[u64; 8]) -> SyscallPoint256 {
    SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] }
}

fn point_to_u64_8(p: &SyscallPoint256) -> [u64; 8] {
    [p.x[0], p.x[1], p.x[2], p.x[3], p.y[0], p.y[1], p.y[2], p.y[3]]
}

/// `p1 += p2` through the precompile.
#[inline]
fn add_assign(
    p1: &mut SyscallPoint256,
    p2: &SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    syscall_ed25519_add(
        p1,
        p2,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// `p += p`
#[inline]
fn double_assign(p: &mut SyscallPoint256, #[cfg(feature = "hints")] hints: &mut Vec<u64>) {
    let copy = *p;
    add_assign(
        p,
        &copy,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Addition of two points on Ed25519 (also doubles when `p1 == p2`).
pub fn add_points_ed25519(
    p1: &[u64; 8],
    p2: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let mut r = point_from_u64_8(p1);
    add_assign(
        &mut r,
        &point_from_u64_8(p2),
        #[cfg(feature = "hints")]
        hints,
    );
    point_to_u64_8(&r)
}

/// Negation of a point on Ed25519: `-(x, y) = (-x, y)`.
pub fn neg_point_ed25519(p: &[u64; 8]) -> [u64; 8] {
    let x = neg_fp_ed25519(p[0..4].try_into().unwrap());
    [x[0], x[1], x[2], x[3], p[4], p[5], p[6], p[7]]
}

/// Whether a point has small order, i.e. `[8]p` is the identity (the curve cofactor is 8).
pub fn is_small_order_ed25519(
    p: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let mut q = point_from_u64_8(p);
    for _ in 0..3 {
        double_assign(
            &mut q,
            #[cfg(feature = "hints")]
            hints,
        );
    }
    eq(&point_to_u64_8(&q), &IDENTITY)
}

/// Decompresses a 32-byte Ed25519 point encoding: the little-endian `y` with the sign of `x`
/// (its low bit) in the top bit.
///
/// `y` is taken mod p (a non-canonical `y >= p` is accepted) and a sign bit of 1 on `x = 0` is
/// ignored. Returns `None` if `y` is not the coordinate of a curve point.
pub fn decompress_ed25519(
    bytes: &[u8; 32],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    let mut y = le_bytes_to_u64_4(bytes);
    let sign = y[3] >> 63;
    y[3] &= 0x7FFF_FFFF_FFFF_FFFF; // Clear the sign bit of `y`
    let y = reduce_fp_ed25519(&y);

    // x² = u / v with u = y² - 1, v = d·y² + 1
    let y2 = square_fp_ed25519(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let u = arith_mod(
        &y2,
        &[1, 0, 0, 0],
        &P_MINUS_ONE,
        &P,
        #[cfg(feature = "hints")]
        hints,
    );
    let v = arith_mod(
        &D,
        &y2,
        &[1, 0, 0, 0],
        &P,
        #[cfg(feature = "hints")]
        hints,
    );
    let root = sqrt_ratio_fp_ed25519(
        &u,
        &v,
        #[cfg(feature = "hints")]
        hints,
    )?;

    // Take the even ("non-negative") root, then apply the sign bit.
    let even = if root[0] & 1 == 0 { root } else { neg_fp_ed25519(&root) };
    let x = if sign == 1 { neg_fp_ed25519(&even) } else { even };
    Some([x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]])
}

/// Compresses a point to its canonical 32-byte encoding (little-endian `y`, sign of `x` on top).
pub fn compress_ed25519(p: &[u64; 8]) -> [u8; 32] {
    let mut out = u64_4_to_le_bytes(p[4..8].try_into().unwrap());
    out[31] |= ((p[0] & 1) as u8) << 7;
    out
}

/// 4-bit window `i` (0 = least significant) of a 256-bit scalar.
#[inline]
fn nibble(k: &[u64; 4], i: usize) -> usize {
    ((k[i / 16] >> (4 * (i % 16))) & 0xF) as usize
}

/// `r += q`, or `r = q` (no precompile call) while `r` is still the identity.
#[inline]
fn accumulate(
    r: &mut SyscallPoint256,
    started: &mut bool,
    q: &SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    if *started {
        add_assign(
            r,
            q,
            #[cfg(feature = "hints")]
            hints,
        );
    } else {
        *r = *q;
        *started = true;
    }
}

/// Computes `[k]a + [s]B` with B the base point (Straus/Shamir, 4-bit windows for both scalars:
/// a runtime table for `a`, the constant [`B_TABLE`] for B). Scalars are any 256-bit values.
pub fn double_scalar_mul_with_base_ed25519(
    k: &[u64; 4],
    a: &[u64; 8],
    s: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    // [i]a for i = 0..16
    let a = point_from_u64_8(a);
    let mut a_table = [IDENTITY_POINT; 16];
    a_table[1] = a;
    for i in 2..16 {
        let mut next = a_table[i - 1];
        add_assign(
            &mut next,
            &a,
            #[cfg(feature = "hints")]
            hints,
        );
        a_table[i] = next;
    }

    let mut r = IDENTITY_POINT;
    let mut started = false; // r is the identity until the first non-zero window
    for i in (0..64).rev() {
        if started {
            for _ in 0..4 {
                double_assign(
                    &mut r,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        }
        let (wk, ws) = (nibble(k, i), nibble(s, i));
        if wk != 0 {
            accumulate(
                &mut r,
                &mut started,
                &a_table[wk],
                #[cfg(feature = "hints")]
                hints,
            );
        }
        if ws != 0 {
            accumulate(
                &mut r,
                &mut started,
                &point_from_u64_8(&B_TABLE[ws]),
                #[cfg(feature = "hints")]
                hints,
            );
        }
    }
    point_to_u64_8(&r)
}
