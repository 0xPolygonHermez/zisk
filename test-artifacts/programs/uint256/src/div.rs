use ziskos::zisklib::{
    checked_div256, checked_rem256, div_ceil256, div_rem256, wrapping_div256, wrapping_rem256,
};

use crate::constants::*;

pub fn div_tests() {
    // ── div_rem256 ────────────────────────────────────────────────────────────
    // a == 0
    assert_eq!(div_rem256(&ZERO, &ONE), (ZERO, ZERO));
    // a == b
    assert_eq!(div_rem256(&ONE, &ONE), (ONE, ZERO));
    // a < b  →  quotient 0, remainder a
    let b_big = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x1];
    let a_small = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x0];
    assert_eq!(div_rem256(&a_small, &b_big), (ZERO, a_small));
    // a > b  (values from the fcalls_impl test suite)
    let a = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x3b4e3fc5e0d8b014];
    let b = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x0];
    let expected_quo = [0x2868ebf5edfaecd5_u64, 0x5, 0x0, 0x0];
    let expected_rem = [0x0dbb84a86764e268_u64, 0xfd48d6ec2b636246, 0x0adbb6db4207ffb8, 0x0];
    assert_eq!(div_rem256(&a, &b), (expected_quo, expected_rem));

    // ── wrapping_div256 / wrapping_rem256 ─────────────────────────────────────
    assert_eq!(wrapping_div256(&a, &b), expected_quo);
    assert_eq!(wrapping_rem256(&a, &b), expected_rem);
    // a % b when a == 0
    assert_eq!(wrapping_rem256(&ZERO, &ONE), ZERO);
    // a % a == 0
    assert_eq!(wrapping_rem256(&a, &a), ZERO);

    // ── checked_div256 ────────────────────────────────────────────────────────
    assert_eq!(checked_div256(&a, &b), Some(expected_quo));
    assert_eq!(checked_div256(&ZERO, &ONE), Some(ZERO));
    assert_eq!(checked_div256(&a, &ZERO), None);

    // ── checked_rem256 ────────────────────────────────────────────────────────
    assert_eq!(checked_rem256(&a, &b), Some(expected_rem));
    assert_eq!(checked_rem256(&ZERO, &ONE), Some(ZERO));
    assert_eq!(checked_rem256(&a, &ZERO), None);

    // ── div_ceil256 ───────────────────────────────────────────────────────────
    // exact division: ceil(a/b) == a/b
    assert_eq!(div_ceil256(&ONE, &ONE), ONE);
    // 3 / 2 = 1 remainder 1  →  ceil = 2
    assert_eq!(div_ceil256(&[3, 0, 0, 0], &TWO), TWO);
    // 4 / 2 = 2 remainder 0  →  ceil = 2
    assert_eq!(div_ceil256(&[4, 0, 0, 0], &TWO), [2, 0, 0, 0]);
    // a/b with remainder: ceil == quo + 1
    assert_eq!(
        div_ceil256(&a, &b),
        [expected_quo[0] + 1, expected_quo[1], expected_quo[2], expected_quo[3]]
    );
}
