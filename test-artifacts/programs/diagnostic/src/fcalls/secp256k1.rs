use ziskos::zisklib::{
    fcall_secp256k1_fn_inv, fcall_secp256k1_fp_inv, fcall_secp256k1_fp_sqrt,
    fcall_secp256k1_glv_decompose,
};

pub fn diagnostic_secp256k1() {
    diagnostic_secp256k1_fp_inv();
    diagnostic_secp256k1_fp_sqrt();
    diagnostic_secp256k1_fn_inv();
    diagnostic_secp256k1_glv_decompose();
}

fn diagnostic_secp256k1_fp_inv() {
    // 1⁻¹ ≡ 1 (mod p)
    let inv = fcall_secp256k1_fp_inv(&[1, 0, 0, 0]);
    assert_eq!(inv, [1, 0, 0, 0]);
}

fn diagnostic_secp256k1_fp_sqrt() {
    // √1 with odd-parity selector returns 1 itself (the even root is p−1).
    let out = fcall_secp256k1_fp_sqrt(&[1, 0, 0, 0], 1);
    assert_eq!(out, [1, 1, 0, 0, 0]);
}

fn diagnostic_secp256k1_fn_inv() {
    let inv = fcall_secp256k1_fn_inv(&[1, 0, 0, 0]);
    assert_eq!(inv, [1, 0, 0, 0]);
}

fn diagnostic_secp256k1_glv_decompose() {
    // k = 1  ⇒  k1 = 1, k2 = 0, both positive.
    let out = fcall_secp256k1_glv_decompose(&[1, 0, 0, 0]);
    assert_eq!(out, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
}
