//! End-to-end tests for the wasm32 → Zisk machine: compile small `.wat` modules, transpile them
//! with `wasm2rom`, run them on the emulator, and check observable output.
//!
//! Test convention: a guest computes an `i32`/`i64` and writes its 8 little-endian bytes to wasm
//! linear memory, then calls the imported `wasi_snapshot_preview1::fd_write` with a single iovec
//! pointing at those bytes.  The WASI layer mirrors stdout into the public output region, so the
//! result is readable via the emulator's output bytes.

use zisk_common::EmuTrace;
use zisk_transpiler_wasm::wasm2rom;
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
    let out = run(&module_printing_i64("(i64.extend_i32_s (i32.const -1))"), &[]);
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

#[test]
fn memory_copy_overlapping_forward_and_backward() {
    // Fill 0..16 with 0..15, then shift up by 3 (dst > src: must copy backward) and read the
    // 8 bytes at 3, then shift down by 5 (dst < src) and read the 8 bytes at 0.
    let body = |shift: &str, at: u32| {
        format!(
            r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory 1)
          (func (export "_start")
            (local $i i32)
            (loop $l
              (i32.store8 (i32.add (i32.const 64) (local.get $i)) (local.get $i))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br_if $l (i32.lt_u (local.get $i) (i32.const 16))))
            {shift}
            (i32.store (i32.const 0) (i32.const {at}))
            (i32.store (i32.const 4) (i32.const 8))
            (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 48)))))"#
        )
    };
    let up = body("(memory.copy (i32.const 67) (i32.const 64) (i32.const 13))", 67);
    let out = run(&wat::parse_str(up).unwrap(), &[]);
    assert_eq!(out_u64(&out), u64::from_le_bytes([0, 1, 2, 3, 4, 5, 6, 7]));
    let down = body("(memory.copy (i32.const 64) (i32.const 69) (i32.const 11))", 64);
    let out = run(&wat::parse_str(down).unwrap(), &[]);
    assert_eq!(out_u64(&out), u64::from_le_bytes([5, 6, 7, 8, 9, 10, 11, 12]));
}

#[test]
fn branch_to_function_body_returns() {
    // `br 1` from inside a block targets the function itself (Go emits this shape).
    let body = r#"(block (br 1 (i64.const 77))) (i64.const 1)"#;
    let out = run(&module_printing_i64(body), &[]);
    assert_eq!(out_u64(&out), 77);
}

#[test]
fn data_segments_must_not_overlap() {
    // Touching segments are fine and both land in memory ...
    let touching = r#"(module
      (import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))
      (memory 1)
      (data (i32.const 64) "\01\02\03\04")
      (data (i32.const 68) "\05\06\07\08")
      (func (export "_start")
        (i32.store (i32.const 0) (i32.const 64))
        (i32.store (i32.const 4) (i32.const 8))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 48)))))"#;
    let out = run(&wat::parse_str(touching).unwrap(), &[]);
    assert_eq!(out_u64(&out), u64::from_le_bytes([1, 2, 3, 4, 5, 6, 7, 8]));

    // ... but a shared byte is rejected at transpile time (the spec's "last wins" is not
    // implemented), regardless of segment order.
    let overlapping = r#"(module
      (memory 1)
      (data (i32.const 68) "\05\06\07\08")
      (data (i32.const 64) "\01\02\03\04\ff")
      (func (export "_start")))"#;
    let err = wasm2rom(&wat::parse_str(overlapping).unwrap()).expect_err("must be rejected");
    assert!(err.to_string().contains("overlapping data segments"), "{err}");
}

