//! Miller loop for BLS12-381

use crate::{
    fcall_bls12_381_add_line_coeffs, fcall_bls12_381_dbl_line_coeffs,
    zisklib::lib::{
        bls12_381::{constants::EXT_U_INV, fp12::conjugate_fp12_bls12_381},
        utils::eq,
    },
};

use super::{
    constants::X_ABS_BIN_BE,
    fp::{inv_fp_bls12_381, mul_fp_bls12_381, neg_fp_bls12_381},
    fp12::{sparse_mul_fp12_bls12_381, square_fp12_bls12_381},
    fp2::{
        add_fp2_bls12_381, dbl_fp2_bls12_381, mul_fp2_bls12_381, neg_fp2_bls12_381,
        scalar_mul_fp2_bls12_381, square_fp2_bls12_381, sub_fp2_bls12_381,
    },
};

/// Computes the Miller loop for the BLS12-381 curve
pub fn miller_loop_bls12_381(p: &[u64; 12], q: &[u64; 24]) -> [u64; 72] {
    // Before the loop starts, compute xp' = (-xp/yp)·1/(1+u) and yp' = (1/yp)·1/(1+u)
    let mut xp: [u64; 6] = p[0..6].try_into().unwrap();
    let mut yp: [u64; 6] = p[6..12].try_into().unwrap();
    yp = inv_fp_bls12_381(&yp);
    xp = neg_fp_bls12_381(&xp);
    xp = mul_fp_bls12_381(&xp, &yp);

    let xp_prime: [u64; 12] = scalar_mul_fp2_bls12_381(&EXT_U_INV, &xp);
    let yp_prime: [u64; 12] = scalar_mul_fp2_bls12_381(&EXT_U_INV, &yp);

    // Initialize the Miller loop with r = q and f = 1
    let mut r: [u64; 24] = q[0..24].try_into().unwrap();
    let mut f = {
        let mut one = [0u64; 72];
        one[0] = 1;
        one
    };
    for &bit in X_ABS_BIN_BE.iter().skip(1) {
        // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(r)}
        let (lambda, mu) = fcall_bls12_381_dbl_line_coeffs(&r);

        // Check that the line is correct
        assert!(is_tangent_twist_bls12_381(&r, &lambda, &mu));

        // Compute f = f² · line_{twist(r),twist(r)}(p)
        f = square_fp12_bls12_381(&f);
        let l = line_eval_twist_bls12_381(&lambda, &mu, &xp_prime, &yp_prime);
        f = sparse_mul_fp12_bls12_381(&f, &l);

        // Double r
        r = line_dbl_twist_bls12_381(&r, &lambda, &mu);

        if bit == 1 {
            // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(q)}
            let (lambda, mu) = fcall_bls12_381_add_line_coeffs(&r, q);

            // Check that the line is correct
            assert!(is_line_twist_bls12_381(&r, q, &lambda, &mu));

            // Compute f = f · line_{twist(r),twist(q)}
            let l = line_eval_twist_bls12_381(&lambda, &mu, &xp_prime, &yp_prime);
            f = sparse_mul_fp12_bls12_381(&f, &l);

            // Add r and q
            r = line_add_twist_bls12_381(&r, q, &lambda, &mu);
        }
    }

    // Finally, compute f̅
    conjugate_fp12_bls12_381(&f)
}

// We follow https://eprint.iacr.org/2024/640.pdf for the line computations.
//
// The main idea is as follows:
// Instead of computing the line, we will hint the coefficients of the line (𝜆,𝜇)
// and check that:
//   1] Line passes through q1 by checking: y1 = 𝜆x1 + 𝜇
//   2] Line passes through q2 by checking: y2 = 𝜆x2 + 𝜇
// In fact, one can use the coefficients of the line to compute the
// evaluation of the line at p and compute the addition q1 + q2

