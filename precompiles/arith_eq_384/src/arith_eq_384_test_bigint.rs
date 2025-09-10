mod arith_eq_384_constants;
mod equations;
mod executors;
mod test_data;

use arith_eq_384_constants::*;
use executors::{
    arith384_mod::Arith384Mod, bls12_381_complex::Bls12_381Complex, bls12_381_curve::Bls12_381Curve,
};
use test_data::{
    get_arith384_mod_test_data, get_bls12_381_complex_add_test_data,
    get_bls12_381_complex_mul_test_data, get_bls12_381_complex_sub_test_data,
    get_bls12_381_curve_add_test_data, get_bls12_381_curve_dbl_test_data,
};

// cargo run --release --features="test_data" --bin arith_eq_384_test_bigint

fn main() {
    let verbose = false;
    let mut index = 0;
    while let Some((a, b, c, module, d)) = get_arith384_mod_test_data(index) {
        println!("testing index arith384_mod #{} ....", index);
        if verbose {
            println!("ARITH384_MOD a:{:?}\nb:{:?}\nc:{:?}dh:{:?}\ndl:{:?}", a, b, c, module, d);
        }
        Arith384Mod::verify(&a, &b, &c, &module, &d);
        index += 1;
    }

    index = 0;
    while let Some((p1, p2, p3)) = get_bls12_381_curve_add_test_data(index) {
        println!("testing index bls12_381_curve_add #{} ....", index);
        if verbose {
            println!("BLS12_381_CURVE_ADD\n  p1: {:?},\n  p2: {:?},\n  p3: {:?}", p1, p2, p3);
        }
        Bls12_381Curve::verify_add(&p1, &p2, &p3);
        index += 1;
    }

    index = 0;
    while let Some((p1, p3)) = get_bls12_381_curve_dbl_test_data(index) {
        println!("testing index bls12_381_curve_dbl #{} ....", index);
        if verbose {
            println!("BLS12_381_CURVE_DBL\n  p1: {:?},\n  p3: {:?}", p1, p3);
        }
        Bls12_381Curve::verify_dbl(&p1, &p3);
        index += 1;
    }

    index = 0;
    while let Some((f1, f2, f3)) = get_bls12_381_complex_add_test_data(index) {
        println!("testing index bls12_381_complex_add #{} ....", index);
        if verbose {
            println!("BLS12_381_COMPLEX_ADD\n  f1: {:?},\n  f2: {:?},\n  f3: {:?}", f1, f2, f3);
        }
        Bls12_381Complex::verify_add(&f1, &f2, &f3);
        index += 1;
    }

    index = 0;
    while let Some((f1, f2, f3)) = get_bls12_381_complex_sub_test_data(index) {
        println!("testing index bls12_381_complex_sub #{} ....", index);
        if verbose {
            println!("BLS12_381_COMPLEX_SUB\n  f1: {:?},\n  f2: {:?},\n  f3: {:?}", f1, f2, f3);
        }
        Bls12_381Complex::verify_sub(&f1, &f2, &f3);
        index += 1;
    }

    index = 0;
    while let Some((f1, f2, f3)) = get_bls12_381_complex_mul_test_data(index) {
        println!("testing index bls12_381_complex_mul #{} ....", index);
        if verbose {
            println!("BLS12_381_COMPLEX_MUL\n  f1: {:?},\n  f2: {:?},\n  f3: {:?}", f1, f2, f3);
        }
        Bls12_381Complex::verify_mul(&f1, &f2, &f3);
        index += 1;
    }
}
