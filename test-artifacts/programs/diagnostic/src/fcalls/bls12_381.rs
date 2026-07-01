use ziskos::zisklib::{
    fcall_bls12_381_fp2_inv, fcall_bls12_381_fp2_sqrt, fcall_bls12_381_fp_inv,
    fcall_bls12_381_fp_sqrt, fcall_bls12_381_twist_add_line_coeffs,
    fcall_bls12_381_twist_dbl_line_coeffs,
};

pub fn diagnostic_bls12_381() {
    diagnostic_bls12_381_fp_inv();
    diagnostic_bls12_381_fp_sqrt();
    diagnostic_bls12_381_fp2_inv();
    diagnostic_bls12_381_fp2_sqrt();
    diagnostic_bls12_381_twist_add_line_coeffs();
    diagnostic_bls12_381_twist_dbl_line_coeffs();
}

fn diagnostic_bls12_381_fp_inv() {
    let inv = fcall_bls12_381_fp_inv(&[1, 0, 0, 0, 0, 0]);
    assert_eq!(inv, [1, 0, 0, 0, 0, 0]);
}

fn diagnostic_bls12_381_fp_sqrt() {
    // √1 in Fp via a^((p+1)/4) is 1; flag = 1.
    let out = fcall_bls12_381_fp_sqrt(&[1, 0, 0, 0, 0, 0]);
    assert_eq!(out, [1, 1, 0, 0, 0, 0, 0]);
}

fn diagnostic_bls12_381_fp2_inv() {
    let inv = fcall_bls12_381_fp2_inv(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(inv, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}

fn diagnostic_bls12_381_fp2_sqrt() {
    // √(1+0i) selects p−1 by convention; here we only assert the existence flag.
    let out = fcall_bls12_381_fp2_sqrt(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(out[0], 1);
}

fn diagnostic_bls12_381_twist_add_line_coeffs() {
    // p1 = (1+0i, 2+0i), p2 = (3+0i, 4+0i)  →  λ = 1+0i, μ = 1+0i.
    let mut p1 = [0u64; 24];
    p1[0] = 1; // x1 real
    p1[12] = 2; // y1 real
    let mut p2 = [0u64; 24];
    p2[0] = 3; // x2 real
    p2[12] = 4; // y2 real
    let (lambda, mu) = fcall_bls12_381_twist_add_line_coeffs(&p1, &p2);
    let mut expected = [0u64; 12];
    expected[0] = 1;
    assert_eq!(lambda, expected);
    assert_eq!(mu, expected);
}

fn diagnostic_bls12_381_twist_dbl_line_coeffs() {
    // p = (0+0i, 1+0i): λ = 0, μ = 1+0i.
    let mut p = [0u64; 24];
    p[12] = 1; // y real
    let (lambda, mu) = fcall_bls12_381_twist_dbl_line_coeffs(&p);
    let mut expected_mu = [0u64; 12];
    expected_mu[0] = 1;
    assert_eq!(lambda, [0u64; 12]);
    assert_eq!(mu, expected_mu);
}
