//! Operations on the twist E': y² = x³ + 3 / (9 + u) of the BN254 curve

use crate::zisklib::lib::utils::eq;

use super::{
    constants::{ETWISTED_B, E_B, FROBENIUS_GAMMA12, FROBENIUS_GAMMA13, IDENTITY_G2},
    fp2::{
        add_fp2_bn254, conjugate_fp2_bn254, dbl_fp2_bn254, inv_fp2_bn254, mul_fp2_bn254,
        neg_fp2_bn254, scalar_mul_fp2_bn254, square_fp2_bn254, sub_fp2_bn254,
    },
};

/// Check if a non-zero point `p` is on the BN254 twist
pub fn is_on_curve_twist_bn254(
    p: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // q in E' iff y² == x³ + 3 / (9 + u)
    let x: [u64; 8] = p[0..8].try_into().unwrap();
    let y: [u64; 8] = p[8..16].try_into().unwrap();
    let x_sq = square_fp2_bn254(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cubed = mul_fp2_bn254(
        &x_sq,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let x_cubed_plus_b = add_fp2_bn254(
        &x_cubed,
        &ETWISTED_B,
        #[cfg(feature = "hints")]
        hints,
    );
    let y_sq = square_fp2_bn254(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&x_cubed_plus_b, &y_sq)
}

/// Check if a non-zero point `p` is on the BN254 twist subgroup
pub fn is_on_subgroup_twist_bn254(
    p: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // p in subgroup iff:
    //      (x+1)·Q + 𝜓(x·Q) + 𝜓²(x·Q) == 𝜓³((2x)·Q)
    // where 𝜓 is the Frobenius endomorphism
    // as described in https://eprint.iacr.org/2022/348.pdf
    let xp: [u64; 16] = scalar_mul_by_x_twist_bn254(
        p,
        #[cfg(feature = "hints")]
        hints,
    );
    let x1p = add_twist_bn254(
        p,
        &xp,
        #[cfg(feature = "hints")]
        hints,
    );
    let psi_one = utf_endomorphism_twist_bn254(
        &xp,
        #[cfg(feature = "hints")]
        hints,
    );
    let psi_two = utf_endomorphism_twist_bn254(
        &psi_one,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut lhs = add_twist_bn254(
        &x1p,
        &psi_one,
        #[cfg(feature = "hints")]
        hints,
    );
    lhs = add_twist_bn254(
        &lhs,
        &psi_two,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut rhs = dbl_twist_bn254(
        &xp,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = utf_endomorphism_twist_bn254(
        &rhs,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = utf_endomorphism_twist_bn254(
        &rhs,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = utf_endomorphism_twist_bn254(
        &rhs,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&lhs, &rhs)
}

/// Converts a point `p` on the BN254 curve from Jacobian coordinates to affine coordinates
pub fn to_affine_twist_bn254(
    p: &[u64; 24],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 16] {
    let z: [u64; 8] = p[16..24].try_into().unwrap();

    if z == [0u64; 8] {
        return IDENTITY_G2;
    } else if z == [1u64, 0, 0, 0, 0, 0, 0, 0] {
        return [
            p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13],
            p[14], p[15],
        ];
    }

    let x: [u64; 8] = p[0..8].try_into().unwrap();
    let y: [u64; 8] = p[8..16].try_into().unwrap();

    let zinv = inv_fp2_bn254(
        &z,
        #[cfg(feature = "hints")]
        hints,
    );
    let zinv_sq = square_fp2_bn254(
        &zinv,
        #[cfg(feature = "hints")]
        hints,
    );

    let x_res = mul_fp2_bn254(
        &x,
        &zinv_sq,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut y_res = mul_fp2_bn254(
        &y,
        &zinv_sq,
        #[cfg(feature = "hints")]
        hints,
    );
    y_res = mul_fp2_bn254(
        &y_res,
        &zinv,
        #[cfg(feature = "hints")]
        hints,
    );
    [
        x_res[0], x_res[1], x_res[2], x_res[3], x_res[4], x_res[5], x_res[6], x_res[7], y_res[0],
        y_res[1], y_res[2], y_res[3], y_res[4], y_res[5], y_res[6], y_res[7],
    ]
}

/// Addition of two non-zero points
pub fn add_twist_bn254(
    p1: &[u64; 16],
    p2: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 16] {
    let x1: [u64; 8] = p1[0..8].try_into().unwrap();
    let y1: [u64; 8] = p1[8..16].try_into().unwrap();
    let x2: [u64; 8] = p2[0..8].try_into().unwrap();
    let y2: [u64; 8] = p2[8..16].try_into().unwrap();

    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling
            return dbl_twist_bn254(
                p1,
                #[cfg(feature = "hints")]
                hints,
            );
        } else {
            // Points are the inverse of each other, return the point at infinity
            return IDENTITY_G2;
        }
    }

    // Compute the addition
    let mut den = sub_fp2_bn254(
        &x2,
        &x1,
        #[cfg(feature = "hints")]
        hints,
    );
    den = inv_fp2_bn254(
        &den,
        #[cfg(feature = "hints")]
        hints,
    );
    let mut lambda = sub_fp2_bn254(
        &y2,
        &y1,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = mul_fp2_bn254(
        &lambda,
        &den,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut x3 = square_fp2_bn254(
        &lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bn254(
        &x3,
        &x1,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bn254(
        &x3,
        &x2,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut y3 = sub_fp2_bn254(
        &x1,
        &x3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = mul_fp2_bn254(
        &lambda,
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = sub_fp2_bn254(
        &y3,
        &y1,
        #[cfg(feature = "hints")]
        hints,
    );
    [
        x3[0], x3[1], x3[2], x3[3], x3[4], x3[5], x3[6], x3[7], y3[0], y3[1], y3[2], y3[3], y3[4],
        y3[5], y3[6], y3[7],
    ]
}

/// Doubling of a non-zero point
pub fn dbl_twist_bn254(p: &[u64; 16], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 16] {
    let x: [u64; 8] = p[0..8].try_into().unwrap();
    let y: [u64; 8] = p[8..16].try_into().unwrap();

    // Compute the doubling
    let mut lambda = dbl_fp2_bn254(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = inv_fp2_bn254(
        &lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = scalar_mul_fp2_bn254(
        &lambda,
        &E_B,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = mul_fp2_bn254(
        &lambda,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    lambda = mul_fp2_bn254(
        &lambda,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut x3 = square_fp2_bn254(
        &lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bn254(
        &x3,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bn254(
        &x3,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut y3 = sub_fp2_bn254(
        &x,
        &x3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = mul_fp2_bn254(
        &lambda,
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = sub_fp2_bn254(
        &y3,
        &y,
        #[cfg(feature = "hints")]
        hints,
    );

    [
        x3[0], x3[1], x3[2], x3[3], x3[4], x3[5], x3[6], x3[7], y3[0], y3[1], y3[2], y3[3], y3[4],
        y3[5], y3[6], y3[7],
    ]
}

/// Negation of a point
pub fn neg_twist_bn254(p: &[u64; 16], #[cfg(feature = "hints")] hints: &mut Vec<u64>) -> [u64; 16] {
    let x: [u64; 8] = p[0..8].try_into().unwrap();
    let y: [u64; 8] = p[8..16].try_into().unwrap();

    // Compute the negation
    let y_neg = neg_fp2_bn254(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );
    [
        x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7], y_neg[0], y_neg[1], y_neg[2], y_neg[3],
        y_neg[4], y_neg[5], y_neg[6], y_neg[7],
    ]
}

/// Scalar multiplication of a non-zero point by x
pub fn scalar_mul_by_x_twist_bn254(
    p: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 16] {
    // Binary representation of the exponent x = 4965661367192848881 in big-endian format
    const X_BIN_BE: [u8; 63] = [
        1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0,
        0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 0,
        0, 0, 1,
    ];

    let mut q = *p;
    for &bit in X_BIN_BE.iter().skip(1) {
        q = dbl_twist_bn254(
            &q,
            #[cfg(feature = "hints")]
            hints,
        );
        if bit == 1 {
            q = add_twist_bn254(
                &q,
                p,
                #[cfg(feature = "hints")]
                hints,
            );
        }
    }
    q
}

/// Compute the untwist-frobenius-twist (utf) endomorphism 𝜓: (x,y) = (𝛾₁₂·x̄,𝛾₁₃·ȳ)
pub fn utf_endomorphism_twist_bn254(
    p: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 16] {
    let mut x: [u64; 8] = p[0..8].try_into().unwrap();
    let mut y: [u64; 8] = p[8..16].try_into().unwrap();

    // Compute the conjugate of x and y
    x = conjugate_fp2_bn254(
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    y = conjugate_fp2_bn254(
        &y,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute the multiplication
    let qx = mul_fp2_bn254(
        &FROBENIUS_GAMMA12,
        &x,
        #[cfg(feature = "hints")]
        hints,
    );
    let qy = mul_fp2_bn254(
        &FROBENIUS_GAMMA13,
        &y,
        #[cfg(feature = "hints")]
        hints,
    );

    [
        qx[0], qx[1], qx[2], qx[3], qx[4], qx[5], qx[6], qx[7], qy[0], qy[1], qy[2], qy[3], qy[4],
        qy[5], qy[6], qy[7],
    ]
}

/// # Safety
/// `p_ptr` must point to a valid `[u64; 16]` (128 bytes, affine G2 twist point).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_is_on_curve_twist_bn254_c")]
pub unsafe extern "C" fn is_on_curve_twist_bn254_c(
    p_ptr: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p = unsafe { &*(p_ptr as *const [u64; 16]) };
    #[cfg(zisk_hints_debug)]
    println!("is_on_curve_twist_bn254_c called with p = {:?}", p);

    is_on_curve_twist_bn254(
        p,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// # Safety
/// `p_ptr` must point to a valid `[u64; 16]` (128 bytes, affine G2 twist point).
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_is_on_subgroup_twist_bn254_c")]
pub unsafe extern "C" fn is_on_subgroup_twist_bn254_c(
    p_ptr: *const u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let p = unsafe { &*(p_ptr as *const [u64; 16]) };
    #[cfg(zisk_hints_debug)]
    println!("is_on_subgroup_twist_bn254_c called with p = {:?}", p);

    is_on_subgroup_twist_bn254(
        p,
        #[cfg(feature = "hints")]
        hints,
    )
}

/// # Safety
/// - `p_ptr` must point to a valid `[u64; 24]` (192 bytes, Jacobian G2 twist point).
/// - `out_ptr` must point to a valid `[u64; 16]` (128 bytes) writable buffer.
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_to_affine_twist_bn254_c")]
pub unsafe extern "C" fn to_affine_twist_bn254_c(
    p_ptr: *const u64,
    out_ptr: *mut u64,
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) {
    let p = unsafe { &*(p_ptr as *const [u64; 24]) };
    #[cfg(zisk_hints_debug)]
    println!("to_affine_twist_bn254_c called with p = {:?}", p);

    let result = to_affine_twist_bn254(
        p,
        #[cfg(feature = "hints")]
        hints,
    );

    *out_ptr.add(0) = result[0];
    *out_ptr.add(1) = result[1];
    *out_ptr.add(2) = result[2];
    *out_ptr.add(3) = result[3];
    *out_ptr.add(4) = result[4];
    *out_ptr.add(5) = result[5];
    *out_ptr.add(6) = result[6];
    *out_ptr.add(7) = result[7];
    *out_ptr.add(8) = result[8];
    *out_ptr.add(9) = result[9];
    *out_ptr.add(10) = result[10];
    *out_ptr.add(11) = result[11];
    *out_ptr.add(12) = result[12];
    *out_ptr.add(13) = result[13];
    *out_ptr.add(14) = result[14];
    *out_ptr.add(15) = result[15];
}
