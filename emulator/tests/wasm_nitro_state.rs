//! Native state controls. Build transpilers/wasm/runtime/build.py before running ignored tests.
use std::io::Write;
use std::process::{Command, Stdio};
use zisk_common::EmuTrace;
use zisk_transpiler_wasm::nitro;
use ziskemu::{EmuOptions, ZiskEmulator};

const KECCAK_ABC: &str = "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45";
const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
// Fixed Keccak vectors over domain | zero start80 | end80 | zero anchors64 | zero counts16.
// end80 = (abc + 29 zero bytes) twice | u64le(5) | u64le(0).
// Generated independently with prepare-witness.py's oracle, crosschecked by state-tests.c.
const SEQ_CLAIM: &str = "fd571e8218aaa0453516ccaafa17e3f969b33fbb08ca90372b829f25115b514b";
const DELAYED_CLAIM: &str = "dc0baf2b7277e484922d1a0b18651efa454e1ef57b866953a93a1abf6c704aa6";

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn wat_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("\\{byte:02x}")).collect()
}

fn module(offset: u32, exit: u32) -> Vec<u8> {
    wat::parse_str(format!(r#"(module
      (import "wavmio" "resolveTypedPreimage" (func $resolve (param i32 i32 i32 i32) (result i32)))
      (import "wavmio" "setGlobalStateBytes32" (func $set32 (param i32 i32)))
      (import "wavmio" "setGlobalStateU64" (func $set64 (param i32 i64)))
      (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
      (memory 1)
      (data (i32.const 128) "{}")
      (data (i32.const 160) "{}")
      (func $assert (param $value i32) (if (i32.eqz (local.get $value)) (then unreachable)))
      (func (export "_start")
        (call $assert (i32.eq (call $resolve (i32.const 0) (i32.const 128) (i32.const {offset}) (i32.const 64)) (i32.const 3)))
        (call $assert (i32.eq (call $resolve (i32.const 1) (i32.const 160) (i32.const 0) (i32.const 96)) (i32.const 3)))
        (call $assert (i32.eq (i32.and (i32.load (i32.const 64)) (i32.const 16777215)) (i32.const 6513249)))
        (call $assert (i32.eq (i32.and (i32.load (i32.const 96)) (i32.const 16777215)) (i32.const 6513249)))
        (call $set32 (i32.const 0) (i32.const 64))
        (call $set32 (i32.const 1) (i32.const 96))
        (call $set64 (i32.const 0) (i64.const 5))
        (call $exit (i32.const {exit}))
        unreachable))"#, wat_bytes(&hex(KECCAK_ABC)), wat_bytes(&hex(SHA256_ABC)))).unwrap()
}

fn witness(keccak: Option<&[u8]>, sha256: Option<&[u8]>) -> Vec<u8> {
    let entries: Vec<_> = [(0u8, KECCAK_ABC, keccak), (1u8, SHA256_ABC, sha256)]
        .into_iter()
        .filter_map(|(kind, key, value)| value.map(|value| (kind, key, value)))
        .collect();
    let mut payload = b"NZKWIT01".to_vec();
    payload.extend_from_slice(&[0; 160]);
    payload.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    payload.extend_from_slice(&[0; 8]);
    for (kind, key, value) in entries {
        payload.push(kind);
        payload.extend(hex(key));
        payload.extend_from_slice(&(value.len() as u32).to_le_bytes());
        payload.extend_from_slice(value);
    }
    let mut framed = (payload.len() as u64).to_le_bytes().to_vec();
    framed.extend(payload);
    framed.resize(framed.len().next_multiple_of(8), 0);
    framed
}

fn execute(wasm: &[u8], witness: &[u8]) -> Result<Vec<u8>, String> {
    let elf = std::fs::read("../target/nitro-runtime/nitro-runtime.elf")
        .expect("run transpilers/wasm/runtime/build.py first");
    let rom = nitro::wasm2rom(wasm, &elf).map_err(|error| error.to_string())?;
    let options = EmuOptions { max_steps: 30_000_000, ..Default::default() };
    ZiskEmulator::process_rom(&rom, witness, &options, None::<fn(EmuTrace)>)
        .map_err(|error| error.to_string())
}

fn expected(wasm: &[u8]) -> Vec<u8> {
    // Python hashlib is independent of the compiler's module hashing and VM crypto.
    let mut child = Command::new("python3")
        .args(["-c", "import hashlib,sys;sys.stdout.buffer.write(hashlib.sha256(sys.stdin.buffer.read()).digest())"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("python3 for independent SHA-256");
    child.stdin.take().unwrap().write_all(wasm).unwrap();
    let hashed = child.wait_with_output().unwrap();
    assert!(hashed.status.success());
    assert_eq!(hashed.stdout.len(), 32);
    let mut output = hashed.stdout;
    output.extend_from_slice(&[0; 80]);
    for _ in 0..2 {
        output.extend_from_slice(b"abc");
        output.extend_from_slice(&[0; 29]);
    }
    output.extend_from_slice(&5u64.to_le_bytes());
    output.extend_from_slice(&0u64.to_le_bytes());
    output.extend(hex(SEQ_CLAIM));
    output.extend(hex(DELAYED_CLAIM));
    assert_eq!(output.len(), 256);
    output
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn authenticated_keccak_and_sha256_preimages_finalize_all_256_claim_bytes() {
    let wasm = module(0, 0);
    let output = execute(&wasm, &witness(Some(b"abc"), Some(b"abc"))).unwrap();
    assert_eq!(output, expected(&wasm));
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn corrupt_used_keccak_or_sha256_preimage_traps_inside_vm() {
    let wasm = module(0, 0);
    for input in [witness(Some(b"abd"), Some(b"abc")), witness(Some(b"abc"), Some(b"abd"))] {
        let error = execute(&wasm, &input).unwrap_err();
        assert!(error.to_lowercase().contains("trap"), "{error}");
    }
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn missing_required_preimage_traps_inside_vm() {
    let error = execute(&module(0, 0), &witness(None, Some(b"abc"))).unwrap_err();
    assert!(error.to_lowercase().contains("trap"), "{error}");
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn unaligned_preimage_offset_traps_inside_vm() {
    let error = execute(&module(1, 0), &witness(Some(b"abc"), Some(b"abc"))).unwrap_err();
    assert!(error.to_lowercase().contains("trap"), "{error}");
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn nonzero_proc_exit_rejects_instead_of_finalizing_claim() {
    let error = execute(&module(0, 1), &witness(Some(b"abc"), Some(b"abc"))).unwrap_err();
    assert!(error.to_lowercase().contains("trap"), "{error}");
}

fn certificate_module(kind: u32) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
          (import "wavmio" "validateCertificate" (func $certificate (param i32 i32) (result i32)))
          (import "wavmio" "setGlobalStateU64" (func $set (param i32 i64)))
          (memory 1)
          (data (i32.const 128) "{}")
          (func (export "_start")
            (if (i32.eqz (call $certificate (i32.const {kind}) (i32.const 128)))
              (then (call $set (i32.const 0) (i64.const 5))))))"#,
        wat_bytes(&hex(if kind as u8 == 1 { SHA256_ABC } else { KECCAK_ABC }))
    ))
    .unwrap()
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn missing_custom_da_certificate_cannot_finalize_an_empty_batch() {
    // Nitro's CustomDA reader treats false as an empty payload without an error.
    let error = execute(&certificate_module(3), &witness(None, None)).unwrap_err();
    assert!(error.to_lowercase().contains("trap"), "{error}");
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn certificate_validation_traps_for_present_and_absent_witnesses() {
    for kind in [0, 1, 2, 3, 255, 257, 259] {
        let wasm = certificate_module(kind);
        let mut present =
            if kind as u8 == 1 { witness(None, Some(b"abc")) } else { witness(Some(b"abc"), None) };
        // The sole entry starts after the framing word and 180-byte header.
        present[8 + 180] = kind as u8;
        for input in [&present, &witness(None, None)] {
            let error = execute(&wasm, input).unwrap_err();
            assert!(error.to_lowercase().contains("trap"), "type {kind}: {error}");
        }
    }
}
