//! The functions of the ZisK C headers (the EF accelerators, U256, random, write_log, zisklib and
//! fcalls) imported by a wasm guest from `env`, as the wasm32 C ABI emits them (see
//! `transpilers/wasm/src/zkvm.rs`): each `.wat` module below calls them the way a C or TinyGo
//! guest compiled against the headers does, and the ZisK library routines run in their place.
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

/// The example of `ziskasm/lang/go/example/zkvm`, which checks every function of the Go binding
/// (accelerators, U256, zisklib, hints) against a known answer, built with TinyGo (cgo calls,
/// `data/tinygo-zkvm-example.wasm`) and with standard Go (`//go:wasmimport` declarations,
/// `data/go-zkvm-example.wasm`), with the commands in its `main.go`. Prebuilt so the test needs
/// neither toolchain; both runtimes use floats, hence the `float` feature.
#[cfg(feature = "float")]
#[test]
fn tinygo_example_passes_every_check() {
    example_passes_every_check(include_bytes!("data/tinygo-zkvm-example.wasm"));
}

#[cfg(feature = "float")]
#[test]
fn standard_go_example_passes_every_check() {
    example_passes_every_check(include_bytes!("data/go-zkvm-example.wasm"));
}

#[cfg(feature = "float")]
fn example_passes_every_check(wasm: &[u8]) {
    let out = run(wasm);
    let expected = b"zkvm: 197 checks passed\n";
    assert_eq!(&out[..expected.len()], expected, "{}", String::from_utf8_lossy(&out));
}

#[test]
fn u256_functions_compute_evm_words_in_both_byte_orders() {
    // 2^255 + 1 plus 2^255 + 2 wraps to 3, and 7 / 2 = 3, in each byte order: big-endian bytes
    // at odd addresses, little-endian limbs at 8-byte aligned ones. The inline functions (add)
    // set no status of their own, the calls (div) do, and both must report ZKVM_EOK.
    let imports = r#"
      (import "env" "zkvm_u256_add" (func $add (param i32 i32 i32) (result i32)))
      (import "env" "zkvm_u256_div" (func $div (param i32 i32 i32) (result i32)))
      (import "env" "zkvm_u256_le_add" (func $add_le (param i32 i32 i32) (result i32)))
      (import "env" "zkvm_u256_le_div" (func $div_le (param i32 i32 i32) (result i32)))"#;
    let body = r#"
      (i32.store8 (i32.const 101) (i32.const 0x80)) (i32.store8 (i32.const 132) (i32.const 1))
      (i32.store8 (i32.const 133) (i32.const 0x80)) (i32.store8 (i32.const 164) (i32.const 2))
      (i32.store8 (i32.const 229) (i32.const 7)) (i32.store8 (i32.const 261) (i32.const 2))
      (i64.store (i32.const 512) (i64.const 1)) (i64.store (i32.const 536) (i64.const 0x8000000000000000))
      (i64.store (i32.const 544) (i64.const 2)) (i64.store (i32.const 568) (i64.const 0x8000000000000000))
      (i64.store (i32.const 576) (i64.const 7)) (i64.store (i32.const 608) (i64.const 2))
      (i32.store (i32.const 1001)
        (i32.or (i32.or (call $add (i32.const 101) (i32.const 133) (i32.const 1101))
                        (call $div (i32.const 198) (i32.const 230) (i32.const 1133)))
                (i32.or (call $add_le (i32.const 512) (i32.const 544) (i32.const 1168))
                        (call $div_le (i32.const 576) (i32.const 608) (i32.const 1200)))))
      (i32.store (i32.const 1232) (i32.load (i32.const 1001)))"#;
    let out = run(&module(imports, body, 1101, 135));
    let three_be = [[0u8; 31].as_slice(), &[3]].concat();
    assert_eq!(out[0..32], three_be, "BE add");
    assert_eq!(out[32..64], three_be, "BE div");
    let three_le = [[3u8].as_slice(), &[0; 31]].concat();
    assert_eq!(out[67..99], three_le, "LE add");
    assert_eq!(out[99..131], three_le, "LE div");
    assert_eq!(out[131..135], [0, 0, 0, 0], "every call returns ZKVM_EOK");
}

