//! End-to-end tests for the wasm32 → Zisk machine: compile small `.wat` modules, transpile them
//! with `wasm2rom`, run them on the emulator, and check observable output.
//!
//! Test convention: a guest computes an `i32`/`i64` and writes its 8 little-endian bytes to wasm
//! linear memory, then calls the imported `wasi_snapshot_preview1::fd_write` with a single iovec
//! pointing at those bytes.  The WASI layer mirrors stdout into the public output region, so the
//! result is readable via the emulator's output bytes.

use zisk_common::EmuTrace;
use zisk_core::wasm::wasm2rom;
use ziskemu::{EmuOptions, ZiskEmulator};

/// Wraps a function body that leaves a single i64 (or i32, zero/sign-extended by the caller) on the
/// stack into a complete WASI command module that prints the 8 result bytes.
fn module_printing_i64(body: &str) -> Vec<u8> {
    let wat = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory 1)
          (export "memory" (memory 0))
          (func $compute (result i64)
            {body}
          )
          (func (export "_start")
            ;; store result bytes at addr 16
            (i64.store (i32.const 16) (call $compute))
            ;; iovec at addr 0: buf=16, len=8
            (i32.store (i32.const 0) (i32.const 16))
            (i32.store (i32.const 4) (i32.const 8))
            ;; fd_write(1, iovec=0, iovec_len=1, nwritten=40)
            (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))
          )
        )"#
    );
    wat::parse_str(&wat).expect("wat compile")
}

fn run(wasm: &[u8], input: &[u8]) -> Vec<u8> {
    let rom = wasm2rom(wasm).expect("wasm2rom");
    let opts = EmuOptions::default();
    ZiskEmulator::process_rom(&rom, input, &opts, None::<fn(EmuTrace)>).expect("emulation")
}

/// Reads the first 8 output bytes as a little-endian u64.
fn out_u64(out: &[u8]) -> u64 {
    u64::from_le_bytes(out[0..8].try_into().unwrap())
}

#[test]
fn empty_start_terminates() {
    let wasm = wat::parse_str(r#"(module (func (export "_start")))"#).unwrap();
    // Should transpile and run to completion without producing output.
    let _ = run(&wasm, &[]);
}

#[test]
fn i64_arithmetic() {
    let out = run(&module_printing_i64("(i64.add (i64.const 40) (i64.const 2))"), &[]);
    assert_eq!(out_u64(&out), 42);
}

#[test]
fn i64_mul_sub() {
    let out = run(
        &module_printing_i64("(i64.sub (i64.mul (i64.const 6) (i64.const 9)) (i64.const 12))"),
        &[],
    );
    assert_eq!(out_u64(&out), 42);
}

#[test]
fn i64_div_rem() {
    let out = run(&module_printing_i64("(i64.rem_u (i64.const 100) (i64.const 7))"), &[]);
    assert_eq!(out_u64(&out), 100 % 7);
}

#[test]
fn i32_wrap_and_extend() {
    // (0xFFFFFFFF as i32) sign-extended to i64 == -1
    let out = run(
        &module_printing_i64("(i64.extend_i32_s (i32.const -1))"),
        &[],
    );
    assert_eq!(out_u64(&out), u64::MAX);
}

#[test]
fn shifts_and_bitops() {
    let out = run(
        &module_printing_i64("(i64.shl (i64.or (i64.const 1) (i64.const 4)) (i64.const 3))"),
        &[],
    );
    assert_eq!(out_u64(&out), (1 | 4) << 3);
}

#[test]
fn popcnt_clz_ctz() {
    let out = run(&module_printing_i64("(i64.popcnt (i64.const 0xFF))"), &[]);
    assert_eq!(out_u64(&out), 8);
    let out = run(&module_printing_i64("(i64.ctz (i64.const 8))"), &[]);
    assert_eq!(out_u64(&out), 3);
    let out = run(&module_printing_i64("(i64.clz (i64.const 1))"), &[]);
    assert_eq!(out_u64(&out), 63);
}

#[test]
fn locals_and_loop_sum() {
    // sum of 1..=10 via a loop = 55
    let body = r#"
        (local $i i64) (local $acc i64)
        (local.set $i (i64.const 1))
        (local.set $acc (i64.const 0))
        (block $done
          (loop $cont
            (br_if $done (i64.gt_s (local.get $i) (i64.const 10)))
            (local.set $acc (i64.add (local.get $acc) (local.get $i)))
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (br $cont)))
        (local.get $acc)
    "#;
    let out = run(&module_printing_i64(body), &[]);
    assert_eq!(out_u64(&out), 55);
}

#[test]
fn if_else() {
    let body = r#"
        (if (result i64) (i32.const 1)
          (then (i64.const 111))
          (else (i64.const 222)))
    "#;
    let out = run(&module_printing_i64(body), &[]);
    assert_eq!(out_u64(&out), 111);
}

#[test]
fn recursion_factorial() {
    // separate module: recursive factorial
    let wat = r#"(module
      (import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))
      (memory 1)
      (func $fact (param $n i64) (result i64)
        (if (result i64) (i64.le_s (local.get $n) (i64.const 1))
          (then (i64.const 1))
          (else (i64.mul (local.get $n) (call $fact (i64.sub (local.get $n) (i64.const 1)))))))
      (func (export "_start")
        (i64.store (i32.const 16) (call $fact (i64.const 5)))
        (i32.store (i32.const 0) (i32.const 16))
        (i32.store (i32.const 4) (i32.const 8))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#;
    let wasm = wat::parse_str(wat).unwrap();
    let out = run(&wasm, &[]);
    assert_eq!(out_u64(&out), 120);
}

#[test]
fn memory_load_store() {
    let body = r#"
        (i32.store (i32.const 100) (i32.const 0xdeadbeef))
        (i64.extend_i32_u (i32.load (i32.const 100)))
    "#;
    let out = run(&module_printing_i64(body), &[]);
    assert_eq!(out_u64(&out), 0xdeadbeef);
}

#[test]
fn reads_input() {
    // fd_read one byte from stdin and return it.
    let wat = r#"(module
      (import "wasi_snapshot_preview1" "fd_read"
        (func $fd_read (param i32 i32 i32 i32) (result i32)))
      (import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))
      (memory 1)
      (func (export "_start")
        ;; read iovec at 0: { buf=16, len=8 }
        (i32.store (i32.const 0) (i32.const 16))
        (i32.store (i32.const 4) (i32.const 8))
        (drop (call $fd_read (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 40)))
        ;; write the 8 bytes we read back out
        (i32.store (i32.const 0) (i32.const 16))
        (i32.store (i32.const 4) (i32.const 8))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 48)))))"#;
    let wasm = wat::parse_str(wat).unwrap();
    // Input blob: 8-byte length prefix (8) followed by the 8 data bytes.
    let mut input = Vec::new();
    input.extend_from_slice(&8u64.to_le_bytes());
    input.extend_from_slice(&1234u64.to_le_bytes());
    let out = run(&wasm, &input);
    assert_eq!(out_u64(&out), 1234);
}
