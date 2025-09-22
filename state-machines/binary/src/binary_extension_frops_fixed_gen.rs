use std::error::Error;

mod binary_extension_frops;
use binary_extension_frops::BinaryExtensionFrops;

fn main() -> Result<(), Box<dyn Error>> {
    let mut frops = BinaryExtensionFrops::new();
    frops.generate_cmd(
        "binary_extension_frops_fixed_gen",
        "state-machines/binary/src/binary_extension_frops_fixed.bin",
    )
}
