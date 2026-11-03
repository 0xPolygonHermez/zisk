//! The ZisK precompiles as `env` imports of a wasm guest: each import lowers to the precompile's
//! own Zisk instruction, so the results must match the reference implementations bit for bit,
//! and a bad pointer must be a trap rather than an emulator panic.
//!
//! Guest convention: the module stores its operands in linear memory, calls the import, and
//! prints the first `len` result bytes through `fd_write`; the test reads them back from the
//! public output (which holds 256 bytes, enough for every result here).

use zisk_common::EmuTrace;
use zisk_transpiler_wasm::wasm2rom;
use ziskemu::{Emu, EmuOptions, ZiskEmulator};

/// Builds a guest importing `env::{name}` with the given param/result types, running `body`, and
/// printing `print_len` bytes from linear address `print_from`.
fn guest(name: &str, ty: &str, body: &str, print_from: u32, print_len: u32) -> Vec<u8> {
    let wat = format!(
        r#"(module
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (import "env" "{name}" (func $pre {ty}))
          (memory 1)
          (func (export "_start")
            {body}
            (i32.store (i32.const 0) (i32.const {print_from}))
            (i32.store (i32.const 4) (i32.const {print_len}))
            (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 40)))))"#
    );
    wat::parse_str(wat).expect("wat")
}

/// Runs a guest to completion; `Ok(output)` or `Err(())` when it trapped.
fn run(wasm: &[u8]) -> Result<Vec<u8>, ()> {
    let rom = wasm2rom(wasm).expect("wasm2rom");
    let mut emu = Emu::new(&rom);
    emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
    assert!(emu.terminated(), "emulation did not terminate");
    if emu.ctx.inst_ctx.error {
        Err(())
    } else {
        Ok(emu.get_output_8())
    }
}

