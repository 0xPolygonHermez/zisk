use crate::{
    fcall_bn254_add_line_coeffs, fcall_bn254_dbl_line_coeffs,
    zisklib::lib::{
        bn254::{
            fp12::{mul_fp12_bn254, square_fp12_bn254},
            twist::neg_twist_bn254,
        },
        utils::eq,
    },
};

use super::{
    fp::{inv_fp_bn254, mul_fp_bn254, neg_fp_bn254},
    fp2::{
        add_fp2_bn254, dbl_fp2_bn254, mul_fp2_bn254, neg_fp2_bn254, scalar_mul_fp2_bn254,
        square_fp2_bn254, sub_fp2_bn254,
    },
    twist::utf_endomorphism_twist_bn254,
};

/// Pseudobinary representation of the loop length 6·X+2 of the
/// optimal ate pairing over the BN254.
const LOOP_LENGHT_BE: [i8; 65] = [
    1, 1, 0, 1, 0, 0, -1, 0, 1, 1, 0, 0, 0, -1, 0, 0, 1, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 0, -1,
    0, 0, 1, 1, 1, 0, 0, 0, 0, -1, 0, 1, 0, 0, -1, 0, 1, 1, 0, 0, 1, 0, 0, -1, 1, 0, 0, -1, 0, 1,
    0, 1, 0, 0, 0,
];

