//! Execute canonical Stylus through the same native boundary used by Nitro replay.
use zisk_common::EmuTrace;
use zisk_transpiler_wasm::nitro;
use ziskemu::{Emu, EmuOptions};

const TEXT: &[u8] = b"hello from canonical Stylus";

fn bytes(data: &[u8]) -> String {
    data.iter().map(|byte| format!("\\{byte:02x}")).collect()
}

fn execute(after_activation: &str) -> Result<Vec<u8>, String> {
    let user = wat::parse_str(
        r#"(module
      (import "vm_hooks" "read_args" (func $read (param i32)))
      (import "vm_hooks" "write_result" (func $write (param i32 i32)))
      (memory (export "memory") 1 1)
      (func (export "user_entrypoint") (param i32) (result i32)
        (call $read (i32.const 0))
        (call $write (i32.const 0) (local.get 0))
        (i32.const 0)))"#,
    )
    .unwrap();
    // Computed by the pinned Nitro implementation using software Keccak on the host.
    let expected_hash = "d76fdf042cf0e9c707430c68e9c1c6ca89a321e7bd789d0fc347fbeb237f20c3";
    let hash_checks: String = (0..32)
        .map(|i| {
            let byte = u8::from_str_radix(&expected_hash[2 * i..2 * i + 2], 16).unwrap();
            format!(
                "(call $assert (i32.eq (i32.load8_u (i32.const {})) (i32.const {byte})))",
                128 + i
            )
        })
        .collect();
    let wasm = wat::parse_str(format!(
        r#"(module
      (import "programs" "activate_v2" (func $activate
        (param i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32 i32 i32) (result i32)))
      (import "programs" "create_stylus_config" (func $config (param i32 i32 i32 i32) (result i64)))
      (import "programs" "create_evm_data_v2" (func $evm
        (param i64 i32 i64 i32 i64 i64 i64 i32 i32 i32 i32 i32 i32 i32 i32) (result i64)))
      (import "programs" "new_program" (func $new (param i32 i32 i32 i64 i64 i64) (result i32)))
      (import "programs" "start_program" (func $start (param i32) (result i32)))
      (import "programs" "get_request" (func $request (param i32 i32) (result i32)))
      (import "programs" "get_request_data" (func $request_data (param i32 i32)))
      (import "programs" "pop" (func $pop))
      (import "programs" "program_requires_prepare" (func $requires (param i32) (result i32)))
      (import "programs" "program_prepare" (func $prepare (param i32 i64 i32 i32 i32 i32 i32 i32)))
      (import "wavmio" "setGlobalStateBytes32" (func $commit (param i32 i32)))
      (memory 1)
      (data (i32.const 2048) "{}")
      (data (i32.const 4096) "{}")
      (func $assert (param i32) (if (i32.eqz (local.get 0)) (then unreachable)))
      (func (export "_start") (local $id i32)
        (call $assert (call $requires (i32.const 128)))
        (i32.store16 (i32.const 64) (i32.const 128))
        (i64.store (i32.const 160) (i64.const 1000000))
        (call $assert (i32.eqz (call $activate
          (i32.const 2048) (i32.const {}) (i32.const 64) (i32.const 68)
          (i32.const 72) (i32.const 74) (i32.const 2) (i64.const 60) (i32.const 0)
          (i32.const 96) (i32.const 128) (i32.const 160) (i32.const 192) (i32.const 1024))))
        {hash_checks}
        (call $assert (i64.eq (i64.load (i32.const 160)) (i64.const 935684)))
        (call $assert (i32.eq (i32.load16_u (i32.const 64)) (i32.const 1)))
        (call $assert (i32.eqz (call $requires (i32.const 128))))
        (i64.store (i32.const 32) (i64.const 2048))
        (i64.store (i32.const 40) (i64.const {}))
        (i64.store (i32.const 48) (i64.const {}))
        {after_activation}
        (call $prepare (i32.const 32) (i64.const {}) (i32.const 128) (i32.const 96)
          (i32.const 65536) (i32.const 128) (i32.const 0) (i32.const 2))
        (local.set $id (call $start (call $new
          (i32.const 128) (i32.const 4096) (i32.const {})
          (call $config (i32.const 2) (i32.const 65536) (i32.const 10000) (i32.const 0))
          (call $evm (i64.const 60) (i32.const 96) (i64.const 42161) (i32.const 96)
            (i64.const 100000000) (i64.const 1) (i64.const 1) (i32.const 96)
            (i32.const 128) (i32.const 96) (i32.const 96) (i32.const 96)
            (i32.const 96) (i32.const 0) (i32.const 0))
          (i64.const 1000000))))
        (call $assert (i32.eqz (call $request (local.get $id) (i32.const 64))))
        (call $assert (i32.eq (i32.load (i32.const 64)) (i32.const {})))
        (call $request_data (local.get $id) (i32.const 8192))
        (call $pop)
        (call $commit (i32.const 0) (i32.const 8200))))"#,
        bytes(&user),
        bytes(TEXT),
        user.len(),
        user.len(),
        user.len(),
        user.len(),
        TEXT.len(),
        TEXT.len() + 8
    ))
    .unwrap();
    let path = std::env::var("NITRO_RUNTIME_ELF")
        .unwrap_or_else(|_| "../target/nitro-runtime/nitro-runtime.elf".to_owned());
    let elf = std::fs::read(path).expect("run nitro_tools build first");
    let rom = nitro::wasm2rom(&wasm, &elf).map_err(|error| error.to_string())?;
    let mut witness = b"NZKWIT01".to_vec();
    witness.extend_from_slice(&[0; 172]);
    let mut input = (witness.len() as u64).to_le_bytes().to_vec();
    input.extend(witness);
    input.resize(input.len().next_multiple_of(8), 0);
    let options = EmuOptions { max_steps: 2_000_000_000, log_metrics: true, ..Default::default() };
    let mut emu = Emu::new(&rom);
    emu.run(input, &options, None::<fn(EmuTrace)>);
    if let Some((reason, step, pc)) = emu.failure() {
        return Err(format!("{reason:?} at step {step}, pc={pc:#x}, ra={:#x}", emu.get_reg(1)));
    }
    if !emu.terminated() {
        return Err(format!(
            "step limit exceeded at pc={:#x}, ra={:#x}, args={:x?}",
            emu.ctx.inst_ctx.pc,
            emu.get_reg(1),
            (10..18).map(|reg| emu.get_reg(reg)).collect::<Vec<_>>()
        ));
    }
    eprintln!("Stylus execution: {} steps", emu.number_of_steps());
    Ok(emu.get_output_8())
}

#[test]
#[ignore = "requires built native Stylus runtime ELF and float feature"]
fn canonical_stylus_activation_preparation_and_execution_run_inside_zisk() {
    let output = execute("").expect("native Stylus execution");
    assert_eq!(&output[112..112 + TEXT.len()], TEXT);
}

#[test]
#[ignore = "requires built native Stylus runtime ELF and float feature"]
fn native_stylus_rejects_wrong_module_hash_and_malformed_go_slice() {
    for alteration in [
        "(i32.store8 (i32.const 128) (i32.xor (i32.load8_u (i32.const 128)) (i32.const 1)))",
        "(i64.store (i32.const 48) (i64.const 0))",
        "(i64.store (i32.const 32) (i64.const 4294967296))",
    ] {
        let error = execute(alteration).expect_err("untrusted preparation input must fail");
        assert!(error.to_lowercase().contains("trap"), "{error}");
    }
}
