mod test_data;
use test_data::{
    get_arith256_mod_test_data, get_arith256_test_data, get_secp256k1_add_test_data,
    get_secp256k1_dbl_test_data,
};

mod equations;
mod executors;
use executors::arith256::Arith256;
use executors::arith256_mod::Arith256Mod;
use executors::secp256k1::Secp256k1;

// cargo run --release --features="test_data" --bin arith_eq_test_bigint

fn main() {
    let verbose = false;
    let mut index = 0;
    while let Some((p1, p2, p3)) = get_secp256k1_add_test_data(index) {
        println!("testing index secp256k1_add #{} ....", index);
        println!("p1: {:?}", p1);
        println!("p2: {:?}", p2);
        println!("p3: {:?}", p3);
        Secp256k1::verify_add(&p1, &p2, &p3);
        index += 1;
    }
    index = 0;
    while let Some((p1, p3)) = get_secp256k1_dbl_test_data(index) {
        println!("testing index secp256k1_dbl #{} ....", index);
        println!("p1: {:?}", p1);
        println!("p3: {:?}", p3);
        Secp256k1::verify_dbl(&p1, &p3);
        index += 1;
    }

    index = 0;
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
}