pub fn miller_loop_bn254(p: &[u64; 8], q: &[u64; 16]) -> [u64; 48] {
    // Before the loop starts, compute xp' = -xp/yp and yp' = 1/yp
    let mut xp_prime: [u64; 4] = p[0..4].try_into().unwrap();
    let mut yp_prime: [u64; 4] = p[4..8].try_into().unwrap();
    yp_prime = inv_fp_bn254(&yp_prime);
    xp_prime = neg_fp_bn254(&xp_prime);
    xp_prime = mul_fp_bn254(&xp_prime, &yp_prime);

    // Initialize the Miller loop with r = q and f = 1
    let mut r: [u64; 16] = q[0..16].try_into().unwrap();
    let mut f = [0u64; 48];
    f[0] = 1;
    for &bit in LOOP_LENGHT_BE.iter().skip(1) {
        // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(r)}
        let (lambda, mu) = fcall_bn254_dbl_line_coeffs(&r);

        // Check that the line is correct
        assert!(is_tangent_bn254(&r, &lambda, &mu));

        // Compute f = f² · line_{twist(R),twist(R)}(P)
        f = square_fp12_bn254(&f);
        let l = line_eval_bn254(&lambda, &mu, &xp_prime, &yp_prime);
        f = mul_fp12_bn254(&f, &l);

        // Double r
        r = line_dbl_bn254(&r, &lambda, &mu);

        if bit * bit == 1 {
            let q_prime = if bit == 1 { q } else { &neg_twist_bn254(&q) };

            // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(q')}
            let (lambda, mu) = fcall_bn254_add_line_coeffs(&r, q_prime);

            // Check that the line is correct
            assert!(is_line_bn254(&r, q_prime, &lambda, &mu));

            // Compute f = f · line_{twist(R),twist(Q')}
            f = mul_fp12_bn254(&f, &line_eval_bn254(&lambda, &mu, &xp_prime, &yp_prime));

            // Add r and q'
            r = line_add_bn254(&r, q_prime, &lambda, &mu);
        }
    }

    // Compute the last two lines

    // f = f · line_{twist(R),twist(UTF(Q))}(P)
    let q_frob = utf_endomorphism_twist_bn254(&q);

    // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(utf(q))}
    let (lambda, mu) = fcall_bn254_add_line_coeffs(&r, &q_frob);
    assert!(is_line_bn254(&r, &q_frob, &lambda, &mu));

    f = mul_fp12_bn254(&f, &line_eval_bn254(&lambda, &mu, &xp_prime, &yp_prime));

    // Update r by r + utf(q)
    r = line_add_bn254(&r, &q_frob, &lambda, &mu);

    // f = f · line_{twist(R),twist(-UTF(UTF(Q)))}(P)
    let q_frob2 = neg_twist_bn254(&utf_endomorphism_twist_bn254(&q_frob));

    // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(-utf(utf(q)))}
    let (lambda, mu) = fcall_bn254_add_line_coeffs(&r, &q_frob2);
    assert!(is_line_bn254(&r, &q_frob2, &lambda, &mu));

    f = mul_fp12_bn254(&f, &line_eval_bn254(&lambda, &mu, &xp_prime, &yp_prime));

    f
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

fn is_line_bn254(q1: &[u64; 16], q2: &[u64; 16], lambda: &[u64; 8], mu: &[u64; 8]) -> bool {
    // Check if the line passes through q1 and q2
    let x1: &[u64; 8] = q1[0..8].try_into().unwrap();
    let y1: &[u64; 8] = q1[8..16].try_into().unwrap();
    let x2: &[u64; 8] = q2[0..8].try_into().unwrap();
    let y2: &[u64; 8] = q2[8..16].try_into().unwrap();

    // Check if the line passes through q1
    let check_q1 = line_check(x1, y1, lambda, mu);
    // Check if the line passes through q2
    let check_q2 = line_check(x2, y2, lambda, mu);

    check_q1 && check_q2
}

fn is_tangent_bn254(q: &[u64; 16], lambda: &[u64; 8], mu: &[u64; 8]) -> bool {
    // Check if the line is tangent to the curve at q
    let x: &[u64; 8] = q[0..8].try_into().unwrap();
    let y: &[u64; 8] = q[8..16].try_into().unwrap();

    // Check if the line is tangent to the curve at q
    let check_q = line_check(x, y, lambda, mu);

    // Check that 2𝜆y = 3x²
    let mut lhs = mul_fp2_bn254(lambda, y);
    lhs = dbl_fp2_bn254(&lhs);

    let mut rhs = square_fp2_bn254(x);
    rhs = scalar_mul_fp2_bn254(&rhs, &[3, 0, 0, 0]);

    check_q && eq(&lhs, &rhs)
}

fn line_check(x: &[u64; 8], y: &[u64; 8], lambda: &[u64; 8], mu: &[u64; 8]) -> bool {
    // Check if y = λx + μ
    let mut rhs = mul_fp2_bn254(lambda, x);
    rhs = add_fp2_bn254(&rhs, mu);
    eq(&rhs, y)
}

/// Evaluates the line function l(x,y) := 1 + λxw - μyw³
fn line_eval_bn254(lambda: &[u64; 8], mu: &[u64; 8], x: &[u64; 4], y: &[u64; 4]) -> [u64; 48] {
    let coeff1 = scalar_mul_fp2_bn254(lambda, x);
    let coeff2 = scalar_mul_fp2_bn254(mu, &neg_fp_bn254(y));

    let mut result = [0; 48];
    result[0] = 1;
    result[8..16].copy_from_slice(&coeff1);
    result[24..32].copy_from_slice(&coeff2);

    result
}

fn line_add_bn254(q1: &[u64; 16], q2: &[u64; 16], lambda: &[u64; 8], mu: &[u64; 8]) -> [u64; 16] {
    let x1: &[u64; 8] = q1[0..8].try_into().unwrap();
    let x2: &[u64; 8] = q2[0..8].try_into().unwrap();

    // Compute x3 = λ² - x1 - x2
    let mut x3 = square_fp2_bn254(lambda);
    x3 = sub_fp2_bn254(&x3, x1);
    x3 = sub_fp2_bn254(&x3, x2);

    // Compute y3 = - μ - λx3
    let mut y3 = mul_fp2_bn254(lambda, &x3);
    y3 = add_fp2_bn254(mu, &y3);
    y3 = neg_fp2_bn254(&y3);

    [
        x3[0], x3[1], x3[2], x3[3], x3[4], x3[5], x3[6], x3[7], y3[0], y3[1], y3[2], y3[3], y3[4],
        y3[5], y3[6], y3[7],
    ]
}

fn line_dbl_bn254(q: &[u64; 16], lambda: &[u64; 8], mu: &[u64; 8]) -> [u64; 16] {
    let x: &[u64; 8] = q[0..8].try_into().unwrap();

    // Compute x3 = λ² - 2x
    let mut x3 = square_fp2_bn254(lambda);
    x3 = sub_fp2_bn254(&x3, &dbl_fp2_bn254(x));

    // Compute y3 = - μ - λx3
    let mut y3 = mul_fp2_bn254(lambda, &x3);
    y3 = add_fp2_bn254(mu, &y3);
    y3 = neg_fp2_bn254(&y3);

    [
        x3[0], x3[1], x3[2], x3[3], x3[4], x3[5], x3[6], x3[7], y3[0], y3[1], y3[2], y3[3], y3[4],
        y3[5], y3[6], y3[7],
    ]
}
