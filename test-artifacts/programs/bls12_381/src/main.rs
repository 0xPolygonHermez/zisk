#![no_main]
ziskos::entrypoint!(main);

mod constants;
mod cyclotomic;
mod final_exp;
mod fp;
mod fp12;
mod fp2;
mod fp6;
mod pairing;
mod twist;

use cyclotomic::cyclotomic_tests;
use final_exp::final_exp_tests;
use fp::fp_tests;
use fp12::fp12_tests;
use fp2::fp2_tests;
use fp6::fp6_tests;
use pairing::pairing_valid_tests;
use twist::twist_tests;

fn main() {
    // Fp
    fp_tests();

    // Fp2
    fp2_tests();

    // Fp6
    fp6_tests();

    // Fp12
    fp12_tests();

    // Twist
    twist_tests();

    // Cyclotomic
    cyclotomic_tests();

    // Final exponentiation
    final_exp_tests();

    // Pairing
    pairing_valid_tests();
    // pairing_invalid_tests();
}