#[test]
fn a_uint64_t_pointer_must_be_aligned() {
    let imports = r#"
      (import "env" "zkvm_u256_le_add" (func $add_le (param i32 i32 i32) (result i32)))"#;
    let call = |a: u32| {
        let body = format!("(drop (call $add_le (i32.const {a}) (i32.const 64) (i32.const 96)))");
        module(imports, &body, 0, 0)
    };
    assert!(!traps(&call(32)));
    assert!(traps(&call(36)), "a pointer 4 bytes off must trap");
}

#[test]
fn random_u64_returns_the_seed_stream() {
    // The first two words of SHAKE256 of the seed bytes 00 01 .. 1f (see tests/zkvm_random.rs).
    let imports = r#"(import "env" "zkvm_random_u64" (func $random (result i64)))"#;
    let body = r#"
      (i64.store (i32.const 1000) (call $random))
      (i64.store (i32.const 1008) (call $random))"#;
    let rom = wasm2rom(&module(imports, body, 1000, 16)).expect("wasm2rom");
    assert!(rom.uses_random_seed(), "the guest needs a seed, like a RISC-V one");
    let seed = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    let options = EmuOptions { random_seed: Some(seed.parse().unwrap()), ..Default::default() };
    let out = ZiskEmulator::process_rom(&rom, &[], &options, None::<fn(EmuTrace)>).unwrap();
    let word = |i: usize| u64::from_le_bytes(out[8 * i..8 * i + 8].try_into().unwrap());
    assert_eq!([word(0), word(1)], [0x0280ce40887cf069, 0x5b3d2c883909b34d]);
}

#[test]
fn write_log_leaves_the_output_alone() {
    let imports = r#"(import "env" "write_log" (func $log (param i32 i32)))"#;
    let body = "(call $log (i32.const 3) (i32.const 3))";
    assert_eq!(run(&module(imports, body, 3, 3))[..3], *b"abc");
    let body = "(call $log (i32.const 65535) (i32.const 2))";
    assert!(traps(&module(imports, body, 0, 0)), "the text is bounds-checked");
}

#[test]
fn zisklib_functions_take_and_return_64_bit_values() {
    // uint64_t is an i64 in the wasm32 C ABI. modexp_u64_c: 3^2 mod 5 = 4, one limb each, and it
    // returns the 4 limbs it wrote.
    let imports = r#"
      (import "env" "zkvm_zisklib_add" (func $add (param i64 i64) (result i64)))
      (import "env" "zkvm_zisklib_modexp_u64_c"
        (func $modexp (param i32 i32 i32 i32 i32 i32 i32) (result i32)))"#;
    let body = r#"
      (i64.store (i32.const 1000) (call $add (i64.const 0x10000000000) (i64.const 5)))
      (i64.store (i32.const 1008) (i64.const 3))
      (i64.store (i32.const 1016) (i64.const 2))
      (i64.store (i32.const 1024) (i64.const 5))
      (i32.store (i32.const 1032)
        (call $modexp (i32.const 1008) (i32.const 1) (i32.const 1016) (i32.const 1)
                      (i32.const 1024) (i32.const 1) (i32.const 1040)))"#;
    let out = run(&module(imports, body, 1000, 48));
    assert_eq!(u64::from_le_bytes(out[0..8].try_into().unwrap()), 0x100_0000_0005);
    assert_eq!(out[32..36], [4, 0, 0, 0], "4 limbs written");
    assert_eq!(u64::from_le_bytes(out[40..48].try_into().unwrap()), 4);
}

