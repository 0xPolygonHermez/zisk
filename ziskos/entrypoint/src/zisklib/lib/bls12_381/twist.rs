//! Operations on the twist E': y² = x³ + 4·(1+u) of the BLS12-381 curve

use crate::{fcall_msb_pos_384, zisklib::lib::utils::eq};

use super::{
    constants::{ETWISTED_B, EXT_U, EXT_U_INV, FROBENIUS_GAMMA13, FROBENIUS_GAMMA14, X_ABS_BIN_BE},
    fp2::{
        add_fp2_bls12_381, conjugate_fp2_bls12_381, dbl_fp2_bls12_381, inv_fp2_bls12_381,
        mul_fp2_bls12_381, neg_fp2_bls12_381, scalar_mul_fp2_bls12_381, square_fp2_bls12_381,
        sub_fp2_bls12_381,
    },
};

/// Check if a point `p` is on the BLS12-381 twist
pub fn is_on_curve_twist_bls12_381(p: &[u64; 24]) -> bool {
    // q in E' iff y² == x³ + 4·(1+u)
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();
    let x_sq = square_fp2_bls12_381(&x);
    let x_cubed = mul_fp2_bls12_381(&x_sq, &x);
    let x_cubed_plus_b = add_fp2_bls12_381(&x_cubed, &ETWISTED_B);
    let y_sq = square_fp2_bls12_381(&y);
    eq(&x_cubed_plus_b, &y_sq)
}

/// Check if a point `p` is on the BLS12-381 twist subgroup
pub fn is_on_subgroup_twist_bls12_381(p: &[u64; 24]) -> bool {
    // p in subgroup iff:
    //          x·𝜓³(P) + P == 𝜓²(P)
    // where ψ := 𝜑⁻¹𝜋ₚ𝜑 is the untwist-Frobenius-twist endomorphism

    // Compute ψ²(P), ψ³(P)
    let utf1 = utf_endomorphism_twist_bls12_381(p);
    let rhs = utf_endomorphism_twist_bls12_381(&utf1);
    let utf3 = utf_endomorphism_twist_bls12_381(&rhs);

    // Compute [x]ψ³(P) + P (since x is negative, we compute -[|x|]ψ³(P))
    let xutf3: [u64; 24] = scalar_mul_by_abs_x_twist_bls12_381(&utf3);
    let mut lhs = neg_twist_bls12_381(&xutf3);
    lhs = add_twist_bls12_381(&lhs, p);

    eq(&lhs, &rhs)
}

/// Addition of two non-zero points
pub fn add_twist_bls12_381(p1: &[u64; 24], p2: &[u64; 24]) -> [u64; 24] {
    let x1: [u64; 12] = p1[0..12].try_into().unwrap();
    let y1: [u64; 12] = p1[12..24].try_into().unwrap();
    let x2: [u64; 12] = p2[0..12].try_into().unwrap();
    let y2: [u64; 12] = p2[12..24].try_into().unwrap();

    // Is x1 == x2?
    if eq(&x1, &x2) {
        // Is y1 == y2?
        if eq(&y1, &y2) {
            // Compute the doubling
            let mut lambda = dbl_fp2_bls12_381(&y1);
            lambda = inv_fp2_bls12_381(&lambda);
            lambda = scalar_mul_fp2_bls12_381(&lambda, &[0x3, 0, 0, 0, 0, 0]);
            lambda = mul_fp2_bls12_381(&lambda, &x1);
            lambda = mul_fp2_bls12_381(&lambda, &x1);

            let mut x3 = square_fp2_bls12_381(&lambda);
            x3 = sub_fp2_bls12_381(&x3, &x1);
            x3 = sub_fp2_bls12_381(&x3, &x2);

            let mut y3 = sub_fp2_bls12_381(&x1, &x3);
            y3 = mul_fp2_bls12_381(&lambda, &y3);
            y3 = sub_fp2_bls12_381(&y3, &y1);

            return [x3, y3].concat().try_into().unwrap();
        } else {
            // Points are the inverse of each other, return the point at infinity
            return [0u64; 24];
        }
    }

    // Compute the addition
    let mut den = sub_fp2_bls12_381(&x2, &x1);
    den = inv_fp2_bls12_381(&den);
    let mut lambda = sub_fp2_bls12_381(&y2, &y1);
    lambda = mul_fp2_bls12_381(&lambda, &den);

    let mut x3 = square_fp2_bls12_381(&lambda);
    x3 = sub_fp2_bls12_381(&x3, &x1);
    x3 = sub_fp2_bls12_381(&x3, &x2);

    let mut y3 = sub_fp2_bls12_381(&x1, &x3);
    y3 = mul_fp2_bls12_381(&lambda, &y3);
    y3 = sub_fp2_bls12_381(&y3, &y1);

    [x3, y3].concat().try_into().unwrap()
}

