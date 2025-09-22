use std::error::Error;

mod binary_basic_frops;
use binary_basic_frops::BinaryBasicFrops;

fn main() -> Result<(), Box<dyn Error>> {
    let mut frops = BinaryBasicFrops::new();
    frops.generate_cmd(
        "binary_basic_frops_fixed_gen",
        "state-machines/binary/src/binary_basic_frops_fixed.bin",
    )
}
