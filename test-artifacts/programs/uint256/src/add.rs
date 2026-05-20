use super::common::*;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
extern "C" {
    fn checked_add256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_sub256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_neg256_c(a: *const u64, result: *mut u64) -> u8;

    fn overflowing_add256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn overflowing_sub256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn overflowing_neg256_c(a: *const u64, result: *mut u64) -> u8;

    fn saturating_add256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn saturating_sub256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_add256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_sub256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_neg256_c(a: *const u64, result: *mut u64);
}

fn checked_add(a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_add, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_add256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
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
            RU256::from_limbs(a).checked_add(RU256::from_limbs(b)).map(|v| *v.as_limbs())
        }
    })
}

fn checked_sub(a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_sub, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_sub256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
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
            RU256::from_limbs(a).checked_sub(RU256::from_limbs(b)).map(|v| *v.as_limbs())
        }
    })
}

fn checked_neg(a: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_neg, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_neg256_c(a.as_ptr(), result.as_mut_ptr()) };
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
            RU256::from_limbs(a).checked_neg().map(|v| *v.as_limbs())
        }
    })
}

fn overflowing_add(a: [u64; 4], b: [u64; 4]) -> ([u64; 4], bool) {
    profile_block!(overflowing_add, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let overflow =
                unsafe { overflowing_add256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            let (v, o) = RU256::from_limbs(a).overflowing_add(RU256::from_limbs(b));
            (*v.as_limbs(), o)
        }
    })
}

fn overflowing_sub(a: [u64; 4], b: [u64; 4]) -> ([u64; 4], bool) {
    profile_block!(overflowing_sub, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let overflow =
                unsafe { overflowing_sub256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            let (v, o) = RU256::from_limbs(a).overflowing_sub(RU256::from_limbs(b));
            (*v.as_limbs(), o)
        }
    })
}

fn overflowing_neg(a: [u64; 4]) -> ([u64; 4], bool) {
    profile_block!(overflowing_neg, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let overflow = unsafe { overflowing_neg256_c(a.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            let (v, o) = RU256::from_limbs(a).overflowing_neg();
            (*v.as_limbs(), o)
        }
    })
}

pub fn saturating_add(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(saturating_add, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { saturating_add256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).saturating_add(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn saturating_sub(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(saturating_sub, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { saturating_sub256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).saturating_sub(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn wrapping_add(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_add, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_add256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_add(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn wrapping_sub(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_sub, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_sub256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_sub(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn wrapping_neg(a: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_neg, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_neg256_c(a.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_neg().as_limbs()
        }
    })
}

pub fn add_tests() {
    // ── checked_add256 ────────────────────────────────────────────────────────
    assert_eq!(checked_add(ONE, TWO), Some([3, 0, 0, 0]));
    assert_eq!(checked_add(MAX, ZERO), Some(MAX));
    assert_eq!(checked_add(MAX, ONE), None);

    // ── checked_sub256 ────────────────────────────────────────────────────────
    assert_eq!(checked_sub(TWO, ONE), Some(ONE));
    assert_eq!(checked_sub(ONE, ONE), Some(ZERO));
    assert_eq!(checked_sub(ZERO, ONE), None);

    // ── checked_neg256 ────────────────────────────────────────────────────────
    assert_eq!(checked_neg(ZERO), Some(ZERO));
    assert_eq!(checked_neg(ONE), None);
    assert_eq!(checked_neg(MAX), None);

    // ── overflowing_add256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_add(ZERO, ZERO), (ZERO, false));
    assert_eq!(overflowing_add(ONE, TWO), ([3, 0, 0, 0], false));
    // carry propagates across one limb
    assert_eq!(overflowing_add([u64::MAX, 0, 0, 0], ONE), ([0, 1, 0, 0], false));
    // carry propagates across two limbs
    assert_eq!(overflowing_add([u64::MAX, u64::MAX, 0, 0], ONE), ([0, 0, 1, 0], false));
    assert_eq!(overflowing_add(MAX, ONE), (ZERO, true));
    // 2*(2^256 - 1) mod 2^256 = MAX - 1, carry 1
    assert_eq!(overflowing_add(MAX, MAX), ([u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX], true));

    // ── overflowing_sub256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_sub(TWO, ONE), (ONE, false));
    assert_eq!(overflowing_sub(ONE, ONE), (ZERO, false));
    assert_eq!(overflowing_sub(MAX, MAX), (ZERO, false));
    // borrow propagates across one limb
    assert_eq!(overflowing_sub([0, 1, 0, 0], ONE), ([u64::MAX, 0, 0, 0], false));
    assert_eq!(overflowing_sub(ZERO, ONE), (MAX, true));
    assert_eq!(overflowing_sub(ONE, TWO), (MAX, true));

    // ── overflowing_neg256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_neg(ZERO), (ZERO, false));
    assert_eq!(overflowing_neg(ONE), (MAX, true));
    assert_eq!(overflowing_neg(MAX), (ONE, true));

    // ── saturating_add256 ─────────────────────────────────────────────────────
    assert_eq!(saturating_add(ONE, TWO), [3, 0, 0, 0]);
    assert_eq!(saturating_add(MAX, ONE), MAX);
    assert_eq!(saturating_add(MAX, MAX), MAX);

    // ── saturating_sub256 ─────────────────────────────────────────────────────
    assert_eq!(saturating_sub(TWO, ONE), ONE);
    assert_eq!(saturating_sub(MAX, MAX_MINUS_ONE), ONE);
    assert_eq!(saturating_sub(ONE, TWO), ZERO);

    // ── wrapping_add256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_add(ONE, TWO), [3, 0, 0, 0]);
    assert_eq!(wrapping_add(MAX, ONE), ZERO);
    assert_eq!(wrapping_add(MAX, MAX), [u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX]);

    // ── wrapping_sub256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_sub(TWO, ONE), ONE);
    assert_eq!(wrapping_sub(ZERO, ONE), MAX);

    // ── wrapping_neg256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_neg(ZERO), ZERO);
    assert_eq!(wrapping_neg(ONE), MAX);
    assert_eq!(wrapping_neg(MAX), ONE);
    // double negation is the identity
    let a = [0xdeadbeef_cafebabe_u64, 0x1234567890abcdef, 0, 0];
    assert_eq!(wrapping_neg(wrapping_neg(a)), a);

    println!("All U256 Add tests passed!");
}
