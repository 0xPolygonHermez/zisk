//! Nitro crypto executes the production library in the generated ROM.
use zisk_common::EmuTrace;
use zisk_transpiler_wasm::{nitro::crypto_imports, wasm2rom_with_imports};
use ziskemu::{EmuOptions, ZiskEmulator};

fn module(data: &str, body: &str, out: u32, len: u32) -> Vec<u8> {
    wat::parse_str(format!(r#"(module
      (import "arbcrypto" "keccak256" (func $hash (param i32 i32 i32)))
      (import "arbcrypto" "ecrecovery" (func $recover (param i32 i32 i32 i32 i32) (result i32)))
      (import "wasi_snapshot_preview1" "fd_write" (func $write (param i32 i32 i32 i32) (result i32)))
      (memory 1 2)
      {data}
      (func $nested (param $p i32) (param $n i32) (param $o i32) (result i32)
        (local $keep i32) (local.set $keep (i32.const 42))
        (call $hash (local.get $p) (local.get $n) (local.get $o))
        (local.get $keep))
      (func (export "_start") {body}
        (i32.store (i32.const 0) (i32.const {out}))
        (i32.store (i32.const 4) (i32.const {len}))
        (drop (call $write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8)))))"#)).unwrap()
}

fn execute(wasm: &[u8]) -> Result<Vec<u8>, String> {
    let rom = wasm2rom_with_imports(wasm, &crypto_imports()).map_err(|e| e.to_string())?;
    let opts = EmuOptions { max_steps: 5_000_000, ..Default::default() };
    ZiskEmulator::process_rom(&rom, &[], &opts, None::<fn(EmuTrace)>).map_err(|e| e.to_string())
}

