use crate::{
    eq, fcall_msb_pos_256,
    point::SyscallPoint256,
    secp256k1_add::{syscall_secp256k1_add, SyscallSecp256k1AddParams},
    secp256k1_dbl::syscall_secp256k1_dbl,
};

use super::{
    constants::{E_B, G_X, G_Y},
    field::{
        secp256k1_fp_add, secp256k1_fp_inv, secp256k1_fp_mul, secp256k1_fp_sqrt,
        secp256k1_fp_square,
    },
    scalar::{secp256k1_fn_inv, secp256k1_fn_mul, secp256k1_fn_reduce},
};

/// Converts a point `p` on the Secp256k1 curve from projective coordinates to affine coordinates
pub fn secp256k1_to_affine(p: &[u64; 12]) -> Option<[u64; 8]> {
    let z: [u64; 4] = p[8..12].try_into().unwrap();

    // Check if p is the point at infinity
    if z == [0u64; 4] {
        // Point at infinity cannot be converted to affine
        return None;
    }

    // Check if p is already in affine coordinates
    if z == [1u64, 0, 0, 0] {
        return Some([p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]]);
    }

    let zinv = secp256k1_fp_inv(&z);
    let zinv_sq = secp256k1_fp_square(&zinv);

    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    let x_res = secp256k1_fp_mul(&x, &zinv_sq);
    let mut y_res = secp256k1_fp_mul(&y, &zinv_sq);
    y_res = secp256k1_fp_mul(&y_res, &zinv);

    Some([x_res[0], x_res[1], x_res[2], x_res[3], y_res[0], y_res[1], y_res[2], y_res[3]])
}

/// Checks if two points `p1` and `p2` on the Secp256k1 curve in projective coordinates are equal
pub fn secp256k1_eq_projective(p1: &[u64; 12], p2: &[u64; 12]) -> bool {
    // In essence given two points in projective form p1 = (x₁z₁,y₁z₁,z₁) and p2 = (x₂z₂,y₂z₂,z₂)
    // We can simply multiply p1 by z2 and p2 by z1 to get tuples:
    //  p1 = (x₁z₁z₂,y₁z₁z₂,z₁z₂) and p2 = (x₂z₂z₁,y₂z₂z₁,z₂z₁)
    // So we can compare the two points by checking if (x₁z₁)z₂ == (x₂z₁)z₂ and (y₁z₂)z₁ == (y₂z₂)z₁
    let x1 = p1[0..4].try_into().unwrap();
    let y1 = p1[4..8].try_into().unwrap();
    let z1 = p1[8..12].try_into().unwrap();
    let x2 = p2[0..4].try_into().unwrap();
    let y2 = p2[4..8].try_into().unwrap();
    let z2 = p2[8..12].try_into().unwrap();

    let lhs_x = secp256k1_fp_mul(x1, z2);
    let rhs_x = secp256k1_fp_mul(x2, z1);
    if !eq(&lhs_x, &rhs_x) {
        return false;
    }

    let lhs_y = secp256k1_fp_mul(y1, z2);
    let rhs_y = secp256k1_fp_mul(y2, z1);
    if !eq(&lhs_y, &rhs_y) {
        return false;
    }

    true
}

/// Given a x-coordinate `x_bytes` and a parity `y_is_odd`,
/// this function decompresses the point on the secp256k1 curve.
pub fn secp256k1_decompress(x_bytes: &[u8; 32], y_is_odd: bool) -> (([u64; 4], [u64; 4]), bool) {
    // Convert the x-coordinate from BEu8 to LEu64
    let mut x = [0u64; 4];
    for i in 0..32 {
        x[3 - i / 8] |= (x_bytes[i] as u64) << (8 * (7 - (i % 8)));
    }

    // Calculate the y-coordinate of the point: y = sqrt(x³ + 7)
    let x_sq = secp256k1_fp_square(&x);
    let x_cb = secp256k1_fp_mul(&x_sq, &x);
    let y_sq = secp256k1_fp_add(&x_cb, &E_B);
    let (y, has_sqrt) = secp256k1_fp_sqrt(&y_sq, y_is_odd as u64);
    if !has_sqrt {
        return (([0u64; 4], [0u64; 4]), false);
    }

    // Check the received parity of the y-coordinate is correct
    let parity = (y[0] & 1) != 0;
    assert_eq!(parity, y_is_odd);

    ((x, y), true)
}

