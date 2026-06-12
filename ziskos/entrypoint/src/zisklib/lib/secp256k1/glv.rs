extern crate alloc;
use alloc::vec::Vec;

use crate::{
    syscalls::{syscall_secp256k1_dbl, SyscallPoint256},
    zisklib::{
        eq, fcall_msb_pos_256_2, fcall_msb_pos_256_4, is_one, is_two, is_zero, ONE_256, TWO_256,
        ZERO_256,
    },
};

use super::{
    constants::{G, G_NEG_Y, G_PHI_X, G_PHI_Y, G_X, G_Y, IDENTITY_X, IDENTITY_Y},
    curve::{
        add_non_infinity_points_secp256k1, msm_secp256k1_max_bits, neg_secp256k1, phi_secp256k1,
    },
    scalar::{add_fn_secp256k1, glv_decompose_fn_secp256k1, reduce_fn_secp256k1, sub_fn_secp256k1},
};

// Precomputed points
const IDENTITY_POINT: SyscallPoint256 = SyscallPoint256 { x: IDENTITY_X, y: IDENTITY_Y };
const G_POINT: SyscallPoint256 = SyscallPoint256 { x: G_X, y: G_Y };
const G_NEG_POINT: SyscallPoint256 = SyscallPoint256 { x: G_X, y: G_NEG_Y };
const G_PHI_POINT: SyscallPoint256 = SyscallPoint256 { x: G_PHI_X, y: G_PHI_Y };
const G_PHI_NEG_POINT: SyscallPoint256 = SyscallPoint256 { x: G_PHI_X, y: G_NEG_Y };

// Some ideas were extracted from https://eprint.iacr.org/2025/933.pdf

