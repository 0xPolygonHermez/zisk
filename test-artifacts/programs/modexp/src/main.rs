#![no_main]
ziskos::entrypoint!(main);

mod constants;

mod array_arith;
mod modexp;
mod square;

use array_arith::array_arith_tests;
use modexp::modexp_tests;
use square::square_tests;

fn main() {
    array_arith_tests();

    square_tests();

    modexp_tests();
}
