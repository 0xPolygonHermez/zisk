use ziskos::zisklib::{fcall_uint256_div, fcall_uint256_inv, fcall_uint256_inv_mod};

pub fn diagnostic_uint256() {
    diagnostic_uint256_div();
    diagnostic_uint256_inv();
    diagnostic_uint256_inv_mod();
}

fn diagnostic_uint256_div() {
    let (quo, rem) = fcall_uint256_div(&[10, 0, 0, 0], &[3, 0, 0, 0]);
    assert_eq!(quo, [3, 0, 0, 0]);
    assert_eq!(rem, [1, 0, 0, 0]);
}

fn diagnostic_uint256_inv() {
    // 1⁻¹ ≡ 1 (mod 2²⁵⁶).
    let inv = fcall_uint256_inv(&[1, 0, 0, 0]);
    assert_eq!(inv, Some([1, 0, 0, 0]));
}

fn diagnostic_uint256_inv_mod() {
    // 2⁻¹ mod 5 = 3.
    let inv = fcall_uint256_inv_mod(&[2, 0, 0, 0], &[5, 0, 0, 0]);
    assert_eq!(inv, Some([3, 0, 0, 0]));
}
