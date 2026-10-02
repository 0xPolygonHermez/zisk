//! End-to-end test for a real, compiler-produced wasm guest: builds `examples/wasm-fibonacci`
//! with the stock Rust `wasm32-wasip1` target, transpiles the module with `wasm2rom`, runs it on
//! the emulator, and checks the observable output.
//!
//! Unlike the hand-written modules in `wasm.rs`, this exercises LLVM-generated code: real data
//! segments, the Rust std WASI runtime (`environ_sizes_get`, buffered `fd_write`, `proc_exit`),
//! and non-trivial control flow.
//!
//! The test is skipped with a notice when the `wasm32-wasip1` rustup target is not installed
//! (`rustup target add wasm32-wasip1`).

use std::path::PathBuf;
use std::process::Command;

use zisk_common::EmuTrace;
use zisk_transpiler_wasm::wasm2rom;
use ziskemu::{EmuOptions, ZiskEmulator};

/// Compiles the wasm-fibonacci example crate and returns the wasm module bytes.
fn build_example() -> Vec<u8> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/wasm-fibonacci");
    let target_dir = dir.join("target");
    let output = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-wasip1", "--target-dir"])
        .arg(&target_dir)
        .current_dir(&dir)
        .output()
        .expect("failed to spawn cargo");
    assert!(
        output.status.success(),
        "building wasm-fibonacci failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read(target_dir.join("wasm32-wasip1/release/wasm-fibonacci.wasm"))
        .expect("wasm artifact missing after successful build")
}

/// Transpiles and emulates `wasm` with the given raw input bytes, returning the public output.
fn run(wasm: &[u8], input: &[u8]) -> Vec<u8> {
    let rom = wasm2rom(wasm).expect("wasm2rom");
    let opts = EmuOptions::default();
    ZiskEmulator::process_rom(&rom, input, &opts, None::<fn(EmuTrace)>).expect("emulation")
}

/// Wraps `data` in the emulator input blob format: an 8-byte little-endian length prefix
/// followed by the data bytes.
fn input_blob(data: &[u8]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(8 + data.len());
    blob.extend_from_slice(&(data.len() as u64).to_le_bytes());
    blob.extend_from_slice(data);
    blob
}

/// Interprets the fixed-size public output region as a NUL-terminated text string.
fn out_text(out: &[u8]) -> &str {
    let end = out.iter().position(|&b| b == 0).unwrap_or(out.len());
    std::str::from_utf8(&out[..end]).expect("output is not valid UTF-8")
}

#[test]
#[cfg_attr(
    not(wasm32_wasip1_target),
    ignore = "the wasm32-wasip1 target is not installed (rustup target add wasm32-wasip1)"
)]
fn compiled_fibonacci_guest() {
    let wasm = build_example();

    // Default run: no input, n = 10.
    let out = run(&wasm, &[]);
    assert_eq!(out_text(&out), "fib(10) = 55\n");

    // Input-driven run: n = 90, near the u64 limit.
    let out = run(&wasm, &input_blob(&90u64.to_le_bytes()));
    assert_eq!(out_text(&out), "fib(90) = 2880067194370816120\n");
}
