//! Constraint verification for WebAssembly guests on the Emulator executor and, on Linux, the
//! Assembly executor.
//!
//! `wasm2rom` produces an ordinary `ZiskRom`, so a wasm guest goes through the same witness
//! computation and constraint checks as a RISC-V one. This test runs `verify_constraints` on a set
//! of guests that together cover the wasm lowering paths:
//!
//! - `wasm-fibonacci`: the minimal Rust example (`examples/wasm-fibonacci`).
//! - `wasm-coverage`: a Rust std guest built by stock rustc (`wasm-guests/coverage`), covering
//!   heap growth, bulk memory, trait-object calls, recursion and the std WASI runtime.
//! - `integer_ops` (WAT): every i32/i64 operator, sign extension, every load/store width at
//!   unaligned addresses, `select`, `br_table`, `call_indirect` over several signatures, globals,
//!   `memory.grow`/`memory.size`/`memory.fill`/`memory.copy`, and deep recursion.
//! - `wasi_surface` (WAT): every implemented WASI call, an unimplemented one (`ENOSYS`), a start
//!   section, chunked stdin reads to EOF, output past the 256-byte public limit, and `proc_exit(0)`.
//! - `memory_limits` (WAT): the edges of the wasm memory layout — a data segment at the top of a
//!   7,000-page initial memory, `memory.grow` to the 7,680-page maximum and past it, a store to the
//!   last byte of linear memory, and a 2,000-deep recursion that takes the call stack close to its
//!   lower limit (frames reserve 1,024 operand slots, so about 2,020 frames fit).
//!
//! Guests that trap (`unreachable`, an out-of-bounds load) or call `proc_exit` with a nonzero
//! status must be rejected. They halt through the `halt` op, which no valid main trace can
//! contain (for an internal op, main.pil forces `flag = 1` and `flag = 0` when `op = 0xFF`), so
//! both executors refuse them before any constraint is checked.
//!
//! Ignored by default because `verify_constraints` requires a generated proving key on disk.
//! Run with:
//!
//!     cargo test --release -p integration-tests --test wasm_verify_constraints -- --ignored --nocapture

use std::path::PathBuf;

use zisk_sdk::{
    EmbeddedClientBuilder, ExecutorKind, GuestProgram, VerifyConstraintsExtension,
    WitnessBuilderExt, ZiskStdin,
};

const WASM_FIBONACCI: &[u8] = include_bytes!("../../emulator/tests/data/wasm-fibonacci.wasm");
const WASM_COVERAGE: &[u8] = include_bytes!("data/wasm-coverage.wasm");

