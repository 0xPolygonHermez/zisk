extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

use crate::{
    syscalls::{
        syscall_secp256k1_add, syscall_secp256k1_dbl, SyscallPoint256, SyscallSecp256k1AddParams,
    },
    zisklib::{
        be_bytes_to_u64_4, eq, fcall_msb_pos_256, fcall_msb_pos_256_2, is_one, is_zero, ONE_256,
        TWO_256, ZERO_256,
    },
};

use super::{
    constants::{BETA, E_B, G, G_NEG_Y, G_X, G_Y, IDENTITY_X, IDENTITY_Y},
    field::{
        add_fp_secp256k1, inv_fp_secp256k1, mul_fp_secp256k1, neg_fp_secp256k1, sqrt_fp_secp256k1,
        square_fp_secp256k1,
    },
    scalar::{add_fn_secp256k1, sub_fn_secp256k1},
};

// Precomputed points
const IDENTITY_POINT: SyscallPoint256 = SyscallPoint256 { x: IDENTITY_X, y: IDENTITY_Y };
const G_POINT: SyscallPoint256 = SyscallPoint256 { x: G_X, y: G_Y };

/// Converts a non-infinity point `p` on the Secp256k1 curve from jacobian coordinates to affine coordinates
pub fn jacobian_to_affine_secp256k1(
    p: &[u64; 12],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 8] {
    let z: [u64; 4] = [p[8], p[9], p[10], p[11]];

    // Point at infinity cannot be converted to affine
    debug_assert!(z != ZERO_256, "Cannot convert point at infinity to affine");

    let zinv = inv_fp_secp256k1(
        &z,
        #[cfg(feature = "hints")]
        hints,
    );
    let zinv_sq = square_fp_secp256k1(
        &zinv,
        #[cfg(feature = "hints")]
        hints,
    );

    let x: [u64; 4] = [p[0], p[1], p[2], p[3]];
    let y: [u64; 4] = [p[4], p[5], p[6], p[7]];

    let x_res = mul_fp_secp256k1(
        &x,
        &zinv_sq,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_res = mul_fp_secp256k1(
        &mul_fp_secp256k1(
            &y,
            &zinv_sq,
            #[cfg(feature = "hints")]
            hints,
        ),
        &zinv,
        #[cfg(feature = "hints")]
        hints,
    );

    [x_res[0], x_res[1], x_res[2], x_res[3], y_res[0], y_res[1], y_res[2], y_res[3]]
}

/// Given a x-coordinate and a parity bit, returns the corresponding point (x, y) on the curve if it exists
pub fn lift_x_secp256k1(
    x: &[u64; 4],
    y_is_odd: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<[u64; 8], bool> {
    // Calculate the y-coordinate of the point: y = sqrt(x³ + 7)
    let x_sq = square_fp_secp256k1(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cb = mul_fp_secp256k1(
        &x_sq,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_sq = add_fp_secp256k1(
        &x_cb,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    let (y, has_sqrt) = sqrt_fp_secp256k1(
        &y_sq,
        y_is_odd as u64,
        #[cfg(feature = "hints")]
        hints,
    );

    if !has_sqrt {
        return Err(false);
    }

    // Check the received parity of the y-coordinate is correct
    let parity = (y[0] & 1) != 0;
    assert_eq!(parity, y_is_odd);

    Ok([x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]])
}

/// Checks whether the given point `p` is on the Secp256k1 curve.
/// It assumes that `p` is not the point at infinity.
pub fn is_on_curve_secp256k1(p: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> bool {
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    // p in E iff y² == x³ + 7
    let lhs = square_fp_secp256k1(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut rhs = square_fp_secp256k1(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = mul_fp_secp256k1(
        &rhs,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = add_fp_secp256k1(
        &rhs,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&lhs, &rhs)
}

/// Applies the secp256k1 GLV endomorphism `φ : (x, y) ↦ (β·x, y)` to a point.
/// `φ(P) = [λ]P` for any point `P` of order `n`.
#[inline]
pub(crate) fn phi_secp256k1(
    p: &SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> SyscallPoint256 {
    let beta_x = mul_fp_secp256k1(
        &BETA,
        &[p.x[0], p.x[1], p.x[2], p.x[3]],
        #[cfg(feature = "hints")]
        hints,
    );
    SyscallPoint256 { x: beta_x, y: [p.y[0], p.y[1], p.y[2], p.y[3]] }
}

/// Negates a point on the secp256k1 curve.
/// The identity point is mapped to itself.
#[inline]
pub(crate) fn neg_secp256k1(
    p: &SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> SyscallPoint256 {
    SyscallPoint256 {
        x: p.x,
        y: neg_fp_secp256k1(
            &p.y,
            #[cfg(feature = "hints")]
            hints,
        ),
    }
}

/// Given points `p1` and `p2`, performs the point addition `p1 + p2` and assigns the result to `p1`.
/// It assumes that `p1` and `p2` are from the Secp256k1 curve, that `p1,p2 != 𝒪`
/// Returns true if the result is the point at infinity.
#[inline]
pub(crate) fn add_non_infinity_points_secp256k1(
    p1: &mut SyscallPoint256,
    p2: &SyscallPoint256,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    if p1.x != p2.x {
        let mut params = SyscallSecp256k1AddParams { p1, p2 };
        syscall_secp256k1_add(
            &mut params,
            #[cfg(feature = "hints")]
            hints,
        );
        false
    } else if p1.y == p2.y {
        syscall_secp256k1_dbl(
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

/// Adds two points on the secp256k1 curve. Assumes both are non-infinity.
/// Returns None if the result is the point at infinity.
pub fn point_add_secp256k1(
    p1: &[u64; 8],
    p2: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    let mut r =
        SyscallPoint256 { x: [p1[0], p1[1], p1[2], p1[3]], y: [p1[4], p1[5], p1[6], p1[7]] };
    let q = SyscallPoint256 { x: [p2[0], p2[1], p2[2], p2[3]], y: [p2[4], p2[5], p2[6], p2[7]] };
    let is_inf = add_non_infinity_points_secp256k1(
        &mut r,
        &q,
        #[cfg(feature = "hints")]
        hints,
    );
    if is_inf {
        None
    } else {
        Some([r.x[0], r.x[1], r.x[2], r.x[3], r.y[0], r.y[1], r.y[2], r.y[3]])
    }
}

/// Given a non-infinity point `p` and a scalar `k`, computes the scalar multiplication `k·p`
///
/// Note: There are no (non-infinity) points of order 2 in Secp256k1.
///       All (non-infinity) points are of prime order N.
pub fn scalar_mul_secp256k1(
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
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;
    let k_top = (k[max_limb] >> max_bit) & 1;
    assert!(k_top == 1);

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
                res_is_infinity = add_non_infinity_points_secp256k1(
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
                syscall_secp256k1_dbl(
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
pub fn double_scalar_mul_with_g_secp256k1(
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
            return scalar_mul_secp256k1(
                k2,
                p,
                #[cfg(feature = "hints")]
                hints,
            );
        }
        (false, true) => {
            return scalar_mul_secp256k1(
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
        let gp_is_infinity = add_non_infinity_points_secp256k1(
            &mut gp,
            &SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] },
            #[cfg(feature = "hints")]
            hints,
        );
        if gp_is_infinity {
            return None;
        }
        return scalar_mul_secp256k1(
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
            true => sub_fn_secp256k1(
                k1,
                k2,
                #[cfg(feature = "hints")]
                hints,
            ),
            false => add_fn_secp256k1(
                k1,
                k2,
                #[cfg(feature = "hints")]
                hints,
            ),
        };

        return scalar_mul_secp256k1(
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
    let gp_is_inf = add_non_infinity_points_secp256k1(
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
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;
    let k1_top = (k1[max_limb] >> max_bit) & 1;
    let k2_top = (k2[max_limb] >> max_bit) & 1;
    assert!(k1_top == 1 || k2_top == 1);

    // 3. Strauss-Shamir loop with bit-by-bit reconstruction of each scalar.
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

/// Multi-scalar multiplication using Pippenger's bucket method: Σ kᵢ·Pᵢ.
/// Returns None if the result is the point at infinity.
/// Assumes all points are non-infinity and on the curve. Scalars must be in [0, N-1].
pub fn multi_scalar_mul_secp256k1(
    scalars: &[[u64; 4]],
    points: &[[u64; 8]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    let n = scalars.len();
    assert_eq!(n, points.len());
    if n == 0 {
        return None;
    }

    let w = optimal_window_size(n);
    let num_buckets = (1usize << w) - 1;
    let num_windows = 256_usize.div_ceil(w);

    let mut result = IDENTITY_POINT;
    let mut result_is_inf = true;

    // Allocate buckets once, reset each window
    let mut buckets: Vec<SyscallPoint256> = Vec::with_capacity(num_buckets);
    let mut bucket_is_inf: Vec<bool> = vec![true; num_buckets];
    for _ in 0..num_buckets {
        buckets.push(SyscallPoint256 { x: IDENTITY_X, y: IDENTITY_Y });
    }

    // Process windows from most significant to least significant
    for window_idx in (0..num_windows).rev() {
        // Double the accumulator w times (combine with previous windows)
        if !result_is_inf {
            for _ in 0..w {
                syscall_secp256k1_dbl(
                    &mut result,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        }

        // Reset buckets
        for flag in bucket_is_inf.iter_mut() {
            *flag = true;
        }

        // Scatter: add each point to its bucket
        for i in 0..n {
            let win = get_scalar_window(&scalars[i], window_idx, w);
            if win == 0 {
                continue;
            }
            let bucket_idx = win as usize - 1;

            let p = SyscallPoint256 {
                x: [points[i][0], points[i][1], points[i][2], points[i][3]],
                y: [points[i][4], points[i][5], points[i][6], points[i][7]],
            };

            if bucket_is_inf[bucket_idx] {
                buckets[bucket_idx] = p;
                bucket_is_inf[bucket_idx] = false;
            } else {
                bucket_is_inf[bucket_idx] = add_non_infinity_points_secp256k1(
                    &mut buckets[bucket_idx],
                    &p,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        }

        // Aggregate buckets: compute Σ j·buckets[j]
        // running_sum accumulates from high to low; partial_sum accumulates running_sums.
        let mut running_sum = IDENTITY_POINT;
        let mut running_is_inf = true;
        let mut partial_sum = IDENTITY_POINT;
        let mut partial_is_inf = true;

        for j in (0..num_buckets).rev() {
            // running_sum += buckets[j]
            if !bucket_is_inf[j] {
                if running_is_inf {
                    running_sum = SyscallPoint256 { x: buckets[j].x, y: buckets[j].y };
                    running_is_inf = false;
                } else {
                    running_is_inf = add_non_infinity_points_secp256k1(
                        &mut running_sum,
                        &buckets[j],
                        #[cfg(feature = "hints")]
                        hints,
                    );
                }
            }

            // partial_sum += running_sum
            if !running_is_inf {
                if partial_is_inf {
                    partial_sum = SyscallPoint256 { x: running_sum.x, y: running_sum.y };
                    partial_is_inf = false;
                } else {
                    partial_is_inf = add_non_infinity_points_secp256k1(
                        &mut partial_sum,
                        &running_sum,
                        #[cfg(feature = "hints")]
                        hints,
                    );
                }
            }
        }

        // Add window contribution to result
        if !partial_is_inf {
            if result_is_inf {
                result = partial_sum;
                result_is_inf = false;
            } else {
                result_is_inf = add_non_infinity_points_secp256k1(
                    &mut result,
                    &partial_sum,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        }
    }

    if result_is_inf {
        None
    } else {
        Some([
            result.x[0],
            result.x[1],
            result.x[2],
            result.x[3],
            result.y[0],
            result.y[1],
            result.y[2],
            result.y[3],
        ])
    }
}

/// Extracts a `w`-bit window from a 256-bit scalar at the given window index.
/// Window 0 is the least significant.
fn get_scalar_window(scalar: &[u64; 4], window_idx: usize, w: usize) -> u64 {
    let bit_offset = window_idx * w;
    let limb_idx = bit_offset / 64;
    let bit_in_limb = bit_offset % 64;
    let mask = (1u64 << w) - 1;

    if limb_idx >= 4 {
        return 0;
    }

    let mut val = (scalar[limb_idx] >> bit_in_limb) & mask;

    if bit_in_limb + w > 64 && limb_idx + 1 < 4 {
        let remaining_bits = bit_in_limb + w - 64;
        val |= (scalar[limb_idx + 1] & ((1u64 << remaining_bits) - 1)) << (64 - bit_in_limb);
    }

    val
}

/// Chooses the Pippenger window size that minimizes total group operations for `n` points.
fn optimal_window_size(n: usize) -> usize {
    if n <= 1 {
        1
    } else if n <= 4 {
        2
    } else if n <= 8 {
        3
    } else if n <= 16 {
        4
    } else if n <= 64 {
        5
    } else if n <= 400 {
        6
    } else {
        7
    }
}

// ==================== C FFI Functions ====================

/// Lift an x-coordinate (32 big-endian bytes) to a secp256k1 point.
/// Writes the resulting point as `[u64; 8]` little-endian limbs (x ‖ y) to `result_ptr`.
/// Returns 1 on success, 0 if no point with that x-coordinate exists on the curve.
///
/// # Safety
/// - `x_ptr` must point to at least 32 bytes
/// - `result_ptr` must point to a writable `[u64; 8]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_lift_x_secp256k1_c")]
pub unsafe extern "C" fn lift_x_secp256k1_c(
    x_ptr: *const u8,
    y_is_odd: u8,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let x_bytes: &[u8; 32] = &*(x_ptr as *const [u8; 32]);
    let x = be_bytes_to_u64_4(x_bytes);

    match lift_x_secp256k1(
        &x,
        y_is_odd != 0,
        #[cfg(feature = "hints")]
        hints,
    ) {
        Err(_) => 0,
        Ok(point) => {
            let result = &mut *(result_ptr as *mut [u64; 8]);
            *result = point;
            1
        }
    }
}

/// Converts a non-infinity secp256k1 point from Jacobian `[u64; 12]` to affine `[u64; 8]`.
///
/// # Safety
/// - `p_ptr` must point to a valid `[u64; 12]` array (Jacobian coordinates, non-infinity)
/// - `result_ptr` must point to a writable `[u64; 8]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_jacobian_to_affine_secp256k1_c")]
pub unsafe extern "C" fn jacobian_to_affine_secp256k1_c(
    p_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let p = &*(p_ptr as *const [u64; 12]);
    let result = &mut *(result_ptr as *mut [u64; 8]);
    *result = jacobian_to_affine_secp256k1(
        p,
        #[cfg(feature = "hints")]
        hints,
    );
}

/// Computes `k1·G + k2·p` on the secp256k1 curve. Writes the result to `result_ptr`.
/// Returns 1 if the result is a finite point, 0 if it is the point at infinity.
///
/// # Safety
/// - `k1_ptr` must point to a valid `[u64; 4]` array
/// - `k2_ptr` must point to a valid `[u64; 4]` array
/// - `p_ptr` must point to a valid `[u64; 8]` array (non-infinity affine point)
/// - `result_ptr` must point to a writable `[u64; 8]` array
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_double_scalar_mul_with_g_secp256k1_c")]
pub unsafe extern "C" fn double_scalar_mul_with_g_secp256k1_c(
    k1_ptr: *const u64,
    k2_ptr: *const u64,
    p_ptr: *const u64,
    result_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> u8 {
    let k1 = &*(k1_ptr as *const [u64; 4]);
    let k2 = &*(k2_ptr as *const [u64; 4]);
    let p = &*(p_ptr as *const [u64; 8]);

    match double_scalar_mul_with_g_secp256k1(
        k1,
        k2,
        p,
        #[cfg(feature = "hints")]
        hints,
    ) {
        None => 0,
        Some(point) => {
            let result = &mut *(result_ptr as *mut [u64; 8]);
            *result = point;
            1
        }
    }
}
