#![no_main]
ziskos::entrypoint!(main);

mod constants;
mod ecdsa;

use ecdsa::ecdsa_tests;

fn main() {
    // ECDSA
    ecdsa_tests();
}
