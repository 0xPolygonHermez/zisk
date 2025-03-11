use ark_ff::{BigInt, PrimeField};
use ark_secp256k1::Fq as Secp256k1Field; // Camp finit de secp256k1
use ark_std::{One, Zero};
mod test_data;
use test_data::{get_secp256k1_add_test_data, get_secp256k1_dbl_test_data};

fn secp256k1_add(p1: &[u64; 8], p2: &[u64; 8], p: &mut [u64; 8]) {
    let x1 = Secp256k1Field::from(BigInt::<4>(p1[0..4].try_into().unwrap()));
    let y1 = Secp256k1Field::from(BigInt::<4>(p1[4..8].try_into().unwrap()));
    let x2 = Secp256k1Field::from(BigInt::<4>(p2[0..4].try_into().unwrap()));
    let y2 = Secp256k1Field::from(BigInt::<4>(p2[4..8].try_into().unwrap()));

    let s = (y2 - y1) / (x2 - x1);
    let x3 = s * s - (x1 + x2);
    let y3 = s * (x1 - x3) - y1;

    p[..4].copy_from_slice(&x3.into_bigint().0);
    p[4..].copy_from_slice(&y3.into_bigint().0);
}

fn secp256k1_dbl(p1: &[u64; 8], p: &mut [u64; 8]) {
    let x1 = Secp256k1Field::from(BigInt::<4>(p1[0..4].try_into().unwrap()));
    let y1 = Secp256k1Field::from(BigInt::<4>(p1[4..8].try_into().unwrap()));

    let s = (Secp256k1Field::from(3u64) * x1 * x1) / (y1 + y1);
    let x3 = s * s - (x1 + x1);
    let y3 = s * (x1 - x3) - y1;

    p[..4].copy_from_slice(&x3.into_bigint().0);
    p[4..].copy_from_slice(&y3.into_bigint().0);
}

fn verify_secp256k1_add(test_id: usize, p1: &[u64; 8], p2: &[u64; 8], p: &mut [u64; 8]) {
    let mut _p = [0u64; 8];
    secp256k1_add(p1, p2, &mut _p);
    assert_eq!(&p[..], &_p[..8], "fail test {}", test_id);
    println!("Test #{} (secp256k1_add) .... [\x1B[32mOK\x1B[0m]", test_id)
}
fn verify_secp256k1_dbl(test_id: usize, p1: &[u64; 8], p: &mut [u64; 8]) {
    let mut _p = [0u64; 8];
    secp256k1_dbl(p1, &mut _p);
    assert_eq!(&p[..], &_p[..8], "fail test {}", test_id);
    println!("Test #{} (secp256k1_dbl) .... [\x1B[32mOK\x1B[0m]", test_id)
}

fn test() {
    let mut index = 0;
    while let Some((p1, p2, mut p3)) = get_secp256k1_add_test_data(index) {
        verify_secp256k1_add(index, &p1, &p2, &mut p3);
        index += 1;
    }
    index = 0;
    while let Some((p1, mut p3)) = get_secp256k1_dbl_test_data(index) {
        verify_secp256k1_dbl(index, &p1, &mut p3);
        index += 1;
    }
}

fn main() {
    let arr = BigInt::<4>([
        0xFFFF_FFFE_FFFF_FC2E,
        0xFFFFFFFFFFFFFFFF,
        0xFFFFFFFFFFFFFFFF,
        0xFFFFFFFFFFFFFFFF,
    ]);

    let element = Secp256k1Field::from(arr);
    println!("Element: {:?}", element);
    let one = Secp256k1Field::one();
    let zero = Secp256k1Field::zero();
    let sum = zero - one;
    println!("0-1: {:?}", sum);
    test();
}
