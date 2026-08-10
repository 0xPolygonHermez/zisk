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
use zisk_core::wasm::wasm2rom;
use ziskemu::{EmuOptions, ZiskEmulator};

/// Returns true if the `wasm32-wasip1` rustup target is available for the active toolchain.
fn wasm_target_installed() -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| {
            o.status.success()
                && String::from_utf8_lossy(&o.stdout).lines().any(|l| l.trim() == "wasm32-wasip1")
        })
        .unwrap_or(false)
}

/// Compiles the wasm-fibonacci example crate and returns the wasm module bytes.
fn build_example() -> Vec<u8> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/wasm-fibonacci");
    let output = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-wasip1"])
        .current_dir(&dir)
        .output()
        .expect("failed to spawn cargo");
    assert!(
        output.status.success(),
        "building wasm-fibonacci failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::read(dir.join("target/wasm32-wasip1/release/wasm-fibonacci.wasm"))
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
fn compiled_fibonacci_guest() {
    if !wasm_target_installed() {
        eprintln!(
            "SKIPPED compiled_fibonacci_guest: rustup target wasm32-wasip1 is not installed \
             (run `rustup target add wasm32-wasip1`)"
        );
        return;
    }

    let wasm = build_example();

    // Default run: no input, n = 10.
    let out = run(&wasm, &[]);
    assert_eq!(out_text(&out), "fib(10) = 55\n");

    // Input-driven run: n = 90, near the u64 limit.
    let out = run(&wasm, &input_blob(&90u64.to_le_bytes()));
    assert_eq!(out_text(&out), "fib(90) = 2880067194370816120\n");
}
