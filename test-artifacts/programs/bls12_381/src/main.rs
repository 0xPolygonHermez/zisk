#![no_main]
ziskos::entrypoint!(main);

mod constants;
mod hash_to_curve;
// Everything except hash_to_curve exercises internal helpers (fp/twist/pairing/
// msm/...) or entry points with no redirected ziskasm binding — only
// hash_to_curve_g2 is redirected. They build against the Rust zisklib only.
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
mod msm;
#[cfg(not(feature = "ziskasm"))]
mod pairing;
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
use hash_to_curve::hash_to_curve_tests;
#[cfg(not(feature = "ziskasm"))]
use msm::msm_tests;
#[cfg(not(feature = "ziskasm"))]
use pairing::pairing_valid_tests;
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

    // Hash to curve
    hash_to_curve_tests();

    // Pairing
    #[cfg(not(feature = "ziskasm"))]
    pairing_valid_tests();
    // pairing_invalid_tests();

    // MSM
    #[cfg(not(feature = "ziskasm"))]
    msm_tests();
}
