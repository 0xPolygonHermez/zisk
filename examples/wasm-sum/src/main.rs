//! Example wasm32-wasip1 guest for the ZisK zkVM.
//!
//! Reads an 8-byte little-endian `u64` `n` from stdin and prints the sum `1 + 2 + ... + n`.
//! Build it with the stock wasm toolchain (no custom Zisk toolchain needed) and run it with
//! `wasmemu` or `cargo-zisk` — see the README.

use std::io::Read;

fn main() {
    // Read n from stdin (defaults to 100 when no input is provided).
    let mut buf = [0u8; 8];
    let read = std::io::stdin().read(&mut buf).unwrap_or(0);
    let n = if read == 8 { u64::from_le_bytes(buf) } else { 100 };

    let sum: u64 = (1..=n).sum();
    println!("sum of 1..={n} = {sum}");
}
