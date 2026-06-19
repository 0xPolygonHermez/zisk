use super::common::*;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
extern "C" {
    fn checked_div256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_rem256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn div_rem256_c(a: *const u64, b: *const u64, quo: *mut u64, rem: *mut u64);

    fn div_ceil256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_div256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_rem256_c(a: *const u64, b: *const u64, result: *mut u64);
}

fn checked_div(base: [u64; 4], exp: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_div, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success =
                unsafe { checked_div256_c(base.as_ptr(), exp.as_ptr(), result.as_mut_ptr()) };
            if success == 1 {
                Some(result)
            } else {
                None
            }
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            RU256::from_limbs(base).checked_div(RU256::from_limbs(exp)).map(|v| *v.as_limbs())
        }
    })
}

fn checked_rem(base: [u64; 4], exp: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_rem, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success =
                unsafe { checked_rem256_c(base.as_ptr(), exp.as_ptr(), result.as_mut_ptr()) };
            if success == 1 {
                Some(result)
            } else {
                None
            }
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            RU256::from_limbs(base).checked_rem(RU256::from_limbs(exp)).map(|v| *v.as_limbs())
        }
    })
}

fn wrapping_div(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_div, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_div256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_div(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn wrapping_rem(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_rem, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_rem256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_rem(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn div_rem(a: &[u64; 4], b: &[u64; 4]) -> ([u64; 4], [u64; 4]) {
    profile_block!(div_rem, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut quo = [0u64; 4];
            let mut rem = [0u64; 4];
            unsafe { div_rem256_c(a.as_ptr(), b.as_ptr(), quo.as_mut_ptr(), rem.as_mut_ptr()) };
            (quo, rem)
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            let (q, r) = RU256::from_limbs(*a).div_rem(RU256::from_limbs(*b));
            (*q.as_limbs(), *r.as_limbs())
        }
    })
}

fn div_ceil(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    profile_block!(div_ceil, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { div_ceil256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(*a).div_ceil(RU256::from_limbs(*b)).as_limbs()
        }
    })
}

pub fn div_tests() {
    // ── checked_div256 ────────────────────────────────────────────────────────
    let a = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x3b4e3fc5e0d8b014];
    let b = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x0];
    let expected_quo = [0x2868ebf5edfaecd5_u64, 0x5, 0x0, 0x0];
    let expected_rem = [0x0dbb84a86764e268_u64, 0xfd48d6ec2b636246, 0x0adbb6db4207ffb8, 0x0];
    assert_eq!(checked_div(a, b), Some(expected_quo));
    assert_eq!(checked_div(ZERO, ONE), Some(ZERO));
    assert_eq!(checked_div(a, ZERO), None);

    // ── checked_rem256 ────────────────────────────────────────────────────────
    assert_eq!(checked_rem(a, b), Some(expected_rem));
    assert_eq!(checked_rem(ZERO, ONE), Some(ZERO));
    assert_eq!(checked_rem(a, ZERO), None);

    // ── div_rem256 ────────────────────────────────────────────────────────────
    // a == 0
    assert_eq!(div_rem(&ZERO, &ONE), (ZERO, ZERO));
    // a == b
    assert_eq!(div_rem(&ONE, &ONE), (ONE, ZERO));
    // a < b  →  quotient 0, remainder a
    let b_big = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x1];
    let a_small = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x0];
    assert_eq!(div_rem(&a_small, &b_big), (ZERO, a_small));
    // a > b  (values from the fcalls_impl test suite)
    let a = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x3b4e3fc5e0d8b014];
    let b = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x0];
    let expected_quo = [0x2868ebf5edfaecd5_u64, 0x5, 0x0, 0x0];
    let expected_rem = [0x0dbb84a86764e268_u64, 0xfd48d6ec2b636246, 0x0adbb6db4207ffb8, 0x0];
    assert_eq!(div_rem(&a, &b), (expected_quo, expected_rem));

    // ── div_ceil256 ───────────────────────────────────────────────────────────
    // exact division: ceil(a/b) == a/b
    assert_eq!(div_ceil(&ONE, &ONE), ONE);
    // 3 / 2 = 1 remainder 1  →  ceil = 2
    assert_eq!(div_ceil(&[3, 0, 0, 0], &TWO), TWO);
    // 4 / 2 = 2 remainder 0  →  ceil = 2
    assert_eq!(div_ceil(&[4, 0, 0, 0], &TWO), [2, 0, 0, 0]);
    // a/b with remainder: ceil == quo + 1
    assert_eq!(
        div_ceil(&a, &b),
        [expected_quo[0] + 1, expected_quo[1], expected_quo[2], expected_quo[3]]
    );

    // ── wrapping_div256 / wrapping_rem256 ─────────────────────────────────────
    let a = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x3b4e3fc5e0d8b014];
    let b = [0x16b12176aedd308e_u64, 0x9d331c2b34766fc9, 0x0b7f85b22001249e, 0x0];
    let expected_quo = [0x2868ebf5edfaecd5_u64, 0x5, 0x0, 0x0];
    let expected_rem = [0x0dbb84a86764e268_u64, 0xfd48d6ec2b636246, 0x0adbb6db4207ffb8, 0x0];
    assert_eq!(wrapping_div(a, b), expected_quo);
    assert_eq!(wrapping_rem(a, b), expected_rem);
    // a % b when a == 0
    assert_eq!(wrapping_rem(ZERO, ONE), ZERO);
    // a % a == 0
    assert_eq!(wrapping_rem(a, a), ZERO);

    println!("All U256 Div tests passed!");
}