/// Doubling of a non-zero point
pub fn dbl_twist_bls12_381(p: &[u64; 24]) -> [u64; 24] {
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();

    // Compute the doubling
    let mut lambda = dbl_fp2_bls12_381(&y);
    lambda = inv_fp2_bls12_381(&lambda);
    lambda = scalar_mul_fp2_bls12_381(&lambda, &[0x3, 0, 0, 0, 0, 0]);
    lambda = mul_fp2_bls12_381(&lambda, &x);
    lambda = mul_fp2_bls12_381(&lambda, &x);

    let mut x3 = square_fp2_bls12_381(&lambda);
    x3 = sub_fp2_bls12_381(&x3, &x);
    x3 = sub_fp2_bls12_381(&x3, &x);

    let mut y3 = sub_fp2_bls12_381(&x, &x3);
    y3 = mul_fp2_bls12_381(&lambda, &y3);
    y3 = sub_fp2_bls12_381(&y3, &y);

    [x3, y3].concat().try_into().unwrap()
}

/// Negation of a point
pub fn neg_twist_bls12_381(p: &[u64; 24]) -> [u64; 24] {
    let x: [u64; 12] = p[0..12].try_into().unwrap();
    let y: [u64; 12] = p[12..24].try_into().unwrap();

    // Compute the negation
    let y_neg = neg_fp2_bls12_381(&y);
    [x, y_neg].concat().try_into().unwrap()
}

/// Multiplies a point `p` on the BLS12-381 curve by a scalar `k` on the BLS12-381 scalar field
pub fn scalar_mul_twist_bls12_381(p: &[u64; 24], k: &[u64; 6]) -> [u64; 24] {
    // Is p = 𝒪?
    if *p == [0u64; 24] {
        // Return 𝒪
        return [0u64; 24];
    }

    // Direct cases: k = 0, k = 1, k = 2
    match k {
        [0, 0, 0, 0, 0, 0] => {
            // Return 𝒪
            return [0u64; 24];
        }
        [1, 0, 0, 0, 0, 0] => {
            // Return p
            return *p;
        }
        [2, 0, 0, 0, 0, 0] => {
            // Return 2p
            return dbl_twist_bls12_381(p);
        }
        _ => {}
    }

    // We can assume k > 2 from now on
    // Hint the length the binary representations of k
    // We will verify the output by recomposing k
    // Moreover, we should check that the first received bit is 1
    let (max_limb, max_bit) = fcall_msb_pos_384(k, &[0, 0, 0, 0, 0, 0]);

    // Perform the loop, based on the binary representation of k

    // We do the first iteration separately
    let _max_limb = max_limb as usize;
    let k_bit = (k[_max_limb] >> max_bit) & 1;
    assert_eq!(k_bit, 1); // the first received bit should be 1

    // Start at P
    let mut q = *p;
    let mut k_rec = [0u64; 6];
    k_rec[_max_limb] |= 1 << max_bit;

    // Perform the rest of the loop
    let _max_bit = max_bit as usize;
    for i in (0..=_max_limb).rev() {
        let bit_len = if i == _max_limb { _max_bit - 1 } else { 63 };
        for j in (0..=bit_len).rev() {
            // Always double
            q = dbl_twist_bls12_381(&q);

            // Get the next bit b of k.
            // If b == 1, we should add P to Q, otherwise start the next iteration
            if ((k[i] >> j) & 1) == 1 {
                q = add_twist_bls12_381(&q, p);

                // Reconstruct k
                k_rec[i] |= 1 << j;
            }
        }
    }

    // Check that the reconstructed k is equal to the input k
    assert_eq!(k_rec, *k);

    // Convert the result back to a single array
    q
}

