#![no_main]
ziskos::entrypoint!(main);

mod constants;
mod curve;
mod ecdsa;
mod fp;
mod scalar;
mod schnorr;

use curve::curve_tests;
use ecdsa::ecdsa_tests;
use fp::fp_tests;
use scalar::scalar_tests;
use schnorr::schnorr_tests;

fn main() {
    // Fp
    fp_tests();

    // Scalar
    scalar_tests();

    // Curve
    curve_tests();

    // ECDSA
    ecdsa_tests();

    // Schnorr
    schnorr_tests();
}