#[test]
fn memory_must_fit_the_machine() {
    use zisk_transpiler_wasm::layout::WASM_MAX_PAGES;

    // The largest supported initial memory is accepted and reported by memory.size ...
    let max = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory {WASM_MAX_PAGES})
          (func (export "_start")
            (i64.store (i32.const 16) (i64.extend_i32_u (memory.size)))
            (i32.store (i32.const 0) (i32.const 16))
            (i32.store (i32.const 4) (i32.const 8))
            (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
    );
    let out = run(&wat::parse_str(max).unwrap(), &[]);
    assert_eq!(out_u64(&out), WASM_MAX_PAGES);

    // ... one page more is a valid module this machine cannot host.
    let too_big = format!(r#"(module (memory {}) (func (export "_start")))"#, WASM_MAX_PAGES + 1);
    let err = wasm2rom(&wat::parse_str(too_big).unwrap()).expect_err("must be rejected");
    assert!(err.to_string().contains("initial memory"), "{err}");

    // A data segment past the end of the initial memory is rejected too (instantiation would
    // trap), even by a single byte.
    let past_end = r#"(module
      (memory 1)
      (data (i32.const 65533) "\01\02\03\04")
      (func (export "_start")))"#;
    let err = wasm2rom(&wat::parse_str(past_end).unwrap()).expect_err("must be rejected");
    assert!(err.to_string().contains("does not fit the initial memory"), "{err}");
    // A negative i32 offset is the unsigned memory32 address 0xffffffff, not a 64-bit value.
    let negative = r#"(module
      (memory 1)
      (data (i32.const -1) "\01")
      (func (export "_start")))"#;
    let err = wasm2rom(&wat::parse_str(negative).unwrap()).expect_err("must be rejected");
    assert!(err.to_string().contains("offset 0xffffffff"), "{err}");
}

#[test]
fn globals_and_table_must_fit_the_machine() {
    use zisk_transpiler_wasm::layout::{WASM_MAX_GLOBALS, WASM_MAX_TABLE_ENTRIES};

    // Exactly the capacity is fine; the last global is readable at its own address.
    let globals = |n: u64| {
        let decls: String = (0..n).map(|i| format!("(global i64 (i64.const {i}))")).collect();
        let last = n - 1;
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (memory 1)
              {decls}
              (func (export "_start")
                (i64.store (i32.const 16) (global.get {last}))
                (i32.store (i32.const 0) (i32.const 16))
                (i32.store (i32.const 4) (i32.const 8))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        wat::parse_str(wat).unwrap()
    };
    assert_eq!(out_u64(&run(&globals(WASM_MAX_GLOBALS), &[])), WASM_MAX_GLOBALS - 1);
    let err = wasm2rom(&globals(WASM_MAX_GLOBALS + 1)).expect_err("must be rejected");
    assert!(err.to_string().contains("globals exceed"), "{err}");

    // A table larger than the reserved area is rejected ...
    let big_table = format!(
        r#"(module (table {} funcref) (func (export "_start")))"#,
        WASM_MAX_TABLE_ENTRIES + 1
    );
    let err = wasm2rom(&wat::parse_str(big_table).unwrap()).expect_err("must be rejected");
    assert!(err.to_string().contains("table of"), "{err}");

    // ... and so is an element segment that does not fit the declared table.
    let past_end = r#"(module
      (table 2 funcref)
      (elem (i32.const 1) $f $f)
      (func $f)
      (func (export "_start")))"#;
    let err = wasm2rom(&wat::parse_str(past_end).unwrap()).expect_err("must be rejected");
    assert!(err.to_string().contains("element segment"), "{err}");
}

