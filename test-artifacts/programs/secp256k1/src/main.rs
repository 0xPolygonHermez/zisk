#![no_main]
ziskos::entrypoint!(main);

mod constants;
mod ecdsa;
// fp/scalar/curve/schnorr exercise internal helpers (or the byte-based schnorr
// entry point) that have no redirected ziskasm binding — only ECDSA verify is
// redirected. They build against the Rust zisklib only.
#[cfg(not(feature = "ziskasm"))]
mod curve;
#[cfg(not(feature = "ziskasm"))]
mod fp;
#[cfg(not(feature = "ziskasm"))]
mod scalar;
#[cfg(not(feature = "ziskasm"))]
mod schnorr;

#[cfg(not(feature = "ziskasm"))]
use curve::curve_tests;
use ecdsa::ecdsa_tests;
#[cfg(not(feature = "ziskasm"))]
use fp::fp_tests;
#[cfg(not(feature = "ziskasm"))]
use scalar::scalar_tests;
#[cfg(not(feature = "ziskasm"))]
use schnorr::schnorr_tests;

fn main() {
    // Fp
    #[cfg(not(feature = "ziskasm"))]
    fp_tests();

    // Scalar
    #[cfg(not(feature = "ziskasm"))]
    scalar_tests();

    // Curve
    #[cfg(not(feature = "ziskasm"))]
    curve_tests();

    // ECDSA
    ecdsa_tests();

    // Schnorr
    #[cfg(not(feature = "ziskasm"))]
    schnorr_tests();
}
