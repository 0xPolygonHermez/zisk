use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

/// Converts a single `u64` into a `BigUint`.
pub fn biguint_from_u64(value: u64) -> BigUint {
    BigUint::from(value)
}

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