/// Checks if the line defined by (𝜆,𝜇) passes through q1 and q2
#[inline]
fn is_line_twist_bls12_381(
    q1: &[u64; 24],
    q2: &[u64; 24],
    lambda: &[u64; 12],
    mu: &[u64; 12],
) -> bool {
    line_check_twist_bls12_381(q1, lambda, mu) && line_check_twist_bls12_381(q2, lambda, mu)
}

/// Checks if the line defined by (𝜆,𝜇) is tangent to the curve at q
#[inline]
fn is_tangent_twist_bls12_381(q: &[u64; 24], lambda: &[u64; 12], mu: &[u64; 12]) -> bool {
    // Check the line passes through q
    let curve_check = line_check_twist_bls12_381(q, lambda, mu);

    // Check the line is tangent at q by checking that 2𝜆y = 3x²
    let x: &[u64; 12] = q[0..12].try_into().unwrap();
    let y: &[u64; 12] = q[12..24].try_into().unwrap();
    let mut lhs = mul_fp2_bls12_381(lambda, y);
    lhs = dbl_fp2_bls12_381(&lhs);

    let mut rhs = square_fp2_bls12_381(x);
    rhs = scalar_mul_fp2_bls12_381(&rhs, &[3, 0, 0, 0, 0, 0]);
    let tangent_check = eq(&lhs, &rhs);

    curve_check && tangent_check
}

/// Check if the line defined by (𝜆,𝜇) passes through the point q
#[inline]
fn line_check_twist_bls12_381(q: &[u64; 24], lambda: &[u64; 12], mu: &[u64; 12]) -> bool {
    let x: &[u64; 12] = q[0..12].try_into().unwrap();
    let y: &[u64; 12] = q[12..24].try_into().unwrap();

    // Check if y = λx + μ
    let mut rhs = mul_fp2_bls12_381(lambda, x);
    rhs = add_fp2_bls12_381(&rhs, mu);
    eq(&rhs, y)
}

/// Evaluates the line function l(x,y) := (1 + 0·v + 0·v²) + (0 - μy·v + λx·v²)·w
#[inline]
fn line_eval_twist_bls12_381(
    lambda: &[u64; 12],
    mu: &[u64; 12],
    x: &[u64; 12],
    y: &[u64; 12],
) -> [u64; 24] {
    let coeff1 = mul_fp2_bls12_381(mu, &neg_fp2_bls12_381(y));
    let coeff2 = mul_fp2_bls12_381(lambda, x);

    [coeff1, coeff2].concat().try_into().unwrap()
}

#[inline]
fn line_add_twist_bls12_381(
    q1: &[u64; 24],
    q2: &[u64; 24],
    lambda: &[u64; 12],
    mu: &[u64; 12],
) -> [u64; 24] {
    let x1: &[u64; 12] = q1[0..12].try_into().unwrap();
    let x2: &[u64; 12] = q2[0..12].try_into().unwrap();

    // Compute x3 = λ² - x1 - x2
    let mut x3 = square_fp2_bls12_381(lambda);
    x3 = sub_fp2_bls12_381(&x3, x1);
    x3 = sub_fp2_bls12_381(&x3, x2);

    // Compute y3 = -λx3 - μ
    let mut y3 = mul_fp2_bls12_381(lambda, &x3);
    y3 = add_fp2_bls12_381(mu, &y3);
    y3 = neg_fp2_bls12_381(&y3);

    [x3, y3].concat().try_into().unwrap()
}

#[inline]
fn line_dbl_twist_bls12_381(q: &[u64; 24], lambda: &[u64; 12], mu: &[u64; 12]) -> [u64; 24] {
    let x: &[u64; 12] = q[0..12].try_into().unwrap();

    // Compute x3 = λ² - 2x
    let mut x3 = square_fp2_bls12_381(lambda);
    x3 = sub_fp2_bls12_381(&x3, &dbl_fp2_bls12_381(x));

    // Compute y3 = -λx3 - μ
    let mut y3 = mul_fp2_bls12_381(lambda, &x3);
    y3 = add_fp2_bls12_381(mu, &y3);
    y3 = neg_fp2_bls12_381(&y3);

    [x3, y3].concat().try_into().unwrap()
}
