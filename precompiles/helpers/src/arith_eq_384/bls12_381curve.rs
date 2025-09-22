// TODO: Implement these functions in assembly to speed things up!

use ark_bls12_381::Fq as Bls12_381Field;
use ark_ff::{BigInt, PrimeField};

#[inline(always)]
pub fn bls12_381_curve_add(p1: &[u64; 12], p2: &[u64; 12], p: &mut [u64; 12]) {
    let x1 = Bls12_381Field::from(BigInt::<6>(p1[0..6].try_into().unwrap()));
    let y1 = Bls12_381Field::from(BigInt::<6>(p1[6..12].try_into().unwrap()));
    let x2 = Bls12_381Field::from(BigInt::<6>(p2[0..6].try_into().unwrap()));
    let y2 = Bls12_381Field::from(BigInt::<6>(p2[6..12].try_into().unwrap()));

    let s = (y2 - y1) / (x2 - x1);
    let x3 = s * s - (x1 + x2);
    let y3 = s * (x1 - x3) - y1;

    p[..6].copy_from_slice(&x3.into_bigint().0);
    p[6..].copy_from_slice(&y3.into_bigint().0);
}

#[inline(always)]
pub fn bls12_381_curve_dbl(p1: &[u64; 12], p: &mut [u64; 12]) {
    let x1 = Bls12_381Field::from(BigInt::<6>(p1[0..6].try_into().unwrap()));
    let y1 = Bls12_381Field::from(BigInt::<6>(p1[6..12].try_into().unwrap()));

    let s = (Bls12_381Field::from(3u64) * x1 * x1) / (y1 + y1);
    let x3 = s * s - (x1 + x1);
    let y3 = s * (x1 - x3) - y1;

    p[..6].copy_from_slice(&x3.into_bigint().0);
    p[6..].copy_from_slice(&y3.into_bigint().0);
}
