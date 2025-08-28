use std::error::Error;

mod arith_frops;
use arith_frops::ArithFrops;

fn main() -> Result<(), Box<dyn Error>> {
    let mut frops = ArithFrops::new();
    frops.generate_cmd("arith_frops_fixed_gen", "state-machines/arith/src/arith_frops_fixed.bin")
}
