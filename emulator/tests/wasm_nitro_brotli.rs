//! Execute the pinned C Brotli codec through the Wasm-to-native bridge.
use zisk_common::EmuTrace;
use zisk_transpiler_wasm::nitro;
use ziskemu::{EmuOptions, ZiskEmulator};

const TEXT: &[u8] = b"Nitro Brotli native runtime: input input input input";
// Pinned Brotli f4153a09, C streaming Finish, quality11/window22, empty dictionary.
const COMPRESSED: &str = "1b3300f81d09760ce18d174817462de42457a74174dce8741adb9b1c98c8a106e459bbd9fb80139ee007e5832ddfd4a97400";
fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn bytes(data: &[u8]) -> String {
    data.iter().map(|b| format!("\\{b:02x}")).collect()
}
fn execute(data: &str, body: &str, out: u32) -> Result<Vec<u8>, String> {
    execute_with_step_limit(data, body, out, 500_000_000)
}
fn execute_with_step_limit(
    data: &str,
    body: &str,
    out: u32,
    max_steps: u64,
) -> Result<Vec<u8>, String> {
    let elf = std::fs::read("../target/nitro-runtime/nitro-runtime.elf")
        .expect("run runtime/build.py first");
    let wasm = wat::parse_str(format!(r#"(module
      (import "arbcompress" "brotli_decompress" (func $decode (param i32 i32 i32 i32 i32) (result i32)))
      (import "arbcompress" "brotli_compress" (func $encode (param i32 i32 i32 i32 i32 i32 i32) (result i32)))
      (import "wavmio" "setGlobalStateBytes32" (func $commit (param i32 i32)))
      (memory 1)
      {data}
      (func $assert (param $value i32) (if (i32.eqz (local.get $value)) (then unreachable)))
      (func (export "_start") {body} (call $commit (i32.const 0) (i32.const {out})) ))"#)).unwrap();
    let rom = nitro::wasm2rom(&wasm, &elf).map_err(|e| e.to_string())?;
    let mut witness = b"NZKWIT01".to_vec();
    witness.extend_from_slice(&[0; 172]);
    let mut input = (witness.len() as u64).to_le_bytes().to_vec();
    input.extend(witness);
    input.resize(input.len().next_multiple_of(8), 0);
    let options = EmuOptions { max_steps, log_metrics: true, ..Default::default() };
    ZiskEmulator::process_rom(&rom, &input, &options, None::<fn(EmuTrace)>)
        .map_err(|e| e.to_string())
}
fn fixture() -> String {
    format!(r#"(data (i32.const 128) "{}")"#, bytes(&hex(COMPRESSED)))
}
fn decode(input: i32, len: usize, output: i32) -> String {
    format!("(call $decode (i32.const {input}) (i32.const {len}) (i32.const {output}) (i32.const 16) (i32.const 0))")
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn known_decompression_and_input_output_aliases_execute_in_vm() {
    for out in [128, 131, 1025] {
        let body = format!("(i32.store (i32.const 16) (i32.const 512)) (call $assert {}) (call $assert (i32.eq (i32.load (i32.const 16)) (i32.const {})))", decode(128, COMPRESSED.len()/2, out), TEXT.len());
        let result = execute(&fixture(), &body, out as u32).expect("native decoding");
        assert_eq!(result.len(), 256);
        assert_eq!(&result[112..144], &TEXT[..32]);
    }
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn quality11_compression_matches_the_pinned_codec_in_vm() {
    let expected = hex(COMPRESSED);
    let data = format!(
        r#"(data (i32.const 128) "{}") (data (i32.const 2048) "{}")"#,
        bytes(TEXT),
        bytes(&expected)
    );
    let mut body = format!("(i32.store (i32.const 16) (i32.const 512)) (call $assert (call $encode (i32.const 128) (i32.const {}) (i32.const 1025) (i32.const 16) (i32.const 11) (i32.const 22) (i32.const 0)))", TEXT.len());
    body += &format!(
        " (call $assert (i32.eq (i32.load (i32.const 16)) (i32.const {})))",
        expected.len()
    );
    for i in 0..expected.len() {
        body += &format!(
            " (call $assert (i32.eq (i32.load8_u (i32.const {})) (i32.load8_u (i32.const {}))))",
            1025 + i,
            2048 + i
        );
    }
    let result = execute(&data, &body, 1025).expect("native quality11 compression");
    assert_eq!(&result[112..144], &expected[..32]);
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn failed_decoding_preserves_output_and_length_in_vm() {
    let mut data = fixture();
    data += r#" (data (i32.const 2048) "sentinel") (data (i32.const 512) "\ff\ff\ff\ff")"#;
    let mut body = String::new();
    for (input, len, capacity, out) in [
        (128, COMPRESSED.len() / 2, 1, 2048),
        (128, COMPRESSED.len() / 2 - 1, 512, 2048),
        (512, 4, 512, -1),
    ] {
        body += &format!(" (i32.store (i32.const 16) (i32.const {capacity})) (call $assert (i32.eqz {})) (call $assert (i32.eq (i32.load (i32.const 16)) (i32.const {capacity})))", decode(input, len, out));
    }
    let result = execute(&data, &body, 2048).expect("decode errors return failure");
    assert_eq!(&result[112..120], b"sentinel");
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn successful_decode_checks_output_and_unsigned_input_bounds_in_vm() {
    for (input, len, out) in [(128, COMPRESSED.len() / 2, 65530), (-1, 1, 1024), (65535, 2, 1024)] {
        let body = format!(
            "(i32.store (i32.const 16) (i32.const 512)) (drop {})",
            decode(input, len, out)
        );
        assert!(execute(&fixture(), &body, 1024).unwrap_err().to_lowercase().contains("trap"));
    }
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn large_reserved_capacity_only_allocates_actual_decoded_bytes() {
    let data = r#"(data (i32.const 128) "\0b\02\80hello\03")"#;
    let body = format!("(i32.store (i32.const 16) (i32.const -1)) (call $assert {}) (call $assert (i32.eq (i32.load (i32.const 16)) (i32.const 5)))", decode(128, 9, 1024));
    let output = execute(data, &body, 1024).expect("large capacity is only a limit");
    assert_eq!(&output[112..117], b"hello");
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn decoder_grows_scratch_beyond_64k_in_vm() {
    // Brotli quality11/window22 fixture for exactly 131072 ASCII A bytes.
    let data =
        format!(r#"(data (i32.const 128) "{}")"#, bytes(&hex("5bffff815f22281e0b0472ef0300")));
    let body = format!(
        r#"
      (local $i i32)
      (drop (memory.grow (i32.const 2)))
      (i32.store (i32.const 16) (i32.const -1))
      (call $assert {})
      (call $assert (i32.eq (i32.load (i32.const 16)) (i32.const 131072)))
      (block $done (loop $check
        (br_if $done (i32.eq (local.get $i) (i32.const 131072)))
        (call $assert (i32.eq (i32.load8_u (i32.add (i32.const 1024) (local.get $i))) (i32.const 65)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $check)))"#,
        decode(128, 14, 1024)
    );
    let output = execute(&data, &body, 1024).expect("streaming decoder grows scratch");
    assert_eq!(&output[112..144], &[b'A'; 32]);
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn quality11_encoder_grows_scratch_and_matches_known_literal_stream_in_vm() {
    // Pinned one-shot C reference: 8bffff + xorshift32(seed7) bytes + 03.
    let mut state = 7u32;
    let input: Vec<u8> = (0..65536)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as u8
        })
        .collect();
    let data = format!(r#"(data (i32.const 0) "{}")"#, bytes(&input));
    let body = r#"
      (local $i i32)
      (drop (memory.grow (i32.const 3)))
      (i32.store (i32.const 196608) (i32.const -1))
      (call $assert (call $encode (i32.const 0) (i32.const 65536) (i32.const 65536)
        (i32.const 196608) (i32.const 11) (i32.const 22) (i32.const 0)))
      (call $assert (i32.eq (i32.load (i32.const 196608)) (i32.const 65540)))
      (call $assert (i32.eq (i32.load8_u (i32.const 65536)) (i32.const 139)))
      (call $assert (i32.eq (i32.load16_u (i32.const 65537)) (i32.const 65535)))
      (call $assert (i32.eq (i32.load8_u (i32.const 131075)) (i32.const 3)))
      (block $done (loop $check
        (br_if $done (i32.eq (local.get $i) (i32.const 65536)))
        (call $assert (i32.eq (i32.load8_u (local.get $i))
          (i32.load8_u (i32.add (i32.const 65539) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $check)))"#;
    // This 64 KiB quality 11 fixture measured 2.88B steps; keep its 5B budget local.
    let output = execute_with_step_limit(&data, body, 65536, 5_000_000_000)
        .expect("streaming quality11 encoder grows scratch");
    let mut expected = vec![0x8b, 0xff, 0xff];
    expected.extend_from_slice(&input[..29]);
    assert_eq!(&output[112..144], expected);
}
