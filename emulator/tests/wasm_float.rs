//! f32/f64 lowering tests (require the `float` feature: the soft-float library is linked into
//! the ROM and every float operator dispatches to it).  Same output convention as `wasm.rs`.
#![cfg(feature = "float")]

use zisk_common::EmuTrace;
use zisk_transpiler_wasm::wasm2rom;
use ziskemu::{Emu, EmuOptions, ZiskEmulator};

/// Wraps a body that leaves one i64 on the stack into a module printing its 8 bytes.
fn module_printing_i64(body: &str) -> Vec<u8> {
    let wat = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory 1)
          (func $compute (result i64) {body})
          (func (export "_start")
            (i64.store (i32.const 16) (call $compute))
            (i32.store (i32.const 0) (i32.const 16))
            (i32.store (i32.const 4) (i32.const 8))
            (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
    );
    wat::parse_str(&wat).expect("wat compile")
}

fn eval_i64(body: &str) -> u64 {
    let rom = wasm2rom(&module_printing_i64(body)).expect("wasm2rom");
    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation");
    u64::from_le_bytes(out[0..8].try_into().unwrap())
}

/// Evaluates an f64-valued body and returns the resulting double.
fn eval_f64(body: &str) -> f64 {
    f64::from_bits(eval_i64(&format!("(i64.reinterpret_f64 {body})")))
}

/// Evaluates an f32-valued body and returns the resulting single.
fn eval_f32(body: &str) -> f32 {
    f32::from_bits(eval_i64(&format!("(i64.extend_i32_u (i32.reinterpret_f32 {body}))")) as u32)
}

/// Runs the module and reports whether the guest trapped (halted with the emulator's error flag
/// set) instead of running to completion.
fn traps(body: &str) -> bool {
    let rom = wasm2rom(&module_printing_i64(body)).expect("wasm2rom");
    let mut emu = Emu::new(&rom);
    emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
    assert!(emu.terminated(), "emulation did not reach an end instruction");
    emu.ctx.inst_ctx.error
}

#[test]
fn f64_arithmetic() {
    assert_eq!(eval_f64("(f64.add (f64.const 1.5) (f64.const 2.25))"), 3.75);
    assert_eq!(eval_f64("(f64.sub (f64.const 1.0) (f64.const 3.0))"), -2.0);
    assert_eq!(eval_f64("(f64.mul (f64.const 1e100) (f64.const 1e100))"), 1e200);
    assert_eq!(eval_f64("(f64.div (f64.const 1.0) (f64.const 3.0))"), 1.0 / 3.0);
    assert_eq!(eval_f64("(f64.sqrt (f64.const 2.0))"), 2f64.sqrt());
    // Round-to-nearest-even is observable: 1 + 2^-53 rounds back to 1.
    assert_eq!(eval_f64("(f64.add (f64.const 1.0) (f64.const 0x1p-53))"), 1.0);
    assert!(eval_f64("(f64.div (f64.const 0.0) (f64.const 0.0))").is_nan());
    assert_eq!(eval_f64("(f64.div (f64.const 1.0) (f64.const 0.0))"), f64::INFINITY);
}

#[test]
fn f32_arithmetic_and_precision() {
    assert_eq!(eval_f32("(f32.add (f32.const 0.1) (f32.const 0.2))"), 0.1f32 + 0.2f32);
    assert_eq!(eval_f32("(f32.mul (f32.const 3.0) (f32.const 7.0))"), 21.0);
    assert_eq!(eval_f32("(f32.div (f32.const 1.0) (f32.const 3.0))"), 1.0f32 / 3.0f32);
    assert_eq!(eval_f32("(f32.sqrt (f32.const 2.0))"), 2f32.sqrt());
    // f32 rounding differs from f64 rounding: 16777217 is not representable.
    assert_eq!(eval_f32("(f32.add (f32.const 16777216.0) (f32.const 1.0))"), 16777216.0);
}

#[test]
fn sign_operations() {
    assert_eq!(eval_f64("(f64.neg (f64.const 2.5))"), -2.5);
    assert_eq!(eval_f64("(f64.abs (f64.const -2.5))"), 2.5);
    assert_eq!(eval_f64("(f64.copysign (f64.const 2.5) (f64.const -0.0))"), -2.5);
    assert_eq!(eval_f32("(f32.neg (f32.const 2.5))"), -2.5);
    assert_eq!(eval_f32("(f32.copysign (f32.const -2.5) (f32.const 1.0))"), 2.5);
    assert!(eval_f64("(f64.neg (f64.const 0.0))").is_sign_negative());
}

