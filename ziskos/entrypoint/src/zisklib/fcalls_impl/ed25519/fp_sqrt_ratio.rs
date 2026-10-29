use lazy_static::lazy_static;
use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::zisklib::fcalls_impl::utils::{biguint_from_u64_digits, n_u64_digits_from_biguint};

lazy_static! {
    /// p = 2^255 - 19
    static ref P: BigUint = (BigUint::one() << 255u32) - BigUint::from(19u32);
    /// (p + 3) / 8
    static ref P_PLUS_3_DIV_8: BigUint = (&*P + BigUint::from(3u32)) >> 3u32;
    /// sqrt(-1) = 2^((p - 1) / 4)
    static ref SQRT_M1: BigUint = BigUint::from(2u32).modpow(&((&*P - BigUint::one()) >> 2u32), &P);
}

/// Hints the square root of `u / v` in the base field of Ed25519.
///
/// Params: `u[4] || v[4]`. Results: `[is_square, r[4]]`, with `r` canonical and even (the
/// "non-negative" root):
/// - `is_square = 1`: `v * r^2 = u` (`r = 0` when `u = 0`).
/// - `is_square = 0`: `v * r^2 = 2 * u`, which proves `u / v` is not a square since 2 is a
///   non-residue mod p (p = 5 mod 8).
///
/// `v = 0` with `u != 0` has no valid answer; `[0, 0]` is returned and the guest check fails.
pub fn fcall_ed25519_fp_sqrt_ratio(params: &[u64], results: &mut [u64]) -> i64 {
    let u: &[u64; 4] = &params[0..4].try_into().unwrap();
    let v: &[u64; 4] = &params[4..8].try_into().unwrap();
    ed25519_fp_sqrt_ratio(u, v, results);
    5
}

pub fn ed25519_fp_sqrt_ratio(u: &[u64; 4], v: &[u64; 4], results: &mut [u64]) {
    let p = &*P;
    let u = biguint_from_u64_digits(u) % p;
    let v = biguint_from_u64_digits(v) % p;

    results[0..5].fill(0);
    if u.is_zero() {
        results[0] = 1;
        return;
    }
    if v.is_zero() {
        return;
    }

    let w = (&u * v.modpow(&(p - BigUint::from(2u32)), p)) % p;
    let (is_square, root) = match sqrt(&w) {
        Some(r) => (true, r),
        None => (false, sqrt(&((&w << 1u32) % p)).expect("2 is a non-residue mod p")),
    };
    // Return the even root.
    let root = if root.bit(0) { p - root } else { root };

    results[0] = is_square as u64;
    results[1..5].copy_from_slice(&n_u64_digits_from_biguint::<4>(&root));
}

/// Square root mod p = 5 (mod 8): c = w^((p+3)/8) satisfies c^2 = w or c^2 = -w; in the second case
/// c * sqrt(-1) is the root. Returns `None` if w is not a square.
fn sqrt(w: &BigUint) -> Option<BigUint> {
    let p = &*P;
    let c = w.modpow(&P_PLUS_3_DIV_8, p);
    let c2 = (&c * &c) % p;
    if &c2 == w {
        Some(c)
    } else if (&c2 + w) % p == BigUint::zero() {
        Some((c * &*SQRT_M1) % p)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limbs(x: &BigUint) -> [u64; 4] {
        n_u64_digits_from_biguint::<4>(x)
    }

    fn check(u: &BigUint, v: &BigUint) {
        let p = &*P;
        let mut r = [0u64; 5];
        ed25519_fp_sqrt_ratio(&limbs(u), &limbs(v), &mut r);
        let root = biguint_from_u64_digits(&r[1..5]);
        assert!(&root < p && !root.bit(0), "root must be canonical and even");
        let lhs = (v * &root * &root) % p;
        if r[0] == 1 {
            assert_eq!(lhs, u % p);
        } else {
            assert_eq!(lhs, (u << 1u32) % p);
            assert!(!(u % p).is_zero());
        }
    }

    #[test]
    fn squares_and_non_squares() {
        let p = &*P;
        for a in 0u32..40 {
            for b in 1u32..12 {
                check(&BigUint::from(a), &BigUint::from(b));
                check(&(p - BigUint::from(a + 1)), &BigUint::from(b * 7 + 3));
            }
        }
    }

    #[test]
    fn two_is_a_non_residue() {
        let mut r = [0u64; 5];
        ed25519_fp_sqrt_ratio(&[2, 0, 0, 0], &[1, 0, 0, 0], &mut r);
        assert_eq!(r[0], 0);
    }

    #[test]
    fn decompresses_base_point_x() {
        // u = y^2 - 1, v = d*y^2 + 1 for the RFC 8032 base point; x is even.
        let p = &*P;
        let y = BigUint::parse_bytes(
            b"46316835694926478169428394003475163141307993866256225615783033603165251855960",
            10,
        )
        .unwrap();
        let x = BigUint::parse_bytes(
            b"15112221349535400772501151409588531511454012693041857206046113283949847762202",
            10,
        )
        .unwrap();
        let d = BigUint::parse_bytes(
            b"52036cee2b6ffe738cc740797779e89800700a4d4141d8ab75eb4dca135978a3",
            16,
        )
        .unwrap();
        let y2 = (&y * &y) % p;
        let u = (&y2 + p - BigUint::one()) % p;
        let v = (&d * &y2 + BigUint::one()) % p;
        let mut r = [0u64; 5];
        ed25519_fp_sqrt_ratio(&limbs(&u), &limbs(&v), &mut r);
        assert_eq!(r[0], 1);
        assert_eq!(biguint_from_u64_digits(&r[1..5]), x);
    }
}
