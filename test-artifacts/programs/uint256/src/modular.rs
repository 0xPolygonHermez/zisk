use ziskos::zisklib::{
    add_mod256, inv_mod256, mul_mod256, pow_mod256, reduce_mod256, square_mod256,
};

use crate::constants::*;

pub fn modular_tests() {
    let m7: [u64; 4] = [7, 0, 0, 0];
    let m13: [u64; 4] = [13, 0, 0, 0];
    let m100: [u64; 4] = [100, 0, 0, 0];
    let m1000: [u64; 4] = [1000, 0, 0, 0];

    // ── reduce_mod256 ─────────────────────────────────────────────────────────
    // modulus == 0 → always ZERO
    assert_eq!(reduce_mod256(&ONE, &ZERO), ZERO);
    // a < modulus → unchanged
    assert_eq!(reduce_mod256(&[5, 0, 0, 0], &m7), [5, 0, 0, 0]);
    // a > modulus → reduced
    assert_eq!(reduce_mod256(&[8, 0, 0, 0], &m7), [1, 0, 0, 0]);
    assert_eq!(reduce_mod256(&[100, 0, 0, 0], &m7), [2, 0, 0, 0]);

    // ── add_mod256 ────────────────────────────────────────────────────────────
    // modulus == 0 → always ZERO
    assert_eq!(add_mod256(&ONE, &ONE, &ZERO), ZERO);
    // (2 + 3) mod 7 = 5
    assert_eq!(add_mod256(&TWO, &[3, 0, 0, 0], &m7), [5, 0, 0, 0]);
    // (5 + 5) mod 7 = 3
    assert_eq!(add_mod256(&[5, 0, 0, 0], &[5, 0, 0, 0], &m7), [3, 0, 0, 0]);
    // (0 + 0) mod 7 = 0
    assert_eq!(add_mod256(&ZERO, &ZERO, &m7), ZERO);

    // ── mul_mod256 ────────────────────────────────────────────────────────────
    // modulus == 0 → always ZERO
    assert_eq!(mul_mod256(&ONE, &ONE, &ZERO), ZERO);
    // 0 * anything = 0
    assert_eq!(mul_mod256(&ZERO, &[6, 0, 0, 0], &m7), ZERO);
    // 3 * 4 mod 7 = 12 mod 7 = 5
    assert_eq!(mul_mod256(&[3, 0, 0, 0], &[4, 0, 0, 0], &m7), [5, 0, 0, 0]);
    // 6 * 6 mod 7 = 36 mod 7 = 1
    assert_eq!(mul_mod256(&[6, 0, 0, 0], &[6, 0, 0, 0], &m7), ONE);

    // ── square_mod256 ─────────────────────────────────────────────────────────
    // 3^2 mod 7 = 9 mod 7 = 2
    assert_eq!(square_mod256(&[3, 0, 0, 0], &m7), [2, 0, 0, 0]);
    // 4^2 mod 13 = 16 mod 13 = 3
    assert_eq!(square_mod256(&[4, 0, 0, 0], &m13), [3, 0, 0, 0]);

    // ── pow_mod256 ────────────────────────────────────────────────────────────
    // modulus == 0 → ZERO
    assert_eq!(pow_mod256(&TWO, &TWO, &ZERO), ZERO);
    // base^0 = 1
    assert_eq!(pow_mod256(&[42, 0, 0, 0], &ZERO, &m7), ONE);
    // base^1 = base (mod modulus)
    assert_eq!(pow_mod256(&[3, 0, 0, 0], &ONE, &m7), [3, 0, 0, 0]);
    // 0^exp = 0
    assert_eq!(pow_mod256(&ZERO, &[5, 0, 0, 0], &m7), ZERO);
    // 1^exp = 1
    assert_eq!(pow_mod256(&ONE, &[100, 0, 0, 0], &m7), ONE);
    // 2^10 mod 1000 = 1024 mod 1000 = 24
    assert_eq!(pow_mod256(&TWO, &[10, 0, 0, 0], &m1000), [24, 0, 0, 0]);
    // 3^4 mod 100 = 81
    assert_eq!(pow_mod256(&[3, 0, 0, 0], &[4, 0, 0, 0], &m100), [81, 0, 0, 0]);
    // 5^3 mod 13 = 125 mod 13 = 8
    assert_eq!(pow_mod256(&[5, 0, 0, 0], &[3, 0, 0, 0], &m13), [8, 0, 0, 0]);

    // ── inv_mod256 ────────────────────────────────────────────────────────────
    // inv(1, M) = 1 for any M > 1
    assert_eq!(inv_mod256(&ONE, &m7), Some(ONE));
    // inv(13, 12) = 1 since 13 * 1 ≡ 1 mod 12
    assert_eq!(inv_mod256(&[13, 0, 0, 0], &[12, 0, 0, 0]), Some(ONE));
    // gcd(6, 12) != 1 → no inverse
    assert_eq!(inv_mod256(&[6, 0, 0, 0], &[12, 0, 0, 0]), None);
    // large known-good values (from fcalls_impl test suite)
    let a_big =
        [0x48c964556ed2d279_u64, 0xf692d9a779303069, 0xcc8d5e70e9f03415, 0xec53e64d5abb6d04];
    let modulus_big =
        [0xacca9ca1b4f3b763_u64, 0x57d556242ac9c0ed, 0x6e3d795231a618cb, 0x36835e1b448f5df6];
    let inv_big =
        [0xcede99fad6bbe0a2_u64, 0x2c99e1d7ed681658, 0x2a8d1689b5e7bfaf, 0x20d97a86f6e5e3a4];
    assert_eq!(inv_mod256(&a_big, &modulus_big), Some(inv_big));
    // no inverse (gcd != 1)
    let a_no_inv = [0x844efa1db3aaaa7d_u64, 0xfbc4783fdfea63b7, 0xd30100f0dc1f7df6, 0x444a];
    assert_eq!(inv_mod256(&a_no_inv, &modulus_big), None);
}
