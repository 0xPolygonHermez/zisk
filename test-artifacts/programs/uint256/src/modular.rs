use super::common::*;

#[cfg(zisk_guest)]
extern "C" {
    fn reduce_mod256_c(a: *const u64, modulus: *const u64, result: *mut u64);

    fn add_mod256_c(a: *const u64, b: *const u64, modulus: *const u64, result: *mut u64);

    fn mul_mod256_c(a: *const u64, b: *const u64, modulus: *const u64, result: *mut u64);

    fn square_mod256_c(a: *const u64, modulus: *const u64, result: *mut u64);

    fn pow_mod256_c(base: *const u64, exp: *const u64, modulus: *const u64, result: *mut u64);

    fn inv_mod256_c(a: *const u64, modulus: *const u64, result: *mut u64) -> u8;
}

fn reduce_mod(a: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    profile_block!(reduce_mod, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { reduce_mod256_c(a.as_ptr(), modulus.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(*a).reduce_mod(RU256::from_limbs(*modulus)).as_limbs()
        }
    })
}

fn add_mod(a: &[u64; 4], b: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    profile_block!(add_mod, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { add_mod256_c(a.as_ptr(), b.as_ptr(), modulus.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(*a)
                .add_mod(RU256::from_limbs(*b), RU256::from_limbs(*modulus))
                .as_limbs()
        }
    })
}

fn mul_mod(a: &[u64; 4], b: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    profile_block!(mul_mod, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { mul_mod256_c(a.as_ptr(), b.as_ptr(), modulus.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(*a)
                .mul_mod(RU256::from_limbs(*b), RU256::from_limbs(*modulus))
                .as_limbs()
        }
    })
}

fn square_mod(a: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    profile_block!(square_mod, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe { square_mod256_c(a.as_ptr(), modulus.as_ptr(), result.as_mut_ptr()) };
            result
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            let av = RU256::from_limbs(*a);
            *av.mul_mod(av, RU256::from_limbs(*modulus)).as_limbs()
        }
    })
}

fn pow_mod(base: &[u64; 4], exp: &[u64; 4], modulus: &[u64; 4]) -> [u64; 4] {
    profile_block!(pow_mod, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            unsafe {
                pow_mod256_c(base.as_ptr(), exp.as_ptr(), modulus.as_ptr(), result.as_mut_ptr())
            };
            result
        }

        #[cfg(any(
            not(zisk_guest),
            feature = "ruint-fallback"
        ))]
        {
            *RU256::from_limbs(*base)
                .pow_mod(RU256::from_limbs(*exp), RU256::from_limbs(*modulus))
                .as_limbs()
        }
    })
}

fn inv_mod(a: &[u64; 4], modulus: &[u64; 4]) -> Option<[u64; 4]> {
    profile_block!(inv_mod, {
        #[cfg(all(zisk_guest, not(feature = "ruint-fallback")))]
        {
            let mut result = [0u64; 4];
            let success =
                unsafe { inv_mod256_c(a.as_ptr(), modulus.as_ptr(), result.as_mut_ptr()) };
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
            RU256::from_limbs(*a).inv_mod(RU256::from_limbs(*modulus)).map(|v| *v.as_limbs())
        }
    })
}