/// `i64.store` instructions placing `words` at linear address `at`.
fn store_words(at: u32, words: &[u64]) -> String {
    words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            format!("(i64.store (i32.const {}) (i64.const {}))", at + 8 * i as u32, *w as i64)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn words_of(bytes: &[u8]) -> Vec<u64> {
    bytes.chunks(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
}

#[test]
fn keccak_f_matches_the_reference_permutation() {
    let mut state = [0u64; 25];
    for (i, w) in state.iter_mut().enumerate() {
        *w = 0x0123_4567_89ab_cdef_u64.wrapping_mul(i as u64 + 1) ^ 0xdead_beef;
    }
    let mut expected = state;
    tiny_keccak::keccakf(&mut expected);

    let body = format!("{}\n(call $pre (i32.const 1024))", store_words(1024, &state));
    let out = run(&guest("keccak_f", "(param i32)", &body, 1024, 200)).expect("no trap");
    assert_eq!(words_of(&out[..200]), expected);
}

#[test]
fn sha256_f_matches_the_reference_compression() {
    // state: the SHA-256 IV as 4 little-endian u64 of two u32 each (the precompile's layout:
    // h0 in the low half of word 0); block: 64 bytes of a known message.
    let iv: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let block: [u8; 64] = core::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(3));

    let mut expected = iv;
    sha2::compress256(&mut expected, &[block.into()]);
    let pack = |h: &[u32; 8]| -> Vec<u64> {
        h.chunks(2).map(|p| u64::from(p[0]) | (u64::from(p[1]) << 32)).collect()
    };

    let body = format!(
        "{}\n{}\n(call $pre (i32.const 1024) (i32.const 2048))",
        store_words(1024, &pack(&iv)),
        store_words(2048, &words_of(&block))
    );
    let out = run(&guest("sha256_f", "(param i32 i32)", &body, 1024, 32)).expect("no trap");
    assert_eq!(words_of(&out[..32]), pack(&expected));
}

/// secp256k1's generator and its double, as the precompiles' 4-limb little-endian coordinates.
const G: [u64; 8] = [
    0x59f2815b16f81798,
    0x029bfcdb2dce28d9,
    0x55a06295ce870b07,
    0x79be667ef9dcbbac, // x
    0x9c47d08ffb10d4b8,
    0xfd17b448a6855419,
    0x5da4fbfc0e1108a8,
    0x483ada7726a3c465, // y
];
const TWO_G: [u64; 8] = [
    0xabac09b95c709ee5,
    0x5c778e4b8cef3ca7,
    0x3045406e95c07cd8,
    0xc6047f9441ed7d6d,
    0x236431a950cfe52a,
    0xf7f632653266d0e1,
    0xa3c58419466ceaee,
    0x1ae168fea63dc339,
];

#[test]
fn secp256k1_dbl_and_add_agree_with_the_curve() {
    // dbl(G) == 2G
    let body = format!("{}\n(call $pre (i32.const 1024))", store_words(1024, &G));
    let out = run(&guest("secp256k1_dbl", "(param i32)", &body, 1024, 64)).expect("no trap");
    assert_eq!(words_of(&out[..64]), TWO_G);
    // add(2G, G) == 3G, and add(G, 2G) gives the same point.
    let three_g = {
        let body = format!(
            "{}\n{}\n(call $pre (i32.const 1024) (i32.const 2048))",
            store_words(1024, &TWO_G),
            store_words(2048, &G)
        );
        let out =
            run(&guest("secp256k1_add", "(param i32 i32)", &body, 1024, 64)).expect("no trap");
        words_of(&out[..64])
    };
    let body = format!(
        "{}\n{}\n(call $pre (i32.const 1024) (i32.const 2048))",
        store_words(1024, &G),
        store_words(2048, &TWO_G)
    );
    let out = run(&guest("secp256k1_add", "(param i32 i32)", &body, 1024, 64)).expect("no trap");
    assert_eq!(words_of(&out[..64]), three_g);
    assert_eq!(
        three_g[..4],
        [0x8601f113bce036f9, 0xb531c845836f99b0, 0x49344f85f89d5229, 0xf9308a019258c310],
        "3G.x"
    );
}

#[test]
fn arith256_mod_takes_its_operands_through_a_block() {
    // d = (a * b + c) mod m with small values: (7 * 9 + 5) mod 11 = 68 mod 11 = 2.
    let a = [7u64, 0, 0, 0];
    let b = [9u64, 0, 0, 0];
    let c = [5u64, 0, 0, 0];
    let m = [11u64, 0, 0, 0];
    let d = [0xffu64; 4]; // poisoned result slot
                          // Operands at 1024.., the block (wasm addresses, one per 8-byte slot) at 2048.
    let block = [1024u64, 1056, 1088, 1120, 1152];
    let body = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n(call $pre (i32.const 2048))",
        store_words(1024, &a),
        store_words(1056, &b),
        store_words(1088, &c),
        store_words(1120, &m),
        store_words(1152, &d),
        store_words(2048, &block)
    );
    let out = run(&guest("arith256_mod", "(param i32)", &body, 1152, 32)).expect("no trap");
    assert_eq!(words_of(&out[..32]), [2, 0, 0, 0]);
}

#[test]
fn add256_returns_the_carry() {
    // a + b + cin with a = 2^256 - 1, b = 0, cin = 1: c = 0, carry-out 1.
    let a = [u64::MAX; 4];
    let b = [0u64; 4];
    let c = [0x77u64; 4];
    let block = [1024u64, 1056, 1, 1088]; // a, b, cin (direct), c
    let body = format!(
        "{}\n{}\n{}\n{}\n(i64.store (i32.const 1120) (call $pre (i32.const 2048)))",
        store_words(1024, &a),
        store_words(1056, &b),
        store_words(1088, &c),
        store_words(2048, &block)
    );
    let out = run(&guest("add256", "(param i32) (result i64)", &body, 1088, 40)).expect("no trap");
    assert_eq!(words_of(&out[..40]), [0, 0, 0, 0, 1], "c then the carry-out");
}

