use crate::{
    syscalls::{
        syscall_secp256r1_add, syscall_secp256r1_dbl, SyscallPoint256, SyscallSecp256r1AddParams,
    },
    zisklib::{
        eq, fcall_msb_pos_256, fcall_msb_pos_256_2, is_one, is_zero, ONE_256, TWO_256, ZERO_256,
    },
};

use super::{
    constants::{E_A, E_B, G, G_NEG_Y, G_X, G_Y, IDENTITY_X, IDENTITY_Y},
    field::{add_fp_secp256r1, mul_fp_secp256r1, square_fp_secp256r1},
    scalar::{add_fn_secp256r1, sub_fn_secp256r1},
};

// Precomputed points
const IDENTITY_POINT: SyscallPoint256 = SyscallPoint256 { x: IDENTITY_X, y: IDENTITY_Y };
const G_POINT: SyscallPoint256 = SyscallPoint256 { x: G_X, y: G_Y };

/// Given points `p1` and `p2`, performs the point addition `p1 + p2` and assigns the result to `p1`.
/// It assumes that `p1` and `p2` are from the Secp256r1 curve, that `p1,p2 != 𝒪`
/// Returns true if the result is the point at infinity.
#[inline]
fn add_non_infinity_points_secp256r1(
    p1: &mut SyscallPoint256,
    p2: &SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    if p1.x != p2.x {
        let mut params = SyscallSecp256r1AddParams { p1, p2 };
        syscall_secp256r1_add(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        false
    } else if p1.y == p2.y {
        syscall_secp256r1_dbl(
            p1,
            #[cfg(feature = "hints")]
            hints,
        );
        false
    } else {
        // p1 + (-p1) = 𝒪
        true
    }
}

/// Checks whether the given point `p` is on the Secp256r1 curve.
/// It assumes that `p` is not the point at infinity.
pub fn is_on_curve_secp256r1(p: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> bool {
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    // p in E iff y² == x³ + a·x + b
    let lhs = square_fp_secp256r1(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut rhs = square_fp_secp256r1(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = mul_fp_secp256r1(
        &rhs,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = add_fp_secp256r1(
        &rhs,
        &mul_fp_secp256r1(
            &x,
            &E_A,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = add_fp_secp256r1(
        &rhs,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&lhs, &rhs)
}

/// Given a non-infinity point `p` and a scalar `k`, computes the scalar multiplication `k·p`
///
/// Note: There are no (non-infinity) points of order 2 in Secp256r1.
///       All (non-infinity) points are of prime order N.
pub fn scalar_mul_secp256r1(
    k: &[u64; 4],
    p: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    // Trivial cases: k = 0, k = 1, k = 2.
    if eq(k, &ZERO_256) {
        return None;
    } else if eq(k, &ONE_256) {
        return Some(*p);
    } else if eq(k, &TWO_256) {
        let mut res = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
        syscall_secp256r1_dbl(
            &mut res,
            #[cfg(feature = "hints")]
            hints,
        );
        return Some([
            res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3],
        ]);
    }
    // From here on, k > 2.

    // 1. Convert p to SyscallPoint256.
    let base = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };

    // 4. Hint the position of the most significant set bit of k.
    //    At the hinted position the scalar must have a 1 bit;
    //    the loop reconstructs the scalar bit-by-bit and asserts the
    //    recomposition matches the input.
    let (max_limb, max_bit) = fcall_msb_pos_256(
        k,
        #[cfg(feature = "hints")]
        hints,
    );
    // Bound before use as index/shift
    assert!(max_limb < 4 && max_bit < 64, "msb_pos hint out of range");

    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    let k_top = (k[max_limb] >> max_bit) & 1;
    assert!(k_top == 1, "At least the top bit of the scalar must be set");

    // 3. Strauss-Shamir loop with bit-by-bit reconstruction of the scalar.
    let mut res = IDENTITY_POINT;
    let mut res_is_infinity = true;
    let mut k_rec = ZERO_256;

    // Helper macros to add a point to the accumulator
    macro_rules! add_pt {
        ($pt:expr) => {{
            if res_is_infinity {
                res = $pt;
                res_is_infinity = false;
            } else {
                res_is_infinity = add_non_infinity_points_secp256r1(
                    &mut res,
                    &$pt,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        }};
    }

    // Perform the loop, based on the binary representation of k.
    let mut start_bit = max_bit;
    for i in (0..=max_limb).rev() {
        let k_word = k[i];
        let mut k_rec_word = 0u64;

        for j in (0..=start_bit).rev() {
            let k_bit = (k_word >> j) & 1;
            let one_j: u64 = 1 << j;

            // Double first (a no-op while res is still 𝒪).
            if !res_is_infinity {
                syscall_secp256r1_dbl(
                    &mut res,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }

            if k_bit == 1 {
                add_pt!(base);
                k_rec_word |= one_j;
            }
        }

        k_rec[i] = k_rec_word;
        start_bit = 63;
    }

    // Soundness: the reconstructed scalar must match the input.
    assert!(eq(&k_rec, k));

    if res_is_infinity {
        None
    } else {
        Some([res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3]])
    }
}

/// Given a point `p` and scalars `k1` and `k2`, computes the double scalar multiplication `k1·G + k2·p`
/// It assumes that `k1,k2 ∈ [0, N-1]` and that `p != 𝒪`
pub fn double_scalar_mul_with_g_secp256r1(
    k1: &[u64; 4],
    k2: &[u64; 4],
    p: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    // Handle zero scalars:
    //  - If k1 = k2 = 0, then k1·G + k2·p = 𝒪.
    //  - If k1 = 0 and k2 > 0, then k1·G + k2·p = k2·p
    //  - If k2 = 0 and k1 > 0, then k1·G + k2·p = k1·G
    match (is_zero(k1), is_zero(k2)) {
        (true, true) => return None,
        (true, false) => {
            return scalar_mul_secp256r1(
                k2,
                p,
                #[cfg(feature = "hints")]
                hints,
            );
        }
        (false, true) => {
            return scalar_mul_secp256r1(
                k1,
                &G,
                #[cfg(feature = "hints")]
                hints,
            );
        }
        (false, false) => {}
    }

    // If k1 = k2 => k1·G + k2·P = k1·(G + P)
    if eq(k1, k2) {
        let mut gp = G_POINT;
        let gp_is_infinity = add_non_infinity_points_secp256r1(
            &mut gp,
            &SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] },
            #[cfg(feature = "hints")]
            hints,
        );
        if gp_is_infinity {
            return None;
        }
        return scalar_mul_secp256r1(
            k1,
            &[gp.x[0], gp.x[1], gp.x[2], gp.x[3], gp.y[0], gp.y[1], gp.y[2], gp.y[3]],
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // If P = -G => k1·G + (-k2)·G = (k1-k2)·G
    // If P = G => k1·G + k2·G = (k1+k2)·G
    if eq(&p[0..4], &G_X) {
        let k1k2 = match eq(&p[4..8], &G_NEG_Y) {
            true => sub_fn_secp256r1(
                k1,
                k2,
                #[cfg(feature = "hints")]
                hints,
            ),
            false => add_fn_secp256r1(
                k1,
                k2,
                #[cfg(feature = "hints")]
                hints,
            ),
        };

        return scalar_mul_secp256r1(
            &k1k2,
            &G,
            #[cfg(feature = "hints")]
            hints,
        );
    }
    // From here on, at least one of k1 or k2 is greater than 1

    // 1. Convert p to SyscallPoint256 and precompute the single multi-base sum `G + P`.
    let base_p = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
    let mut gp = G_POINT;
    let gp_is_inf = add_non_infinity_points_secp256r1(
        &mut gp,
        &base_p,
        #[cfg(feature = "hints")]
        hints,
    );

    // 2. Hint the position of the most significant set bit across (k1, k2).
    //    At the hinted position at least one of the two scalars must have a 1 bit;
    //    the loop reconstructs each scalar bit-by-bit and asserts the
    //    recomposition matches the input.
    let (max_limb, max_bit) = fcall_msb_pos_256_2(
        k1,
        k2,
        #[cfg(feature = "hints")]
        hints,
    );
    // Bound before use as index/shift
    assert!(max_limb < 4 && max_bit < 64, "msb_pos hint out of range");

    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    let k1_top = (k1[max_limb] >> max_bit) & 1;
    let k2_top = (k2[max_limb] >> max_bit) & 1;
    assert!(
        k1_top == 1 || k2_top == 1,
        "At least one of the half-scalars must have its top bit set"
    );

    // 3. Strauss-Shamir loop with bit-by-bit reconstruction of each scalar.
    let mut res = IDENTITY_POINT;
    let mut res_is_infinity = true;
    let mut k1_rec = ZERO_256;
    let mut k2_rec = ZERO_256;

    macro_rules! add_pt {
        ($pt:expr) => {{
            if res_is_infinity {
                res = $pt;
                res_is_infinity = false;
            } else {
                res_is_infinity = add_non_infinity_points_secp256r1(
                    &mut res,
                    &$pt,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        }};
    }
    macro_rules! add_pt_if_not_inf {
        ($pt:expr, $is_inf:expr) => {{
            if !$is_inf {
                add_pt!($pt);
            }
        }};
    }

    let mut start_bit = max_bit;
    for i in (0..=max_limb).rev() {
        let k1_word = k1[i];
        let k2_word = k2[i];
        let mut k1_rec_word = 0u64;
        let mut k2_rec_word = 0u64;

        for j in (0..=start_bit).rev() {
            let k1_bit = (k1_word >> j) & 1;
            let k2_bit = (k2_word >> j) & 1;
            let one_j: u64 = 1 << j;

            // Double first (a no-op while res is still 𝒪).
            if !res_is_infinity {
                syscall_secp256r1_dbl(
                    &mut res,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }

            match (k1_bit, k2_bit) {
                (0, 0) => {}
                (1, 0) => {
                    add_pt!(G_POINT);
                    k1_rec_word |= one_j;
                }
                (0, 1) => {
                    add_pt!(base_p);
                    k2_rec_word |= one_j;
                }
                (1, 1) => {
                    add_pt_if_not_inf!(gp, gp_is_inf); // 0b11 = G + P
                    k1_rec_word |= one_j;
                    k2_rec_word |= one_j;
                }
                _ => unreachable!(),
            }
        }

        k1_rec[i] = k1_rec_word;
        k2_rec[i] = k2_rec_word;
        start_bit = 63;
    }

    // Soundness: the reconstructed scalars must match the input.
    assert!(eq(&k1_rec, k1));
    assert!(eq(&k2_rec, k2));

    if res_is_infinity {
        None
    } else {
        Some([res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3]])
    }
}
