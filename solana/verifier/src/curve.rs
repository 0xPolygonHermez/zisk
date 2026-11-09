use crate::{profile, VerifyError};
use solana_bn254::prelude::{alt_bn128_addition, alt_bn128_multiplication, alt_bn128_pairing};

const Q: [u8; 32] = [
    48, 100, 78, 114, 225, 49, 160, 41, 184, 80, 69, 182, 129, 129, 88, 93, 151, 129, 106, 145,
    104, 113, 202, 141, 60, 32, 140, 22, 216, 124, 253, 71,
];

pub(crate) fn validate(point: &[u8], absorbed: bool) -> Result<(), VerifyError> {
    if point == [0; 64]
        || point.chunks_exact(32).any(|c| c >= Q.as_slice() || (absorbed && c[..8] == [0; 8]))
    {
        return Err(VerifyError::InvalidEncoding);
    }
    // The syscall checks the curve equation; G1 has cofactor one.
    add(point, &[0; 64]).map_err(|_| VerifyError::InvalidEncoding)?;
    Ok(())
}

pub(crate) fn add(a: &[u8], b: &[u8]) -> Result<[u8; 64], VerifyError> {
    let mut input = [0; 128];
    input[..64].copy_from_slice(a);
    input[64..].copy_from_slice(b);
    alt_bn128_addition(&input)
        .map_err(|_| VerifyError::CurveOperation)?
        .try_into()
        .map_err(|_| VerifyError::CurveOperation)
}

pub(crate) fn mul(point: &[u8], scalar: &[u8; 32]) -> Result<[u8; 64], VerifyError> {
    let mut input = [0; 96];
    input[..64].copy_from_slice(point);
    input[64..].copy_from_slice(scalar);
    alt_bn128_multiplication(&input)
        .map_err(|_| VerifyError::CurveOperation)?
        .try_into()
        .map_err(|_| VerifyError::CurveOperation)
}

pub(crate) fn negate(mut point: [u8; 64]) -> [u8; 64] {
    if point[32..] != [0; 32] {
        let mut borrow = 0i16;
        for i in (0..32).rev() {
            let difference = i16::from(Q[i]) - i16::from(point[32 + i]) - borrow;
            point[32 + i] = difference as u8;
            borrow = i16::from(difference < 0);
        }
    }
    point
}

pub(crate) fn pair(a: &[u8; 64], b: &[u8; 64]) -> Result<(), VerifyError> {
    let mut input = [0; 384];
    input[..64].copy_from_slice(a);
    input[64..192].copy_from_slice(&profile::G2);
    input[192..256].copy_from_slice(b);
    input[256..].copy_from_slice(&profile::TAU);
    let output = alt_bn128_pairing(&input).map_err(|_| VerifyError::CurveOperation)?;
    let mut one = [0; 32];
    one[31] = 1;
    if output.as_slice() == one {
        Ok(())
    } else {
        Err(VerifyError::InvalidProof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::{Fq, Fq2, G1Affine, G2Affine};
    use ark_ff::{BigInteger, PrimeField};
    #[test]
    fn fixed_key_is_canonical_on_curve_and_in_subgroup() {
        assert_eq!(Q.as_slice(), Fq::MODULUS.to_bytes_be());
        for p in profile::FIXED {
            assert!(p.chunks_exact(32).all(|c| c < Q.as_slice()));
            let p = G1Affine::new_unchecked(
                Fq::from_be_bytes_mod_order(&p[..32]),
                Fq::from_be_bytes_mod_order(&p[32..]),
            );
            assert!(p.is_on_curve() && p.is_in_correct_subgroup_assuming_on_curve() && !p.infinity);
        }
        for p in [profile::G2, profile::TAU] {
            assert!(p.chunks_exact(32).all(|c| c < Q.as_slice()));
            let q = |i: usize| Fq::from_be_bytes_mod_order(&p[i * 32..(i + 1) * 32]);
            let p = G2Affine::new_unchecked(Fq2::new(q(1), q(0)), Fq2::new(q(3), q(2)));
            assert!(p.is_on_curve() && p.is_in_correct_subgroup_assuming_on_curve() && !p.infinity);
        }
    }
    #[test]
    fn errors_propagate_and_negation_preserves_infinity() {
        let mut generator = [0; 64];
        generator[31] = 1;
        generator[63] = 2;
        assert_eq!(validate(&generator, false), Ok(()));
        assert_eq!(validate(&generator, true), Err(VerifyError::InvalidEncoding));
        assert_eq!(negate([0; 64]), [0; 64]);
        assert_eq!(negate(negate(profile::FIXED[0])), profile::FIXED[0]);
        assert_eq!(add(&[255; 64], &[0; 64]), Err(VerifyError::CurveOperation));
        assert_eq!(mul(&[255; 64], &[0; 32]), Err(VerifyError::CurveOperation));
        assert_eq!(pair(&[255; 64], &[0; 64]), Err(VerifyError::CurveOperation));
    }
}
