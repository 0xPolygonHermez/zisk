#[cfg(feature = "bn254_complex_rust")]
use ark_bn254::Fq as Bn254Field;
#[cfg(feature = "bn254_complex_rust")]
use ark_ff::{BigInt, PrimeField};

#[cfg(not(feature = "bn254_complex_rust"))]
use ark_bn254::Fq as Bn254Field;
#[cfg(not(feature = "bn254_complex_rust"))]
use ark_ff::{BigInt, PrimeField};

// TODO: Implement this function in assembly
#[cfg(not(feature = "bn254_complex_rust"))]
#[inline(always)]
pub fn bn254_complex_add(f1: &[u64; 8], f2: &[u64; 8], f: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(f1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(f1[4..8].try_into().unwrap()));
    let x2 = Bn254Field::from(BigInt::<4>(f2[0..4].try_into().unwrap()));
    let y2 = Bn254Field::from(BigInt::<4>(f2[4..8].try_into().unwrap()));

    let x3 = x1 + x2;
    let y3 = y1 + y2;

    f[..4].copy_from_slice(&x3.into_bigint().0);
    f[4..].copy_from_slice(&y3.into_bigint().0);
}

// TODO: Implement this function in assembly
#[cfg(not(feature = "bn254_complex_rust"))]
#[inline(always)]
pub fn bn254_complex_sub(f1: &[u64; 8], f2: &[u64; 8], f: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(f1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(f1[4..8].try_into().unwrap()));
    let x2 = Bn254Field::from(BigInt::<4>(f2[0..4].try_into().unwrap()));
    let y2 = Bn254Field::from(BigInt::<4>(f2[4..8].try_into().unwrap()));

    let x3 = x1 - x2;
    let y3 = y1 - y2;

    f[..4].copy_from_slice(&x3.into_bigint().0);
    f[4..].copy_from_slice(&y3.into_bigint().0);
}

// TODO: Implement this function in assembly
#[cfg(not(feature = "bn254_complex_rust"))]
#[inline(always)]
pub fn bn254_complex_mul(f1: &[u64; 8], f2: &[u64; 8], f: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(f1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(f1[4..8].try_into().unwrap()));
    let x2 = Bn254Field::from(BigInt::<4>(f2[0..4].try_into().unwrap()));
    let y2 = Bn254Field::from(BigInt::<4>(f2[4..8].try_into().unwrap()));

    let x3 = x1 * x2 - y1 * y2;
    let y3 = y1 * x2 + x1 * y2;

    f[..4].copy_from_slice(&x3.into_bigint().0);
    f[4..].copy_from_slice(&y3.into_bigint().0);
}

#[cfg(feature = "bn254_complex_rust")]
pub fn bn254_complex_add(f1: &[u64; 8], f2: &[u64; 8], f: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(f1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(f1[4..8].try_into().unwrap()));
    let x2 = Bn254Field::from(BigInt::<4>(f2[0..4].try_into().unwrap()));
    let y2 = Bn254Field::from(BigInt::<4>(f2[4..8].try_into().unwrap()));

    let x3 = x1 + x2;
    let y3 = y1 + y2;

    f[..4].copy_from_slice(&x3.into_bigint().0);
    f[4..].copy_from_slice(&y3.into_bigint().0);
}

#[cfg(feature = "bn254_complex_rust")]
pub fn bn254_complex_sub(f1: &[u64; 8], f2: &[u64; 8], f: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(f1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(f1[4..8].try_into().unwrap()));
    let x2 = Bn254Field::from(BigInt::<4>(f2[0..4].try_into().unwrap()));
    let y2 = Bn254Field::from(BigInt::<4>(f2[4..8].try_into().unwrap()));

    let x3 = x1 - x2;
    let y3 = y1 - y2;

    f[..4].copy_from_slice(&x3.into_bigint().0);
    f[4..].copy_from_slice(&y3.into_bigint().0);
}

#[cfg(feature = "bn254_complex_rust")]
pub fn bn254_complex_mul(f1: &[u64; 8], f2: &[u64; 8], f: &mut [u64; 8]) {
    let x1 = Bn254Field::from(BigInt::<4>(f1[0..4].try_into().unwrap()));
    let y1 = Bn254Field::from(BigInt::<4>(f1[4..8].try_into().unwrap()));
    let x2 = Bn254Field::from(BigInt::<4>(f2[0..4].try_into().unwrap()));
    let y2 = Bn254Field::from(BigInt::<4>(f2[4..8].try_into().unwrap()));

    let x3 = x1 * x2 - y1 * y2;
    let y3 = y1 * x2 + x1 * y2;

    f[..4].copy_from_slice(&x3.into_bigint().0);
    f[4..].copy_from_slice(&y3.into_bigint().0);
}
