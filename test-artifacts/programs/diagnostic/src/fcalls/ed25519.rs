use ziskos::zisklib::fcall_ed25519_fp_sqrt_ratio;

pub fn diagnostic_ed25519() {
    diagnostic_ed25519_fp_sqrt_ratio();
}

fn diagnostic_ed25519_fp_sqrt_ratio() {
    // 4/1 is a square: the even root is 2.
    assert_eq!(fcall_ed25519_fp_sqrt_ratio(&[4, 0, 0, 0], &[1, 0, 0, 0]), [1, 2, 0, 0, 0]);
    // 2/1 is not (2 is a non-residue mod p): the hint is the root of 2·2/1 = 4.
    assert_eq!(fcall_ed25519_fp_sqrt_ratio(&[2, 0, 0, 0], &[1, 0, 0, 0]), [0, 2, 0, 0, 0]);
    // 0/v = 0 is a square.
    assert_eq!(fcall_ed25519_fp_sqrt_ratio(&[0, 0, 0, 0], &[5, 0, 0, 0]), [1, 0, 0, 0, 0]);
}
