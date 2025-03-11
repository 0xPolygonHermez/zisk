use ark_ff::PrimeField;
use ark_secp256k1::Fq as Secp256k1Field;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::Zero;
mod test_data;
use test_data::{
    get_arith256_mod_test_data, get_arith256_test_data, get_secp256k1_add_test_data,
    get_secp256k1_dbl_test_data,
};

mod helpers;
use helpers::eq_arith_256::EqArith256;
use helpers::eq_arith_256_mod::EqArith256Mod;
use helpers::eq_secp256k1_add::EqSecp256k1Add;
use helpers::eq_secp256k1_dbl::EqSecp256k1Dbl;
use helpers::eq_secp256k1_x3::EqSecp256k1X3;
use helpers::eq_secp256k1_y3::EqSecp256k1Y3;

mod executors;
use executors::arith256::Arith256;
use executors::arith256_mod::Arith256Mod;
use executors::secp256k1::Secp256k1;

fn main() {
    let verbose = false;
    let test = Secp256k1::new();
    let mut index = 0;
    while let Some((p1, p2, p3)) = get_secp256k1_add_test_data(index) {
        println!("testing index secp256k1_add #{} ....", index);
        println!("p1: {:?}", p1);
        println!("p2: {:?}", p2);
        println!("p3: {:?}", p3);
        test.verify_add(&p1, &p2, &p3);
        index += 1;
    }
    index = 0;
    while let Some((p1, p3)) = get_secp256k1_dbl_test_data(index) {
        println!("testing index secp256k1_dbl #{} ....", index);
        println!("p1: {:?}", p1);
        println!("p3: {:?}", p3);
        test.verify_dbl(&p1, &p3);
        index += 1;
    }

    let test = Arith256::new();
    index = 0;
    while let Some((a, b, c, dh, dl)) = get_arith256_test_data(index) {
        println!("testing index arith256 #{} ....", index);
        if verbose {
            println!("ARITH256 a:{:?}\nb:{:?}\nc:{:?}dh:{:?}\ndl:{:?}", a, b, c, dh, dl);
        }
        test.verify(&a, &b, &c, &dl, &dh);
        index += 1;
    }

    let test = Arith256Mod::new();
    index = 0;
    while let Some((a, b, c, module, d)) = get_arith256_mod_test_data(index) {
        println!("testing index arith256_mod #{} ....", index);
        if verbose {
            println!("ARITH256_MOD a:{:?}\nb:{:?}\nc:{:?}dh:{:?}\ndl:{:?}", a, b, c, module, d);
        }
        test.verify(&a, &b, &c, &module, &d);
        index += 1;
    }
}
