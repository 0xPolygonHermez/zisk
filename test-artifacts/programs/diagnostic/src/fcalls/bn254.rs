use ziskos::zisklib::{
    fcall_bn254_fp2_inv, fcall_bn254_fp_inv, fcall_bn254_twist_add_line_coeffs,
    fcall_bn254_twist_dbl_line_coeffs,
};

pub fn diagnostic_bn254() {
    diagnostic_bn254_fp_inv();
    diagnostic_bn254_fp2_inv();
    diagnostic_bn254_twist_add_line_coeffs();
    diagnostic_bn254_twist_dbl_line_coeffs();
}

fn diagnostic_bn254_fp_inv() {
    let inv = fcall_bn254_fp_inv(&[1, 0, 0, 0]);
    assert_eq!(inv, [1, 0, 0, 0]);
}

fn diagnostic_bn254_fp2_inv() {
    // (1 + 0i)⁻¹ = 1 + 0i
    let inv = fcall_bn254_fp2_inv(&[1, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(inv, [1, 0, 0, 0, 0, 0, 0, 0]);
}

fn diagnostic_bn254_twist_add_line_coeffs() {
    // p1 = (1+0i, 2+0i), p2 = (3+0i, 4+0i)
    //   λ = (y2-y1)/(x2-x1) = 2/2 = 1+0i
    //   μ = y1 − λ·x1       = 2 − 1·1 = 1+0i
    let p1: [u64; 16] = [1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
    let p2: [u64; 16] = [3, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0];
    let (lambda, mu) = fcall_bn254_twist_add_line_coeffs(&p1, &p2);
    assert_eq!(lambda, [1, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(mu, [1, 0, 0, 0, 0, 0, 0, 0]);
}

fn diagnostic_bn254_twist_dbl_line_coeffs() {
    // p = (0+0i, 1+0i):  λ = 3x²/(2y) = 0,  μ = y − λx = 1+0i.
    let p: [u64; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0];
    let (lambda, mu) = fcall_bn254_twist_dbl_line_coeffs(&p);
    assert_eq!(lambda, [0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(mu, [1, 0, 0, 0, 0, 0, 0, 0]);
}
