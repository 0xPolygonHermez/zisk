mod add;
mod common;
mod div;
mod modular;
mod mul;
mod pow;

use add::add_tests;
use div::div_tests;
use modular::modular_tests;
use mul::mul_tests;
use pow::pow_tests;

fn main() {
    add_tests();
    div_tests();
    modular_tests();
    mul_tests();
    pow_tests();
}
