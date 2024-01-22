#[cfg(feature = "goldilocksp3")]
mod goldilocksp3 {
    use p3_field::{AbstractField as AbstractFieldP3, PrimeField64};
    use p3_goldilocks::Goldilocks as GoldilocksP3;
    use crate::{Goldilocks, AbstractField};

    impl AbstractField for GoldilocksP3 {
        #[inline]
        fn zero() -> Goldilocks {
            AbstractFieldP3::zero()
        }

        #[inline]
        fn one() -> Goldilocks {
            AbstractFieldP3::one()
        }

        #[inline]
        fn as_canonical_u64(&self) -> u64 {
            PrimeField64::as_canonical_u64(self)
        }

        #[inline]
        fn from_canonical_u64(value: u64) -> Self {
            AbstractFieldP3::from_canonical_u64(value)
        }
        #[inline]
        fn from_canonical_u8(value: u8) -> Self {
            AbstractFieldP3::from_canonical_u8(value)
        }
    }
}