/// Given points `p1` and `p2`, performs the point addition `p1 + p2` and assigns the result to `p1`.
/// It assumes that `p1` and `p2` are from the Secp256k1 curve, that `p1,p2 != 𝒪` and that `p2 != p1,-p1`
fn add_points_assign(p1: &mut SyscallPoint256, p2: &SyscallPoint256) {
    let mut params = SyscallSecp256k1AddParams { p1, p2 };
    syscall_secp256k1_add(&mut params);
}

/// Given a point `p1`, performs the point doubling `2·p1` and assigns the result to `p1`.
/// It assumes that `p1` is from the Secp256k1 curve and that `p1 != 𝒪`
///
/// Note: We don't need to assume that 2·p1 != 𝒪 because there are not points of order 2 on the Secp256k1 curve
fn double_point_assign(p1: &mut SyscallPoint256) {
    syscall_secp256k1_dbl(p1);
}

/// Given points `p1` and `p2`, performs the point addition `p1 + p2` and assigns the result to `p1`.
/// It assumes that `p1` and `p2` are from the Secp256k1 curve, that `p2 != 𝒪`
fn add_points_complete_assign(
    p1: &mut SyscallPoint256,
    p1_is_infinity: &mut bool,
    p2: &SyscallPoint256,
) {
    if p1.x != p2.x {
        add_points_assign(p1, p2);
    } else if p1.y == p2.y {
        double_point_assign(p1);
    } else {
        *p1_is_infinity = true;
    }
}

