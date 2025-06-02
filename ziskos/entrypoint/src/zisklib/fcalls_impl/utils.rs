use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};

pub fn from_limbs_le(limbs: &[u64; 4]) -> BigUint {
    limbs.iter().rev().fold(BigUint::zero(), |acc, &limb| (acc << 64) + BigUint::from(limb))
}

pub fn to_limbs_le(value: &BigUint) -> [u64; 4] {
    let mut limbs = [0u64; 4];
    let mut _value = value.clone();
    for limb in limbs.iter_mut() {
        *limb = (_value.clone() & BigUint::from(u64::MAX)).to_u64().unwrap();
        _value >>= 64;
    }
    limbs
}
