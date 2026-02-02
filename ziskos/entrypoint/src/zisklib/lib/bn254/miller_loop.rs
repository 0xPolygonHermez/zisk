//! Miller Loop for the pairings over BN254

use crate::zisklib::{eq, fcall_bn254_twist_add_line_coeffs, fcall_bn254_twist_dbl_line_coeffs};

use super::{
    fp::{inv_fp_bn254, mul_fp_bn254, neg_fp_bn254},
    fp12::{sparse_mul_fp12_bn254, square_fp12_bn254},
    fp2::{
        add_fp2_bn254, dbl_fp2_bn254, mul_fp2_bn254, neg_fp2_bn254, scalar_mul_fp2_bn254,
        square_fp2_bn254, sub_fp2_bn254,
    },
    twist::{neg_twist_bn254, utf_endomorphism_twist_bn254},
};

/// Pseudobinary representation of the loop length 6·X+2 of the
/// optimal ate pairing over the BN254.
const LOOP_LENGTH: [i8; 65] = [
    1, 1, 0, 1, 0, 0, -1, 0, 1, 1, 0, 0, 0, -1, 0, 0, 1, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 0, -1,
    0, 0, 1, 1, 1, 0, 0, 0, 0, -1, 0, 1, 0, 0, -1, 0, 1, 1, 0, 0, 1, 0, 0, -1, 1, 0, 0, -1, 0, 1,
    0, 1, 0, 0, 0,
];