fn bytes(data: &[u8]) -> String {
    data.iter().map(|b| format!("\\{b:02x}")).collect()
}
fn hex(data: &str) -> Vec<u8> {
    data.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn keccak_empty_executes_inside_rom() {
    let wasm = module("", "(call $hash (i32.const 65536) (i32.const 0) (i32.const 33))", 33, 32);
    assert_eq!(
        &execute(&wasm).expect("native keccak succeeds")[..32],
        hex("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
    );
}

const HASH: &str = "1111111122222222333333334444444455555555666666667777777788888888";
const SIG: &str = "5424df55c64936e1ea26fe1bafc9f7becb09b4e08badb05f3d5a399b11af56ce621c59ef710fb0221066d9af00bf482dd3d49991f2afa8f745a88a5c9f58755201";
const PUB: &str = "0475f2165273d36c4603da9f4c3af30fa4f798d3d880d5338acccf1cfb8399017c3c006c2f5c884a4c42f10609551b55b214dbcfa2f407a6d13cb53f7f8c09fc85";
fn recovery_data(sig: &str) -> String {
    format!(
        r#"(data (i32.const 17) "{}") (data (i32.const 65) "{}")"#,
        bytes(&hex(HASH)),
        bytes(&hex(sig))
    )
}
#[test]
fn recovery_known_key_executes_inside_rom() {
    let wasm = module(&recovery_data(SIG), "(i32.store (i32.const 300) (call $recover (i32.const 17) (i32.const 32) (i32.const 65) (i32.const 65) (i32.const 193)))",193,65);
    assert_eq!(&execute(&wasm).expect("native recovery succeeds")[..65], hex(PUB));
}

// Fixed vectors generated with the workspace's tiny-keccak 2.0.2, independently of ROM execution.
#[test]
fn keccak_rate_boundaries_alignment_and_overlap() {
    for (len, digest) in [
        (3, "b9a5dc0048db9a7d13548781df3cd4b2334606391f75f40c14225a92f4cb3537"),
        (135, "34367dc248bbd832f4e3e69dfaac2f92638bd0bbd18f2912ba4ef454919cf446"),
        (136, "a6c4d403279fe3e0af03729caada8374b5ca54d8065329a3ebcaeb4b60aa386e"),
        (137, "d869f639c7046b4929fc92a4d988a8b22c55fbadb802c0c66ebcd484f1915f39"),
        (300, "5b7e0e47a96f32a88b4f14ca177982790807c40e1a105742ba0fc1babe1ef826"),
        (544, "88956d998e28ada27fa7580f7e2918fbe22e92e5046081065d258ff614f8ec6e"),
    ] {
        for alignment in 0..8 {
            let input = 64 + alignment;
            // Exact and partial aliasing, in both directions, plus a separate unaligned output.
            for out in [input, input - 3, input + 2, 1025] {
                let data = format!(r#"(data (i32.const {input}) "{}")"#, "a".repeat(len));
                let body =
                    format!("(call $hash (i32.const {input}) (i32.const {len}) (i32.const {out}))");
                let wasm = module(&data, &body, out, 32);
                assert_eq!(
                    &execute(&wasm).unwrap()[..32],
                    hex(digest),
                    "len={len}, in={input}, out={out}"
                );
            }
        }
    }
}

#[test]
fn native_calls_preserve_nested_frames_and_growing_memory() {
    let wasm = module(
        r#"(data (i32.const 17) "aaa")"#,
        r#"
      (i32.store (i32.const 1057) (call $nested (i32.const 17) (i32.const 3) (i32.const 1025)))
      (call $hash (i32.const 65536) (i32.const 0) (i32.const 1025))
      (drop (memory.grow (i32.const 1)))
      (call $hash (i32.const 17) (i32.const 3) (i32.const 131040))
      (memory.copy (i32.const 1025) (i32.const 131040) (i32.const 32))
      (i32.store (i32.const 1061) (memory.size))"#,
        1025,
        40,
    );
    let mut expected = hex("b9a5dc0048db9a7d13548781df3cd4b2334606391f75f40c14225a92f4cb3537");
    expected.extend(42u32.to_le_bytes());
    expected.extend(2u32.to_le_bytes());
    assert_eq!(&execute(&wasm).unwrap()[..40], expected);
    let wasm = module("", "(call $hash (i32.const 0) (i32.const 0) (i32.const 33)) (drop (i32.load (i32.const 65536)))", 33,32);
    assert!(
        execute(&wasm).unwrap_err().contains("trap"),
        "post-native Wasm memory bounds must survive"
    );
}

#[test]
fn keccak_rejects_unsigned_out_of_bounds_and_overflow() {
    for (input, len, out) in [
        (-1, 0, 33),
        (0, -1, 33),
        (65535, 2, 33),
        (65536, 1, 33),
        (65537, 0, 33),
        (0, 0, 65505),
        (0, 0, -1),
        (-1, 2, 33),
    ] {
        let body = format!("(call $hash (i32.const {input}) (i32.const {len}) (i32.const {out}))");
        assert!(
            execute(&module("", &body, 33, 32)).unwrap_err().contains("trap"),
            "{input}/{len}/{out}"
        );
    }
}

fn recovery_call(hash: i32, hash_len: i32, sig: i32, sig_len: i32, out: i32) -> String {
    format!("(call $recover (i32.const {hash}) (i32.const {hash_len}) (i32.const {sig}) (i32.const {sig_len}) (i32.const {out}))")
}
fn status(data: &str, call: &str) -> u32 {
    let wasm = module(data, &format!("(i32.store (i32.const 300) {call})"), 300, 4);
    u32::from_le_bytes(execute(&wasm).unwrap()[..4].try_into().unwrap())
}

#[test]
fn recovery_lengths_precede_pointer_access() {
    assert_eq!(status("", &recovery_call(-1, 31, -1, 64, -1)), 1);
    assert_eq!(status("", &recovery_call(-1, -1, -1, 65, -1)), 1);
    assert_eq!(status("", &recovery_call(-1, 32, -1, 64, -1)), 2);
    assert_eq!(status("", &recovery_call(-1, 32, -1, -1, -1)), 2);
}

#[test]
fn recovery_id_and_crypto_failures_leave_output_untouched() {
    for id in [4, 255] {
        let sig = format!("{}{:02x}", &SIG[..128], id);
        assert_eq!(status(&recovery_data(&sig), &recovery_call(17, 32, 65, 65, -1)), 3);
    }
    let order = "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141";
    for sig in [
        "00".repeat(64) + "01",
        format!("{}{}01", order, &SIG[64..128]),
        format!("{}{}01", &SIG[..64], order),
    ] {
        assert_eq!(status(&recovery_data(&sig), &recovery_call(17, 32, 65, 65, -1)), 4);
        let data = recovery_data(&sig) + r#" (data (i32.const 193) "sentinel")"#;
        let wasm = module(&data, &format!("(drop {})", recovery_call(17, 32, 65, 65, 193)), 193, 8);
        assert_eq!(&execute(&wasm).unwrap()[..8], b"sentinel");
    }
}

#[test]
fn recovery_traps_on_unsupported_ids_and_pointer_bounds() {
    for id in [2, 3] {
        let data = recovery_data(&format!("{}{:02x}", &SIG[..128], id));
        let body = format!("(drop {})", recovery_call(17, 32, 65, 65, 193));
        assert!(execute(&module(&data, &body, 193, 65)).unwrap_err().contains("trap"));
    }
    for args in [
        (-1, 65, 193),
        (65505, 65, 193),
        (17, -1, 193),
        (17, 65472, 193),
        (17, 65, -1),
        (17, 65, 65472),
    ] {
        let body = format!("(drop {})", recovery_call(args.0, 32, args.1, 65, args.2));
        assert!(
            execute(&module(&recovery_data(SIG), &body, 193, 65)).unwrap_err().contains("trap"),
            "{args:?}"
        );
    }
}

#[test]
fn recovery_supports_unaligned_and_aliased_output() {
    for out in [17, 65, 193] {
        let body = format!("(i32.store (i32.const 300) {})", recovery_call(17, 32, 65, 65, out));
        assert_eq!(
            &execute(&module(&recovery_data(SIG), &body, out as u32, 65)).unwrap()[..65],
            hex(PUB)
        );
    }
    let body = format!("(i32.store (i32.const 300) {})", recovery_call(17, 32, 65, 65, 65471));
    assert_eq!(&execute(&module(&recovery_data(SIG), &body, 65471, 65)).unwrap()[..65], hex(PUB));
    // ID 0 is supported too; the full ID 1 expected key is pinned above and in the C library test.
    let data = recovery_data(&(SIG[..128].to_owned() + "00"));
    assert_eq!(status(&data, &recovery_call(17, 32, 65, 65, 193)), 0);
}

#[test]
fn native_library_link_is_lazy_and_instruction_indexes_are_unique() {
    use std::collections::BTreeSet;
    use zisk_core::{
        FLOAT_LIB_RAM_ADDR, STACK_ADDR, STACK_SIZE, ZISKLIB_RAM_ADDR, ZISKLIB_ROM_ADDR,
        ZISKLIB_ROM_ADDR_MAX,
    };
    use zisk_transpiler_wasm::layout::*;
    let plain = wat::parse_str(r#"(module (func (export "_start")))"#).unwrap();
    let rom = wasm2rom_with_imports(&plain, &crypto_imports()).unwrap();
    assert!(rom.insts.range(ZISKLIB_ROM_ADDR..=ZISKLIB_ROM_ADDR_MAX).next().is_none());
    assert!(rom.zkvmcalls.is_empty());
    let wasm = module("", "(call $hash (i32.const 0) (i32.const 0) (i32.const 33))", 33, 32);
    let rom = wasm2rom_with_imports(&wasm, &crypto_imports()).unwrap();
    assert!(rom.insts.range(ZISKLIB_ROM_ADDR..=ZISKLIB_ROM_ADDR_MAX).next().is_some());
    assert_eq!(
        rom.insts.values().map(|i| i.i.index).collect::<BTreeSet<_>>().len(),
        rom.insts.len()
    );
    assert_eq!(rom.build_counter as usize, rom.insts.len());
    assert!(rom.zkvmcalls.contains(&zisk_definitions::zkvmcall_id("zkvm_keccak256")));
    assert!(rom.zkvmcalls.contains(&zisk_definitions::zkvmcall_id("zkvm_secp256k1_ecrecover")));
    assert!(!rom.uses_random_seed());
    assert_eq!(WASM_STACK_TOP, ZISKLIB_RAM_ADDR);
    assert!(WASM_CRYPTO_OUTPUT_ADDR + 64 < WASM_MEM_BASE);
    assert!(STACK_ADDR + STACK_SIZE <= WASM_GLOBALS_ADDR);
    assert!(WASM_MEM_LIMIT <= WASM_STACK_LIMIT && WASM_STACK_TOP < FLOAT_LIB_RAM_ADDR);
}

#[test]
fn unknown_and_unsupported_native_symbols_fail_translation() {
    use zisk_transpiler_wasm::{emit::Code, Import};
    fn unknown(code: &mut Code) {
        code.call_library("not_a_library_symbol");
    }
    fn unsupported(code: &mut Code) {
        code.call_library("ziskasm_zkvm_random_u64");
    }
    let wasm = wat::parse_str(r#"(module (import "test" "native" (func $native)) (func (export "_start") (call $native)))"#).unwrap();
    for (emit, expected) in
        [(unknown as fn(&mut Code), "unknown"), (unsupported as fn(&mut Code), "unsupported")]
    {
        let import = Import { module: "test", name: "native", params: &[], results: &[], emit };
        let error =
            wasm2rom_with_imports(&wasm, &[import]).err().expect("symbol rejected").to_string();
        assert!(error.contains(expected), "{error}");
    }
}
