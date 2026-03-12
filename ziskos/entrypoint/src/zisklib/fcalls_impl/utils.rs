use num_bigint::BigUint;
use num_traits::Zero;

pub fn biguint_from_u64(value: u64) -> BigUint {
    BigUint::from(value)
}

pub fn biguint_from_u64_digits(limbs: &[u64]) -> BigUint {
    limbs.iter().rev().fold(BigUint::zero(), |acc, &limb| (acc << 64) + BigUint::from(limb))
}

pub fn u64_digits_from_biguint(value: &BigUint) -> Vec<u64> {
    value.to_u64_digits()
}

pub fn n_u64_digits_from_biguint<const N: usize>(value: &BigUint) -> [u64; N] {
    let digits = value.to_u64_digits();
    assert!(digits.len() <= N, "Value requires {} limbs > {}", digits.len(), N);

    let mut limbs = [0u64; N];
    for (i, d) in digits.iter().enumerate().take(N) {
        limbs[i] = *d;
    }
    limbs
}
