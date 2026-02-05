use crate::{
    syscalls::{
        syscall_secp256k1_add, syscall_secp256k1_dbl, SyscallPoint256, SyscallSecp256k1AddParams,
    },
    zisklib::{eq, fcall_msb_pos_256},
};

use super::{
    constants::{E_B, G_X, G_Y},
    field::{
        secp256k1_fp_add, secp256k1_fp_inv, secp256k1_fp_mul, secp256k1_fp_sqrt,
        secp256k1_fp_square,
    },
    scalar::{secp256k1_fn_inv, secp256k1_fn_mul, secp256k1_fn_reduce},
};

/// Converts a non-zero point `p` on the Secp256k1 curve from projective coordinates to affine coordinates
pub fn secp256k1_to_affine(p: &[u64; 12]) -> [u64; 8] {
    let z: [u64; 4] = p[8..12].try_into().unwrap();

    // Point at infinity cannot be converted to affine
    debug_assert!(z != [0u64; 4], "Cannot convert point at infinity to affine");

    let zinv = secp256k1_fp_inv(&z);
    let zinv_sq = secp256k1_fp_square(&zinv);

    let x: [u64; 4] = p[0..4].try_into().unwrap();
    let y: [u64; 4] = p[4..8].try_into().unwrap();

    let x_res = secp256k1_fp_mul(&x, &zinv_sq);
    let mut y_res = secp256k1_fp_mul(&y, &zinv_sq);
    y_res = secp256k1_fp_mul(&y_res, &zinv);

    [x_res[0], x_res[1], x_res[2], x_res[3], y_res[0], y_res[1], y_res[2], y_res[3]]
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

    let one = [1u64, 0, 0, 0];
    if *k1 == one && *k2 == one {
        // Return G + p
        return (gp_is_infinity, gp);
    }
    // From here on, at least one of k1 or k2 is greater than 1

    // Hint the maximum length between the binary representations of k1 and k2
    // We will verify the output by recomposing both k1 and k2
    // Moreover, we should check that the first received bit (of either k1 or k2) is 1
    let (max_limb, max_bit) = fcall_msb_pos_256(k1, k2);

    // Perform the loop, based on the binary representation of k1 and k2

    // We do the first iteration separately
    let max_limb = max_limb as usize;
    let max_bit = max_bit as usize;

    // At least one of the scalars should have the first received bit as 1
    let k1_bit = (k1[max_limb] >> max_bit) & 1;
    let k2_bit = (k2[max_limb] >> max_bit) & 1;
    assert!(k1_bit == 1 || k2_bit == 1);

    // Start at 𝒪
    let mut res = SyscallPoint256 { x: [0u64; 4], y: [0u64; 4] };
    let mut res_is_infinity = true;
    let mut k1_rec = [0u64; 4];
    let mut k2_rec = [0u64; 4];
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
        k2_rec[max_limb] |= 1 << max_bit;
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
        k1_rec[max_limb] |= 1 << max_bit;
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
        k1_rec[max_limb] |= 1 << max_bit;
        k2_rec[max_limb] |= 1 << max_bit;
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
                k2_rec[i] |= 1 << j;
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
                k1_rec[i] |= 1 << j;
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
                k1_rec[i] |= 1 << j;
                k2_rec[i] |= 1 << j;
            }
        }
        bit = 63;
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

/// # Safety
/// - `p_ptr` must point to 12 u64s (projective point: x[4], y[4], z[4])
/// - `out_ptr` must point to at least 8 u64s (will write affine x[4], y[4])
///
/// Returns 1 on success, 0 if point is at infinity
#[no_mangle]
pub unsafe extern "C" fn secp256k1_to_affine_c(p_ptr: *const u64, out_ptr: *mut u64) {
    let p: &[u64; 12] = &*(p_ptr as *const [u64; 12]);
    let result = secp256k1_to_affine(p);

    *out_ptr.add(0) = result[0];
    *out_ptr.add(1) = result[1];
    *out_ptr.add(2) = result[2];
    *out_ptr.add(3) = result[3];
    *out_ptr.add(4) = result[4];
    *out_ptr.add(5) = result[5];
    *out_ptr.add(6) = result[6];
    *out_ptr.add(7) = result[7];
}