/// Computes the Miller loop of a non-zero point `p` in G1 and a non-zero point `q` in G2
pub fn miller_loop_bn254(
    p: &[u64; 8],
    q: &[u64; 16],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 48] {
    // Before the loop starts, compute xp' = -xp/yp and yp' = 1/yp
    let mut xp_prime: [u64; 4] = p[0..4].try_into().unwrap();
    let mut yp_prime: [u64; 4] = p[4..8].try_into().unwrap();
    yp_prime = inv_fp_bn254(
        &yp_prime,
        #[cfg(feature = "hints")]
        hints,
    );
    xp_prime = neg_fp_bn254(
        &xp_prime,
        #[cfg(feature = "hints")]
        hints,
    );
    xp_prime = mul_fp_bn254(
        &xp_prime,
        &yp_prime,
        #[cfg(feature = "hints")]
        hints,
    );

    // Initialize the Miller loop with r = q and f = 1
    let mut r: [u64; 16] = q[0..16].try_into().unwrap();
    let mut f = [0u64; 48];
    f[0] = 1;
    for &bit in LOOP_LENGTH.iter().skip(1) {
        // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(r)}
        let (lambda, mu) = fcall_bn254_twist_dbl_line_coeffs(
            &r,
            #[cfg(feature = "hints")]
            hints,
        );

        // Check that the line is correct
        assert!(is_tangent_twist_bn254(
            &r,
            &lambda,
            &mu,
            #[cfg(feature = "hints")]
            hints,
        ));

        // Compute f = f² · line_{twist(r),twist(r)}(p)
        f = square_fp12_bn254(
            &f,
            #[cfg(feature = "hints")]
            hints,
        );
        let l = line_eval_twist_bn254(
            &lambda,
            &mu,
            &xp_prime,
            &yp_prime,
            #[cfg(feature = "hints")]
            hints,
        );
        f = sparse_mul_fp12_bn254(
            &f,
            &l,
            #[cfg(feature = "hints")]
            hints,
        );

        // Double r
        r = dbl_twist_with_hints_bn254(
            &r,
            &lambda,
            &mu,
            #[cfg(feature = "hints")]
            hints,
        );

        if bit * bit == 1 {
            let q_prime = if bit == 1 {
                q
            } else {
                &neg_twist_bn254(
                    q,
                    #[cfg(feature = "hints")]
                    hints,
                )
            };

            // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(q')}
            let (lambda, mu) = fcall_bn254_twist_add_line_coeffs(
                &r,
                q_prime,
                #[cfg(feature = "hints")]
                hints,
            );

            // Check that the line is correct
            assert!(is_line_twist_bn254(
                &r,
                q_prime,
                &lambda,
                &mu,
                #[cfg(feature = "hints")]
                hints,
            ));

            // Compute f = f · line_{twist(r),twist(q')}
            let l = line_eval_twist_bn254(
                &lambda,
                &mu,
                &xp_prime,
                &yp_prime,
                #[cfg(feature = "hints")]
                hints,
            );
            f = sparse_mul_fp12_bn254(
                &f,
                &l,
                #[cfg(feature = "hints")]
                hints,
            );

            // Add r and q'
            r = add_twist_with_hints_bn254(
                &r,
                q_prime,
                &lambda,
                &mu,
                #[cfg(feature = "hints")]
                hints,
            );
        }
    }

    // Compute the last two lines

    // f = f · line_{twist(r),twist(utf(q))}(p)
    let q_frob = utf_endomorphism_twist_bn254(
        q,
        #[cfg(feature = "hints")]
        hints,
    );

    // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(utf(q))}
    let (lambda, mu) = fcall_bn254_twist_add_line_coeffs(
        &r,
        &q_frob,
        #[cfg(feature = "hints")]
        hints,
    );
    assert!(is_line_twist_bn254(
        &r,
        &q_frob,
        &lambda,
        &mu,
        #[cfg(feature = "hints")]
        hints,
    ));

    let l = line_eval_twist_bn254(
        &lambda,
        &mu,
        &xp_prime,
        &yp_prime,
        #[cfg(feature = "hints")]
        hints,
    );
    f = sparse_mul_fp12_bn254(
        &f,
        &l,
        #[cfg(feature = "hints")]
        hints,
    );

    // Update r by r + utf(q)
    r = add_twist_with_hints_bn254(
        &r,
        &q_frob,
        &lambda,
        &mu,
        #[cfg(feature = "hints")]
        hints,
    );

    // f = f · line_{twist(r),twist(-utf(utf(q)))}(p)
    let q_frob2 = neg_twist_bn254(
        &utf_endomorphism_twist_bn254(
            &q_frob,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );

    // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(-utf(utf(q)))}
    let (lambda, mu) = fcall_bn254_twist_add_line_coeffs(
        &r,
        &q_frob2,
        #[cfg(feature = "hints")]
        hints,
    );
    assert!(is_line_twist_bn254(
        &r,
        &q_frob2,
        &lambda,
        &mu,
        #[cfg(feature = "hints")]
        hints
    ));

    let l = line_eval_twist_bn254(
        &lambda,
        &mu,
        &xp_prime,
        &yp_prime,
        #[cfg(feature = "hints")]
        hints,
    );
    f = sparse_mul_fp12_bn254(
        &f,
        &l,
        #[cfg(feature = "hints")]
        hints,
    );

    f
}

/// Computes the Miller loop for the BN254 curve for a batch of non-zero points `p_i` in G1 and non-zero points `q_i` in G2
pub fn miller_loop_batch_bn254(
    g1_points: &[[u64; 8]],
    g2_points: &[[u64; 16]],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 48] {
    // Before the loop starts, compute xp' = -xp/yp and yp' = 1/yp for each point p
    let mut xp_primes: Vec<[u64; 4]> = Vec::with_capacity(g1_points.len());
    let mut yp_primes: Vec<[u64; 4]> = Vec::with_capacity(g1_points.len());
    for p in g1_points.iter() {
        let mut xp_prime: [u64; 4] = p[0..4].try_into().unwrap();
        let mut yp_prime: [u64; 4] = p[4..8].try_into().unwrap();
        yp_prime = inv_fp_bn254(
            &yp_prime,
            #[cfg(feature = "hints")]
            hints,
        );
        xp_prime = neg_fp_bn254(
            &xp_prime,
            #[cfg(feature = "hints")]
            hints,
        );
        xp_prime = mul_fp_bn254(
            &xp_prime,
            &yp_prime,
            #[cfg(feature = "hints")]
            hints,
        );

        xp_primes.push(xp_prime);
        yp_primes.push(yp_prime);
    }

    // Initialize the Miller loop with r_i = q_i and f = 1
    let mut r: Vec<[u64; 16]> = g2_points.iter().map(|q| q[0..16].try_into().unwrap()).collect();
    let mut f = [0u64; 48];
    f[0] = 1;
    let n = g1_points.len();
    for &bit in LOOP_LENGTH.iter().skip(1) {
        // Compute f = f² · line_{twist(r),twist(r)}(p)
        f = square_fp12_bn254(
            &f,
            #[cfg(feature = "hints")]
            hints,
        );

        for i in 0..n {
            let r = &mut r[i];

            // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(r)}
            let (lambda, mu) = fcall_bn254_twist_dbl_line_coeffs(
                r,
                #[cfg(feature = "hints")]
                hints,
            );

            // Check that the line is correct
            assert!(is_tangent_twist_bn254(
                r,
                &lambda,
                &mu,
                #[cfg(feature = "hints")]
                hints,
            ));

            let xp_prime = &xp_primes[i];
            let yp_prime = &yp_primes[i];
            let l = line_eval_twist_bn254(
                &lambda,
                &mu,
                xp_prime,
                yp_prime,
                #[cfg(feature = "hints")]
                hints,
            );
            f = sparse_mul_fp12_bn254(
                &f,
                &l,
                #[cfg(feature = "hints")]
                hints,
            );

            // Double r
            *r = dbl_twist_with_hints_bn254(
                r,
                &lambda,
                &mu,
                #[cfg(feature = "hints")]
                hints,
            );

            if bit * bit == 1 {
                let q = &g2_points[i];
                let q_prime = if bit == 1 {
                    q
                } else {
                    &neg_twist_bn254(
                        q,
                        #[cfg(feature = "hints")]
                        hints,
                    )
                };

                // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(q')}
                let (lambda, mu) = fcall_bn254_twist_add_line_coeffs(
                    r,
                    q_prime,
                    #[cfg(feature = "hints")]
                    hints,
                );

                // Check that the line is correct
                assert!(is_line_twist_bn254(
                    r,
                    q_prime,
                    &lambda,
                    &mu,
                    #[cfg(feature = "hints")]
                    hints,
                ));

                // Compute f = f · line_{twist(r),twist(q')}
                let l = line_eval_twist_bn254(
                    &lambda,
                    &mu,
                    xp_prime,
                    yp_prime,
                    #[cfg(feature = "hints")]
                    hints,
                );
                f = sparse_mul_fp12_bn254(
                    &f,
                    &l,
                    #[cfg(feature = "hints")]
                    hints,
                );

                // Add r and q'
                *r = add_twist_with_hints_bn254(
                    r,
                    q_prime,
                    &lambda,
                    &mu,
                    #[cfg(feature = "hints")]
                    hints,
                );
            }
        }
    }

    // Compute the last two lines
    for i in 0..n {
        let q = &g2_points[i];
        let r = &mut r[i];
        let xp_prime = &xp_primes[i];
        let yp_prime = &yp_primes[i];

        // f = f · line_{twist(r),twist(utf(q))}(p)
        let q_frob = utf_endomorphism_twist_bn254(
            q,
            #[cfg(feature = "hints")]
            hints,
        );

        // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(utf(q))}
        let (lambda, mu) = fcall_bn254_twist_add_line_coeffs(
            r,
            &q_frob,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(is_line_twist_bn254(
            r,
            &q_frob,
            &lambda,
            &mu,
            #[cfg(feature = "hints")]
            hints,
        ));

        let l = line_eval_twist_bn254(
            &lambda,
            &mu,
            xp_prime,
            yp_prime,
            #[cfg(feature = "hints")]
            hints,
        );
        f = sparse_mul_fp12_bn254(
            &f,
            &l,
            #[cfg(feature = "hints")]
            hints,
        );

        // Update r by r + utf(q)
        *r = add_twist_with_hints_bn254(
            r,
            &q_frob,
            &lambda,
            &mu,
            #[cfg(feature = "hints")]
            hints,
        );
        // f = f · line_{twist(r),twist(-utf(utf(q)))}(p)
        let q_frob2 = neg_twist_bn254(
            &utf_endomorphism_twist_bn254(
                &q_frob,
                #[cfg(feature = "hints")]
                hints,
            ),
            #[cfg(feature = "hints")]
            hints,
        );

        // Hint the coefficients (𝜆,𝜇) of the line l_{twist(r),twist(-utf(utf(q)))}
        let (lambda, mu) = fcall_bn254_twist_add_line_coeffs(
            r,
            &q_frob2,
            #[cfg(feature = "hints")]
            hints,
        );
        assert!(is_line_twist_bn254(
            r,
            &q_frob2,
            &lambda,
            &mu,
            #[cfg(feature = "hints")]
            hints,
        ));

        let l = line_eval_twist_bn254(
            &lambda,
            &mu,
            xp_prime,
            yp_prime,
            #[cfg(feature = "hints")]
            hints,
        );
        f = sparse_mul_fp12_bn254(
            &f,
            &l,
            #[cfg(feature = "hints")]
            hints,
        );
    }

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

/// Checks if the line defined by (𝜆,𝜇) passes through non-zero points `q1,q2` in G2
#[inline]
fn is_line_twist_bn254(
    q1: &[u64; 16],
    q2: &[u64; 16],
    lambda: &[u64; 8],
    mu: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // Check if the line passes through q1
    let check_q1 = line_check_twist_bn254(
        q1,
        lambda,
        mu,
        #[cfg(feature = "hints")]
        hints,
    );
    // Check if the line passes through q2
    let check_q2 = line_check_twist_bn254(
        q2,
        lambda,
        mu,
        #[cfg(feature = "hints")]
        hints,
    );
    check_q1 && check_q2
}

/// Checks if the line defined by (𝜆,𝜇) is tangent to the curve at non-zero point `q` in G2
#[inline]
fn is_tangent_twist_bn254(
    q: &[u64; 16],
    lambda: &[u64; 8],
    mu: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    // Check if the line is tangent to the curve at q

    // Check if the line passes through q
    let check_q = line_check_twist_bn254(
        q,
        lambda,
        mu,
        #[cfg(feature = "hints")]
        hints,
    );
    // Check that 2𝜆y = 3x²
    let x: &[u64; 8] = q[0..8].try_into().unwrap();
    let y: &[u64; 8] = q[8..16].try_into().unwrap();
    let mut lhs = mul_fp2_bn254(
        lambda,
        y,
        #[cfg(feature = "hints")]
        hints,
    );
    lhs = dbl_fp2_bn254(
        &lhs,
        #[cfg(feature = "hints")]
        hints,
    );

    let mut rhs = square_fp2_bn254(
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = scalar_mul_fp2_bn254(
        &rhs,
        &[3, 0, 0, 0],
        #[cfg(feature = "hints")]
        hints,
    );
    check_q && eq(&lhs, &rhs)
}

/// Check if the line defined by (𝜆,𝜇) passes through non-zero point `q` in G2
#[inline]
fn line_check_twist_bn254(
    q: &[u64; 16],
    lambda: &[u64; 8],
    mu: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> bool {
    let x: &[u64; 8] = q[0..8].try_into().unwrap();
    let y: &[u64; 8] = q[8..16].try_into().unwrap();

    // Check if y = λx + μ
    let mut rhs = mul_fp2_bn254(
        lambda,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    rhs = add_fp2_bn254(
        &rhs,
        mu,
        #[cfg(feature = "hints")]
        hints,
    );
    eq(&rhs, y)
}

/// Evaluates the line function l(x,y) := (1 + 0·v + 0·v²) + (λx - μy·v + 0·v²)·w
#[inline]
fn line_eval_twist_bn254(
    lambda: &[u64; 8],
    mu: &[u64; 8],
    x: &[u64; 4],
    y: &[u64; 4],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 16] {
    let coeff1 = scalar_mul_fp2_bn254(
        lambda,
        x,
        #[cfg(feature = "hints")]
        hints,
    );
    let coeff2 = scalar_mul_fp2_bn254(
        mu,
        &neg_fp_bn254(
            y,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );

    let mut result = [0; 16];
    result[0..8].copy_from_slice(&coeff1);
    result[8..16].copy_from_slice(&coeff2);

    result
}

/// Addition of two non-zero points `q1,q2` in G2 with hinted line coefficients (𝜆,𝜇)
#[inline]
fn add_twist_with_hints_bn254(
    q1: &[u64; 16],
    q2: &[u64; 16],
    lambda: &[u64; 8],
    mu: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 16] {
    let x1: &[u64; 8] = q1[0..8].try_into().unwrap();
    let x2: &[u64; 8] = q2[0..8].try_into().unwrap();

    // Compute x3 = λ² - x1 - x2
    let mut x3 = square_fp2_bn254(
        lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bn254(
        &x3,
        x1,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bn254(
        &x3,
        x2,
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute y3 = -λx3 - μ
    let mut y3 = mul_fp2_bn254(
        lambda,
        &x3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = add_fp2_bn254(
        mu,
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = neg_fp2_bn254(
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );

    [
        x3[0], x3[1], x3[2], x3[3], x3[4], x3[5], x3[6], x3[7], y3[0], y3[1], y3[2], y3[3], y3[4],
        y3[5], y3[6], y3[7],
    ]
}

/// Doubling of a non-zero point `q` in G2 with hinted line coefficients (𝜆,𝜇)
#[inline]
fn dbl_twist_with_hints_bn254(
    q: &[u64; 16],
    lambda: &[u64; 8],
    mu: &[u64; 8],
    #[cfg(feature = "hints")] hints: &mut Vec<u64>,
) -> [u64; 16] {
    let x: &[u64; 8] = q[0..8].try_into().unwrap();

    // Compute x3 = λ² - 2x
    let mut x3 = square_fp2_bn254(
        lambda,
        #[cfg(feature = "hints")]
        hints,
    );
    x3 = sub_fp2_bn254(
        &x3,
        &dbl_fp2_bn254(
            x,
            #[cfg(feature = "hints")]
            hints,
        ),
        #[cfg(feature = "hints")]
        hints,
    );

    // Compute y3 = -λx3 - μ
    let mut y3 = mul_fp2_bn254(
        lambda,
        &x3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = add_fp2_bn254(
        mu,
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );
    y3 = neg_fp2_bn254(
        &y3,
        #[cfg(feature = "hints")]
        hints,
    );

    [
        x3[0], x3[1], x3[2], x3[3], x3[4], x3[5], x3[6], x3[7], y3[0], y3[1], y3[2], y3[3], y3[4],
        y3[5], y3[6], y3[7],
    ]
}
