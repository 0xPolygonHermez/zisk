// TODO: Implement these functions in assembly to speed things up!

use ark_bls12_381::Fq as Bls12_381Field;
use ark_ff::{BigInt, PrimeField};

#[inline(always)]
pub fn bls12_381_complex_add(f1: &[u64; 12], f2: &[u64; 12], f: &mut [u64; 12]) {
    let x1 = Bls12_381Field::from(BigInt::<6>(f1[0..6].try_into().unwrap()));
    let y1 = Bls12_381Field::from(BigInt::<6>(f1[6..12].try_into().unwrap()));
    let x2 = Bls12_381Field::from(BigInt::<6>(f2[0..6].try_into().unwrap()));
    let y2 = Bls12_381Field::from(BigInt::<6>(f2[6..12].try_into().unwrap()));

    let x3 = x1 + x2;
    let y3 = y1 + y2;

    f[..6].copy_from_slice(&x3.into_bigint().0);
    f[6..].copy_from_slice(&y3.into_bigint().0);
}

#[inline(always)]
pub fn bls12_381_complex_sub(f1: &[u64; 12], f2: &[u64; 12], f: &mut [u64; 12]) {
    let x1 = Bls12_381Field::from(BigInt::<6>(f1[0..6].try_into().unwrap()));
    let y1 = Bls12_381Field::from(BigInt::<6>(f1[6..12].try_into().unwrap()));
    let x2 = Bls12_381Field::from(BigInt::<6>(f2[0..6].try_into().unwrap()));
    let y2 = Bls12_381Field::from(BigInt::<6>(f2[6..12].try_into().unwrap()));

    let x3 = x1 - x2;
    let y3 = y1 - y2;

    f[..6].copy_from_slice(&x3.into_bigint().0);
    f[6..].copy_from_slice(&y3.into_bigint().0);
}

#[inline(always)]
pub fn bls12_381_complex_mul(f1: &[u64; 12], f2: &[u64; 12], f: &mut [u64; 12]) {
    let x1 = Bls12_381Field::from(BigInt::<6>(f1[0..6].try_into().unwrap()));
    let y1 = Bls12_381Field::from(BigInt::<6>(f1[6..12].try_into().unwrap()));
    let x2 = Bls12_381Field::from(BigInt::<6>(f2[0..6].try_into().unwrap()));
    let y2 = Bls12_381Field::from(BigInt::<6>(f2[6..12].try_into().unwrap()));

    let x3 = x1 * x2 - y1 * y2;
    let y3 = y1 * x2 + x1 * y2;

    f[..6].copy_from_slice(&x3.into_bigint().0);
    f[6..].copy_from_slice(&y3.into_bigint().0);
}
