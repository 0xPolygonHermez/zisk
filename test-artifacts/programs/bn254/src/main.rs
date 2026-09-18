#![no_main]
ziskos::entrypoint!(main);

mod constants;
mod pairing;
// Everything except the pairing *check* exercises internal helpers
// (fp/fp2/.../twist/cyclotomic/final_exp) or the raw pairing entry points, none
// of which have a redirected ziskasm binding. They build against Rust zisklib.
#[cfg(not(feature = "ziskasm"))]
mod cyclotomic;
#[cfg(not(feature = "ziskasm"))]
mod final_exp;
#[cfg(not(feature = "ziskasm"))]
mod fp;
#[cfg(not(feature = "ziskasm"))]
mod fp12;
#[cfg(not(feature = "ziskasm"))]
mod fp2;
#[cfg(not(feature = "ziskasm"))]
mod fp6;
#[cfg(not(feature = "ziskasm"))]
mod twist;

#[cfg(not(feature = "ziskasm"))]
use cyclotomic::cyclotomic_tests;
#[cfg(not(feature = "ziskasm"))]
use final_exp::final_exp_tests;
#[cfg(not(feature = "ziskasm"))]
use fp::fp_tests;
#[cfg(not(feature = "ziskasm"))]
use fp12::fp12_tests;
#[cfg(not(feature = "ziskasm"))]
use fp2::fp2_tests;
#[cfg(not(feature = "ziskasm"))]
use fp6::fp6_tests;
use pairing::pairing_check_tests;
#[cfg(not(feature = "ziskasm"))]
use pairing::pairing_tests;
#[cfg(not(feature = "ziskasm"))]
use twist::twist_tests;

fn main() {
    // Fp
    #[cfg(not(feature = "ziskasm"))]
    fp_tests();

    // Fp2
    #[cfg(not(feature = "ziskasm"))]
    fp2_tests();

    // Fp6
    #[cfg(not(feature = "ziskasm"))]
    fp6_tests();

    // Fp12
    #[cfg(not(feature = "ziskasm"))]
    fp12_tests();

    // Twist
    #[cfg(not(feature = "ziskasm"))]
    twist_tests();

    // Cyclotomic
    #[cfg(not(feature = "ziskasm"))]
    cyclotomic_tests();

    // Final exponentiation
    #[cfg(not(feature = "ziskasm"))]
    final_exp_tests();

    // Pairing
    #[cfg(not(feature = "ziskasm"))]
    pairing_tests();
    pairing_check_tests();
}
