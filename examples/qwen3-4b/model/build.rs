use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let mut table = Vec::with_capacity(2 * 65536);
    for bits in 0..=u16::MAX {
        let x = f32::from_bits(u32::from(bits) << 16);
        // Nonfinite inputs take the arithmetic path in the guest.
        let rounded = if x.is_finite() {
            let y = (x / (1.0 + libm::expf(-x))).to_bits();
            (y.wrapping_add(0x7fff + ((y >> 16) & 1)) >> 16) as u16
        } else {
            0
        };
        table.extend_from_slice(&rounded.to_le_bytes());
    }
    let path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("silu-bf16.bin");
    fs::write(path, table).unwrap();
}