/// Scalar multiplication of a non-zero point by x
pub fn scalar_mul_bin_twist_bls12_381(p: &[u64; 24], k: &[u8]) -> [u64; 24] {
    // debug_assert!(k == X2DIV3_BIN_BE);

    let mut r = *p;
    for &bit in k.iter().skip(1) {
        r = dbl_twist_bls12_381(&r);
        if bit == 1 {
            r = add_twist_bls12_381(&r, p);
        }
    }
    r
}

/// Scalar multiplication of a non-zero point by x
pub fn scalar_mul_by_abs_x_twist_bls12_381(p: &[u64; 24]) -> [u64; 24] {
    scalar_mul_bin_twist_bls12_381(p, &X_ABS_BIN_BE)
}

/// Compute the untwist-frobenius-twist (utf) endomorphism ψ := 𝜑⁻¹𝜋ₚ𝜑 of a point `p`, where:
///     𝜑 : E'(Fp2) -> E(Fp12) defined by 𝜑(x,y) = (x/ω²,y/ω³) is the untwist map
///     𝜋ₚ : E(Fp12) -> E(Fp12) defined by 𝜋ₚ(x,y) = (xᵖ,yᵖ) is the Frobenius map
///     𝜑⁻¹ : E(Fp12) -> E'(Fp2) defined by 𝜑⁻¹(x,y) = (x·ω²,y·ω³) is the twist map
pub fn utf_endomorphism_twist_bls12_381(p: &[u64; 24]) -> [u64; 24] {
    let mut x: [u64; 12] = p[0..12].try_into().unwrap();
    let mut y: [u64; 12] = p[12..24].try_into().unwrap();

    // 1] Compute 𝜑(x,y) = (x/ω²,y/ω³) = (x·(%W_INV_X + %W_INV_Y·u)·ω⁴,y·(%W_INV_X + %W_INV_Y·u)·ω³) ∈ E(Fp12)
    x = mul_fp2_bls12_381(&x, &EXT_U_INV);
    y = mul_fp2_bls12_381(&y, &EXT_U_INV);

    // 2] Compute 𝜋ₚ(a,b) = (aᵖ,bᵖ), i.e., apply the frobenius operator
    //    Since the previous result has only one non-zero coefficient, we can apply a specialized frobenius directly
    //    (a·ω⁴)ᵖ = a̅·γ14·ω⁴, (b·ω³)ᵖ = b̅·γ13·ω³
    x = conjugate_fp2_bls12_381(&x);
    x = scalar_mul_fp2_bls12_381(&x, &FROBENIUS_GAMMA14);
    y = conjugate_fp2_bls12_381(&y);
    y = mul_fp2_bls12_381(&y, &FROBENIUS_GAMMA13);

    // 3] Compute 𝜑⁻¹(a,b) = (a·ω²,b·ω³) ∈ E'(Fp2). In our particular case, we have:
    //         𝜑⁻¹((a̅·γ14·ω⁴)·ω²,(b̅·γ13·ω³)·ω³) = (a̅·γ14·(1+u), b̅·γ13·(1+u))
    x = mul_fp2_bls12_381(&x, &EXT_U);
    y = mul_fp2_bls12_381(&y, &EXT_U);

    [x, y].concat().try_into().unwrap()
}
