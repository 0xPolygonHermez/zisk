//! The EF zkVM accelerators (`zkvm_accelerators.h`) imported by a wasm guest from `env`, as the
//! wasm32 C ABI emits them (see `transpilers/wasm/src/zkvm.rs`): each `.wat` module below calls
//! them the way a C or TinyGo guest compiled against the header does, and the ZisK library
//! routines run in their place.
//!
//! Buffers sit at odd addresses on purpose: wasm offers no alignment guarantee, and the routines
//! must not need one.

use zisk_common::EmuTrace;
use zisk_core::{ZISKLIB_ROM_ADDR, ZISKLIB_ROM_ADDR_MAX};
use zisk_transpiler_wasm::wasm2rom;
use ziskemu::{EmuOptions, ZiskEmulator};

/// A module importing `fd_write` and `imports`, whose `_start` runs `body` and then writes the
/// `len` bytes at `addr` to stdout.
fn module(imports: &str, body: &str, addr: u32, len: u32) -> Vec<u8> {
    let wat = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          {imports}
          (memory 1)
          (export "memory" (memory 0))
          (data (i32.const 3) "abc")
          (func (export "_start")
            {body}
            (i32.store (i32.const 2048) (i32.const {addr}))
            (i32.store (i32.const 2052) (i32.const {len}))
            (drop (call $fd_write (i32.const 1) (i32.const 2048) (i32.const 1) (i32.const 2056)))))"#
    );
    wat::parse_str(&wat).expect("wat compile")
}

const HASHES: &str = r#"
  (import "env" "zkvm_keccak256" (func $keccak256 (param i32 i32 i32) (result i32)))
  (import "env" "zkvm_sha256" (func $sha256 (param i32 i32 i32) (result i32)))
  (import "env" "zkvm_ripemd160" (func $ripemd160 (param i32 i32 i32) (result i32)))"#;

fn run(wasm: &[u8]) -> Vec<u8> {
    let rom = wasm2rom(wasm).expect("wasm2rom");
    ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation")
}

/// Runs `wasm` to the end and reports whether it trapped (halted with the error flag).
fn traps(wasm: &[u8]) -> bool {
    let rom = wasm2rom(wasm).expect("wasm2rom");
    let mut emu = ziskemu::Emu::new(&rom);
    emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
    assert!(emu.terminated(), "emulation did not terminate");
    emu.ctx.inst_ctx.error
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn hashes_match_their_reference_digests() {
    // keccak256, sha256 and ripemd160 of "abc" (at address 3) into 101, 133 and 165; then the
    // three statuses OR-ed together at 197.
    let body = r#"
      (i32.store (i32.const 197)
        (i32.or (call $keccak256 (i32.const 3) (i32.const 3) (i32.const 101))
          (i32.or (call $sha256 (i32.const 3) (i32.const 3) (i32.const 133))
                  (call $ripemd160 (i32.const 3) (i32.const 3) (i32.const 165)))))"#;
    let out = run(&module(HASHES, body, 101, 100));
    assert_eq!(
        hex(&out[0..32]),
        "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"
    );
    assert_eq!(
        hex(&out[32..64]),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // RIPEMD-160 is right-aligned in its 32-byte output, after 12 zero bytes.
    assert_eq!(
        hex(&out[64..96]),
        "0000000000000000000000008eb208f7e05d987a9b044a8e98c6b087f15a0bfc"
    );
    assert_eq!(out[96..100], [0, 0, 0, 0], "every call returns ZKVM_EOK");
}

#[test]
fn keccak_f1600_permutes_an_unaligned_state_in_place() {
    // Keccak-f[1600] of the all-zero state; its first lane is 0xF1258F7940E1DDE7.
    let imports = r#"(import "env" "zkvm_keccak_f1600" (func $keccak_f (param i32) (result i32)))"#;
    let body = "(i32.store (i32.const 1201) (call $keccak_f (i32.const 1001)))";
    let out = run(&module(imports, body, 1001, 8));
    assert_eq!(u64::from_le_bytes(out[0..8].try_into().unwrap()), 0xF125_8F79_40E1_DDE7);
}

#[test]
fn a_failure_status_is_a_sign_extended_minus_one() {
    // ecrecover only accepts recovery ids 0 and 1, so id 2 fails with ZKVM_EFAIL (-1). The status
    // must compare equal to i32 -1, and widen to i64 -1.
    let imports = r#"(import "env" "zkvm_secp256k1_ecrecover"
                       (func $ecrecover (param i32 i32 i32 i32) (result i32)))"#;
    let body = r#"
      (local $status i32)
      (local.set $status
        (call $ecrecover (i32.const 301) (i32.const 333) (i32.const 2) (i32.const 397)))
      (i64.store (i32.const 501) (i64.extend_i32_s (local.get $status)))
      (i32.store (i32.const 509) (i32.eq (local.get $status) (i32.const -1)))"#;
    let out = run(&module(imports, body, 501, 12));
    assert_eq!(u64::from_le_bytes(out[0..8].try_into().unwrap()), u64::MAX);
    assert_eq!(out[8..12], [1, 0, 0, 0]);
}

#[test]
fn pointer_ranges_are_bounds_checked() {
    // One page: 65536 bytes. A 32-byte digest ending exactly at the end of memory is fine, one
    // byte further traps, and so does an input running past the end.
    let call = |data: u32, len: u32, out: u32| {
        module(
            HASHES,
            &format!(
                "(drop (call $keccak256 (i32.const {data}) (i32.const {len}) (i32.const {out})))"
            ),
            0,
            0,
        )
    };
    assert!(!traps(&call(3, 3, 65536 - 32)), "output ending at the memory end is in bounds");
    assert!(traps(&call(3, 3, 65536 - 31)), "output one byte past the end must trap");
    assert!(traps(&call(65536 - 2, 3, 101)), "input one byte past the end must trap");
    assert!(traps(&call(3, 0xFFFF_FFFF, 101)), "a huge length must trap, not wrap");
    assert!(traps(&call(0xFFFF_FFF0, 3, 101)), "a pointer above the memory must trap");
}

#[test]
fn an_empty_range_may_point_anywhere() {
    // EIP-198: a zero-length modulus gives an empty output, whose pointer need not be valid, so
    // modexp with mod_len = 0 and an output pointer far outside the memory must succeed.
    let imports = r#"(import "env" "zkvm_modexp"
                       (func $modexp (param i32 i32 i32 i32 i32 i32 i32) (result i32)))"#;
    let body = r#"
      (i32.store (i32.const 501)
        (call $modexp (i32.const 3) (i32.const 1) (i32.const 4) (i32.const 1)
                      (i32.const 5) (i32.const 0) (i32.const 0xFFFFFF00)))"#;
    let out = run(&module(imports, body, 501, 4));
    assert_eq!(out[0..4], [0, 0, 0, 0]);
}

