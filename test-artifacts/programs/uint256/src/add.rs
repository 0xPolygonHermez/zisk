use ziskos::zisklib::{
    checked_add256, checked_neg256, checked_sub256, overflowing_add256, overflowing_neg256,
    overflowing_sub256, saturating_add256, saturating_sub256, wrapping_add256, wrapping_neg256,
    wrapping_sub256,
};

use crate::constants::*;

pub fn add_tests() {
    // ── overflowing_add256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_add256(&ZERO, &ZERO), (ZERO, false));
    assert_eq!(overflowing_add256(&ONE, &TWO), ([3, 0, 0, 0], false));
    // carry propagates across one limb
    assert_eq!(overflowing_add256(&[u64::MAX, 0, 0, 0], &ONE), ([0, 1, 0, 0], false));
    // carry propagates across two limbs
    assert_eq!(overflowing_add256(&[u64::MAX, u64::MAX, 0, 0], &ONE), ([0, 0, 1, 0], false));
    assert_eq!(overflowing_add256(&MAX, &ONE), (ZERO, true));
    // 2*(2^256 - 1) mod 2^256 = MAX - 1, carry 1
    assert_eq!(
        overflowing_add256(&MAX, &MAX),
        ([u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX], true)
    );

    // ── checked_add256 ────────────────────────────────────────────────────────
    assert_eq!(checked_add256(&ONE, &TWO), Some([3, 0, 0, 0]));
    assert_eq!(checked_add256(&MAX, &ZERO), Some(MAX));
    assert_eq!(checked_add256(&MAX, &ONE), None);

    // ── overflowing_sub256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_sub256(&TWO, &ONE), (ONE, false));
    assert_eq!(overflowing_sub256(&ONE, &ONE), (ZERO, false));
    assert_eq!(overflowing_sub256(&MAX, &MAX), (ZERO, false));
    // borrow propagates across one limb
    assert_eq!(overflowing_sub256(&[0, 1, 0, 0], &ONE), ([u64::MAX, 0, 0, 0], false));
    assert_eq!(overflowing_sub256(&ZERO, &ONE), (MAX, true));
    assert_eq!(overflowing_sub256(&ONE, &TWO), (MAX, true));

    // ── checked_sub256 ────────────────────────────────────────────────────────
    assert_eq!(checked_sub256(&TWO, &ONE), Some(ONE));
    assert_eq!(checked_sub256(&ONE, &ONE), Some(ZERO));
    assert_eq!(checked_sub256(&ZERO, &ONE), None);

    // ── overflowing_neg256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_neg256(&ZERO), (ZERO, false));
    assert_eq!(overflowing_neg256(&ONE), (MAX, true));
    assert_eq!(overflowing_neg256(&MAX), (ONE, true));

    // ── checked_neg256 ────────────────────────────────────────────────────────
    assert_eq!(checked_neg256(&ZERO), Some(ZERO));
    assert_eq!(checked_neg256(&ONE), None);
    assert_eq!(checked_neg256(&MAX), None);

    // ── saturating_add256 ─────────────────────────────────────────────────────
    assert_eq!(saturating_add256(&ONE, &TWO), [3, 0, 0, 0]);
    assert_eq!(saturating_add256(&MAX, &ONE), MAX);
    assert_eq!(saturating_add256(&MAX, &MAX), MAX);

    // ── saturating_sub256 ─────────────────────────────────────────────────────
    assert_eq!(saturating_sub256(&TWO, &ONE), ONE);
    assert_eq!(saturating_sub256(&ZERO, &ONE), ZERO);
    assert_eq!(saturating_sub256(&ONE, &TWO), ZERO);

    // ── wrapping_add256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_add256(&ONE, &TWO), [3, 0, 0, 0]);
    assert_eq!(wrapping_add256(&MAX, &ONE), ZERO);
    assert_eq!(wrapping_add256(&MAX, &MAX), [u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX]);

    // ── wrapping_sub256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_sub256(&TWO, &ONE), ONE);
    assert_eq!(wrapping_sub256(&ZERO, &ONE), MAX);

    // ── wrapping_neg256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_neg256(&ZERO), ZERO);
    assert_eq!(wrapping_neg256(&ONE), MAX);
    assert_eq!(wrapping_neg256(&MAX), ONE);
    // double negation is the identity
    let a = [0xdeadbeef_cafebabe_u64, 0x1234567890abcdef, 0, 0];
    assert_eq!(wrapping_neg256(&wrapping_neg256(&a)), a);
}