#[test]
fn min_max_nan_and_signed_zero() {
    assert_eq!(eval_f64("(f64.min (f64.const 1.0) (f64.const -2.0))"), -2.0);
    assert_eq!(eval_f64("(f64.max (f64.const 1.0) (f64.const -2.0))"), 1.0);
    assert!(eval_f64("(f64.min (f64.const 1.0) (f64.const nan))").is_nan());
    assert!(eval_f64("(f64.max (f64.const nan) (f64.const 1.0))").is_nan());
    assert!(eval_f64("(f64.min (f64.const 0.0) (f64.const -0.0))").is_sign_negative());
    assert!(eval_f64("(f64.max (f64.const -0.0) (f64.const 0.0))").is_sign_positive());
    assert_eq!(eval_f32("(f32.min (f32.const 1.0) (f32.const -2.0))"), -2.0);
}

#[test]
fn rounding_to_integral() {
    assert_eq!(eval_f64("(f64.ceil (f64.const 2.1))"), 3.0);
    assert_eq!(eval_f64("(f64.floor (f64.const -2.1))"), -3.0);
    assert_eq!(eval_f64("(f64.trunc (f64.const -2.9))"), -2.0);
    assert_eq!(eval_f64("(f64.nearest (f64.const 2.5))"), 2.0); // ties to even
    assert_eq!(eval_f64("(f64.nearest (f64.const 3.5))"), 4.0);
    assert!(eval_f64("(f64.ceil (f64.const -0.5))").is_sign_negative()); // -0.0
    assert_eq!(eval_f64("(f64.ceil (f64.const -0.5))"), 0.0);
    // Already-integral / huge / non-finite values pass through.
    assert_eq!(eval_f64("(f64.floor (f64.const 0x1p60))"), 2f64.powi(60));
    assert_eq!(eval_f64("(f64.trunc (f64.const inf))"), f64::INFINITY);
    assert!(eval_f64("(f64.nearest (f64.const nan))").is_nan());
    assert_eq!(eval_f32("(f32.ceil (f32.const 2.1))"), 3.0);
    assert_eq!(eval_f32("(f32.floor (f32.const -0.5))"), -1.0);
}

#[test]
fn comparisons() {
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.lt (f64.const 1.0) (f64.const 2.0)))"), 1);
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.gt (f64.const 1.0) (f64.const 2.0)))"), 0);
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.le (f64.const 2.0) (f64.const 2.0)))"), 1);
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.ge (f64.const 1.0) (f64.const 2.0)))"), 0);
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.eq (f64.const -0.0) (f64.const 0.0)))"), 1);
    // NaN is unordered: every comparison but `ne` is false.
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.eq (f64.const nan) (f64.const nan)))"), 0);
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.ne (f64.const nan) (f64.const nan)))"), 1);
    assert_eq!(eval_i64("(i64.extend_i32_u (f64.lt (f64.const nan) (f64.const 1.0)))"), 0);
    assert_eq!(eval_i64("(i64.extend_i32_u (f32.lt (f32.const 1.0) (f32.const 2.0)))"), 1);
    assert_eq!(eval_i64("(i64.extend_i32_u (f32.ne (f32.const 1.0) (f32.const 1.0)))"), 0);
}

#[test]
fn int_to_float_conversions() {
    assert_eq!(eval_f64("(f64.convert_i64_s (i64.const -3))"), -3.0);
    assert_eq!(eval_f64("(f64.convert_i64_u (i64.const -1))"), 18446744073709551615.0);
    assert_eq!(eval_f64("(f64.convert_i32_s (i32.const -3))"), -3.0);
    assert_eq!(eval_f64("(f64.convert_i32_u (i32.const -1))"), 4294967295.0);
    assert_eq!(eval_f32("(f32.convert_i64_s (i64.const 16777217))"), 16777216.0);
    assert_eq!(eval_f32("(f32.convert_i32_u (i32.const -1))"), 4294967296.0);
}

#[test]
fn float_to_int_saturating() {
    assert_eq!(eval_i64("(i64.trunc_sat_f64_s (f64.const -3.9))") as i64, -3);
    assert_eq!(eval_i64("(i64.trunc_sat_f64_u (f64.const 3.9))"), 3);
    assert_eq!(eval_i64("(i64.trunc_sat_f64_s (f64.const nan))"), 0);
    assert_eq!(eval_i64("(i64.trunc_sat_f64_s (f64.const 1e30))") as i64, i64::MAX);
    assert_eq!(eval_i64("(i64.trunc_sat_f64_s (f64.const -1e30))") as i64, i64::MIN);
    assert_eq!(eval_i64("(i64.trunc_sat_f64_u (f64.const -1.0))"), 0);
    assert_eq!(eval_i64("(i64.trunc_sat_f64_u (f64.const 1e30))"), u64::MAX);
    // i32 results come back in the canonical sign-extended form.
    assert_eq!(eval_i64("(i64.extend_i32_s (i32.trunc_sat_f64_s (f64.const -7.5)))") as i64, -7);
    assert_eq!(
        eval_i64("(i64.extend_i32_s (i32.trunc_sat_f64_s (f64.const 1e10)))") as i64,
        i32::MAX as i64
    );
    assert_eq!(
        eval_i64("(i64.extend_i32_u (i32.trunc_sat_f64_u (f64.const 4294967295.5)))"),
        u32::MAX as u64
    );
    assert_eq!(eval_i64("(i64.extend_i32_u (i32.trunc_sat_f32_u (f32.const 100.5)))"), 100);
    assert_eq!(eval_i64("(i64.trunc_sat_f32_s (f32.const -100.5))") as i64, -100);
}