#[test]
fn modexp_reads_and_writes_byte_strings() {
    // 3^2 mod 5 = 4, with base, exponent and modulus one byte each.
    let imports = r#"(import "env" "zkvm_modexp"
                       (func $modexp (param i32 i32 i32 i32 i32 i32 i32) (result i32)))"#;
    let body = r#"
      (i32.store8 (i32.const 601) (i32.const 3))
      (i32.store8 (i32.const 602) (i32.const 2))
      (i32.store8 (i32.const 603) (i32.const 5))
      (i32.store (i32.const 605)
        (call $modexp (i32.const 601) (i32.const 1) (i32.const 602) (i32.const 1)
                      (i32.const 603) (i32.const 1) (i32.const 604)))"#;
    let out = run(&module(imports, body, 604, 5));
    assert_eq!(out[0..5], [4, 0, 0, 0, 0]);
}

#[test]
fn a_routine_with_nested_calls_leaves_the_caller_frame_intact() {
    // bn254_g1_add calls other routines, saving its return address on the stack at `sp` (`r2`),
    // which is also the wasm frame pointer. Adding the point at infinity (all zeros, EIP-196) to
    // itself gives it back. The call happens two frames deep, with locals on both sides that must
    // survive it, and both functions must return to the right place.
    let imports = r#"(import "env" "zkvm_bn254_g1_add"
                       (func $g1_add (param i32 i32 i32) (result i32)))
      (func $inner (param $tag i64) (result i64)
        (local $status i32)
        (local.set $status (call $g1_add (i32.const 701) (i32.const 765) (i32.const 829)))
        (i64.add (local.get $tag) (i64.extend_i32_u (local.get $status))))
      (func $outer (param $tag i64) (result i64)
        (local $keep i64)
        (local.set $keep (i64.const 0x1122334455667788))
        (i64.xor (call $inner (local.get $tag)) (local.get $keep)))"#;
    let body = r#"
      (i64.store (i32.const 901) (call $outer (i64.const 0x0102030405060708)))"#;
    let out = run(&module(imports, body, 829, 80));
    assert_eq!(out[0..64], [0u8; 64], "infinity + infinity = infinity");
    let tag = u64::from_le_bytes(out[72..80].try_into().unwrap());
    assert_eq!(tag, 0x0102030405060708 ^ 0x1122334455667788, "status 0, locals intact");
}

#[test]
fn a_mismatched_signature_is_rejected() {
    // zkvm_keccak256 takes three i32s; declaring a 64-bit length is a different C function.
    let imports = r#"(import "env" "zkvm_keccak256"
                       (func $keccak256 (param i32 i64 i32) (result i32)))"#;
    let wasm = module(imports, "", 0, 0);
    let err = wasm2rom(&wasm).expect_err("must be rejected").to_string();
    assert!(err.contains("env::zkvm_keccak256"), "{err}");
}

#[test]
fn an_unknown_env_import_is_rejected() {
    let imports = r#"(import "env" "zkvm_unknown" (func $f (param i32) (result i32)))"#;
    let err = wasm2rom(&module(imports, "", 0, 0)).expect_err("must be rejected").to_string();
    assert!(err.contains("env::zkvm_unknown"), "{err}");
}

/// The TinyGo example of `ziskasm/lang/tinygo/example`, which checks every accelerator against a
/// known answer through the Go binding. Prebuilt (`data/tinygo-zkvm-example.wasm`, with the
/// command in its `main.go`) so the test needs no TinyGo; the TinyGo runtime uses floats, hence
/// the `float` feature.
#[cfg(feature = "float")]
#[test]
fn tinygo_example_passes_every_check() {
    let wasm = include_bytes!("data/tinygo-zkvm-example.wasm");
    let out = run(wasm);
    let expected = b"zkvm: 30 checks passed\n";
    assert_eq!(&out[..expected.len()], expected, "{}", String::from_utf8_lossy(&out));
}

#[test]
fn the_library_is_merged_only_when_used() {
    let in_library = |wasm: &[u8]| {
        let rom = wasm2rom(wasm).expect("wasm2rom");
        (rom.insts.range(ZISKLIB_ROM_ADDR..=ZISKLIB_ROM_ADDR_MAX).count(), rom.zkvmcalls)
    };
    let (count, zkvmcalls) = in_library(&module("", "", 0, 0));
    assert_eq!((count, zkvmcalls.len()), (0, 0));
    let (count, zkvmcalls) = in_library(&module(HASHES, "", 0, 0));
    assert!(count > 0);
    assert_eq!(zkvmcalls.into_iter().collect::<Vec<_>>(), [0x850, 0x851, 0x852]);
}
