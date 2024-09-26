use std::fmt::Debug;

use num_bigint::{BigInt, ToBigInt};
use p3_field::PrimeField;

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub struct Range<F: PrimeField>(pub F, pub F, pub bool, pub bool); // (min, max, min_neg, max_neg)

impl<F: PrimeField> Debug for Range<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let order = F::order().to_bigint().unwrap();
        let min = self.0.as_canonical_biguint().to_bigint().unwrap();
        let max = self.1.as_canonical_biguint().to_bigint().unwrap();

        let min_result = if self.2 { min.clone() - &order } else { min.clone() };
        let max_result = if self.3 { max.clone() - &order } else { max.clone() };

        write!(f, "[{},{}]", min_result, max_result)
    }
}

impl<F: PrimeField> PartialEq<(BigInt, BigInt)> for Range<F> {
    fn eq(&self, other: &(BigInt, BigInt)) -> bool {
        let order = F::order().to_bigint().unwrap();

        let min = self.0.as_canonical_biguint().to_bigint().unwrap();
        let max = self.1.as_canonical_biguint().to_bigint().unwrap();

        let min_result = if self.2 { min.clone() - &order } else { min.clone() };
        let max_result = if self.3 { max.clone() - &order } else { max.clone() };

        min_result == other.0 && max_result == other.1
    }
}

impl<F: PrimeField> Range<F> {
    pub fn contains(&self, value: F) -> bool {
        let order = F::order().to_bigint().unwrap();

        let value = value.as_canonical_biguint().to_bigint().unwrap();

        let min = self.0.as_canonical_biguint().to_bigint().unwrap();
        let max = self.1.as_canonical_biguint().to_bigint().unwrap();

        let min_result = min.clone();
        let max_result = max.clone();
        if self.2 && !self.3 {
            // If min is negative, then the range looks like [p-a,b], which means that
            // value should lie in [0,b]∪[p-a,p-1]
            return (value >= min_result && value <= max_result)
                || (value >= BigInt::from(0) && value <= max_result + order);
        }

        value >= min_result && value <= max_result
    }

    pub fn contained_in(&self, other: &(BigInt, BigInt)) -> bool {
        let order = F::order().to_bigint().unwrap();

        let min = self.0.as_canonical_biguint().to_bigint().unwrap();
        let max = self.1.as_canonical_biguint().to_bigint().unwrap();

        let min_result = if self.2 { min.clone() - &order } else { min.clone() };
        let max_result = if self.3 { max.clone() - &order } else { max.clone() };

        min_result >= other.0 && max_result <= other.1
    }
}
