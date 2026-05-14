use ziskos::zisklib::{
    checked_mul256, checked_square256, inv256, overflowing_mul256, overflowing_square256,
    saturating_mul256, saturating_square256, wrapping_mul256, wrapping_square256,
};

use crate::constants::*;

pub fn mul_tests() {
    let pow2_64: [u64; 4] = [0, 1, 0, 0]; // 2^64
    let pow2_128: [u64; 4] = [0, 0, 1, 0]; // 2^128

    // ── overflowing_mul256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_mul256(&ZERO, &ONE), (ZERO, false));
    assert_eq!(overflowing_mul256(&ONE, &ONE), (ONE, false));
    assert_eq!(overflowing_mul256(&TWO, &[3, 0, 0, 0]), ([6, 0, 0, 0], false));
    // (2^64)^2 = 2^128 — no overflow
    assert_eq!(overflowing_mul256(&pow2_64, &pow2_64), (pow2_128, false));
    // MAX * 2: low = MAX-1, overflow
    assert_eq!(
        overflowing_mul256(&MAX, &TWO),
        ([u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX], true)
    );
    // 2^128 * 2^128 = 2^256 ≡ 0 (mod 2^256), overflow
    assert_eq!(overflowing_mul256(&pow2_128, &pow2_128), (ZERO, true));

    // ── checked_mul256 ────────────────────────────────────────────────────────
    assert_eq!(checked_mul256(&TWO, &[3, 0, 0, 0]), Some([6, 0, 0, 0]));
    assert_eq!(checked_mul256(&ONE, &ONE), Some(ONE));
    assert_eq!(checked_mul256(&MAX, &TWO), None);
    assert_eq!(checked_mul256(&pow2_128, &pow2_128), None);

    // ── overflowing_square256 ─────────────────────────────────────────────────
    assert_eq!(overflowing_square256(&ZERO), (ZERO, false));
    assert_eq!(overflowing_square256(&ONE), (ONE, false));
    assert_eq!(overflowing_square256(&TWO), ([4, 0, 0, 0], false));
    // (2^64)^2 = 2^128 — no overflow
    assert_eq!(overflowing_square256(&pow2_64), (pow2_128, false));
    // (2^128)^2 = 2^256 ≡ 0 (mod 2^256), overflow
    assert_eq!(overflowing_square256(&pow2_128), (ZERO, true));

    // ── checked_square256 ────────────────────────────────────────────────────
    assert_eq!(checked_square256(&[3, 0, 0, 0]), Some([9, 0, 0, 0]));
    assert_eq!(checked_square256(&pow2_64), Some(pow2_128));
    assert_eq!(checked_square256(&pow2_128), None);

    // ── saturating_mul256 ────────────────────────────────────────────────────
    assert_eq!(saturating_mul256(&TWO, &[3, 0, 0, 0]), [6, 0, 0, 0]);
    assert_eq!(saturating_mul256(&MAX, &TWO), MAX);
    assert_eq!(saturating_mul256(&pow2_128, &pow2_128), MAX);

    // ── saturating_square256 ─────────────────────────────────────────────────
    assert_eq!(saturating_square256(&[3, 0, 0, 0]), [9, 0, 0, 0]);
    assert_eq!(saturating_square256(&pow2_128), MAX);

    // ── wrapping_mul256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_mul256(&TWO, &[3, 0, 0, 0]), [6, 0, 0, 0]);
    assert_eq!(wrapping_mul256(&MAX, &TWO), [u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX]);
    assert_eq!(wrapping_mul256(&pow2_128, &pow2_128), ZERO);

    // ── wrapping_square256 ────────────────────────────────────────────────────
    assert_eq!(wrapping_square256(&[3, 0, 0, 0]), [9, 0, 0, 0]);
    assert_eq!(wrapping_square256(&pow2_64), pow2_128);
    assert_eq!(wrapping_square256(&pow2_128), ZERO);

    // ── inv256 (mod 2^256) ────────────────────────────────────────────────────
    // even numbers have no inverse
    assert_eq!(inv256(&ZERO), None);
    assert_eq!(inv256(&TWO), None);
    assert_eq!(inv256(&[0, 0, 0, 0]), None);
    // 1 is its own inverse
    assert_eq!(inv256(&ONE), Some(ONE));
    // known values from the fcalls_impl test suite
    assert_eq!(
        inv256(&[3, 0, 0, 0]),
        Some([0xaaaaaaaaaaaaaaab, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaaaaaaaaaa])
    );
    assert_eq!(
        inv256(&[
            0xee453cbb08caf011_u64,
            0x403f9ad46fdfbf18,
            0x190bbcf54d8ad535,
            0x9d4a5af226af865c
        ]),
        Some([0x91f6316a1db400f1_u64, 0xa62de0c72fbf1f2b, 0x8cc70b2dcf824747, 0x78bccb02bfaa76af])
    );
}
