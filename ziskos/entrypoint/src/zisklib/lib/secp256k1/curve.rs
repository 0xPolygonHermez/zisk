use std::hint;

use crate::{
    syscalls::{
        syscall_secp256k1_add, syscall_secp256k1_dbl, SyscallPoint256, SyscallSecp256k1AddParams,
    },
    zisklib::{eq, fcall_msb_pos_256, fcall_msb_pos_256_3, is_one, ONE_256, TWO_256, ZERO_256},
};

use super::{
    constants::{E_B, G, G_X, G_Y, IDENTITY_X, IDENTITY_Y},
    field::{
        secp256k1_fp_add, secp256k1_fp_inv, secp256k1_fp_mul, secp256k1_fp_sqrt,
        secp256k1_fp_square,
    },
    scalar::{secp256k1_fn_inv, secp256k1_fn_mul, secp256k1_fn_reduce, secp256k1_fn_sub},
};

const IDENTITY_POINT256: SyscallPoint256 = SyscallPoint256 { x: IDENTITY_X, y: IDENTITY_Y };

const G_POINT256: SyscallPoint256 = SyscallPoint256 { x: G_X, y: G_Y };

/// Given a x-coordinate `x_bytes` and a parity `y_is_odd`,
/// this function decompresses the point on the secp256k1 curve.
pub fn secp256k1_decompress(
    x: &[u64; 4],
    y_is_odd: bool,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Result<([u64; 4], [u64; 4]), bool> {
    // Calculate the y-coordinate of the point: y = sqrt(x³ + 7)
    let x_sq = secp256k1_fp_square(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cb = secp256k1_fp_mul(
        &x_sq,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_sq = secp256k1_fp_add(
        &x_cb,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    let (y, has_sqrt) = secp256k1_fp_sqrt(
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

    Ok((*x, y))
}

/// Checks whether the given point `p` is on the Secp256k1 curve.
/// It assumes that `p` is not the point at infinity.
pub fn secp256k1_is_on_curve(p: &[u64; 8], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> bool {
    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    // p in E iff y² == x³ + 7
    let lhs = secp256k1_fp_square(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut rhs = secp256k1_fp_square(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = secp256k1_fp_mul(
        &rhs,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = secp256k1_fp_add(
        &rhs,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&lhs, &rhs)
}

/// Given points `p1` and `p2`, performs the point addition `p1 + p2` and assigns the result to `p1`.
/// It assumes that `p1` and `p2` are from the Secp256k1 curve, that `p1,p2 != 𝒪`
/// Returns true if the result is the point at infinity.
#[inline]
fn secp256k1_add_non_infinity_points(
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

/// Given a non-infinity point `p` and a scalar `k`, computes the scalar multiplication `k·p`
///
/// Note: There are no (non-infinity) points of order 2 in Secp256k1.
///       All (non-infinity) points are of prime order N.
pub fn secp256k1_scalar_mul(
    k: &[u64; 4],
    p: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    // Direct cases: k = 0, k = 1, k = 2
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
    // We can assume k > 2 from now on

    // Hint the length the binary representations of k
    // We will verify the output by recomposing k
    // Moreover, we should check that the first received bit is 1
    let (max_limb, max_bit) = fcall_msb_pos_256(
        k,
        &ZERO_256,
        #[cfg(feature = "hints")]
        hints,
    );

    // Perform the loop, based on the binary representation of k

    // We do the first iteration separately
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    // The first received bit should be 1
    assert_eq!((k[max_limb] >> max_bit) & 1, 1);

    // Start at P
    let mut res = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
    let mut k_rec = ZERO_256;
    k_rec[max_limb] = 1 << max_bit;

    // Determine starting limb/bit for the loop
    let mut limb = max_limb;
    let mut bit = if max_bit == 0 {
        // If max_bit is 0 then limb > 0; otherwise k = 1, which is excluded here
        limb -= 1;
        63
    } else {
        max_bit - 1
    };

    // Perform the rest of the loop
    let p = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
    for i in (0..=limb).rev() {
        for j in (0..=bit).rev() {
            // Always double
            syscall_secp256k1_dbl(
                &mut res,
                #[cfg(feature = "hints")]
                hints,
            );

            // Get the next bit b of k.
            // If b == 1, we should add P
            if ((k[i] >> j) & 1) == 1 {
                let mut params = SyscallSecp256k1AddParams { p1: &mut res, p2: &p };
                syscall_secp256k1_add(
                    &mut params,
                    #[cfg(feature = "hints")]
                    hints,
                );

                // Reconstruct k
                k_rec[i] |= 1 << j;
            }
        }
        bit = 63;
    }

    // Check that the reconstructed k is equal to the input k
    assert!(eq(&k_rec, k));

    // Convert the result back to a single array
    Some([res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3]])
}

/// Given a point `p` and scalars `k1` and `k2`, computes the double scalar multiplication `k1·G + k2·p`
/// It assumes that `k1,k2 ∈ [1, N-1]` and that `p != 𝒪`
pub fn secp256k1_double_scalar_mul_with_g(
    k1: &[u64; 4],
    k2: &[u64; 4],
    p: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    let p = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };

    // Start by precomputing g + p
    let mut gp = G_POINT256;
    let gp_is_infinity = secp256k1_add_non_infinity_points(
        &mut gp,
        &p,
        #[cfg(feature = "hints")]
        hints,
    );

    // If G + P = 𝒪 => P = -G and therefore the operation is k1·G + (-k2)·G = (k1-k2)·G
    // Fall back to scalar mul
    if gp_is_infinity {
        return secp256k1_scalar_mul(
            &secp256k1_fn_sub(
                k1,
                k2,
                #[cfg(feature = "hints")]
                hints,
            ),
            &G,
            #[cfg(feature = "hints")]
            hints,
        );
    }

    if is_one(k1) && is_one(k2) {
        // Return g + p
        return Some([gp.x[0], gp.x[1], gp.x[2], gp.x[3], gp.y[0], gp.y[1], gp.y[2], gp.y[3]]);
    }
    // From here on, at least one of k1 or k2 is greater than 1

    // Hint the maximum length between the binary representations of k1 and k2
    // We will verify the output by recomposing both k1 and k2
    // Moreover, we should check that the first received bit (of either k1 or k2) is 1
    let (max_limb, max_bit) = fcall_msb_pos_256(
        k1,
        k2,
        #[cfg(feature = "hints")]
        hints,
    );

    // Perform the loop, based on the binary representation of k1 and k2

    // We do the first iteration separately
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    // At least one of the scalars should have the first received bit as 1
    let k1_bit = (k1[max_limb] >> max_bit) & 1;
    let k2_bit = (k2[max_limb] >> max_bit) & 1;
    assert!(k1_bit == 1 || k2_bit == 1);

    // Start at 𝒪
    let mut res = IDENTITY_POINT256;
    let mut res_is_infinity = true;
    let mut k1_rec = ZERO_256;
    let mut k2_rec = ZERO_256;

    // Three cases based on the bits of k1 and k2
    match (k1_bit, k2_bit) {
        (0, 1) => {
            // Set res = p
            res.x = p.x;
            res.y = p.y;
            res_is_infinity = false;

            // Update k2_rec
            k2_rec[max_limb] = 1 << max_bit;
        }
        (1, 0) => {
            // Set res = g
            res.x = G_POINT256.x;
            res.y = G_POINT256.y;
            res_is_infinity = false;

            // Update k1_rec
            k1_rec[max_limb] = 1 << max_bit;
        }
        (1, 1) => {
            // Set res = g + p if not infinity
            if !gp_is_infinity {
                res.x = gp.x;
                res.y = gp.y;
                res_is_infinity = false;
            }

            // Update k1_rec and k2_rec
            k1_rec[max_limb] = 1 << max_bit;
            k2_rec[max_limb] = 1 << max_bit;
        }
        _ => unreachable!(),
    }

    // Determine starting limb/bit for the loop
    let mut limb = max_limb;
    let mut bit = if max_bit == 0 {
        // If max_bit is 0 then limb > 0; otherwise k1,k2 = 1, which is excluded here
        limb -= 1;
        63
    } else {
        max_bit - 1
    };

    // Perform the rest of the loop
    for i in (0..=limb).rev() {
        for j in (0..=bit).rev() {
            let k1_bit = (k1[i] >> j) & 1;
            let k2_bit = (k2[i] >> j) & 1;

            // Four cases based on the bits of k1 and k2
            match (k1_bit, k2_bit) {
                (0, 0) => {
                    // If res is 𝒪, do nothing; otherwise, double
                    if !res_is_infinity {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                    }
                }
                (0, 1) => {
                    // If res is 𝒪, set res = p; otherwise, double res and add p
                    if res_is_infinity {
                        res.x = p.x;
                        res.y = p.y;
                        res_is_infinity = false;
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        res_is_infinity = secp256k1_add_non_infinity_points(
                            &mut res,
                            &p,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                    }

                    // Update k2_rec
                    k2_rec[i] |= 1 << j;
                }
                (1, 0) => {
                    // If res is 𝒪, set res = g; otherwise, double res and add g
                    if res_is_infinity {
                        res.x = G_POINT256.x;
                        res.y = G_POINT256.y;
                        res_is_infinity = false;
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        res_is_infinity = secp256k1_add_non_infinity_points(
                            &mut res,
                            &G_POINT256,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                    }

                    // Update k1_rec
                    k1_rec[i] |= 1 << j;
                }
                (1, 1) => {
                    // If res is 𝒪, set res = g + p if not infinity; otherwise, double res and add (g + p)
                    if res_is_infinity {
                        if !gp_is_infinity {
                            res.x = gp.x;
                            res.y = gp.y;
                            res_is_infinity = false;
                        }
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        if !gp_is_infinity {
                            res_is_infinity = secp256k1_add_non_infinity_points(
                                &mut res,
                                &gp,
                                #[cfg(feature = "hints")]
                                hints,
                            );
                        }
                    }

                    // Update k1_rec and k2_rec
                    k1_rec[i] |= 1 << j;
                    k2_rec[i] |= 1 << j;
                }
                _ => unreachable!(),
            }
        }
        bit = 63;
    }

    // Check that the recomposed scalars are the same as the received scalars
    assert!(eq(&k1_rec, k1));
    assert!(eq(&k2_rec, k2));

    if res_is_infinity {
        None
    } else {
        Some([res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3]])
    }
}

/// Given two points `p` and `q` and scalars `r`, `s`, and `t`, computes the triple scalar multiplication `r·g + s·p + t·q`
/// It assumes that `r,s,t ∈ [1, N-1]` and that `p,q != 𝒪`
pub fn secp256k1_triple_scalar_mul_with_g(
    r: &[u64; 4],
    s: &[u64; 4],
    t: &[u64; 4],
    p: &[u64; 8],
    q: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> Option<[u64; 8]> {
    let p = SyscallPoint256 { x: [p[0], p[1], p[2], p[3]], y: [p[4], p[5], p[6], p[7]] };
    let q = SyscallPoint256 { x: [q[0], q[1], q[2], q[3]], y: [q[4], q[5], q[6], q[7]] };

    // Precompute g + p, g + q, p + q, g + p + q
    let mut gp = G_POINT256;
    let gp_is_infinity = secp256k1_add_non_infinity_points(
        &mut gp,
        &p,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut gq = G_POINT256;
    let gq_is_infinity = secp256k1_add_non_infinity_points(
        &mut gq,
        &q,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut pq = SyscallPoint256 { x: p.x, y: p.y };
    let pq_is_infinity = secp256k1_add_non_infinity_points(
        &mut pq,
        &q,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut gpq = SyscallPoint256 { x: gp.x, y: gp.y };
    let gpq_is_infinity = secp256k1_add_non_infinity_points(
        &mut gpq,
        &q,
        #[cfg(feature = "hints")]
        hints,
    );

    if is_one(r) && is_one(s) && is_one(t) {
        // Return g + p + q
        if gpq_is_infinity {
            return None;
        } else {
            return Some([
                gpq.x[0], gpq.x[1], gpq.x[2], gpq.x[3], gpq.y[0], gpq.y[1], gpq.y[2], gpq.y[3],
            ]);
        }
    }
    // From here on, at least one of r,s,t is greater than 1

    // Hint the maximum length between the binary representations of r,s and t
    let (max_limb, max_bit) = fcall_msb_pos_256_3(
        r,
        s,
        t,
        #[cfg(feature = "hints")]
        hints,
    );

    // Perform the loop, based on the binary representation of r,s and t

    // We do the first iteration separately
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    // At least one of the scalars should have the first received bit as 1
    let r_bit = (r[max_limb] >> max_bit) & 1;
    let s_bit = (s[max_limb] >> max_bit) & 1;
    let t_bit = (t[max_limb] >> max_bit) & 1;
    assert!(r_bit == 1 || s_bit == 1 || t_bit == 1);

    // Start at 𝒪
    let mut res = IDENTITY_POINT256;
    let mut res_is_infinity = true;
    let mut r_rec = ZERO_256;
    let mut s_rec = ZERO_256;
    let mut t_rec = ZERO_256;

    // Eight cases based on the bits of r,s and t
    match (r_bit, s_bit, t_bit) {
        (0, 0, 1) => {
            // Set res = q
            res.x = q.x;
            res.y = q.y;
            res_is_infinity = false;

            // Update t_rec
            t_rec[max_limb] = 1 << max_bit;
        }
        (0, 1, 0) => {
            // Set res = p
            res.x = p.x;
            res.y = p.y;
            res_is_infinity = false;

            // Update s_rec
            s_rec[max_limb] = 1 << max_bit;
        }
        (0, 1, 1) => {
            // Set res = p + q if not infinity
            if !pq_is_infinity {
                res.x = pq.x;
                res.y = pq.y;
                res_is_infinity = false;
            }

            // Update s_rec and t_rec
            s_rec[max_limb] = 1 << max_bit;
            t_rec[max_limb] = 1 << max_bit;
        }
        (1, 0, 0) => {
            // Set res = g
            res.x = G_POINT256.x;
            res.y = G_POINT256.y;
            res_is_infinity = false;

            // Update r_rec
            r_rec[max_limb] = 1 << max_bit;
        }
        (1, 0, 1) => {
            // Set res = g + q if not infinity
            if !gq_is_infinity {
                res.x = gq.x;
                res.y = gq.y;
                res_is_infinity = false;
            }

            // Update r_rec and t_rec
            r_rec[max_limb] = 1 << max_bit;
            t_rec[max_limb] = 1 << max_bit;
        }
        (1, 1, 0) => {
            // Set res = g + p if not infinity
            if !gp_is_infinity {
                res.x = gp.x;
                res.y = gp.y;
                res_is_infinity = false;
            }

            // Update r_rec and s_rec
            r_rec[max_limb] = 1 << max_bit;
            s_rec[max_limb] = 1 << max_bit;
        }
        (1, 1, 1) => {
            // Set res = g + p + q if not infinity
            if !gpq_is_infinity {
                res.x = gpq.x;
                res.y = gpq.y;
                res_is_infinity = false;
            }

            // Update r_rec, s_rec and t_rec
            r_rec[max_limb] = 1 << max_bit;
            s_rec[max_limb] = 1 << max_bit;
            t_rec[max_limb] = 1 << max_bit;
        }
        _ => unreachable!(),
    }

    // Determine starting limb/bit for the loop
    let mut limb = max_limb;
    let mut bit = if max_bit == 0 {
        // If max_bit is 0 then limb > 0; otherwise r,s,t = 1, which is excluded here
        limb -= 1;
        63
    } else {
        max_bit - 1
    };

    // Perform the rest of the loop
    for i in (0..=limb).rev() {
        for j in (0..=bit).rev() {
            let r_bit = (r[i] >> j) & 1;
            let s_bit = (s[i] >> j) & 1;
            let t_bit = (t[i] >> j) & 1;

            // Eight cases based on the bits of r,s and t
            match (r_bit, s_bit, t_bit) {
                (0, 0, 0) => {
                    // If res is 𝒪, do nothing; otherwise, double
                    if !res_is_infinity {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                    }
                }
                (0, 0, 1) => {
                    // If res is 𝒪, set res = q; otherwise, double res and add q
                    if res_is_infinity {
                        res.x = q.x;
                        res.y = q.y;
                        res_is_infinity = false;
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        res_is_infinity = secp256k1_add_non_infinity_points(
                            &mut res,
                            &q,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                    }

                    // Update t_rec
                    t_rec[i] |= 1 << j;
                }
                (0, 1, 0) => {
                    // If res is 𝒪, set res = p; otherwise, double res and add p
                    if res_is_infinity {
                        res.x = p.x;
                        res.y = p.y;
                        res_is_infinity = false;
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        res_is_infinity = secp256k1_add_non_infinity_points(
                            &mut res,
                            &p,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                    }

                    // Update s_rec
                    s_rec[i] |= 1 << j;
                }
                (0, 1, 1) => {
                    // If res is 𝒪, set res = p + q if not infinity; otherwise, double res and add (p + q)
                    if res_is_infinity {
                        if !pq_is_infinity {
                            res.x = pq.x;
                            res.y = pq.y;
                            res_is_infinity = false;
                        }
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        if !pq_is_infinity {
                            res_is_infinity = secp256k1_add_non_infinity_points(
                                &mut res,
                                &pq,
                                #[cfg(feature = "hints")]
                                hints,
                            );
                        }
                    }

                    // Update s_rec and t_rec
                    s_rec[i] |= 1 << j;
                    t_rec[i] |= 1 << j;
                }
                (1, 0, 0) => {
                    // If res is 𝒪, set res = g; otherwise, double res and add g
                    if res_is_infinity {
                        res.x = G_POINT256.x;
                        res.y = G_POINT256.y;
                        res_is_infinity = false;
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        res_is_infinity = secp256k1_add_non_infinity_points(
                            &mut res,
                            &G_POINT256,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                    }

                    // Update r_rec
                    r_rec[i] |= 1 << j;
                }
                (1, 0, 1) => {
                    // If res is 𝒪, set res = g + q if not infinity; otherwise, double res and add (g + q)
                    if res_is_infinity {
                        if !gq_is_infinity {
                            res.x = gq.x;
                            res.y = gq.y;
                            res_is_infinity = false;
                        }
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        if !gq_is_infinity {
                            res_is_infinity = secp256k1_add_non_infinity_points(
                                &mut res,
                                &gq,
                                #[cfg(feature = "hints")]
                                hints,
                            );
                        }
                    }

                    // Update r_rec and t_rec
                    r_rec[i] |= 1 << j;
                    t_rec[i] |= 1 << j;
                }
                (1, 1, 0) => {
                    // If res is 𝒪, set res = g + p if not infinity
                    if res_is_infinity {
                        if !gp_is_infinity {
                            res.x = gp.x;
                            res.y = gp.y;
                            res_is_infinity = false;
                        }
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        if !gp_is_infinity {
                            res_is_infinity = secp256k1_add_non_infinity_points(
                                &mut res,
                                &gp,
                                #[cfg(feature = "hints")]
                                hints,
                            );
                        }
                    }

                    // Update r_rec and s_rec
                    r_rec[i] |= 1 << j;
                    s_rec[i] |= 1 << j;
                }
                (1, 1, 1) => {
                    // If res is 𝒪, set res = g + p + q if not infinity; otherwise, double res and add (g + p + q)
                    if res_is_infinity {
                        if !gpq_is_infinity {
                            res.x = gpq.x;
                            res.y = gpq.y;
                            res_is_infinity = false;
                        }
                    } else {
                        syscall_secp256k1_dbl(
                            &mut res,
                            #[cfg(feature = "hints")]
                            hints,
                        );
                        if !gpq_is_infinity {
                            res_is_infinity = secp256k1_add_non_infinity_points(
                                &mut res,
                                &gpq,
                                #[cfg(feature = "hints")]
                                hints,
                            );
                        }
                    }

                    // Update r_rec, s_rec and t_rec
                    r_rec[i] |= 1 << j;
                    s_rec[i] |= 1 << j;
                    t_rec[i] |= 1 << j;
                }
                _ => unreachable!(),
            }
        }
        bit = 63;
    }

    // Check that the recomposed scalars are the same as the received scalars
    assert!(eq(&r_rec, r));
    assert!(eq(&s_rec, s));
    assert!(eq(&t_rec, t));

    if res_is_infinity {
        None
    } else {
        Some([res.x[0], res.x[1], res.x[2], res.x[3], res.y[0], res.y[1], res.y[2], res.y[3]])
    }
}