const INTEGER_OPS_WAT: &str = r#"(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory 1 4)
  (export "memory" (memory 0))

  (global $g64 (mut i64) (i64.const 7))
  (global $g32 (mut i32) (i32.const -3))

  (type $t_i64 (func (param i64) (result i64)))
  (type $t_i32 (func (param i32 i32) (result i32)))
  (table 4 funcref)
  (elem (i32.const 0) $ind_a $ind_b $ind_c $ind_d)

  (func $ind_a (param $x i64) (result i64) (i64.mul (local.get $x) (i64.const 31)))
  (func $ind_b (param $x i64) (result i64) (i64.xor (local.get $x) (i64.const 0x5555)))
  (func $ind_c (param $a i32) (param $b i32) (result i32) (i32.sub (local.get $a) (local.get $b)))
  (func $ind_d (param $a i32) (param $b i32) (result i32) (i32.mul (local.get $a) (local.get $b)))

  ;; acc = rotl(acc ^ x, 13) * K: folds every intermediate result into one checksum.
  (func $mix (param $acc i64) (param $x i64) (result i64)
    (i64.mul (i64.rotl (i64.xor (local.get $acc) (local.get $x)) (i64.const 13))
             (i64.const 0x9e3779b97f4a7c15)))
  (func $mix32 (param $acc i64) (param $x i32) (result i64)
    (call $mix (local.get $acc) (i64.extend_i32_s (local.get $x))))

  (func $depth (param $n i32) (result i32)
    (if (result i32) (i32.eqz (local.get $n))
      (then (i32.const 1))
      (else (i32.add (call $depth (i32.sub (local.get $n) (i32.const 1))) (i32.const 1)))))

  (func $i32_ops (param $acc i64) (param $a i32) (param $b i32) (result i64)
    (local.set $acc (call $mix32 (local.get $acc) (i32.add (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.sub (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.mul (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.div_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.div_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.rem_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.rem_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.and (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.or (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.xor (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.shl (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.shr_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.shr_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.rotl (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.rotr (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.clz (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.ctz (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.popcnt (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.eqz (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.eq (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.ne (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.lt_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.lt_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.gt_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.gt_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.le_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.le_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.ge_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.ge_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.extend8_s (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.extend16_s (local.get $a))))
    (local.set $acc (call $mix (local.get $acc) (i64.extend_i32_u (local.get $a))))
    (local.get $acc))

  (func $i64_ops (param $acc i64) (param $a i64) (param $b i64) (result i64)
    (local.set $acc (call $mix (local.get $acc) (i64.add (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.sub (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.mul (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.div_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.div_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.rem_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.rem_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.and (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.or (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.xor (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.shl (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.shr_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.shr_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.rotl (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.rotr (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.clz (local.get $a))))
    (local.set $acc (call $mix (local.get $acc) (i64.ctz (local.get $a))))
    (local.set $acc (call $mix (local.get $acc) (i64.popcnt (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.eqz (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.eq (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.ne (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.lt_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.lt_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.gt_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.gt_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.le_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.le_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.ge_s (local.get $a) (local.get $b))))
    (local.set $acc (call $mix32 (local.get $acc) (i64.ge_u (local.get $a) (local.get $b))))
    (local.set $acc (call $mix (local.get $acc) (i64.extend8_s (local.get $a))))
    (local.set $acc (call $mix (local.get $acc) (i64.extend16_s (local.get $a))))
    (local.set $acc (call $mix (local.get $acc) (i64.extend32_s (local.get $a))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.wrap_i64 (local.get $a))))
    (local.get $acc))

  ;; Stores $v at every width to an unaligned address and reads it back with every load.
  (func $mem_ops (param $acc i64) (param $addr i32) (param $v i64) (result i64)
    (i64.store (local.get $addr) (local.get $v))
    (local.set $acc (call $mix (local.get $acc) (i64.load (local.get $addr))))
    (i64.store32 offset=9 (local.get $addr) (local.get $v))
    (i64.store16 offset=17 (local.get $addr) (local.get $v))
    (i64.store8 offset=23 (local.get $addr) (local.get $v))
    (i32.store offset=29 (local.get $addr) (i32.wrap_i64 (local.get $v)))
    (i32.store16 offset=35 (local.get $addr) (i32.wrap_i64 (local.get $v)))
    (i32.store8 offset=39 (local.get $addr) (i32.wrap_i64 (local.get $v)))
    (local.set $acc (call $mix (local.get $acc) (i64.load32_s offset=9 (local.get $addr))))
    (local.set $acc (call $mix (local.get $acc) (i64.load32_u offset=9 (local.get $addr))))
    (local.set $acc (call $mix (local.get $acc) (i64.load16_s offset=17 (local.get $addr))))
    (local.set $acc (call $mix (local.get $acc) (i64.load16_u offset=17 (local.get $addr))))
    (local.set $acc (call $mix (local.get $acc) (i64.load8_s offset=23 (local.get $addr))))
    (local.set $acc (call $mix (local.get $acc) (i64.load8_u offset=23 (local.get $addr))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.load offset=29 (local.get $addr))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.load16_s offset=35 (local.get $addr))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.load16_u offset=35 (local.get $addr))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.load8_s offset=39 (local.get $addr))))
    (local.set $acc (call $mix32 (local.get $acc) (i32.load8_u offset=39 (local.get $addr))))
    (local.get $acc))

  (func (export "_start")
    (local $i i32) (local $acc i64) (local $a64 i64) (local $b64 i64)
    (local.set $acc (i64.const 1))
    (block $done
      (loop $next
        (br_if $done (i32.gt_u (local.get $i) (i32.const 64)))
        ;; Operands spanning signs and magnitudes; divisors are odd, so never zero.
        (local.set $a64 (i64.mul (i64.extend_i32_u (local.get $i)) (i64.const -0x0123456789abcdef)))
        (local.set $b64 (i64.or (i64.extend_i32_u (local.get $i)) (i64.const 1)))
        (local.set $acc (call $i64_ops (local.get $acc) (local.get $a64) (local.get $b64)))
        (local.set $acc (call $i32_ops (local.get $acc) (i32.wrap_i64 (local.get $a64))
                                       (i32.wrap_i64 (local.get $b64))))
        (local.set $acc (call $mem_ops (local.get $acc)
                                       (i32.add (i32.const 256) (i32.and (local.get $i) (i32.const 7)))
                                       (local.get $a64)))
        ;; select and br_table
        (local.set $acc (call $mix (local.get $acc)
          (select (local.get $a64) (local.get $b64) (i32.and (local.get $i) (i32.const 1)))))
        (block $b3 (block $b2 (block $b1 (block $b0
          (br_table $b0 $b1 $b2 $b3 (i32.and (local.get $i) (i32.const 3))))
          (local.set $acc (i64.add (local.get $acc) (i64.const 1))) (br $b3))
          (local.set $acc (i64.add (local.get $acc) (i64.const 2))) (br $b3))
          (local.set $acc (i64.add (local.get $acc) (i64.const 3))))
        ;; call_indirect over two signatures
        (local.set $acc (call $mix (local.get $acc)
          (call_indirect (type $t_i64) (local.get $a64) (i32.and (local.get $i) (i32.const 1)))))
        (local.set $acc (call $mix32 (local.get $acc)
          (call_indirect (type $t_i32) (local.get $i) (i32.const 3)
                         (i32.add (i32.const 2) (i32.and (local.get $i) (i32.const 1))))))
        ;; globals
        (global.set $g64 (i64.add (global.get $g64) (local.get $acc)))
        (global.set $g32 (i32.xor (global.get $g32) (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (local.set $acc (call $mix (local.get $acc) (global.get $g64)))
    (local.set $acc (call $mix32 (local.get $acc) (global.get $g32)))

    ;; memory.grow / memory.size, then touch the last bytes of the new page
    (local.set $acc (call $mix32 (local.get $acc) (memory.grow (i32.const 1))))
    (local.set $acc (call $mix32 (local.get $acc) (memory.size)))
    (local.set $acc (call $mix32 (local.get $acc) (memory.grow (i32.const 8))))
    (i64.store (i32.const 131064) (local.get $acc))
    (local.set $acc (call $mix (local.get $acc) (i64.load (i32.const 131064))))

    ;; memory.fill, then overlapping memory.copy forward and backward
    (memory.fill (i32.const 1024) (i32.const 0x5a) (i32.const 100))
    (i64.store (i32.const 1024) (local.get $acc))
    (memory.copy (i32.const 1027) (i32.const 1024) (i32.const 40))
    (memory.copy (i32.const 1020) (i32.const 1030) (i32.const 40))
    (local.set $acc (call $mix (local.get $acc) (i64.load (i32.const 1021))))
    (local.set $acc (call $mix (local.get $acc) (i64.load (i32.const 1060))))

    ;; deep recursion
    (local.set $acc (call $mix32 (local.get $acc) (call $depth (i32.const 2000))))

    ;; print the 8 checksum bytes
    (i64.store (i32.const 16) (local.get $acc))
    (i32.store (i32.const 0) (i32.const 16))
    (i32.store (i32.const 4) (i32.const 8))
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40))))
)"#;

const WASI_SURFACE_WAT: &str = r#"(module
  (import "wasi_snapshot_preview1" "args_sizes_get" (func $args_sizes_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "args_get" (func $args_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "environ_sizes_get" (func $environ_sizes_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "environ_get" (func $environ_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "clock_time_get" (func $clock_time_get (param i32 i64 i32) (result i32)))
  (import "wasi_snapshot_preview1" "random_get" (func $random_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_fdstat_get" (func $fd_fdstat_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_filestat_get" (func $fd_filestat_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_prestat_get" (func $fd_prestat_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_prestat_dir_name" (func $fd_prestat_dir_name (param i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_close" (func $fd_close (param i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_seek" (func $fd_seek (param i32 i64 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "sched_yield" (func $sched_yield (result i32)))
  (import "wasi_snapshot_preview1" "poll_oneoff" (func $poll_oneoff (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_sync" (func $fd_sync (param i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
  (memory 1)
  (export "memory" (memory 0))

  ;; Scratch layout: 0..64 iovecs and result cells, 1024.. buffers, 4096.. output text.
  (global $sum (mut i64) (i64.const 0))
  (start $init)
  (func $init (global.set $sum (i64.const 0x1234)))

  (func $note (param $errno i32)
    (global.set $sum (i64.add (i64.mul (global.get $sum) (i64.const 31))
                              (i64.extend_i32_u (local.get $errno)))))
  (func $note64 (param $addr i32)
    (global.set $sum (i64.add (i64.mul (global.get $sum) (i64.const 31))
                              (i64.load (local.get $addr)))))

  (func $write (param $fd i32) (param $buf i32) (param $len i32)
    (i32.store (i32.const 0) (local.get $buf))
    (i32.store (i32.const 4) (local.get $len))
    (call $note (call $fd_write (local.get $fd) (i32.const 0) (i32.const 1) (i32.const 8))))

  (func (export "_start")
    (local $n i32)
    (call $note (call $args_sizes_get (i32.const 16) (i32.const 24)))
    (call $note (call $args_get (i32.const 32) (i32.const 1024)))
    (call $note (call $environ_sizes_get (i32.const 16) (i32.const 24)))
    (call $note (call $environ_get (i32.const 32) (i32.const 1024)))
    (call $note (call $clock_time_get (i32.const 1) (i64.const 1) (i32.const 40)))
    (call $note64 (i32.const 40))
    (call $note (call $clock_time_get (i32.const 0) (i64.const 1) (i32.const 40)))
    (call $note64 (i32.const 40))
    (call $note (call $random_get (i32.const 1024) (i32.const 37)))
    (call $note64 (i32.const 1024))
    (call $note64 (i32.const 1053))

    ;; stdin in 5-byte chunks until EOF (nread == 0)
    (block $eof
      (loop $more
        (i32.store (i32.const 0) (i32.const 2048))
        (i32.store (i32.const 4) (i32.const 5))
        (call $note (call $fd_read (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 8)))
        (br_if $eof (i32.eqz (i32.load (i32.const 8))))
        (call $note64 (i32.const 2048))
        (local.set $n (i32.add (local.get $n) (i32.const 1)))
        (br_if $more (i32.lt_u (local.get $n) (i32.const 100)))))
    (call $note (local.get $n))

    (call $note (call $fd_fdstat_get (i32.const 1) (i32.const 1024)))
    (call $note64 (i32.const 1024))
    (call $note (call $fd_filestat_get (i32.const 2) (i32.const 1024)))
    (call $note64 (i32.const 1040))
    (call $note (call $fd_prestat_get (i32.const 3) (i32.const 1024)))
    (call $note (call $fd_prestat_dir_name (i32.const 3) (i32.const 1024) (i32.const 8)))
    (call $note (call $fd_seek (i32.const 1) (i64.const 0) (i32.const 0) (i32.const 40)))
    (call $note (call $sched_yield))
    (call $note (call $fd_sync (i32.const 1)))
    (memory.fill (i32.const 1024) (i32.const 0) (i32.const 96))
    (call $note (call $poll_oneoff (i32.const 1024) (i32.const 3072) (i32.const 2) (i32.const 40)))
    (call $note (i32.load (i32.const 40)))
    (call $note64 (i32.const 3072))

    ;; 8 checksum bytes, then 300 more bytes: past the 256-byte public output window
    (i64.store (i32.const 4096) (global.get $sum))
    (call $write (i32.const 1) (i32.const 4096) (i32.const 8))
    (memory.fill (i32.const 4104) (i32.const 0x41) (i32.const 300))
    (call $write (i32.const 2) (i32.const 4104) (i32.const 150))
    (call $write (i32.const 1) (i32.const 4254) (i32.const 150))
    (call $note (call $fd_close (i32.const 0)))
    (call $proc_exit (i32.const 0))
    unreachable)
)"#;

const MEMORY_LIMITS_WAT: &str = r#"(module
  (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory 7000 7680)
  ;; the last 24 bytes of the initial memory
  (data (i32.const 458751976) "high data segment")
  (func $depth (param $n i32) (result i32)
    (if (result i32) (i32.eqz (local.get $n))
      (then (i32.const 0))
      (else (i32.add (call $depth (i32.sub (local.get $n) (i32.const 1))) (i32.const 1)))))
  (func (export "_start")
    (local $acc i64)
    ;; recursion depth: the u64 on stdin
    (i32.store (i32.const 0) (i32.const 64))
    (i32.store (i32.const 4) (i32.const 8))
    (drop (call $fd_read (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 8)))
    (local.set $acc (i64.extend_i32_u (call $depth (i32.load (i32.const 64)))))
    (local.set $acc (i64.add (local.get $acc) (i64.load (i32.const 458751976))))
    ;; grow to the 7,680-page maximum (returns 7000), then past it (returns -1)
    (local.set $acc (i64.add (local.get $acc) (i64.extend_i32_s (memory.grow (i32.const 680)))))
    (local.set $acc (i64.add (local.get $acc) (i64.extend_i32_s (memory.grow (i32.const 1)))))
    ;; the last 8 bytes of linear memory, then its very last byte
    (i64.store (i32.const 503316472) (local.get $acc))
    (i64.store8 (i32.const 503316479) (i64.const 0x5a))
    (local.set $acc (i64.add (local.get $acc) (i64.load (i32.const 503316472))))
    (i64.store (i32.const 16) (local.get $acc))
    (i32.store (i32.const 0) (i32.const 16))
    (i32.store (i32.const 4) (i32.const 8))
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40))))
)"#;

/// A command module that prints "OK" and then runs `tail`, which must make the run fail.
fn failing_module(tail: &str) -> Vec<u8> {
    let wat = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
          (memory 1)
          (func (export "_start")
            (i64.store (i32.const 16) (i64.const 0x4b4f))
            (i32.store (i32.const 0) (i32.const 16))
            (i32.store (i32.const 4) (i32.const 8))
            (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))
            {tail}))"#
    );
    wat::parse_str(wat).expect("wat compile")
}

struct Case {
    name: &'static str,
    wasm: Vec<u8>,
    stdin: Vec<u8>,
    /// Expected start of the public output; the rest of the 256-byte window must be zero.
    output: Vec<u8>,
}

fn cases() -> Vec<Case> {
    let wat = |name, src| wat::parse_str(src).unwrap_or_else(|e| panic!("{name}: {e}"));
    vec![
        Case {
            name: "wasm-fibonacci",
            wasm: WASM_FIBONACCI.to_vec(),
            stdin: 90u64.to_le_bytes().to_vec(),
            output: b"fib(90) = 2880067194370816120\n".to_vec(),
        },
        Case {
            name: "wasm-coverage",
            wasm: WASM_COVERAGE.to_vec(),
            stdin: 8u64.to_le_bytes().to_vec(),
            // Matches a native build of the same guest; stderr comes first, as it is written first.
            output: b"stderr works\nacc=4eb34ca595de925f v=8000 s8=ffffffffffffe121 s16=8c3763 \
                      areas=156 h=3160 k00000 ack=19 clock=true\n"
                .to_vec(),
        },
        Case {
            name: "integer_ops",
            wasm: wat("integer_ops", INTEGER_OPS_WAT),
            stdin: vec![],
            output: 0xa709_ebb2_b66b_f5f2u64.to_le_bytes().to_vec(),
        },
        Case {
            name: "wasi_surface",
            wasm: wat("wasi_surface", WASI_SURFACE_WAT),
            stdin: (0u8..23).collect(),
            // The checksum, then the 'A' run cut off at the 256-byte public output window.
            output: [0xd60c_9b11_c07a_de25u64.to_le_bytes().as_slice(), &[b'A'; 248]].concat(),
        },
        Case {
            name: "memory_limits",
            wasm: wat("memory_limits", MEMORY_LIMITS_WAT),
            stdin: 2000u64.to_le_bytes().to_vec(),
            output: 0xcec2_c840_d0cf_191eu64.to_le_bytes().to_vec(),
        },
    ]
}

/// The executors every case runs on: the Rust emulator everywhere, and the assembly emulator on
/// Linux (an Assembly-built client serves both).
fn executors() -> Vec<ExecutorKind> {
    let mut executors = vec![ExecutorKind::Emulator];
    if cfg!(target_os = "linux") {
        executors.push(ExecutorKind::Assembly);
    }
    executors
}

#[tokio::test]
#[ignore = "requires a generated proving key; run with --ignored"]
async fn wasm_guests_verify_constraints() {
    let mut builder = EmbeddedClientBuilder::default().verify_constraints();
    builder = if cfg!(target_os = "linux") { builder.assembly() } else { builder.emulator() };
    if let Some(pk) = std::env::var_os("ZISK_TEST_PROVING_KEY").map(PathBuf::from) {
        eprintln!("[wasm_verify_constraints] using ZISK_TEST_PROVING_KEY={}", pk.display());
        builder = builder.proving_key(pk);
    }
    let client = builder.build().expect("failed to build EmbeddedClient");

    let mut failures = Vec::new();
    for case in cases() {
        let program = GuestProgram::from_bytes(case.name, case.wasm);
        if let Err(e) = async { client.setup(&program).run()?.await }.await {
            eprintln!("[wasm_verify_constraints] {}: setup FAILED: {e:#}", case.name);
            failures.push(case.name.to_string());
            continue;
        }
        let mut expected = case.output;
        expected.resize(256, 0);
        for executor in executors() {
            let label = format!("{} ({executor:?})", case.name);
            let result = async {
                client
                    .verify_constraints(&program, ZiskStdin::from_bytes(case.stdin.clone()))
                    .executor(executor)
                    .run()?
                    .await
            }
            .await;
            match result {
                Ok(output) => {
                    let mut publics = [0u8; 256];
                    output.get_public_values_slice(&mut publics);
                    if publics.as_slice() == expected {
                        eprintln!("[wasm_verify_constraints] {label}: constraints verified");
                    } else {
                        eprintln!(
                            "[wasm_verify_constraints] {label}: constraints verified, but the \
                             public output is {publics:02x?}, expected {expected:02x?}"
                        );
                        failures.push(label);
                    }
                }
                Err(e) => {
                    eprintln!("[wasm_verify_constraints] {label}: FAILED: {e:#}");
                    failures.push(label);
                }
            }
        }
    }

    for (name, tail) in [
        ("trap_unreachable", "unreachable"),
        ("trap_out_of_bounds", "(drop (i64.load (i32.const 65536)))"),
        ("exit_nonzero", "(call $proc_exit (i32.const 1))"),
    ] {
        let program = GuestProgram::from_bytes(name, failing_module(tail));
        if let Err(e) = async { client.setup(&program).run()?.await }.await {
            eprintln!("[wasm_verify_constraints] {name}: setup FAILED: {e:#}");
            failures.push(name.to_string());
            continue;
        }
        for executor in executors() {
            let label = format!("{name} ({executor:?})");
            let result = async {
                client
                    .verify_constraints(&program, ZiskStdin::new())
                    .executor(executor)
                    .run()?
                    .await
            }
            .await;
            // The emulator names the failure; the assembly emulator reports it as a non-zero
            // result from its MT service, as it does for failing RISC-V guests.
            let rejected = |msg: &str| {
                msg.contains("Guest execution failed")
                    || (executor == ExecutorKind::Assembly
                        && msg.contains("service returned non-zero result"))
            };
            match result {
                Err(e) if rejected(&format!("{e:#}")) => {
                    eprintln!("[wasm_verify_constraints] {label}: rejected as expected");
                }
                Err(e) => {
                    eprintln!(
                        "[wasm_verify_constraints] {label}: rejected for the wrong reason: {e:#}"
                    );
                    failures.push(label);
                }
                Ok(_) => {
                    eprintln!(
                        "[wasm_verify_constraints] {label}: FAILED: a failing guest verified"
                    );
                    failures.push(label);
                }
            }
        }
    }

    assert!(failures.is_empty(), "constraint verification failed for: {failures:?}");
}
