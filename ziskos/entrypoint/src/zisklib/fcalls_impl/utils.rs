use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

/// Builds a `BigUint` from a little-endian slice of `u64` limbs.
pub fn biguint_from_u64_digits(limbs: &[u64]) -> BigUint {
    limbs.iter().rev().fold(BigUint::zero(), |acc, &limb| (acc << 64) + BigUint::from(limb))
}

/// Builds a `BigInt` from a little-endian slice of `u64` limbs.
pub fn bigint_from_u64_digits(limbs: &[u64]) -> BigInt {
    limbs.iter().rev().fold(BigInt::zero(), |acc, &limb| (acc << 64) + BigInt::from(limb))
}

/// Converts a `BigUint` into a `Vec<u64>` of little-endian limbs.
pub fn u64_digits_from_biguint(value: &BigUint) -> Vec<u64> {
    value.to_u64_digits()
}

/// Converts a `BigUint` into a fixed-size `[u64; N]` of little-endian limbs.
///
/// # Panics
///
/// Panics if the value requires more than `N` limbs.
pub fn n_u64_digits_from_biguint<const N: usize>(value: &BigUint) -> [u64; N] {
    let digits = value.to_u64_digits();
    assert!(digits.len() <= N, "Value requires {} limbs > {}", digits.len(), N);

    let mut limbs = [0u64; N];
    for (i, d) in digits.iter().enumerate().take(N) {
        limbs[i] = *d;
    }
    limbs
}

/// Fp12 arithmetic for [`pow_frob`].
pub trait Fp12Ops: Copy {
    fn one() -> Self;
    fn mul(&self, other: &Self) -> Self;
    fn sqr(&self) -> Self;
    fn frobenius(&self) -> Self;
}

/// The 12 bases Frobᵏ(x) with their 4-bit multiples, for [`pow_frob`].
pub fn frobenius_table<F: Fp12Ops>(x: &F) -> Vec<[F; 16]> {
    let mut base = *x;
    (0..12)
        .map(|_| {
            let mut row = [F::one(); 16];
            for j in 1..16 {
                row[j] = row[j - 1].mul(&base);
            }
            base = base.frobenius();
            row
        })
        .collect()
}

/// x^e as Π Frobᵏ(x)^(eₖ) with e = Σ eₖ·pᵏ, from the table of x: the p-power map is cheap, so the
/// exponent length drops from 12·log p to log p bits, taken in 4-bit windows over the 12 bases.
pub fn pow_frob<F: Fp12Ops>(tab: &[[F; 16]], p: &BigUint, e: &BigUint) -> F {
    let mut digits = Vec::with_capacity(12);
    let mut t = e.clone();
    for _ in 0..12 {
        digits.push(&t % p);
        t /= p;
    }
    let mut r = F::one();
    for wi in (0..(p.bits() as usize).div_ceil(4)).rev() {
        for _ in 0..4 {
            r = r.sqr();
        }
        for (d, row) in digits.iter().zip(tab) {
            let j = (0..4).fold(0, |j, i| (j << 1) | d.bit((4 * wi + 3 - i) as u64) as usize);
            if j != 0 {
                r = r.mul(&row[j]);
            }
        }
    }
    r
}