/// Given a point `p` and scalars `k1` and `k2`, computes the double scalar multiplication `k1·G + k2·p`
/// It assumes that `k1,k2 ∈ [1, N-1]` and that `p != 𝒪`
pub fn secp256k1_double_scalar_mul_with_g(
    k1: &[u64; 4],
    k2: &[u64; 4],
    p: &SyscallPoint256,
) -> (bool, SyscallPoint256) {
    // Start by precomputing g + p
    let mut gp = SyscallPoint256 { x: G_X, y: G_Y };
    let mut gp_is_infinity = false;
    add_points_complete_assign(&mut gp, &mut gp_is_infinity, p);

    // Hint the maximum length between the binary representations of k1 and k2
    // We will verify the output by recomposing both k1 and k2
    // Moreover, we should check that the first received bit (of either k1 or k2) is 1
    let (max_limb, max_bit) = fcall_msb_pos_256(k1, k2);

    // Perform the loop, based on the binary representation of k1 and k2
    // Start at 𝒪
    let mut res = SyscallPoint256 { x: [0u64; 4], y: [0u64; 4] };
    let mut res_is_infinity = true;
    let mut k1_rec = [0u64; 4];
    let mut k2_rec = [0u64; 4];
    // We do the first iteration separately
    let _max_limb = max_limb as usize;
    let k1_bit = (k1[_max_limb] >> max_bit) & 1;
    let k2_bit = (k2[_max_limb] >> max_bit) & 1;
    assert!(k1_bit == 1 || k2_bit == 1); // At least one of the scalars should start with 1
    if (k1_bit == 0) && (k2_bit == 1) {
        // If res is 𝒪, set res = p; otherwise, double res and add p
        if res_is_infinity {
            res.x = p.x;
            res.y = p.y;
            res_is_infinity = false;
        } else {
            double_point_assign(&mut res);
            add_points_complete_assign(&mut res, &mut res_is_infinity, p);
        }

        // Update k2_rec
        k2_rec[_max_limb] |= 1 << max_bit;
    } else if (k1_bit == 1) && (k2_bit == 0) {
        // If res is 𝒪, set res = g; otherwise, double res and add g
        if res_is_infinity {
            res.x = G_X;
            res.y = G_Y;
            res_is_infinity = false;
        } else {
            double_point_assign(&mut res);
            add_points_complete_assign(
                &mut res,
                &mut res_is_infinity,
                &SyscallPoint256 { x: G_X, y: G_Y },
            );
        }

        // Update k1_rec
        k1_rec[_max_limb] |= 1 << max_bit;
    } else if (k1_bit == 1) && (k2_bit == 1) {
        if res_is_infinity {
            // If (g + p) is 𝒪, do nothing; otherwise set res = (g + p)
            if !gp_is_infinity {
                res.x = gp.x;
                res.y = gp.y;
                res_is_infinity = false;
            }
        } else {
            // If (g + p) is 𝒪, simply double res; otherwise double res and add (g + p)
            double_point_assign(&mut res);
            if !gp_is_infinity {
                add_points_complete_assign(&mut res, &mut res_is_infinity, &gp);
            }
        }

        // Update k1_rec and k2_rec
        k1_rec[_max_limb] |= 1 << max_bit;
        k2_rec[_max_limb] |= 1 << max_bit;
    }

    // Perform the rest of the loop
    for i in (0..=max_limb).rev() {
        let _i = i as usize;
        let bit_len = if i == max_limb { max_bit - 1 } else { 63 };
        for j in (0..=bit_len).rev() {
            let k1_bit = (k1[_i] >> j) & 1;
            let k2_bit = (k2[_i] >> j) & 1;

            if (k1_bit == 0) && (k2_bit == 0) {
                // If res is 𝒪, do nothing; otherwise, double
                if !res_is_infinity {
                    double_point_assign(&mut res);
                }
            } else if (k1_bit == 0) && (k2_bit == 1) {
                // If res is 𝒪, set res = p; otherwise, double res and add p
                if res_is_infinity {
                    res.x = p.x;
                    res.y = p.y;
                    res_is_infinity = false;
                } else {
                    double_point_assign(&mut res);
                    add_points_complete_assign(&mut res, &mut res_is_infinity, p);
                }

                // Update k2_rec
                k2_rec[_i] |= 1 << j;
            } else if (k1_bit == 1) && (k2_bit == 0) {
                // If res is 𝒪, set res = g; otherwise, double res and add g
                if res_is_infinity {
                    res.x = G_X;
                    res.y = G_Y;
                    res_is_infinity = false;
                } else {
                    double_point_assign(&mut res);
                    add_points_complete_assign(
                        &mut res,
                        &mut res_is_infinity,
                        &SyscallPoint256 { x: G_X, y: G_Y },
                    );
                }

                // Update k1_rec
                k1_rec[_i] |= 1 << j;
            } else if (k1_bit == 1) && (k2_bit == 1) {
                if res_is_infinity {
                    // If (g + p) is 𝒪, do nothing; otherwise set res = (g + p)
                    if !gp_is_infinity {
                        res.x = gp.x;
                        res.y = gp.y;
                        res_is_infinity = false;
                    }
                } else {
                    // If (g + p) is 𝒪, simply double res; otherwise double res and add (g + p)
                    double_point_assign(&mut res);
                    if !gp_is_infinity {
                        add_points_complete_assign(&mut res, &mut res_is_infinity, &gp);
                    }
                }

                // Update k1_rec and k2_rec
                k1_rec[_i] |= 1 << j;
                k2_rec[_i] |= 1 << j;
            }
        }
    }

    // Check that the recomposed scalars are the same as the received scalars
    assert_eq!(k1_rec, *k1);
    assert_eq!(k2_rec, *k2);

    (res_is_infinity, res)
}

pub fn secp256k1_ecdsa_verify(
    pk: &SyscallPoint256,
    z: &[u64; 4],
    r: &[u64; 4],
    s: &[u64; 4],
) -> bool {
    let s_inv = secp256k1_fn_inv(s);

    let u1 = secp256k1_fn_mul(z, &s_inv);
    let u2 = secp256k1_fn_mul(r, &s_inv);

    let (is_infinity, res) = secp256k1_double_scalar_mul_with_g(&u1, &u2, pk);
    if is_infinity {
        return false;
    }

    eq(&secp256k1_fn_reduce(&res.x), r)
}