#[test]
fn sizes_from_64_bit_and_limb_arguments_are_bounds_checked() {
    // schnorr's msg_len is a uint64_t: 2^32 must trap, not wrap. modexp_u64_c's result holds
    // at least 4 limbs (32 bytes), however short the modulus.
    let schnorr = r#"(import "env" "zkvm_zisklib_schnorr_verify_secp256k1"
                       (func $schnorr (param i32 i32 i32 i32 i64) (result i64)))"#;
    let body = r#"(drop (call $schnorr (i32.const 0) (i32.const 32) (i32.const 64)
                                       (i32.const 128) (i64.const 0x100000000)))"#;
    assert!(traps(&module(schnorr, body, 0, 0)));
    let modexp = r#"(import "env" "zkvm_zisklib_modexp_u64_c"
                      (func $modexp (param i32 i32 i32 i32 i32 i32 i32) (result i32)))"#;
    let call = |result: u32| {
        let body = format!(
            "(drop (call $modexp (i32.const 8) (i32.const 1) (i32.const 16) (i32.const 1)
                                 (i32.const 24) (i32.const 1) (i32.const {result})))"
        );
        module(modexp, &body, 0, 0)
    };
    assert!(traps(&call(65536 - 24)), "a 4-limb result must fit");
}

#[test]
fn fcalls_pass_parameters_and_read_results() {
    // BN254_FP_INV of 2, read with zkvm_fcall_read; UINT256_DIV of 100 by 7, read with
    // zkvm_fcall_get; then the rejected calls: ID 27 (the random seed), ID 0, and 387
    // parameters. Each returns ZKVM_EFAIL, so the statuses OR to -1 and AND to -1.
    let imports = r#"
      (import "env" "zkvm_fcall" (func $fcall (param i32 i32 i32) (result i32)))
      (import "env" "zkvm_fcall_get" (func $get (result i64)))
      (import "env" "zkvm_fcall_read" (func $read (param i32 i32)))"#;
    let body = r#"
      (i64.store (i32.const 1000) (i64.const 2))
      (i32.store (i32.const 3000) (call $fcall (i32.const 6) (i32.const 1000) (i32.const 4)))
      (call $read (i32.const 3008) (i32.const 4))
      (i64.store (i32.const 1000) (i64.const 100))
      (i64.store (i32.const 1032) (i64.const 7))
      (i32.store (i32.const 3004) (call $fcall (i32.const 19) (i32.const 1000) (i32.const 8)))
      (i64.store (i32.const 3040) (call $get))
      (drop (call $get)) (drop (call $get)) (drop (call $get))
      (i64.store (i32.const 3048) (call $get))
      (drop (call $get)) (drop (call $get)) (drop (call $get))
      (i32.store (i32.const 3056)
        (i32.and (call $fcall (i32.const 27) (i32.const 1000) (i32.const 0))
          (i32.and (call $fcall (i32.const 0) (i32.const 1000) (i32.const 4))
                   (call $fcall (i32.const 6) (i32.const 0) (i32.const 387)))))"#;
    let out = run(&module(imports, body, 3000, 60));
    let word = |i: usize| u64::from_le_bytes(out[i..i + 8].try_into().unwrap());
    assert_eq!(out[0..8], [0u8; 8], "both hints were issued");
    // (p + 1) / 2, the inverse of 2 mod the BN254 base field prime.
    let inv2 = [0x9e10460b6c3e7ea4, 0xcbc0b548b438e546, 0xdc2822db40c0ac2e, 0x183227397098d014];
    assert_eq!([word(8), word(16), word(24), word(32)], inv2);
    assert_eq!([word(40), word(48)], [14, 2], "quotient and remainder");
    assert_eq!(out[56..60], [0xFF; 4], "every rejected call returns ZKVM_EFAIL");
}

#[test]
fn read_input_points_to_wasi() {
    let imports = r#"(import "env" "read_input" (func $f (param i32 i32)))"#;
    let err = wasm2rom(&module(imports, "", 0, 0)).expect_err("must be rejected").to_string();
    assert!(err.contains("stdin"), "{err}");
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
