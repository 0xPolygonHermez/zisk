#![no_main]
ziskos::entrypoint!(main);

mod constants;

// The array-arithmetic and squaring tests exercise internal bigint helpers
// (mul_long, div_long, square_long, ...) that have no ziskasm binding — only the
// public `modexp` entry point is redirected. They run against the Rust zisklib
// only; the `ziskasm` backend build skips them.
#[cfg(not(feature = "ziskasm"))]
mod array_arith;
mod modexp;
#[cfg(not(feature = "ziskasm"))]
mod square;

#[cfg(not(feature = "ziskasm"))]
use array_arith::array_arith_tests;
use modexp::modexp_tests;
#[cfg(not(feature = "ziskasm"))]
use square::square_tests;

fn main() {
    #[cfg(not(feature = "ziskasm"))]
    array_arith_tests();

    #[cfg(not(feature = "ziskasm"))]
    square_tests();

    modexp_tests();
}
