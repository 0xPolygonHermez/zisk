use ziskos::zisklib::{checked_pow256, overflowing_pow256, saturating_pow256, wrapping_pow256};

use crate::constants::*;

pub fn pow_tests() {
    // ── overflowing_pow256 ────────────────────────────────────────────────────

    // Special-case early returns
    // base^0 = 1 (including 0^0)
    assert_eq!(overflowing_pow256(&ZERO, &ZERO), (ONE, false));
    assert_eq!(overflowing_pow256(&[42, 0, 0, 0], &ZERO), (ONE, false));
    // base^1 = base
    assert_eq!(overflowing_pow256(&[42, 0, 0, 0], &ONE), ([42, 0, 0, 0], false));
    // 0^exp = 0
    assert_eq!(overflowing_pow256(&ZERO, &[5, 0, 0, 0]), (ZERO, false));
    // 1^exp = 1
    assert_eq!(overflowing_pow256(&ONE, &[100, 0, 0, 0]), (ONE, false));

    // Power-of-two exponent path (repeated squaring only)
    // 2^2 = 4  (exp=2=2^1, one squaring)
    assert_eq!(overflowing_pow256(&TWO, &TWO), ([4, 0, 0, 0], false));
    // 2^4 = 16 (exp=4=2^2, two squarings)
    assert_eq!(overflowing_pow256(&TWO, &[4, 0, 0, 0]), ([16, 0, 0, 0], false));
    // 3^4 = 81
    assert_eq!(overflowing_pow256(&[3, 0, 0, 0], &[4, 0, 0, 0]), ([81, 0, 0, 0], false));

    // General square-and-multiply path
    // 2^3 = 8  (exp=3 = 0b11)
    assert_eq!(overflowing_pow256(&TWO, &[3, 0, 0, 0]), ([8, 0, 0, 0], false));
    // 2^5 = 32 (exp=5 = 0b101)
    assert_eq!(overflowing_pow256(&TWO, &[5, 0, 0, 0]), ([32, 0, 0, 0], false));
    // 3^5 = 243 (exp=5 = 0b101)
    assert_eq!(overflowing_pow256(&[3, 0, 0, 0], &[5, 0, 0, 0]), ([243, 0, 0, 0], false));

    // Overflow cases
    // MAX^2 mod 2^256 = (-1)^2 mod 2^256 = 1, overflow
    assert_eq!(overflowing_pow256(&MAX, &TWO), (ONE, true));
    // 2^256 mod 2^256 = 0, overflow  (exp=256=2^8, power-of-two path)
    assert_eq!(overflowing_pow256(&TWO, &[256, 0, 0, 0]), (ZERO, true));

    // ── checked_pow256 ────────────────────────────────────────────────────────
    assert_eq!(checked_pow256(&TWO, &[10, 0, 0, 0]), Some([1024, 0, 0, 0]));
    assert_eq!(checked_pow256(&[3, 0, 0, 0], &[5, 0, 0, 0]), Some([243, 0, 0, 0]));
    assert_eq!(checked_pow256(&TWO, &ZERO), Some(ONE));
    assert_eq!(checked_pow256(&MAX, &TWO), None);
    assert_eq!(checked_pow256(&TWO, &[256, 0, 0, 0]), None);

    // ── saturating_pow256 ─────────────────────────────────────────────────────
    assert_eq!(saturating_pow256(&TWO, &[10, 0, 0, 0]), [1024, 0, 0, 0]);
    assert_eq!(saturating_pow256(&ZERO, &[99, 0, 0, 0]), ZERO);
    assert_eq!(saturating_pow256(&ONE, &MAX), ONE);
    assert_eq!(saturating_pow256(&MAX, &TWO), MAX);
    assert_eq!(saturating_pow256(&TWO, &[256, 0, 0, 0]), MAX);

    // ── wrapping_pow256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_pow256(&TWO, &[10, 0, 0, 0]), [1024, 0, 0, 0]);
    assert_eq!(wrapping_pow256(&[3, 0, 0, 0], &[5, 0, 0, 0]), [243, 0, 0, 0]);
    assert_eq!(wrapping_pow256(&ZERO, &[5, 0, 0, 0]), ZERO);
    assert_eq!(wrapping_pow256(&ONE, &[100, 0, 0, 0]), ONE);
    // MAX^2 wraps to 1
    assert_eq!(wrapping_pow256(&MAX, &TWO), ONE);
    // 2^256 wraps to 0
    assert_eq!(wrapping_pow256(&TWO, &[256, 0, 0, 0]), ZERO);
}
