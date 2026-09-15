#![no_main]
ziskos::entrypoint!(main);

mod constants;
#[cfg(not(feature = "ziskasm"))]
mod curve;
mod ecdsa;
#[cfg(not(feature = "ziskasm"))]
mod fp;
#[cfg(not(feature = "ziskasm"))]
mod scalar;

#[cfg(not(feature = "ziskasm"))]
use curve::curve_tests;
use ecdsa::ecdsa_tests;
#[cfg(not(feature = "ziskasm"))]
use fp::fp_tests;
#[cfg(not(feature = "ziskasm"))]
use scalar::scalar_tests;

fn main() {
    // Internal-arithmetic tests: Rust backend only (no ziskasm binding).
    #[cfg(not(feature = "ziskasm"))]
    {
        fp_tests();
        scalar_tests();
        curve_tests();
    }

    // ECDSA: redirected to the .zisk routine in the `ziskasm` build.
    ecdsa_tests();
}