#[test]
fn call_indirect_traps_out_of_bounds_and_null() {
    // Table of 4 entries, only entry 1 initialized.  Index 1 works; 3 (null) and 4 (out of
    // bounds) trap, i.e. halt with the emulator's error flag set.
    let module = |index: u32| {
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (type $sig (func (result i64)))
              (table 4 funcref)
              (elem (i32.const 1) $f)
              (memory 1)
              (func $f (result i64) (i64.const 99))
              (func (export "_start")
                (i64.store (i32.const 16) (call_indirect (type $sig) (i32.const {index})))
                (i32.store (i32.const 0) (i32.const 16))
                (i32.store (i32.const 4) (i32.const 8))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        wasm2rom(&wat::parse_str(wat).unwrap()).expect("wasm2rom")
    };
    let run_flag = |index: u32| {
        let rom = module(index);
        let mut emu = ziskemu::Emu::new(&rom);
        emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
        assert!(emu.terminated(), "index {index}: emulation did not terminate");
        (emu.ctx.inst_ctx.error, u64::from_le_bytes(emu.get_output_8()[0..8].try_into().unwrap()))
    };
    assert_eq!(run_flag(1), (false, 99));
    assert!(run_flag(3).0, "null entry must trap");
    assert!(run_flag(4).0, "out-of-bounds index must trap");
}

#[test]
fn linear_memory_accesses_are_bounds_checked() {
    // One page of memory: 65536 bytes.  Each body runs in `_start`; a trap halts with the
    // emulator's error flag set, otherwise the i64 at address 8 is printed.
    let outcome = |body: &str| -> (bool, u64) {
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (memory 1)
              (func (export "_start")
                {body}
                (i32.store (i32.const 0) (i32.const 8))
                (i32.store (i32.const 4) (i32.const 8))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        let rom = wasm2rom(&wat::parse_str(wat).unwrap()).expect("wasm2rom");
        let mut emu = ziskemu::Emu::new(&rom);
        emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
        assert!(emu.terminated(), "emulation did not terminate for {body}");
        (emu.ctx.inst_ctx.error, u64::from_le_bytes(emu.get_output_8()[0..8].try_into().unwrap()))
    };
    // Last addressable word is fine; one byte further, or a static offset that pushes past the
    // end, traps.  Same for stores.
    assert_eq!(outcome("(i64.store (i32.const 8) (i64.load (i32.const 65528)))"), (false, 0));
    assert!(outcome("(drop (i64.load (i32.const 65529)))").0);
    assert!(outcome("(drop (i32.load offset=65533 (i32.const 0)))").0);
    assert!(outcome("(i32.store (i32.const 65533) (i32.const 1))").0);
    assert!(outcome("(i64.store8 (i32.const 65536) (i64.const 1))").0);
    // An address that wraps when the static offset is added is out of bounds, not aliased.
    assert!(outcome("(drop (i32.load8_u offset=1 (i32.const 0xffffffff)))").0);
    // Bulk operations are checked up front, on both operands.
    assert!(outcome("(memory.fill (i32.const 65530) (i32.const 7) (i32.const 7))").0);
    assert!(outcome("(memory.copy (i32.const 0) (i32.const 65530) (i32.const 7))").0);
    assert!(outcome("(memory.copy (i32.const 65530) (i32.const 0) (i32.const 7))").0);
    assert_eq!(
        outcome("(memory.fill (i32.const 8) (i32.const 0x55) (i32.const 8))"),
        (false, 0x5555_5555_5555_5555)
    );
    // Growing the memory moves the bound: the same access succeeds afterwards.
    assert_eq!(
        outcome(
            "(drop (memory.grow (i32.const 1)))
             (i64.store (i32.const 65536) (i64.const 42))
             (i64.store (i32.const 8) (i64.load (i32.const 65536)))"
        ),
        (false, 42)
    );
    assert!(outcome("(drop (memory.grow (i32.const 1))) (drop (i64.load (i32.const 131072)))").0);
}

#[test]
fn deep_recursion_traps_instead_of_overflowing_the_stack() {
    // `$sum n` recurses n deep; `$forever` never returns, directly or through the table.
    let module = |body: &str| {
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (type $sig (func))
              (table 1 funcref)
              (elem (i32.const 0) $forever_indirect)
              (memory 1)
              (func $sum (param $n i64) (result i64)
                (if (result i64) (i64.eqz (local.get $n))
                  (then (i64.const 0))
                  (else (i64.add (local.get $n)
                                 (call $sum (i64.sub (local.get $n) (i64.const 1)))))))
              (func $forever (param $n i64) (result i64) (call $forever (local.get $n)))
              (func $forever_indirect (call_indirect (type $sig) (i32.const 0)))
              (func (export "_start")
                {body}
                (i32.store (i32.const 0) (i32.const 8))
                (i32.store (i32.const 4) (i32.const 8))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        let rom = wasm2rom(&wat::parse_str(wat).unwrap()).expect("wasm2rom");
        let mut emu = ziskemu::Emu::new(&rom);
        emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
        assert!(emu.terminated(), "emulation did not terminate for {body}");
        (emu.ctx.inst_ctx.error, u64::from_le_bytes(emu.get_output_8()[0..8].try_into().unwrap()))
    };
    // 1000 frames fit comfortably.
    assert_eq!(module("(i64.store (i32.const 8) (call $sum (i64.const 1000)))"), (false, 500500));
    // Unbounded recursion halts with the error flag rather than running into linear memory.
    assert!(module("(drop (call $forever (i64.const 0)))").0);
    assert!(module("(call $forever_indirect)").0);
}

#[test]
fn malformed_modules_are_rejected_not_panicked() {
    // Decodable but invalid modules must come back as errors from `wasm2rom` (the lowering indexes
    // and pops with the guarantees validation gives it).
    for (what, wat) in [
        ("stack underflow", r#"(module (func (export "_start") (drop (i64.add (i64.const 1)))))"#),
        (
            "immutable global write",
            r#"(module (global $g i64 (i64.const 0))
                       (func (export "_start") (global.set $g (i64.const 1))))"#,
        ),
        ("undefined function", r#"(module (func (export "_start") (call 7)))"#),
        (
            "result type mismatch",
            r#"(module (func $f (result i64) (i32.const 1)) (func (export "_start")))"#,
        ),
        ("undefined local", r#"(module (func (export "_start") (drop (local.get 3))))"#),
    ] {
        let bytes = wat::parse_str(wat).unwrap_or_else(|e| panic!("{what}: wat encoding: {e}"));
        assert!(wasm2rom(&bytes).is_err(), "{what}: must be rejected");
    }
}

#[test]
fn wasi_stubs_reject_out_of_range_guest_pointers() {
    // One page of memory.  `{call}` yields an errno, stored at 8 and printed through a valid
    // fd_write; EFAULT (21) must come back for any range outside the guest's memory, and nothing
    // must have been written in that case.
    let errno = |call: &str| -> u64 {
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (import "wasi_snapshot_preview1" "fd_read"
                (func $fd_read (param i32 i32 i32 i32) (result i32)))
              (import "wasi_snapshot_preview1" "random_get"
                (func $random_get (param i32 i32) (result i32)))
              (import "wasi_snapshot_preview1" "args_sizes_get"
                (func $args_sizes_get (param i32 i32) (result i32)))
              (memory 1)
              ;; iovec at 64: {{ buf, len }}, filled by the call expression as needed
              (func (export "_start")
                (i64.store (i32.const 8) (i64.extend_i32_u {call}))
                (i32.store (i32.const 0) (i32.const 8))
                (i32.store (i32.const 4) (i32.const 8))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        let mut input = Vec::new();
        input.extend_from_slice(&8u64.to_le_bytes());
        input.extend_from_slice(&[1u8; 8]);
        out_u64(&run(&wat::parse_str(wat).unwrap(), &input))
    };
    let iovec = |buf: u32, len: u32| {
        format!("(i32.store (i32.const 64) (i32.const {buf})) (i32.store (i32.const 68) (i32.const {len}))")
    };
    // Valid: an in-range buffer, EOF-less read into it, and the sizes probe.
    assert_eq!(errno(&format!("(block (result i32) {} (call $fd_write (i32.const 1) (i32.const 64) (i32.const 1) (i32.const 72)))", iovec(128, 8))), 0);
    assert_eq!(errno(&format!("(block (result i32) {} (call $fd_read (i32.const 0) (i32.const 64) (i32.const 1) (i32.const 72)))", iovec(128, 8))), 0);
    assert_eq!(errno("(call $args_sizes_get (i32.const 128) (i32.const 132))"), 0);
    // The iovec array itself past the end (8 bytes at 65532).
    assert_eq!(
        errno("(call $fd_write (i32.const 1) (i32.const 65532) (i32.const 1) (i32.const 72))"),
        21
    );
    // A buffer crossing the end, a buffer wrapping the address space, and a bad nwritten pointer.
    assert_eq!(errno(&format!("(block (result i32) {} (call $fd_write (i32.const 1) (i32.const 64) (i32.const 1) (i32.const 72)))", iovec(65530, 16))), 21);
    assert_eq!(errno(&format!("(block (result i32) {} (call $fd_write (i32.const 1) (i32.const 64) (i32.const 1) (i32.const 72)))", iovec(0xffff_ffff, 1))), 21);
    assert_eq!(errno(&format!("(block (result i32) {} (call $fd_write (i32.const 1) (i32.const 64) (i32.const 1) (i32.const 65533)))", iovec(128, 8))), 21);
    // Same for reads and the other writers.
    assert_eq!(errno(&format!("(block (result i32) {} (call $fd_read (i32.const 0) (i32.const 64) (i32.const 1) (i32.const 72)))", iovec(65530, 16))), 21);
    assert_eq!(errno("(call $random_get (i32.const 65535) (i32.const 2))"), 21);
    assert_eq!(errno("(call $args_sizes_get (i32.const 128) (i32.const 65533))"), 21);
    // A faulting read must not have consumed input: the next valid read still sees the 8 bytes.
    assert_eq!(
        errno(&format!(
            "(block (result i32) {} \
               (drop (call $fd_read (i32.const 0) (i32.const 64) (i32.const 1) (i32.const 72))) \
               {} \
               (drop (call $fd_read (i32.const 0) (i32.const 64) (i32.const 1) (i32.const 72))) \
               (i32.load (i32.const 72)))",
            iovec(65530, 16),
            iovec(128, 8)
        )),
        8
    );
}

#[test]
fn random_get_is_a_seeded_splitmix64_stream() {
    // Reference splitmix64 over the machine's fixed seed.
    fn splitmix64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    let mut state = zisk_transpiler_wasm::wasi::RNG_SEED;
    let module = |calls: &str, len: u32| {
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (import "wasi_snapshot_preview1" "random_get"
                (func $random_get (param i32 i32) (result i32)))
              (memory 1)
              (func (export "_start")
                {calls}
                (i32.store (i32.const 0) (i32.const 128))
                (i32.store (i32.const 4) (i32.const {len}))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        run(&wat::parse_str(wat).unwrap(), &[])
    };
    // Two whole-word calls continue the same stream.
    let out = module(
        "(drop (call $random_get (i32.const 128) (i32.const 8)))
         (drop (call $random_get (i32.const 136) (i32.const 8)))",
        16,
    );
    let (first, second) = (splitmix64(&mut state), splitmix64(&mut state));
    assert_eq!(u64::from_le_bytes(out[0..8].try_into().unwrap()), first);
    assert_eq!(u64::from_le_bytes(out[8..16].try_into().unwrap()), second);
    assert_ne!(first, 0, "the stream must not be the old zero fill");
    // A short request takes one output per byte and leaves the rest untouched.
    let out = module("(drop (call $random_get (i32.const 128) (i32.const 3)))", 8);
    let mut state = zisk_transpiler_wasm::wasi::RNG_SEED;
    let mut byte = || splitmix64(&mut state) as u8;
    let expected = [byte(), byte(), byte(), 0, 0, 0, 0, 0];
    assert_eq!(&out[0..8], &expected);
}

#[test]
fn fd_read_and_fd_write_reject_unknown_descriptors() {
    // The errno of `{call}` is stored at 8 and printed through a valid fd_write on stdout; the
    // 8 bytes at 128 (the only buffer the calls touch) follow, so a rejected call must also have
    // left them alone.
    let outcome = |call: &str| -> (u64, u64) {
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (import "wasi_snapshot_preview1" "fd_read"
                (func $fd_read (param i32 i32 i32 i32) (result i32)))
              (memory 1)
              (func (export "_start")
                (i32.store (i32.const 64) (i32.const 128))
                (i32.store (i32.const 68) (i32.const 8))
                (i64.store (i32.const 8) (i64.extend_i32_u {call}))
                (i64.store (i32.const 16) (i64.load (i32.const 128)))
                (i32.store (i32.const 0) (i32.const 8))
                (i32.store (i32.const 4) (i32.const 16))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        let mut input = Vec::new();
        input.extend_from_slice(&8u64.to_le_bytes());
        input.extend_from_slice(&0x1111_1111_1111_1111u64.to_le_bytes());
        let out = run(&wat::parse_str(wat).unwrap(), &input);
        (out_u64(&out), u64::from_le_bytes(out[8..16].try_into().unwrap()))
    };
    // stdin reads, stdout/stderr writes.
    assert_eq!(
        outcome("(call $fd_read (i32.const 0) (i32.const 64) (i32.const 1) (i32.const 72))"),
        (0, 0x1111_1111_1111_1111)
    );
    assert_eq!(
        outcome("(call $fd_write (i32.const 1) (i32.const 64) (i32.const 1) (i32.const 72))").0,
        0
    );
    assert_eq!(
        outcome("(call $fd_write (i32.const 2) (i32.const 64) (i32.const 1) (i32.const 72))").0,
        0
    );
    // Anything else is not an open descriptor: EBADF, and the buffer is untouched.
    for call in [
        "(call $fd_read (i32.const 1) (i32.const 64) (i32.const 1) (i32.const 72))",
        "(call $fd_read (i32.const 3) (i32.const 64) (i32.const 1) (i32.const 72))",
        "(call $fd_write (i32.const 0) (i32.const 64) (i32.const 1) (i32.const 72))",
        "(call $fd_write (i32.const 3) (i32.const 64) (i32.const 1) (i32.const 72))",
        "(call $fd_write (i32.const -1) (i32.const 64) (i32.const 1) (i32.const 72))",
    ] {
        assert_eq!(outcome(call), (8, 0), "{call}");
    }
}

#[test]
fn wasi_imports_must_have_the_expected_signature() {
    // The stubs read fixed argument slots: an import declared with another type is refused.
    for (what, wat) in [
        (
            "fd_write with one parameter",
            r#"(module (import "wasi_snapshot_preview1" "fd_write" (func (param i32) (result i32)))
                       (func (export "_start")))"#,
        ),
        (
            "fd_write without a result",
            r#"(module (import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32)))
                       (func (export "_start")))"#,
        ),
        (
            "proc_exit with an i64 status",
            r#"(module (import "wasi_snapshot_preview1" "proc_exit" (func (param i64)))
                       (func (export "_start")))"#,
        ),
        (
            "clock_time_get with an i32 precision",
            r#"(module (import "wasi_snapshot_preview1" "clock_time_get"
                         (func (param i32 i32 i32) (result i32)))
                       (func (export "_start")))"#,
        ),
    ] {
        let err = wasm2rom(&wat::parse_str(wat).unwrap()).expect_err(what);
        assert!(err.to_string().contains("expected"), "{what}: {err}");
    }
}

#[test]
fn memory_grow_honors_the_declared_maximum() {
    // `(memory 1 2)`: one grow succeeds (returns the old size 1), the next must fail with -1 and
    // leave the size at 2; `(memory 1 1)` cannot grow at all.
    let grow_results = |memory: &str| -> (i64, i64, i64) {
        let wat = format!(
            r#"(module
              (import "wasi_snapshot_preview1" "fd_write"
                (func $fd_write (param i32 i32 i32 i32) (result i32)))
              (memory {memory})
              (func (export "_start")
                (i64.store (i32.const 8) (i64.extend_i32_s (memory.grow (i32.const 1))))
                (i64.store (i32.const 16) (i64.extend_i32_s (memory.grow (i32.const 1))))
                (i64.store (i32.const 24) (i64.extend_i32_s (memory.size)))
                (i32.store (i32.const 0) (i32.const 8))
                (i32.store (i32.const 4) (i32.const 24))
                (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
        );
        let out = run(&wat::parse_str(wat).unwrap(), &[]);
        let word = |i: usize| i64::from_le_bytes(out[8 * i..8 * i + 8].try_into().unwrap());
        (word(0), word(1), word(2))
    };
    assert_eq!(grow_results("1 2"), (1, -1, 2));
    assert_eq!(grow_results("1 1"), (-1, -1, 1));
    // Without a declared maximum the machine's own limit is the only bound.
    assert_eq!(grow_results("1"), (1, 2, 3));
}

#[test]
fn start_section_runs_before_the_command_entry() {
    // The start section initializes a global that `_start` then reads: if only one of them ran
    // (or they ran in the wrong order) the printed value would differ.
    let wat = r#"(module
      (import "wasi_snapshot_preview1" "fd_write"
        (func $fd_write (param i32 i32 i32 i32) (result i32)))
      (memory 1)
      (global $g (mut i64) (i64.const 1))
      (func $init (global.set $g (i64.mul (global.get $g) (i64.const 10))))
      (start $init)
      (func (export "_start")
        (i64.store (i32.const 16) (i64.add (global.get $g) (i64.const 5)))
        (i32.store (i32.const 0) (i32.const 16))
        (i32.store (i32.const 4) (i32.const 8))
        (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#;
    let out = run(&wat::parse_str(wat).unwrap(), &[]);
    assert_eq!(out_u64(&out), 15);
    // `_start` must be a WASI command entry: () -> ().
    let typed = r#"(module (func (export "_start") (param i32)))"#;
    let err = wasm2rom(&wat::parse_str(typed).unwrap()).expect_err("must be rejected");
    assert!(err.to_string().contains("() -> ()"), "{err}");
}

#[test]
fn stdout_mirror_stops_at_the_public_output_limit() {
    use zisk_core::OUTPUT_MAX_SIZE;
    use zisk_transpiler_wasm::layout::{WASM_GLOBALS_ADDR, WASM_MEM_BASE, WASM_MEM_PAGES_ADDR};
    // Write 200_000 bytes (> 128 KiB) of stdout in one call: the console takes it all and the
    // call reports the full count, but the public output stops at OUTPUT_MAX_SIZE, so the areas
    // that follow it (globals, control cells) are untouched.
    let len = OUTPUT_MAX_SIZE as u32 + 68_928;
    let wat = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory 4)
          (global $g (mut i64) (i64.const 0x4242))
          (func (export "_start")
            (memory.fill (i32.const 1024) (i32.const 0x61) (i32.const {len}))
            (i32.store (i32.const 0) (i32.const 1024))
            (i32.store (i32.const 4) (i32.const {len}))
            (i64.store (i32.const 16)
              (i64.extend_i32_u (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40))))
            (i64.store (i32.const 24) (i64.extend_i32_u (i32.load (i32.const 40))))))"#
    );
    let rom = wasm2rom(&wat::parse_str(wat).unwrap()).expect("wasm2rom");
    let mut emu = ziskemu::Emu::new(&rom);
    emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
    assert!(emu.terminated() && !emu.ctx.inst_ctx.error);
    let mem = &emu.ctx.inst_ctx.mem;
    assert_eq!(mem.read(WASM_MEM_BASE + 16, 8), 0, "errno");
    assert_eq!(mem.read(WASM_MEM_BASE + 24, 8), len as u64, "nwritten reports the full write");
    assert_eq!(mem.read(WASM_GLOBALS_ADDR, 8), 0x4242, "globals survive");
    assert_eq!(mem.read(WASM_MEM_PAGES_ADDR, 8), 4, "control cells survive");
    assert_eq!(emu.get_output_8()[..8], [0x61; 8], "the mirror holds the first bytes");
}
