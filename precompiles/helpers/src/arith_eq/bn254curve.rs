// TODO: Implement these functions in assembly to speed things up!

use ark_bn254::Fq as Bn254Field;
use ark_ff::{BigInt, PrimeField};

#[inline(always)]
pub fn bn254_curve_add(p1: &[u64; 8], p2: &[u64; 8], p: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(p1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(p1[4..8].try_into().unwrap()));
    let x2 = Bn254Field::from(BigInt::<4>(p2[0..4].try_into().unwrap()));
    let y2 = Bn254Field::from(BigInt::<4>(p2[4..8].try_into().unwrap()));

    let s = (y2 - y1) / (x2 - x1);
    let x3 = s * s - (x1 + x2);
    let y3 = s * (x1 - x3) - y1;

    p[..4].copy_from_slice(&x3.into_bigint().0);
    p[4..].copy_from_slice(&y3.into_bigint().0);
}

#[inline(always)]
pub fn bn254_curve_dbl(p1: &[u64; 8], p: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(p1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(p1[4..8].try_into().unwrap()));

    let s = (Bn254Field::from(3u64) * x1 * x1) / (y1 + y1);
    let x3 = s * s - (x1 + x1);
    let y3 = s * (x1 - x3) - y1;

    p[..4].copy_from_slice(&x3.into_bigint().0);
    p[4..].copy_from_slice(&y3.into_bigint().0);
}