#[test]
fn float_to_int_trapping() {
    assert_eq!(eval_i64("(i64.trunc_f64_s (f64.const -3.9))") as i64, -3);
    assert_eq!(
        eval_i64("(i64.extend_i32_s (i32.trunc_f64_s (f64.const -2147483648.9)))") as i64,
        i32::MIN as i64
    );
    assert_eq!(eval_i64("(i64.extend_i32_u (i32.trunc_f64_u (f64.const -0.9)))"), 0);
    assert_eq!(eval_i64("(i64.trunc_f64_u (f64.const 0x1p63))"), 1 << 63);
    assert_eq!(eval_i64("(i64.trunc_f32_s (f32.const -0x1p63))") as i64, i64::MIN);
    for body in [
        "(i64.trunc_f64_s (f64.const nan))",
        "(i64.trunc_f64_s (f64.const 0x1p63))",
        "(i64.trunc_f64_u (f64.const -1.0))",
        "(i64.extend_i32_s (i32.trunc_f64_s (f64.const -2147483649.0)))",
        "(i64.extend_i32_u (i32.trunc_f64_u (f64.const 4294967296.0)))",
        "(i64.trunc_f32_s (f32.const 0x1p63))",
    ] {
        assert!(traps(body), "expected a trap for {body}");
    }
}

#[test]
fn promote_demote_and_reinterpret() {
    assert_eq!(eval_f64("(f64.promote_f32 (f32.const 0.1))"), 0.1f32 as f64);
    assert_eq!(eval_f32("(f32.demote_f64 (f64.const 0.1))"), 0.1f64 as f32);
    assert_eq!(eval_f32("(f32.demote_f64 (f64.const 1e300))"), f32::INFINITY);
    assert_eq!(eval_f64("(f64.reinterpret_i64 (i64.const 0x4000000000000000))"), 2.0);
    // i32.reinterpret_f32 of a negative float yields a negative (sign-extended) i32.
    assert_eq!(
        eval_i64("(i64.extend_i32_s (i32.reinterpret_f32 (f32.const -1.0)))") as i64,
        (-1.0f32).to_bits() as i32 as i64
    );
}

#[test]
fn float_memory_locals_and_globals() {
    let body = r#"
        (local $x f64) (local $y f32)
        (f64.store (i32.const 128) (f64.const 2.5))
        (f32.store (i32.const 136) (f32.const 0.5))
        (local.set $x (f64.load (i32.const 128)))
        (local.set $y (f32.load (i32.const 136)))
        (global.set $g (f64.mul (local.get $x) (f64.promote_f32 (local.get $y))))
        (i64.reinterpret_f64 (global.get $g))"#;
    let wat = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory 1)
          (global $g (mut f64) (f64.const 0))
          (func $compute (result i64) {body})
          (func (export "_start")
            (i64.store (i32.const 16) (call $compute))
            (i32.store (i32.const 0) (i32.const 16))
            (i32.store (i32.const 4) (i32.const 8))
            (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
    );
    let rom = wasm2rom(&wat::parse_str(&wat).unwrap()).expect("wasm2rom");
    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation");
    assert_eq!(f64::from_bits(u64::from_le_bytes(out[0..8].try_into().unwrap())), 1.25);
}

#[test]
fn float_params_and_results_across_calls() {
    let wat = r#"(module
      (import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))
      (memory 1)
      (func $hyp (param $a f64) (param $b f64) (result f64)
        (f64.sqrt (f64.add (f64.mul (local.get $a) (local.get $a))
                           (f64.mul (local.get $b) (local.get $b)))))
      (func (export "_start")
        (i64.store (i32.const 16) (i64.trunc_f64_s (call $hyp (f64.const 3.0) (f64.const 4.0))))
        (i32.store (i32.const 0) (i32.const 16))
        (i32.store (i32.const 4) (i32.const 8))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#;
    let rom = wasm2rom(&wat::parse_str(wat).unwrap()).expect("wasm2rom");
    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation");
    assert_eq!(u64::from_le_bytes(out[0..8].try_into().unwrap()), 5);
}
