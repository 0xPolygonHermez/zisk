use num_bigint::BigUint;
use num_traits::Zero;

pub fn from_limbs_le<const N: usize>(limbs: &[u64; N]) -> BigUint {
    limbs.iter().rev().fold(BigUint::zero(), |acc, &limb| (acc << 64) + BigUint::from(limb))
}

pub fn to_limbs_le<const N: usize>(value: &BigUint) -> [u64; N] {
    let digits = value.to_u64_digits();
    assert!(digits.len() <= N, "to_limbs_le: value requires {} limbs > N={}", digits.len(), N);
    let mut limbs = [0u64; N];
    for (i, d) in digits.iter().enumerate() {
        limbs[i] = *d;
    }
    limbs
}
