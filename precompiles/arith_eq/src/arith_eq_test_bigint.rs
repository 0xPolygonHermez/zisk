mod test_data;
use test_data::{
    get_arith256_mod_test_data, get_arith256_test_data, get_bn254_curve_add_test_data,
    get_secp256k1_add_test_data, get_secp256k1_dbl_test_data,
};

mod equations;
mod executors;
use executors::{
    arith256::Arith256, arith256_mod::Arith256Mod, bn254_curve::Bn254Curve, secp256k1::Secp256k1,
};

// cargo run --release --features="test_data" --bin arith_eq_test_bigint

fn main() {
    let verbose = false;
    let mut index = 0;
    while let Some((a, b, c, dh, dl)) = get_arith256_test_data(index) {
        println!("testing index arith256 #{} ....", index);
        if verbose {
            println!("ARITH256 a:{:?}\nb:{:?}\nc:{:?}dh:{:?}\ndl:{:?}", a, b, c, dh, dl);
        }
        Arith256::verify(&a, &b, &c, &dl, &dh);
        index += 1;
    }

    index = 0;
    while let Some((a, b, c, module, d)) = get_arith256_mod_test_data(index) {
        println!("testing index arith256_mod #{} ....", index);
        if verbose {
            println!("ARITH256_MOD a:{:?}\nb:{:?}\nc:{:?}dh:{:?}\ndl:{:?}", a, b, c, module, d);
        }
        Arith256Mod::verify(&a, &b, &c, &module, &d);
        index += 1;
    }

    index = 0;
    while let Some((p1, p2, p3)) = get_secp256k1_add_test_data(index) {
        println!("testing index secp256k1_add #{} ....", index);
        if verbose {
            println!("SECP256K1_ADD\n  p1: {:?},\n  p2: {:?},\n  p3: {:?}", p1, p2, p3);
        }
        Secp256k1::verify_add(&p1, &p2, &p3);
        index += 1;
    }

    index = 0;
    while let Some((p1, p3)) = get_secp256k1_dbl_test_data(index) {
        println!("testing index secp256k1_dbl #{} ....", index);
        if verbose {
            println!("SECP256K1_DBL\n  p1: {:?},\n  p3: {:?}", p1, p3);
        }
        Secp256k1::verify_dbl(&p1, &p3);
        index += 1;
    }

    // index = 0;
    // while let Some((p1, p2, p3)) = get_bn254_curve_add_test_data(index) {
    //     println!("testing index bn254_curve_add #{} ....", index);
    //     if verbose {
    //         println!("BN254_CURVE_ADD\n  p1: {:?},\n  p2: {:?},\n  p3: {:?}", p1, p2, p3);
    //     }
    //     Bn254Curve::verify_add(&p1, &p2, &p3);
    //     index += 1;
    // }
}