/// Given a non-infinity point `p` and a scalar `k ∈ [0, N-1]`, computes `k·p`
/// using the GLV endomorphism.
///
/// # Soundness
/// The point must be on-curve, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
pub fn glv_scalar_mul_secp256k1(
    k: &[u64; 4],
    p: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    // The idea of using the GLV endomorphism φ is to decompose the scalar k
    // such that k = a1 + a2·λ, and then rewrite [k]·P as:
    //      [a1]·P + [a2]·φ(P)
    // where a1, a2 are approximately half the bit-length of k.

    // Reduce the scalar
    let k = reduce_fn_secp256k1(
        k,
        #[cfg(feature = "hints")]
        hints,
    );

    // Trivial cases: k = 0, k = 1, k = 2.
    if is_zero(&k) {
        return None;
    } else if is_one(&k) {
        return Some(*p);
    } else if is_two(&k) {
        let mut res = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
        syscall_secp256k1_dbl(
            &mut res,
            #[cfg(feature = "hints")]
            hints,
        );
        return Some([
            res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3],
        ]);
    }
    // From here on, k > 2.

    // 1. GLV-decompose k.
    let (k1, k2, sigma_1, sigma_2) = glv_decompose_fn_secp256k1(
        &k,
        #[cfg(feature = "hints")]
        hints,
    );

    // 2. Compute the two sign-adjusted base points: ±P, ±φ(P).
    let p_point = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
    let p_phi = phi_secp256k1(
        &p_point,
        #[cfg(feature = "hints")]
        hints,
    );

    let base_p = if sigma_1 == 0 {
        p_point
    } else {
        neg_secp256k1(
            &p_point,
            #[cfg(feature = "hints")]
            hints,
        )
    };
    let base_p_phi = if sigma_2 == 0 {
        p_phi
    } else {
        neg_secp256k1(
            &p_phi,
            #[cfg(feature = "hints")]
            hints,
        )
    };

    // 3. Precompute the single multi-base sum `base_p + base_p_phi`.
    let mut t_11 = base_p;
    let t_11_is_inf = add_non_infinity_points_secp256k1(
        &mut t_11,
        &base_p_phi,
        #[cfg(feature = "hints")]
        hints,
    );

    // 4. Hint the position of the most significant set bit across (k1, k2).
    //    At the hinted position at least one of the two half-scalars must have a 1 bit;
    //    the loop reconstructs each half-scalar bit-by-bit and asserts the
    //    recomposition matches the GLV-decomposed values.
    let (max_limb, max_bit) = fcall_msb_pos_256_2(
        &k1,
        &k2,
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

    // 5. Strauss-Shamir loop with bit-by-bit reconstruction of each half-scalar.
    let mut res = IDENTITY_POINT;
    let mut res_is_infinity = true;
    let mut k1_rec = ZERO_256;
    let mut k2_rec = ZERO_256;

    // Helper macros to add a point to the accumulator
    macro_rules! add_pt {
        ($pt:expr) => {{
            if res_is_infinity {
                res = $pt;
                res_is_infinity = false;
            } else {
                res_is_infinity = add_non_infinity_points_secp256k1(
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

    // Perform the loop, based on the binary representation of k1,k2.
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
                syscall_secp256k1_dbl(
                    &mut res,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }

            match (k1_bit, k2_bit) {
                (0, 0) => {}
                (1, 0) => {
                    add_pt!(base_p);
                    k1_rec_word |= one_j;
                }
                (0, 1) => {
                    add_pt!(base_p_phi);
                    k2_rec_word |= one_j;
                }
                (1, 1) => {
                    add_pt_if_not_inf!(t_11, t_11_is_inf);
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

    // Soundness: the reconstructed half-scalars must match the GLV-decomposed ones.
    assert!(eq(&k1_rec, &k1));
    assert!(eq(&k2_rec, &k2));

    if res_is_infinity {
        None
    } else {
        Some([res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3]])
    }
}

/// Given a non-infinity point `p` and scalars `k1, k2 ∈ [0, N-1]`, computes the double scalar
/// multiplication `k1·G + k2·p` using the GLV endomorphism.
///
/// # Soundness
/// The points must be on-curve, non-identity, and have **canonical** coordinates
/// (`x, y < p`).
pub fn glv_double_scalar_mul_with_g_secp256k1(
    k1: &[u64; 4],
    k2: &[u64; 4],
    p: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    // The idea of using the GLV endomorphism φ is to decompose the scalars k1 and k2
    // such that k1 = a1 + a2·λ and k2 = b1 + b2·λ, and then rewrite [k1]·G + [k2]·P as:
    //      [a1]·G + [a2]·φ(G) + [b1]·P + [b2]·φ(P)
    // where a1, a2, b1, b2 are approximately half the bit-length of k1 and k2.

    // Reduce the scalars
    let k1 = reduce_fn_secp256k1(
        k1,
        #[cfg(feature = "hints")]
        hints,
    );
    let k2 = reduce_fn_secp256k1(
        k2,
        #[cfg(feature = "hints")]
        hints,
    );

    // Handle zero scalars:
    //  - If k1 = k2 = 0, then k1·G + k2·p = 𝒪.
    //  - If k1 = 0 and k2 > 0, then k1·G + k2·p = k2·p
    //  - If k2 = 0 and k1 > 0, then k1·G + k2·p = k1·G
    match (is_zero(&k1), is_zero(&k2)) {
        (true, true) => return None,
        (true, false) => {
            return glv_scalar_mul_secp256k1(
                &k2,
                p,
                #[cfg(feature = "hints")]
                hints,
            );
        }
        (false, true) => {
            return glv_scalar_mul_secp256k1(
                &k1,
                &G,
                #[cfg(feature = "hints")]
                hints,
            );
        }
        (false, false) => {}
    }

    // If k1 = k2 => k1·G + k2·P = k1·(G + P)
    if eq(&k1, &k2) {
        let mut gp = G_POINT;
        let gp_is_infinity = add_non_infinity_points_secp256k1(
            &mut gp,
            &SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] },
            #[cfg(feature = "hints")]
            hints,
        );
        if gp_is_infinity {
            return None;
        }
        return glv_scalar_mul_secp256k1(
            &k1,
            &[gp.x[0], gp.x[1], gp.x[2], gp.x[3], gp.y[0], gp.y[1], gp.y[2], gp.y[3]],
            #[cfg(feature = "hints")]
            hints,
        );
    }

    // If P = -G => k1·G + (-k2)·G = (k1-k2)·G
    // If P = G => k1·G + k2·G = (k1+k2)·G
    if eq(&p[0..4], &G_X) {
        let k1k2 = match eq(&p[4..8], &G_NEG_Y) {
            true => sub_fn_secp256k1(
                &k1,
                &k2,
                #[cfg(feature = "hints")]
                hints,
            ),
            false => add_fn_secp256k1(
                &k1,
                &k2,
                #[cfg(feature = "hints")]
                hints,
            ),
        };

        return glv_scalar_mul_secp256k1(
            &k1k2,
            &G,
            #[cfg(feature = "hints")]
            hints,
        );
    }
    // From here on, at least one of k1 or k2 is greater than 1, and P is not +-G.

    // 1. GLV-decompose k1 and k2.
    let (a1, a2, sigma_a1, sigma_a2) = glv_decompose_fn_secp256k1(
        &k1,
        #[cfg(feature = "hints")]
        hints,
    );
    let (b1, b2, sigma_b1, sigma_b2) = glv_decompose_fn_secp256k1(
        &k2,
        #[cfg(feature = "hints")]
        hints,
    );

    // 2. Build the four sign-adjusted base points: ±G, ±φ(G), ±p, ±φ(p).
    let base_g = if sigma_a1 == 0 { G_POINT } else { G_NEG_POINT };
    let base_g_phi = if sigma_a2 == 0 { G_PHI_POINT } else { G_PHI_NEG_POINT };
    let p_point = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
    let p_phi = phi_secp256k1(
        &p_point,
        #[cfg(feature = "hints")]
        hints,
    );
    let base_p = if sigma_b1 == 0 {
        p_point
    } else {
        neg_secp256k1(
            &p_point,
            #[cfg(feature = "hints")]
            hints,
        )
    };
    let base_p_phi = if sigma_b2 == 0 {
        p_phi
    } else {
        neg_secp256k1(
            &p_phi,
            #[cfg(feature = "hints")]
            hints,
        )
    };

    // 3. Build the 11 non-trivial multi-base sums.
    //    Bit layout: bit0=base_g, bit1=base_g_phi, bit2=base_p, bit3=base_p_phi.
    let mut t_0011 = base_g; // 0b0011 = base_g + base_g_phi
    let t_0011_is_inf = add_non_infinity_points_secp256k1(
        &mut t_0011,
        &base_g_phi,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t_0101 = base_g; // 0b0101 = base_g + base_p
    let t_0101_is_inf = add_non_infinity_points_secp256k1(
        &mut t_0101,
        &base_p,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t_0110 = base_g_phi; // 0b0110 = base_g_phi + base_p
    let t_0110_is_inf = add_non_infinity_points_secp256k1(
        &mut t_0110,
        &base_p,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t_0111 = t_0110; // 0b0111 = t_0110 + base_g
    let t_0111_is_inf = if t_0110_is_inf {
        // t_0110 = 𝒪 ⇒ t_0111 = base_g
        t_0111 = base_g;
        false
    } else {
        add_non_infinity_points_secp256k1(
            &mut t_0111,
            &base_g,
            #[cfg(feature = "hints")]
            hints,
        )
    };
    let mut t_1001 = base_g; // 0b1001 = base_g + base_p_phi
    let t_1001_is_inf = add_non_infinity_points_secp256k1(
        &mut t_1001,
        &base_p_phi,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t_1010 = base_g_phi; // 0b1010 = t_0010 + base_p_phi
    let t_1010_is_inf = add_non_infinity_points_secp256k1(
        &mut t_1010,
        &base_p_phi,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t_1011 = t_1010; // 0b1011 = t_1010 + base_g
    let t_1011_is_inf = if t_1010_is_inf {
        t_1011 = base_g;
        false
    } else {
        add_non_infinity_points_secp256k1(
            &mut t_1011,
            &base_g,
            #[cfg(feature = "hints")]
            hints,
        )
    };
    let mut t_1100 = base_p; // 0b1100 = t_0100 + base_p_phi
    let t_1100_is_inf = add_non_infinity_points_secp256k1(
        &mut t_1100,
        &base_p_phi,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut t_1101 = t_1100; // 0b1101 = t_1100 + base_g
    let t_1101_is_inf = if t_1100_is_inf {
        t_1101 = base_g;
        false
    } else {
        add_non_infinity_points_secp256k1(
            &mut t_1101,
            &base_g,
            #[cfg(feature = "hints")]
            hints,
        )
    };
    let mut t_1110 = t_1100; // 0b1110 = t_1100 + base_g_phi
    let t_1110_is_inf = if t_1100_is_inf {
        t_1110 = base_g_phi;
        false
    } else {
        add_non_infinity_points_secp256k1(
            &mut t_1110,
            &base_g_phi,
            #[cfg(feature = "hints")]
            hints,
        )
    };
    let mut t_1111 = t_1110; // 0b1111 = t_1110 + base_g
    let t_1111_is_inf = if t_1110_is_inf {
        t_1111 = base_g;
        false
    } else {
        add_non_infinity_points_secp256k1(
            &mut t_1111,
            &base_g,
            #[cfg(feature = "hints")]
            hints,
        )
    };

    // 4. Hint the position of the most significant set bit across (a1, a2, b1, b2).
    //    At the hinted position at least one of the four half-scalars must have a 1 bit;
    //    the loop reconstructs each half-scalar bit-by-bit and asserts the
    //    recomposition matches the GLV-decomposed values.
    let (max_limb, max_bit) = fcall_msb_pos_256_4(
        &a1,
        &a2,
        &b1,
        &b2,
        #[cfg(feature = "hints")]
        hints,
    );
    // Bound before use as index/shift
    assert!(max_limb < 4 && max_bit < 64, "msb_pos hint out of range");

    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    let a1_top = (a1[max_limb] >> max_bit) & 1;
    let a2_top = (a2[max_limb] >> max_bit) & 1;
    let b1_top = (b1[max_limb] >> max_bit) & 1;
    let b2_top = (b2[max_limb] >> max_bit) & 1;
    assert!(
        a1_top == 1 || a2_top == 1 || b1_top == 1 || b2_top == 1,
        "At least one of the half-scalars must have its top bit set"
    );

    // 5. Strauss-Shamir loop with bit-by-bit reconstruction of each half-scalar.
    let mut res = IDENTITY_POINT;
    let mut res_is_infinity = true;
    let mut a1_rec = ZERO_256;
    let mut a2_rec = ZERO_256;
    let mut b1_rec = ZERO_256;
    let mut b2_rec = ZERO_256;

    // Helper macros to add a point to the accumulator
    macro_rules! add_pt {
        ($pt:expr) => {{
            if res_is_infinity {
                res = $pt;
                res_is_infinity = false;
            } else {
                res_is_infinity = add_non_infinity_points_secp256k1(
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

    // Perform the loop, based on the binary representation of a1, a2, b1, b2.
    let mut start_bit = max_bit;
    for i in (0..=max_limb).rev() {
        let a1_word = a1[i];
        let a2_word = a2[i];
        let b1_word = b1[i];
        let b2_word = b2[i];
        let mut a1_rec_word = 0u64;
        let mut a2_rec_word = 0u64;
        let mut b1_rec_word = 0u64;
        let mut b2_rec_word = 0u64;

        for j in (0..=start_bit).rev() {
            let a1_bit = (a1_word >> j) & 1;
            let a2_bit = (a2_word >> j) & 1;
            let b1_bit = (b1_word >> j) & 1;
            let b2_bit = (b2_word >> j) & 1;
            let one_j: u64 = 1 << j;

            // Double first (a no-op while res is still 𝒪).
            if !res_is_infinity {
                syscall_secp256k1_dbl(
                    &mut res,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }

            match (a1_bit, a2_bit, b1_bit, b2_bit) {
                (0, 0, 0, 0) => {}
                (1, 0, 0, 0) => {
                    add_pt!(base_g);
                    a1_rec_word |= one_j;
                }
                (0, 1, 0, 0) => {
                    add_pt!(base_g_phi);
                    a2_rec_word |= one_j;
                }
                (1, 1, 0, 0) => {
                    add_pt_if_not_inf!(t_0011, t_0011_is_inf);
                    a1_rec_word |= one_j;
                    a2_rec_word |= one_j;
                }
                (0, 0, 1, 0) => {
                    add_pt!(base_p);
                    b1_rec_word |= one_j;
                }
                (1, 0, 1, 0) => {
                    add_pt_if_not_inf!(t_0101, t_0101_is_inf);
                    a1_rec_word |= one_j;
                    b1_rec_word |= one_j;
                }
                (0, 1, 1, 0) => {
                    add_pt_if_not_inf!(t_0110, t_0110_is_inf);
                    a2_rec_word |= one_j;
                    b1_rec_word |= one_j;
                }
                (1, 1, 1, 0) => {
                    add_pt_if_not_inf!(t_0111, t_0111_is_inf);
                    a1_rec_word |= one_j;
                    a2_rec_word |= one_j;
                    b1_rec_word |= one_j;
                }
                (0, 0, 0, 1) => {
                    add_pt!(base_p_phi);
                    b2_rec_word |= one_j;
                }
                (1, 0, 0, 1) => {
                    add_pt_if_not_inf!(t_1001, t_1001_is_inf);
                    a1_rec_word |= one_j;
                    b2_rec_word |= one_j;
                }
                (0, 1, 0, 1) => {
                    add_pt_if_not_inf!(t_1010, t_1010_is_inf);
                    a2_rec_word |= one_j;
                    b2_rec_word |= one_j;
                }
                (1, 1, 0, 1) => {
                    add_pt_if_not_inf!(t_1011, t_1011_is_inf);
                    a1_rec_word |= one_j;
                    a2_rec_word |= one_j;
                    b2_rec_word |= one_j;
                }
                (0, 0, 1, 1) => {
                    add_pt_if_not_inf!(t_1100, t_1100_is_inf);
                    b1_rec_word |= one_j;
                    b2_rec_word |= one_j;
                }
                (1, 0, 1, 1) => {
                    add_pt_if_not_inf!(t_1101, t_1101_is_inf);
                    a1_rec_word |= one_j;
                    b1_rec_word |= one_j;
                    b2_rec_word |= one_j;
                }
                (0, 1, 1, 1) => {
                    add_pt_if_not_inf!(t_1110, t_1110_is_inf);
                    a2_rec_word |= one_j;
                    b1_rec_word |= one_j;
                    b2_rec_word |= one_j;
                }
                (1, 1, 1, 1) => {
                    add_pt_if_not_inf!(t_1111, t_1111_is_inf);
                    a1_rec_word |= one_j;
                    a2_rec_word |= one_j;
                    b1_rec_word |= one_j;
                    b2_rec_word |= one_j;
                }
                _ => unreachable!(),
            }
        }

        a1_rec[i] = a1_rec_word;
        a2_rec[i] = a2_rec_word;
        b1_rec[i] = b1_rec_word;
        b2_rec[i] = b2_rec_word;
        start_bit = 63;
    }

    // Soundness: the reconstructed scalars must match the GLV-decomposed ones.
    assert!(eq(&a1_rec, &a1));
    assert!(eq(&a2_rec, &a2));
    assert!(eq(&b1_rec, &b1));
    assert!(eq(&b2_rec, &b2));

    if res_is_infinity {
        None
    } else {
        Some([res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3]])
    }
}

/// GLV-accelerated multi-scalar multiplication: Σᵢ kᵢ·Pᵢ.
///
/// Returns `None` if the result is the point at infinity. Assumes all input points are
/// non-infinity and on the curve, and scalars are in `[0, N-1]`.
///
/// # Soundness
/// All points must be on-curve, non-identity, and have **canonical** coordinates (`x, y < p`).
pub(crate) fn glv_msm_secp256k1(
    scalars: &[[u64; 4]],
    points: &[[u64; 8]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    // Each input pair `(kᵢ, Pᵢ)` is expanded into two pairs `(kᵢ₁, ±Pᵢ)` and `(kᵢ₂, ±φ(Pᵢ))`
    // using the GLV decomposition. The resulting `2n` pairs are then fed into Pippenger's bucket
    // method running over only the bottom 128 bits.

    let n = scalars.len();
    assert_eq!(n, points.len());
    if n == 0 {
        return None;
    }

    // Expand each (k, P) into ((k1, ±P), (k2, ±φ(P))).
    let mut expanded_scalars: Vec<[u64; 4]> = Vec::with_capacity(2 * n);
    let mut expanded_points: Vec<[u64; 8]> = Vec::with_capacity(2 * n);
    for i in 0..n {
        // Reduce the scalar first
        let k = reduce_fn_secp256k1(
            &scalars[i],
            #[cfg(feature = "hints")]
            hints,
        );

        let (k1, k2, sigma_1, sigma_2) = glv_decompose_fn_secp256k1(
            &k,
            #[cfg(feature = "hints")]
            hints,
        );

        let p_point = SyscallPoint256 {
            x: [points[i][0], points[i][1], points[i][2], points[i][3]],
            y: [points[i][4], points[i][5], points[i][6], points[i][7]],
        };
        let p_phi = phi_secp256k1(
            &p_point,
            #[cfg(feature = "hints")]
            hints,
        );

        let base_p = if sigma_1 == 0 {
            p_point
        } else {
            neg_secp256k1(
                &p_point,
                #[cfg(feature = "hints")]
                hints,
            )
        };
        let base_p_phi = if sigma_2 == 0 {
            p_phi
        } else {
            neg_secp256k1(
                &p_phi,
                #[cfg(feature = "hints")]
                hints,
            )
        };

        expanded_scalars.push(k1);
        expanded_points.push([
            base_p.x[0],
            base_p.x[1],
            base_p.x[2],
            base_p.x[3],
            base_p.y[0],
            base_p.y[1],
            base_p.y[2],
            base_p.y[3],
        ]);
        expanded_scalars.push(k2);
        expanded_points.push([
            base_p_phi.x[0],
            base_p_phi.x[1],
            base_p_phi.x[2],
            base_p_phi.x[3],
            base_p_phi.y[0],
            base_p_phi.y[1],
            base_p_phi.y[2],
            base_p_phi.y[3],
        ]);
    }

    // Pippenger over the bottom 128 bits.
    msm_secp256k1_max_bits(
        &expanded_scalars,
        &expanded_points,
        128,
        #[cfg(feature = "hints")]
        hints,
    )
}