#[test]
fn bad_pointers_trap_instead_of_crashing() {
    // Misaligned state.
    let body = "(call $pre (i32.const 1028))";
    assert!(run(&guest("keccak_f", "(param i32)", body, 1024, 8)).is_err(), "misaligned");
    // State past the end of the single page (200 bytes from 65400).
    let body = "(call $pre (i32.const 65400))";
    assert!(run(&guest("keccak_f", "(param i32)", body, 1024, 8)).is_err(), "out of bounds");
    // Second operand misaligned.
    let body = "(call $pre (i32.const 1024) (i32.const 2052))";
    assert!(run(&guest("sha256_f", "(param i32 i32)", body, 1024, 8)).is_err(), "p2 misaligned");
    // A block entry pointing past the end, and a misaligned block entry.
    for bad in [65520u64, 1028] {
        let block = [1024u64, 1056, 1088, 1120, bad];
        let body = format!("{}\n(call $pre (i32.const 2048))", store_words(2048, &block));
        assert!(
            run(&guest("arith256_mod", "(param i32)", &body, 1024, 8)).is_err(),
            "block entry {bad}"
        );
    }
    // Nothing was written through the bad pointers: the guest never gets that far, the trap is
    // the only observable effect (checked above via the error flag).
}

#[test]
fn imports_are_checked_at_transpile_time() {
    for (what, wat) in [
        (
            "unknown name",
            r#"(module (import "env" "keccak_g" (func (param i32))) (func (export "_start")))"#,
        ),
        (
            "wrong arity",
            r#"(module (import "env" "keccak_f" (func (param i32 i32))) (func (export "_start")))"#,
        ),
        (
            "wrong type",
            r#"(module (import "env" "keccak_f" (func (param i64))) (func (export "_start")))"#,
        ),
        (
            "unexpected result",
            r#"(module (import "env" "sha256_f" (func (param i32 i32) (result i32))) (func (export "_start")))"#,
        ),
        (
            "other module",
            r#"(module (import "zisk" "keccak_f" (func (param i32))) (func (export "_start")))"#,
        ),
    ] {
        let err = wasm2rom(&wat::parse_str(wat).unwrap()).expect_err(what);
        assert!(
            err.to_string().contains("env") || err.to_string().contains("module"),
            "{what}: {err}"
        );
    }
    // The process_rom path sees the same ROM: a well-typed keccak_f import transpiles.
    let ok = guest("keccak_f", "(param i32)", "(call $pre (i32.const 1024))", 1024, 8);
    let rom = wasm2rom(&ok).expect("well-typed import");
    let _ = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("runs");
}

/// The Go package (`ziskasm/lang/go/precompiles`) on a real Go guest: `example/keccak`, prebuilt
/// at `tests/data/wasm-go-keccak.wasm`, hashes its stdin with Keccak-256 and SHA-256 (Go sponges
/// around `precompiles.KeccakF` and `precompiles.Sha256F`) and prints both digests in hex.
/// Requires `float`: the Go runtime executes f64 arithmetic.
#[cfg(feature = "float")]
#[test]
fn go_bindings_hash_through_the_precompiles() {
    use sha2::Digest;
    use tiny_keccak::Hasher;

    let wasm =
        std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/wasm-go-keccak.wasm"))
            .expect("read the committed Go guest");
    let rom = wasm2rom(&wasm).expect("wasm2rom");
    // Empty, one short block, and inputs spanning several Keccak (136 B) and SHA (64 B) blocks
    // with every padding shape.
    for len in [0usize, 12, 55, 56, 63, 64, 135, 136, 137, 1000] {
        let data: Vec<u8> = (0..len).map(|i| (i * 131 % 251) as u8).collect();
        let mut input = Vec::new();
        input.extend_from_slice(&(len as u64).to_le_bytes());
        input.extend_from_slice(&data);
        input.resize(input.len().div_ceil(8) * 8, 0);
        let out =
            ZiskEmulator::process_rom(&rom, &input, &EmuOptions::default(), None::<fn(EmuTrace)>)
                .expect("emulation");
        let end = out.iter().position(|&b| b == 0).unwrap_or(out.len());
        let text = std::str::from_utf8(&out[..end]).expect("utf-8");
        let mut keccak = tiny_keccak::Keccak::v256();
        keccak.update(&data);
        let mut expected_keccak = [0u8; 32];
        keccak.finalize(&mut expected_keccak);
        let expected_sha = sha2::Sha256::digest(&data);
        assert_eq!(
            text,
            format!("{}\n{}\n", hex(&expected_keccak), hex(&expected_sha)),
            "input of {len} bytes"
        );
    }
}

#[cfg(feature = "float")]
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
