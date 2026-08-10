//! Example wasm32-wasip1 guest for the ZisK zkVM.
//!
//! Reads an 8-byte little-endian `u64` `n` from stdin (defaults to 10 when no input is provided)
//! and prints the n-th Fibonacci number (`fib(0) = 0`, `fib(1) = 1`).  Build it with the stock
//! wasm toolchain (no custom Zisk toolchain needed) — see the README.
//!
//! This example doubles as the fixture for the `wasm_example` integration test of the emulator,
//! which compiles it and runs the resulting module through the wasm → Zisk ROM transpiler.

use std::io::Read;

fn fib(n: u64) -> u64 {
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 0..n {
        (a, b) = (b, a + b);
    }
    a
}

fn main() {
    // Read n from stdin (defaults to 10 when no input is provided).
    let mut buf = [0u8; 8];
    let read = std::io::stdin().read(&mut buf).unwrap_or(0);
    let n = if read == 8 { u64::from_le_bytes(buf) } else { 10 };

    println!("fib({n}) = {}", fib(n));
}
