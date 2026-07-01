use super::common::*;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
extern "C" {
    fn checked_mul256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn checked_square256_c(a: *const u64, result: *mut u64) -> u8;

    fn overflowing_mul256_c(a: *const u64, b: *const u64, result: *mut u64) -> u8;

    fn overflowing_square256_c(a: *const u64, result: *mut u64) -> u8;

    fn saturating_mul256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn saturating_square256_c(a: *const u64, result: *mut u64);

    fn wrapping_mul256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_square256_c(a: *const u64, result: *mut u64);

    fn inv256_c(a: *const u64, result: *mut u64) -> u8;
}

fn checked_mul(a: [u64; 4], b: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_mul, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_mul256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
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
            RU256::from_limbs(a).checked_mul(RU256::from_limbs(b)).map(|v| *v.as_limbs())
        }
    })
}

fn checked_square(a: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_square, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { checked_square256_c(a.as_ptr(), result.as_mut_ptr()) };
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
            RU256::from_limbs(a).checked_mul(RU256::from_limbs(a)).map(|v| *v.as_limbs())
        }
    })
}

fn overflowing_mul(a: [u64; 4], b: [u64; 4]) -> ([u64; 4], bool) {
    profile_block!(overflowing_mul, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let overflow =
                unsafe { overflowing_mul256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            let (v, o) = RU256::from_limbs(a).overflowing_mul(RU256::from_limbs(b));
            (*v.as_limbs(), o)
        }
    })
}

fn overflowing_square(a: [u64; 4]) -> ([u64; 4], bool) {
    profile_block!(overflowing_square, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let overflow = unsafe { overflowing_square256_c(a.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            let (v, o) = RU256::from_limbs(a).overflowing_mul(RU256::from_limbs(a));
            (*v.as_limbs(), o)
        }
    })
}

fn saturating_mul(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(saturating_mul, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { saturating_mul256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).saturating_mul(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn saturating_square(a: [u64; 4]) -> [u64; 4] {
    profile_block!(saturating_square, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { saturating_square256_c(a.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).saturating_mul(RU256::from_limbs(a)).as_limbs()
        }
    })
}

fn wrapping_mul(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_mul, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_mul256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_mul(RU256::from_limbs(b)).as_limbs()
        }
    })
}

fn wrapping_square(a: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_square, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_square256_c(a.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(all(target_os = "zkvm", target_vendor = "zisk")),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_mul(RU256::from_limbs(a)).as_limbs()
        }
    })
}

fn inv(a: &[u64; 4]) -> Option<[u64; 4]> {
    profile_block!(inv, {
        #[cfg(all(target_os = "zkvm", target_vendor = "zisk", not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success = unsafe { inv256_c(a.as_ptr(), result.as_mut_ptr()) };
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
            RU256::from_limbs(*a).inv_ring().map(|v| *v.as_limbs())
        }
    })
}

pub fn mul_tests() {
    // ── checked_mul256 ────────────────────────────────────────────────────────
    assert_eq!(checked_mul(TWO, [3, 0, 0, 0]), Some([6, 0, 0, 0]));
    assert_eq!(checked_mul(ONE, ONE), Some(ONE));
    assert_eq!(checked_mul(MAX, TWO), None);
    assert_eq!(checked_mul(POW2_128, POW2_128), None);

    // ── checked_square256 ────────────────────────────────────────────────────
    assert_eq!(checked_square([3, 0, 0, 0]), Some([9, 0, 0, 0]));
    assert_eq!(checked_square(POW2_64), Some(POW2_128));
    assert_eq!(checked_square(POW2_128), None);

    // ── overflowing_mul256 ────────────────────────────────────────────────────
    assert_eq!(overflowing_mul(ZERO, ONE), (ZERO, false));
    assert_eq!(overflowing_mul(ONE, ONE), (ONE, false));
    assert_eq!(overflowing_mul(TWO, [3, 0, 0, 0]), ([6, 0, 0, 0], false));
    // (2^64)^2 = 2^128 — no overflow
    assert_eq!(overflowing_mul(POW2_64, POW2_64), (POW2_128, false));
    // MAX * 2: low = MAX-1, overflow
    assert_eq!(overflowing_mul(MAX, TWO), ([u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX], true));
    // 2^128 * 2^128 = 2^256 ≡ 0 (mod 2^256), overflow
    assert_eq!(overflowing_mul(POW2_128, POW2_128), (ZERO, true));

    // ── overflowing_square256 ─────────────────────────────────────────────────
    assert_eq!(overflowing_square(ZERO), (ZERO, false));
    assert_eq!(overflowing_square(ONE), (ONE, false));
    assert_eq!(overflowing_square(TWO), ([4, 0, 0, 0], false));
    // (2^64)^2 = 2^128 — no overflow
    assert_eq!(overflowing_square(POW2_64), (POW2_128, false));
    // (2^128)^2 = 2^256 ≡ 0 (mod 2^256), overflow
    assert_eq!(overflowing_square(POW2_128), (ZERO, true));

    // ── saturating_mul256 ────────────────────────────────────────────────────
    assert_eq!(saturating_mul(TWO, [3, 0, 0, 0]), [6, 0, 0, 0]);
    assert_eq!(saturating_mul(MAX, TWO), MAX);
    assert_eq!(saturating_mul(POW2_128, POW2_128), MAX);

    // ── saturating_square256 ─────────────────────────────────────────────────
    assert_eq!(saturating_square([3, 0, 0, 0]), [9, 0, 0, 0]);
    assert_eq!(saturating_square(POW2_128), MAX);

    // ── wrapping_mul256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_mul(TWO, [3, 0, 0, 0]), [6, 0, 0, 0]);
    assert_eq!(wrapping_mul(MAX, TWO), [u64::MAX - 1, u64::MAX, u64::MAX, u64::MAX]);
    assert_eq!(wrapping_mul(POW2_128, POW2_128), ZERO);

    // ── wrapping_square256 ────────────────────────────────────────────────────
    assert_eq!(wrapping_square([3, 0, 0, 0]), [9, 0, 0, 0]);
    assert_eq!(wrapping_square(POW2_64), POW2_128);
    assert_eq!(wrapping_square(POW2_128), ZERO);

    // ── inv256 (mod 2^256) ────────────────────────────────────────────────────
    // even numbers have no inverse
    assert_eq!(inv(&ZERO), None);
    assert_eq!(inv(&TWO), None);
    assert_eq!(inv(&[0, 0, 0, 0]), None);
    // 1 is its own inverse
    assert_eq!(inv(&ONE), Some(ONE));
    // known values from the fcalls_impl test suite
    assert_eq!(
        inv(&[3, 0, 0, 0]),
        Some([0xaaaaaaaaaaaaaaab, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaaaaaaaaaa, 0xaaaaaaaaaaaaaaaa])
    );
    assert_eq!(
        inv(&[0xee453cbb08caf011_u64, 0x403f9ad46fdfbf18, 0x190bbcf54d8ad535, 0x9d4a5af226af865c]),
        Some([0x91f6316a1db400f1_u64, 0xa62de0c72fbf1f2b, 0x8cc70b2dcf824747, 0x78bccb02bfaa76af])
    );

    println!("All U256 Mul tests passed!");
}
