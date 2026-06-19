use ziskos::zisklib::{
    fcall_msb_pos_256, fcall_msb_pos_256_2, fcall_msb_pos_256_4, fcall_msb_pos_384,
};

pub fn diagnostic_msb() {
    diagnostic_msb_pos_256();
    diagnostic_msb_pos_256_2();
    diagnostic_msb_pos_256_4();
    diagnostic_msb_pos_384();
}

fn diagnostic_msb_pos_256() {
    // 5 = 0b101  →  limb 0, bit 2.
    let (limb, bit) = fcall_msb_pos_256(&[5, 0, 0, 0]);
    assert_eq!((limb, bit), (0, 2));
}

fn diagnostic_msb_pos_256_2() {
    // max(5 in limb 0 bit 2, 8 in limb 2 bit 3) → (2, 3).
    let (limb, bit) = fcall_msb_pos_256_2(&[5, 0, 0, 0], &[0, 0, 8, 0]);
    assert_eq!((limb, bit), (2, 3));
}

fn diagnostic_msb_pos_256_4() {
    // Only w is non-zero: 16 in limb 0 bit 4.
    let (limb, bit) =
        fcall_msb_pos_256_4(&[0, 0, 0, 0], &[0, 0, 0, 0], &[0, 0, 0, 0], &[16, 0, 0, 0]);
    assert_eq!((limb, bit), (0, 4));
}

fn diagnostic_msb_pos_384() {
    // x has bit 2 of limb 0; y has bit 3 of limb 5 — y wins.
    let (limb, bit) = fcall_msb_pos_384(&[5, 0, 0, 0, 0, 0], &[0, 0, 0, 0, 0, 8]);
    assert_eq!((limb, bit), (5, 3));
}
