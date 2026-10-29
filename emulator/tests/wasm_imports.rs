//! Custom import bodies execute inside the ROM, with the ordinary Wasm call convention.

use zisk_common::EmuTrace;
use zisk_transpiler_wasm::{
    emit::Code,
    layout::{local_offset, REG_RET, REG_T0, REG_T1},
    module::ValKind::{I32, I64},
    wasm2rom, wasm2rom_with_imports, Import,
};
use ziskemu::{EmuOptions, ZiskEmulator};

fn add(code: &mut Code) {
    code.load_slot_to_reg(REG_T0, local_offset(0));
    code.load_slot_to_reg(REG_T1, local_offset(1));
    code.alu_rr("add", REG_RET, REG_T0, REG_T1);
}

fn add_import() -> Import<'static> {
    Import { module: "example", name: "add", params: &[I64, I64], results: &[I64], emit: add }
}

fn module(imports_and_functions: &str, body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $write (param i32 i32 i32 i32) (result i32)))
          {imports_and_functions}
          (memory 1)
          (func $compute (result i64) {body})
          (func (export "_start")
            (i64.store (i32.const 16) (call $compute))
            (i32.store (i32.const 0) (i32.const 16))
            (i32.store (i32.const 4) (i32.const 8))
            (drop (call $write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
    ))
    .unwrap()
}

fn output(wasm: &[u8], imports: &[Import<'_>]) -> Vec<u8> {
    let rom = wasm2rom_with_imports(wasm, imports).expect("custom imports transpile");
    ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("custom imports execute")
}

fn assert_public_value(wasm: &[u8], imports: &[Import<'_>], expected: u64) {
    let mut bytes = vec![0; 256];
    bytes[..8].copy_from_slice(&expected.to_le_bytes());
    assert_eq!(output(wasm, imports), bytes);
}

#[test]
fn custom_import_receives_arguments_and_returns_a_result() {
    let wasm = module(
        r#"(import "example" "add" (func $add (param i64 i64) (result i64)))"#,
        "(call $add (i64.const 40) (i64.const 2))",
    );
    assert_public_value(&wasm, &[add_import()], 42);
    assert!(wasm2rom(&wasm).is_err(), "default entry point must still reject custom imports");
}

#[test]
fn nested_indirect_import_call_preserves_the_callers_frame() {
    let wasm = module(
        r#"(type $sum (func (param i64 i64) (result i64)))
          (import "example" "add" (func $add (type $sum)))
          (table 1 funcref)
          (elem (i32.const 0) $add)
          (func $nested (param i64) (result i64)
            (i64.add
              (call_indirect (type $sum) (local.get 0) (i64.const 1) (i32.const 0))
              (i64.const 2)))"#,
        r#"(local $keep i64)
          (local.set $keep (i64.const 30))
          (i64.add (call $nested (i64.const 9)) (local.get $keep))"#,
    );
    assert_public_value(&wasm, &[add_import()], 42);
}

#[test]
fn repeated_import_declarations_share_one_registration() {
    let wasm = module(
        r#"(import "example" "add" (func $first (param i64 i64) (result i64)))
          (import "example" "add" (func $second (param i64 i64) (result i64)))"#,
        "(call $second (call $first (i64.const 10) (i64.const 20)) (i64.const 12))",
    );
    assert_public_value(&wasm, &[add_import()], 42);
}

#[test]
fn void_hook_returns_to_the_caller() {
    let hook =
        Import { module: "hooks", name: "beforeFirstIO", params: &[], results: &[], emit: |_| {} };
    let wasm =
        module(r#"(import "hooks" "beforeFirstIO" (func $hook))"#, "(call $hook) (i64.const 42)");
    assert_public_value(&wasm, &[hook], 42);
}

#[test]
fn custom_i32_results_use_signed_wasm_representation() {
    let negative = Import {
        module: "example",
        name: "negative",
        params: &[],
        results: &[I32],
        emit: |code| code.load_imm_to_reg(REG_RET, 0xffff_ffff),
    };
    let wasm = module(
        r#"(import "example" "negative" (func $negative (result i32)))"#,
        "(i64.extend_i32_u (i32.lt_s (call $negative) (i32.const 0)))",
    );
    assert_public_value(&wasm, &[negative], 1);
}

#[test]
fn custom_import_trap_fails_execution() {
    let trap =
        Import { module: "example", name: "abort", params: &[], results: &[], emit: Code::trap };
    let wasm =
        module(r#"(import "example" "abort" (func $abort))"#, "(call $abort) (i64.const 42)");
    let rom = wasm2rom_with_imports(&wasm, &[trap]).unwrap();
    assert!(
        ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>).is_err()
    );
}

#[test]
fn unregistered_module_or_function_is_rejected() {
    for (namespace, name) in [("unknown", "add"), ("example", "unknown")] {
        let wasm = module(
            &format!(r#"(import "{namespace}" "{name}" (func $add (param i64 i64) (result i64)))"#),
            "(call $add (i64.const 40) (i64.const 2))",
        );
        assert!(wasm2rom_with_imports(&wasm, &[add_import()]).is_err());
    }
}

#[test]
fn signature_mismatches_are_rejected_before_emission() {
    for signature in [
        "(param i32 i64) (result i64)",
        "(param i64) (result i64)",
        "(param i64 i64 i64) (result i64)",
        "(param i64 i64) (result i32)",
        "(param i64 i64)",
    ] {
        let mut registration = add_import();
        registration.emit = |_| panic!("mismatched signature reached the emitter");
        let wasm = module(
            &format!(r#"(import "example" "add" (func $add {signature}))"#),
            "(i64.const 42)",
        );
        let error = wasm2rom_with_imports(&wasm, &[registration])
            .err()
            .expect("signature rejected")
            .to_string();
        assert!(error.contains("signature"), "{signature}: {error}");
    }
}

#[test]
fn duplicate_registrations_are_rejected() {
    let wasm = module("", "(i64.const 42)");
    let error = wasm2rom_with_imports(&wasm, &[add_import(), add_import()])
        .err()
        .expect("duplicate rejected")
        .to_string();
    assert!(error.contains("duplicate"), "{error}");
}

#[test]
fn custom_registration_cannot_override_wasi() {
    let mut registration = add_import();
    registration.module = "wasi_snapshot_preview1";
    registration.name = "fd_write";
    registration.emit = |_| panic!("reserved registration reached the emitter");
    let error = wasm2rom_with_imports(&module("", "(i64.const 42)"), &[registration])
        .err()
        .expect("reserved namespace rejected")
        .to_string();
    assert!(error.contains("reserved"), "{error}");
}