/// # Safety
/// - `x_bytes_ptr` must point to 32 bytes (big-endian x-coordinate)
/// - `out_ptr` must point to at least 8 u64s (will write x[4] and y[4] in little-endian)
///
/// Returns 1 on success, 0 if no valid point exists
#[no_mangle]
pub unsafe extern "C" fn secp256k1_decompress_c(
    x_bytes_ptr: *const u8,
    y_is_odd: u8,
    out_ptr: *mut u64,
) -> u8 {
    let x_bytes: &[u8; 32] = &*(x_bytes_ptr as *const [u8; 32]);

    let ((x, y), success) = secp256k1_decompress(x_bytes, y_is_odd != 0);

    if !success {
        return 0;
    }

    *out_ptr.add(0) = x[0];
    *out_ptr.add(1) = x[1];
    *out_ptr.add(2) = x[2];
    *out_ptr.add(3) = x[3];
    *out_ptr.add(4) = y[0];
    *out_ptr.add(5) = y[1];
    *out_ptr.add(6) = y[2];
    *out_ptr.add(7) = y[3];

    1
}

/// # Safety
/// - `k1_ptr` must point to 4 u64s (scalar k1)
/// - `k2_ptr` must point to 4 u64s (scalar k2)
/// - `p_ptr` must point to 8 u64s (point P: x[4], y[4])
/// - `out_ptr` must point to at least 8 u64s (will write result x[4], y[4])
///
/// Returns 1 if result is point at infinity, 0 otherwise
#[no_mangle]
pub unsafe extern "C" fn secp256k1_double_scalar_mul_with_g_c(
    k1_ptr: *const u64,
    k2_ptr: *const u64,
    p_ptr: *const u64,
    out_ptr: *mut u64,
) -> bool {
    let k1: &[u64; 4] = &*(k1_ptr as *const [u64; 4]);
    let k2: &[u64; 4] = &*(k2_ptr as *const [u64; 4]);

    let p = SyscallPoint256 {
        x: [*p_ptr.add(0), *p_ptr.add(1), *p_ptr.add(2), *p_ptr.add(3)],
        y: [*p_ptr.add(4), *p_ptr.add(5), *p_ptr.add(6), *p_ptr.add(7)],
    };

    let (is_infinity, res) = secp256k1_double_scalar_mul_with_g(k1, k2, &p);

    *out_ptr.add(0) = res.x[0];
    *out_ptr.add(1) = res.x[1];
    *out_ptr.add(2) = res.x[2];
    *out_ptr.add(3) = res.x[3];
    *out_ptr.add(4) = res.y[0];
    *out_ptr.add(5) = res.y[1];
    *out_ptr.add(6) = res.y[2];
    *out_ptr.add(7) = res.y[3];

    is_infinity
}

/// # Safety
/// - `pk_ptr` must point to 8 u64s (public key: x[4], y[4])
/// - `z_ptr` must point to 4 u64s (message hash)
/// - `r_ptr` must point to 4 u64s (signature r)
/// - `s_ptr` must point to 4 u64s (signature s)
///
/// Returns 1 if signature is valid, 0 otherwise
#[no_mangle]
pub unsafe extern "C" fn secp256k1_ecdsa_verify_c(
    pk_ptr: *const u64,
    z_ptr: *const u64,
    r_ptr: *const u64,
    s_ptr: *const u64,
) -> bool {
    let pk = SyscallPoint256 {
        x: [*pk_ptr.add(0), *pk_ptr.add(1), *pk_ptr.add(2), *pk_ptr.add(3)],
        y: [*pk_ptr.add(4), *pk_ptr.add(5), *pk_ptr.add(6), *pk_ptr.add(7)],
    };
    let z: &[u64; 4] = &*(z_ptr as *const [u64; 4]);
    let r: &[u64; 4] = &*(r_ptr as *const [u64; 4]);
    let s: &[u64; 4] = &*(s_ptr as *const [u64; 4]);

    secp256k1_ecdsa_verify(&pk, z, r, s)
}
