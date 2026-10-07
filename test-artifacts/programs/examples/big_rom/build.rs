//! Generates the functions that make `big_rom`'s ROM large. See `src/main.rs` for why.

use std::{env, fmt::Write as _, fs, path::PathBuf};

/// How many distinct functions to emit. Each compiles to roughly two hundred RISC-V
/// instructions, so this gives the ROM about two hundred thousand more than a small guest.
const FUNCTIONS: usize = 1024;

/// Mixing rounds per function: what makes each one long enough to count.
const ROUNDS: usize = 16;

/// SplitMix64, so the constants are fixed, distinct, and not something the compiler can fold.
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn main() {
    let mut src = String::new();
    for f in 0..FUNCTIONS {
        writeln!(src, "#[inline(never)]\nfn f{f}(mut x: u64) -> u64 {{").unwrap();
        for r in 0..ROUNDS {
            let seed = (f * ROUNDS + r) as u64;
            let (mul, xor, add) =
                (splitmix64(3 * seed) | 1, splitmix64(3 * seed + 1), splitmix64(3 * seed + 2));
            let rot = 1 + seed % 63;
            writeln!(
                src,
                "    x = (x.wrapping_mul({mul:#x}) ^ {xor:#x}).rotate_left({rot}).wrapping_add({add:#x});"
            )
            .unwrap();
        }
        writeln!(src, "    x\n}}").unwrap();
    }
    write!(src, "static FUNCTIONS: [fn(u64) -> u64; {FUNCTIONS}] = [").unwrap();
    for f in 0..FUNCTIONS {
        write!(src, "f{f}, ").unwrap();
    }
    writeln!(src, "];").unwrap();

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("functions.rs");
    fs::write(out, src).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
