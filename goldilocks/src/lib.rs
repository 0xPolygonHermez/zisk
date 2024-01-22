// This library is a wrapper around a goldilocks library
// that provides a canonical interface for the field operations
// that we need.
// The canonical interface is defined in the AbstractField trait.
// The canonical interface is implemented for the goldilocks library
// that we are using.
// Currently we are using the goldilocks library from Plonky 3.

#[cfg(feature = "goldilocksp3")]
pub type Goldilocks = p3_goldilocks::Goldilocks;

#[cfg(feature = "goldilocksp2")]
pub type Goldilocks = plonky2::field::goldilocks_field::GoldilocksField;

pub trait AbstractField {
    fn zero() -> Goldilocks;
    fn one() -> Goldilocks;

    fn as_canonical_u64(&self) -> u64;
    fn from_canonical_u64(value: u64) -> Self;
    fn from_canonical_u8(value: u8) -> Self;
}

#[cfg(feature = "goldilocksp3")]
mod goldilocksp3;

#[cfg(feature = "goldilocksp2")]
mod goldilocksp2;

#[cfg(test)]
mod tests {
    use super::{Goldilocks, AbstractField};

    #[test]
    fn test_goldilocks() {
        let a = Goldilocks::from_canonical_u64(1);
        let b = Goldilocks::from_canonical_u64(2);
        let c = Goldilocks::from_canonical_u64(3);

        assert_eq!(a.as_canonical_u64(), 1);
        assert_eq!(b.as_canonical_u64(), 2);
        assert_eq!(c.as_canonical_u64(), 3);
        assert_eq!(Goldilocks::zero().as_canonical_u64(), 0);
        assert_eq!(Goldilocks::one().as_canonical_u64(), 1);
    }
}