pub fn modular_tests() {
    let m7: [u64; 4] = [7, 0, 0, 0];
    let m13: [u64; 4] = [13, 0, 0, 0];
    let m100: [u64; 4] = [100, 0, 0, 0];
    let m1000: [u64; 4] = [1000, 0, 0, 0];

    // ── reduce_mod ─────────────────────────────────────────────────────────
    // modulus == 0 → always ZERO
    assert_eq!(reduce_mod(&ONE, &ZERO), ZERO);
    // a < modulus → unchanged
    assert_eq!(reduce_mod(&[5, 0, 0, 0], &m7), [5, 0, 0, 0]);
    // a > modulus → reduced
    assert_eq!(reduce_mod(&[8, 0, 0, 0], &m7), [1, 0, 0, 0]);
    assert_eq!(reduce_mod(&[100, 0, 0, 0], &m7), [2, 0, 0, 0]);

    // ── add_mod ────────────────────────────────────────────────────────────
    // modulus == 0 → always ZERO
    assert_eq!(add_mod(&ONE, &ONE, &ZERO), ZERO);
    // (2 + 3) mod 7 = 5
    assert_eq!(add_mod(&TWO, &[3, 0, 0, 0], &m7), [5, 0, 0, 0]);
    // (5 + 5) mod 7 = 3
    assert_eq!(add_mod(&[5, 0, 0, 0], &[5, 0, 0, 0], &m7), [3, 0, 0, 0]);
    // (0 + 0) mod 7 = 0
    assert_eq!(add_mod(&ZERO, &ZERO, &m7), ZERO);

    // ── mul_mod ────────────────────────────────────────────────────────────
    // modulus == 0 → always ZERO
    assert_eq!(mul_mod(&ONE, &ONE, &ZERO), ZERO);
    // 0 * anything = 0
    assert_eq!(mul_mod(&ZERO, &[6, 0, 0, 0], &m7), ZERO);
    // 3 * 4 mod 7 = 12 mod 7 = 5
    assert_eq!(mul_mod(&[3, 0, 0, 0], &[4, 0, 0, 0], &m7), [5, 0, 0, 0]);
    // 6 * 6 mod 7 = 36 mod 7 = 1
    assert_eq!(mul_mod(&[6, 0, 0, 0], &[6, 0, 0, 0], &m7), ONE);

    // ── square_mod ─────────────────────────────────────────────────────────
    // 3^2 mod 7 = 9 mod 7 = 2
    assert_eq!(square_mod(&[3, 0, 0, 0], &m7), [2, 0, 0, 0]);
    // 4^2 mod 13 = 16 mod 13 = 3
    assert_eq!(square_mod(&[4, 0, 0, 0], &m13), [3, 0, 0, 0]);

    // ── pow_mod ────────────────────────────────────────────────────────────
    // modulus == 0 → ZERO
    assert_eq!(pow_mod(&TWO, &TWO, &ZERO), ZERO);
    // base^0 = 1
    assert_eq!(pow_mod(&[42, 0, 0, 0], &ZERO, &m7), ONE);
    // base^1 = base (mod modulus)
    assert_eq!(pow_mod(&[3, 0, 0, 0], &ONE, &m7), [3, 0, 0, 0]);
    // 0^exp = 0
    assert_eq!(pow_mod(&ZERO, &[5, 0, 0, 0], &m7), ZERO);
    // 1^exp = 1
    assert_eq!(pow_mod(&ONE, &[100, 0, 0, 0], &m7), ONE);
    // 2^10 mod 1000 = 1024 mod 1000 = 24
    assert_eq!(pow_mod(&TWO, &[10, 0, 0, 0], &m1000), [24, 0, 0, 0]);
    // 3^4 mod 100 = 81
    assert_eq!(pow_mod(&[3, 0, 0, 0], &[4, 0, 0, 0], &m100), [81, 0, 0, 0]);
    // 5^3 mod 13 = 125 mod 13 = 8
    assert_eq!(pow_mod(&[5, 0, 0, 0], &[3, 0, 0, 0], &m13), [8, 0, 0, 0]);

    // ── inv_mod ────────────────────────────────────────────────────────────
    // inv(1, M) = 1 for any M > 1
    assert_eq!(inv_mod(&ONE, &m7), Some(ONE));
    // inv(13, 12) = 1 since 13 ≡ 1 (mod 12)
    assert_eq!(inv_mod(&[13, 0, 0, 0], &[12, 0, 0, 0]), Some(ONE));
    // gcd(6, 12) != 1 → no inverse
    assert_eq!(inv_mod(&[6, 0, 0, 0], &[12, 0, 0, 0]), None);
    // large known-good values (from fcalls_impl test suite)
    let a_big =
        [0x48c964556ed2d279_u64, 0xf692d9a779303069, 0xcc8d5e70e9f03415, 0xec53e64d5abb6d04];
    let modulus_big =
        [0xacca9ca1b4f3b763_u64, 0x57d556242ac9c0ed, 0x6e3d795231a618cb, 0x36835e1b448f5df6];
    let inv_big =
        [0xcede99fad6bbe0a2_u64, 0x2c99e1d7ed681658, 0x2a8d1689b5e7bfaf, 0x20d97a86f6e5e3a4];
    assert_eq!(inv_mod(&a_big, &modulus_big), Some(inv_big));
    // no inverse (gcd != 1)
    let a_no_inv = [0x844efa1db3aaaa7d_u64, 0xfbc4783fdfea63b7, 0xd30100f0dc1f7df6, 0x444a];
    assert_eq!(inv_mod(&a_no_inv, &modulus_big), None);

    println!("All U256 Modular tests passed!");
}
