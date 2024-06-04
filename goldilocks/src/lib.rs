// This library is a wrapper around a goldilocks library
// that provides a canonical interface for the field operations
// that we need.
// The canonical interface is defined in the AbstractField trait.
// The canonical interface is implemented for the goldilocks library
// that we are using.
// Currently we are using the goldilocks library from Plonky 3.

mod goldilocks_utils;

#[cfg(feature = "goldilocksp3")]
pub use p3_field::*;
pub use p3_goldilocks::*;
pub use goldilocks_utils::*;

#[cfg(test)]
mod tests {
    use super::{Goldilocks, PrimeField64, AbstractField};

    #[test]
    fn test_goldilocks() {
        let zero = Goldilocks::zero();
        let a = Goldilocks::from_canonical_u64(1);
        let b = Goldilocks::from_canonical_u64(2);
        let c = Goldilocks::from_canonical_u64(3);

        assert_eq!(zero.as_canonical_u64(), 0);
        assert_eq!(a.as_canonical_u64(), 1);
        assert_eq!(b.as_canonical_u64(), 2);
        assert_eq!(c.as_canonical_u64(), 3);
        assert_eq!(Goldilocks::zero().as_canonical_u64(), 0);
        assert_eq!(Goldilocks::one().as_canonical_u64(), 1);
    }
}
