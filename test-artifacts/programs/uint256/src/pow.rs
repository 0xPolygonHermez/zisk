use super::common::*;

#[cfg(zisk_guest)]
extern "C" {
    fn checked_pow256_c(base: *const u64, exp: *const u64, result: *mut u64) -> u8;

    fn overflowing_pow256_c(base: *const u64, exp: *const u64, result: *mut u64) -> u8;

    fn saturating_pow256_c(a: *const u64, b: *const u64, result: *mut u64);

    fn wrapping_pow256_c(a: *const u64, b: *const u64, result: *mut u64);
}

fn checked_pow(base: [u64; 4], exp: [u64; 4]) -> Option<[u64; 4]> {
    profile_block!(checked_pow, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success =
                unsafe { checked_pow256_c(base.as_ptr(), exp.as_ptr(), result.as_mut_ptr()) };
            if success == 1 {
                Some(result)
            } else {
                None
            }
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            RU256::from_limbs(base).checked_pow(RU256::from_limbs(exp)).map(|v| *v.as_limbs())
        }
    })
}

fn overflowing_pow(base: [u64; 4], exp: [u64; 4]) -> ([u64; 4], bool) {
    profile_block!(overflowing_pow, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let overflow =
                unsafe { overflowing_pow256_c(base.as_ptr(), exp.as_ptr(), result.as_mut_ptr()) };
            (result, overflow != 0)
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            let (v, o) = RU256::from_limbs(base).overflowing_pow(RU256::from_limbs(exp));
            (*v.as_limbs(), o)
        }
    })
}

fn saturating_pow(base: [u64; 4], exp: [u64; 4]) -> [u64; 4] {
    profile_block!(saturating_pow, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { saturating_pow256_c(base.as_ptr(), exp.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(base).saturating_pow(RU256::from_limbs(exp)).as_limbs()
        }
    })
}

fn wrapping_pow(a: [u64; 4], b: [u64; 4]) -> [u64; 4] {
    profile_block!(wrapping_pow, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { wrapping_pow256_c(a.as_ptr(), b.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(a).wrapping_pow(RU256::from_limbs(b)).as_limbs()
        }
    })
}

pub fn pow_tests() {
    // ── checked_pow256 ────────────────────────────────────────────────────────
    assert_eq!(checked_pow(TWO, [10, 0, 0, 0]), Some([1024, 0, 0, 0]));
    assert_eq!(checked_pow([3, 0, 0, 0], [5, 0, 0, 0]), Some([243, 0, 0, 0]));
    assert_eq!(checked_pow(TWO, ZERO), Some(ONE));
    assert_eq!(checked_pow(MAX, TWO), None);
    assert_eq!(checked_pow(TWO, [256, 0, 0, 0]), None);

    // ── overflowing_pow256 ────────────────────────────────────────────────────
    // Special-case early returns
    // base^0 = 1 (including 0^0)
    assert_eq!(overflowing_pow(ZERO, ZERO), (ONE, false));
    assert_eq!(overflowing_pow([42, 0, 0, 0], ZERO), (ONE, false));
    // base^1 = base
    assert_eq!(overflowing_pow([42, 0, 0, 0], ONE), ([42, 0, 0, 0], false));
    // 0^exp = 0
    assert_eq!(overflowing_pow(ZERO, [5, 0, 0, 0]), (ZERO, false));
    // 1^exp = 1
    assert_eq!(overflowing_pow(ONE, [100, 0, 0, 0]), (ONE, false));

    // Power-of-two exponent path (repeated squaring only)
    // 2^2 = 4  (exp=2=2^1, one squaring)
    assert_eq!(overflowing_pow(TWO, TWO), ([4, 0, 0, 0], false));
    // 2^4 = 16 (exp=4=2^2, two squarings)
    assert_eq!(overflowing_pow(TWO, [4, 0, 0, 0]), ([16, 0, 0, 0], false));
    // 3^4 = 81
    assert_eq!(overflowing_pow([3, 0, 0, 0], [4, 0, 0, 0]), ([81, 0, 0, 0], false));

    // General square-and-multiply path
    // 2^3 = 8  (exp=3 = 0b11)
    assert_eq!(overflowing_pow(TWO, [3, 0, 0, 0]), ([8, 0, 0, 0], false));
    // 2^5 = 32 (exp=5 = 0b101)
    assert_eq!(overflowing_pow(TWO, [5, 0, 0, 0]), ([32, 0, 0, 0], false));
    // 3^5 = 243 (exp=5 = 0b101)
    assert_eq!(overflowing_pow([3, 0, 0, 0], [5, 0, 0, 0]), ([243, 0, 0, 0], false));

    // Overflow cases
    // MAX^2 mod 2^256 = (-1)^2 mod 2^256 = 1, overflow
    assert_eq!(overflowing_pow(MAX, TWO), (ONE, true));
    // 2^256 mod 2^256 = 0, overflow  (exp=256=2^8, power-of-two path)
    assert_eq!(overflowing_pow(TWO, [256, 0, 0, 0]), (ZERO, true));

    // ── saturating_pow256 ─────────────────────────────────────────────────────
    assert_eq!(saturating_pow(TWO, [10, 0, 0, 0]), [1024, 0, 0, 0]);
    assert_eq!(saturating_pow(ZERO, [99, 0, 0, 0]), ZERO);
    assert_eq!(saturating_pow(ONE, MAX), ONE);
    assert_eq!(saturating_pow(MAX, TWO), MAX);
    assert_eq!(saturating_pow(TWO, [256, 0, 0, 0]), MAX);

    // ── wrapping_pow256 ───────────────────────────────────────────────────────
    assert_eq!(wrapping_pow(TWO, [10, 0, 0, 0]), [1024, 0, 0, 0]);
    assert_eq!(wrapping_pow([3, 0, 0, 0], [5, 0, 0, 0]), [243, 0, 0, 0]);
    assert_eq!(wrapping_pow(ZERO, [5, 0, 0, 0]), ZERO);
    assert_eq!(wrapping_pow(ONE, [100, 0, 0, 0]), ONE);
    // MAX^2 wraps to 1
    assert_eq!(wrapping_pow(MAX, TWO), ONE);
    // 2^256 wraps to 0
    assert_eq!(wrapping_pow(TWO, [256, 0, 0, 0]), ZERO);

    println!("All U256 Pow tests passed!");
}
