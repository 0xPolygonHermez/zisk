use ark_ff::BigInt;
use ark_secp256k1::Fq as Secp256k1Field;
use ark_std::{One, Zero};
use std::time::Instant;
#[cfg(any(feature = "test_data", feature = "test_data_secp256k1"))]
mod test_data;
#[cfg(any(feature = "test_data", feature = "test_data_secp256k1"))]
use precompiles_helpers::{secp256k1_add, secp256k1_dbl};
#[cfg(any(feature = "test_data", feature = "test_data_secp256k1"))]
use test_data::{get_secp256k1_add_test_data, get_secp256k1_dbl_test_data};
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

#[cfg(any(feature = "test_data", feature = "test_data_secp256k1"))]
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

    // Run the first test a million times to measure performance
    if let Some((p1, p2, mut p3)) = get_secp256k1_add_test_data(0) {
        let start = Instant::now();
        for _ in 0..1000000 {
            secp256k1_add(&p1, &p2, &mut p3);
        }
        let duration = start.elapsed();
        let secs = duration.as_secs_f64();
        let tp = if secs == 0.0 { 1_f64 } else { 1_f64 / secs };
        println!("Duration = {:.4} sec, TP = {:.4} M/sec", secs, tp);
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
