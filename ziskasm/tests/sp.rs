//! End-to-end tests for the ZisK `sp` register: the `sp` a-source, sp-relative memory
//! operands (`[sp ± N]`) on a, b and the store, sp-relative indirections
//! (`W[a + sp ± N]`) on b and the store, and the `setsp` / `incsp(N)` updates, which
//! take effect after the store, so the instruction itself still sees the old sp.
//!
//! Run with `cargo test -p zisk-asm --features sp`; the checks that also hold without the
//! feature are in `sp_syntax.rs`.
#![cfg(feature = "sp")]

use zisk_common::{BusId, DataBusTrait, EmuTrace};
use ziskasm::{assemble, parser::parse_program};
use ziskemu::{Emu, EmuOptions, ZiskEmulator};

/// sp-relative lines of [`SRC`], exactly as `ZiskInst::to_zisk_asm` renders them back.
const SP_LINES: &[&str] = &[
    "copyb(0x0, 0xc0100000), setsp",
    "copyb(0x0, 0xb) -> [sp + 0]",
    "copyb(0x0, 0x16) -> [sp - 8]",
    "add([sp + 0], [sp - 8]) -> [sp + 16]",
    "copyb(0x0, 0x0), incsp(-8)",
    "copyb(0x0, [sp + 24]) -> r5",
    "add(sp, 0x0) -> r6",
    "copyb(r9, 8[a + sp + 3]) -> r7",
    "copyb(r9, r5) -> 4[a + sp - 21]",
    "copyb(0x0, [sp - 16]) -> r8",
    "copyb(0x0, 0x1000), setsp, incsp(16)",
    "add(sp, 0x0) -> r12",
    "copyb(0x0, 0xc0100000), setsp, incsp(-8)",
    "add(sp, 0x0) -> r14",
];

const SRC: &str = "\
define OUTPUT_ADDR 0xc0410000
main:
\tcopyb(0, 0xc0100000), setsp                ; sp = 0xc0100000 (B)
\tcopyb(0, 11) -> [sp + 0]                   ; mem[B] = 11
\tcopyb(0, 22) -> [sp - 8]                   ; mem[B - 8] = 22
\tadd([sp + 0], [sp - 8]) -> [sp + 16]       ; mem[B + 16] = 33
\tcopyb(0, 0), incsp(-8)                     ; sp = B - 8
\tcopyb(0, [sp + 24]) -> r5                  ; r5 = mem[B + 16] = 33
\tadd(sp, 0) -> r6                           ; r6 = B - 8
\tcopyb(0, 5) -> r9                          ; r9 = 5
\tcopyb(r9, 8[a + sp + 3]) -> r7             ; r7 = mem[5 + B - 8 + 3] = mem[B] = 11
\tcopyb(r9, r5) -> 4[a + sp - 21]            ; mem32[5 + B - 8 - 21] = mem32[B - 24] = 33
\tcopyb(0, [sp - 16]) -> r8                  ; r8 = mem[B - 24]; its low 32 bits are 33 ...
\tand(r8, 0xffffffff) -> r8                  ; ... and nothing else was ever written there
\tcopyb(0, 0x1000), setsp, incsp(16)         ; sp = c + 16 = 0x1010
\tadd(sp, 0) -> r12                          ; r12 = 0x1010
\tcopyb(0, 0xc0100000), setsp, incsp(-8)     ; sp = B - 8: the store below sees the old sp
\tcopyb(0, OUTPUT_ADDR) -> r13
\tcopyb(r13, r5) -> 8[a + 0]
\tcopyb(r13, r6) -> 8[a + 8]
\tcopyb(r13, r7) -> 8[a + 16]
\tcopyb(r13, r8) -> 8[a + 24]
\tcopyb(r13, r12) -> 8[a + 32]
\tadd(sp, 0) -> r14
\tcopyb(r13, r14) -> 8[a + 40]
\tret
";

#[test]
fn sp_register_end_to_end() {
    let program = parse_program(SRC, "sp_test").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");

    let input: Vec<u8> = Vec::new();
    let out = ZiskEmulator::process_rom(&rom, &input, &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");
    let word = |k: usize| u64::from_le_bytes(out[8 * k..8 * k + 8].try_into().unwrap());

    let base = 0xc010_0000u64;
    assert_eq!(word(0), 33, "[sp + N] a/b sources and store");
    assert_eq!(word(1), base - 8, "incsp(-8) and the sp a-source");
    assert_eq!(word(2), 11, "8[a + sp + N] b source");
    assert_eq!(word(3), 33, "4[a + sp - N] store");
    assert_eq!(word(4), 0x1010, "setsp + incsp: sp = c + inc_sp");
    assert_eq!(word(5), base - 8, "setsp + negative incsp");
}

/// A data bus that drops everything: the replay below only checks the emulator state.
struct NoBus;
impl DataBusTrait<u64, ()> for NoBus {
    fn write_to_bus(&mut self, _: BusId, _: &[u64], _: &[u64]) -> bool {
        true
    }
    fn on_close(&mut self) {}
    fn into_devices(self, _: bool) -> Vec<(usize, ())> {
        Vec::new()
    }
}

/// The prover replays the minimal trace chunk by chunk, each from its recorded start state
/// and memory reads: with one step per chunk, every sp-relative access and every sp update
/// sits on a chunk boundary, so a chunk that lost sp (or read the wrong memory) would not
/// land on the next chunk's start state.
#[test]
fn sp_survives_minimal_trace_chunks() {
    let program = parse_program(SRC, "sp_test").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");
    let options = EmuOptions { chunk_size: Some(1), ..EmuOptions::default() };
    let chunks = ZiskEmulator::compute_minimal_traces(&rom, &[], &options, 1)
        .expect("emulation should succeed");
    assert!(chunks.iter().any(|c| c.start_state.sp != 0), "no chunk started with sp set");

    for (k, chunk) in chunks.iter().enumerate() {
        let mut emu = Emu::new_with_memory_data(&rom, false);
        emu.process_emu_trace(chunk, &mut NoBus, false);
        let ctx = &emu.ctx.inst_ctx;
        if let Some(next) = chunks.get(k + 1) {
            let s = &next.start_state;
            assert_eq!(
                (ctx.pc, ctx.sp, ctx.c, ctx.step),
                (s.pc, s.sp, s.c, s.step),
                "chunk {k} does not end where chunk {} starts",
                k + 1
            );
            assert_eq!(ctx.regs, s.regs, "chunk {k} registers");
        }
    }
}

#[test]
fn sp_instructions_render_back_to_their_source() {
    let program = parse_program(SRC, "sp_test").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");

    let mut rendered: Vec<String> =
        rom.insts.values().map(|b| &b.i).filter(|i| i.uses_sp()).map(|i| i.to_zisk_asm()).collect();
    let mut expected: Vec<String> = SP_LINES.iter().map(|s| s.to_string()).collect();
    rendered.sort();
    expected.sort();
    assert_eq!(rendered, expected);

    // And each rendered line parses back into the same instruction.
    for line in SP_LINES {
        let src = format!("main:\n\t{line}\n\tret\n");
        let rom2 = assemble(&parse_program(&src, "line").unwrap()).unwrap();
        let again: Vec<String> = rom2
            .insts
            .values()
            .map(|b| &b.i)
            .filter(|i| i.uses_sp())
            .map(|i| i.to_zisk_asm())
            .collect();
        assert_eq!(again, vec![line.to_string()], "round trip of `{line}`");
    }
}

#[test]
fn sp_syntax_errors() {
    let parse_err = |line: &str| {
        let src = format!("main:\n\t{line}\n\tret\n");
        parse_program(&src, "err").err().unwrap_or_else(|| panic!("`{line}` should not parse"))
    };
    // sp is an a-source only.
    assert!(parse_err("add(r1, sp) -> r2").contains("only valid as the `a` source"));
    // Offsets are `+ N` / `- N`.
    assert!(parse_err("copyb(0, [sp 8]) -> r1").contains("offset"));
    assert!(parse_err("copyb(r1, 8[a + sp 8]) -> r2").contains("offset"));
    // Spacing is free, as in the rest of the language.
    let compact = "main:\n\tadd([sp+0], [sp-8]) -> [sp+16], incsp(-8)\n\tcopyb(r1, 8[a+sp+3]) -> 8[a+sp-5]\n\tret\n";
    let rom = assemble(&parse_program(compact, "compact").unwrap()).unwrap();
    let mut lines: Vec<String> =
        rom.insts.values().map(|b| &b.i).filter(|i| i.uses_sp()).map(|i| i.to_zisk_asm()).collect();
    lines.sort();
    assert_eq!(
        lines,
        [
            "add([sp + 0], [sp - 8]) -> [sp + 16], incsp(-8)",
            "copyb(r1, 8[a + sp + 3]) -> 8[a + sp - 5]"
        ]
    );
    // The b and store indirections share one ind_width, also when sp-relative.
    assert!(parse_err("copyb(r1, 2[a + sp + 0]) -> 8[a + sp - 8]").contains("same width"));
}

/// Addresses are computed modulo 2^64: `a + offset [+ sp]` may pass through values at or
/// above 2^63 (negative as `i64`) and still land on valid memory. Here sp = 2^63 and
/// a = 2^63 + B (store) or a = 2^63 + B + 8 with offset -8 (load), so the signed sums
/// overflowed (a debug-build panic) although the final address is B.
#[test]
fn sp_addresses_wrap_modulo_2_64() {
    const WRAP_SRC: &str = "\
define OUTPUT_ADDR 0xc0410000
main:
\tcopyb(0, 0x8000000000000000), setsp
\tcopyb(0, 0x80000000c0100000) -> r9
\tcopyb(0, 77) -> r5
\tcopyb(r9, r5) -> 8[a + sp + 0]
\tcopyb(0, 0x80000000c0100008) -> r10
\tcopyb(r10, 8[a + sp - 8]) -> r6
\tcopyb(0, OUTPUT_ADDR) -> r13
\tcopyb(r13, r6) -> 8[a + 0]
\tret
";
    let program = parse_program(WRAP_SRC, "sp_wrap").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");
    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");
    assert_eq!(u64::from_le_bytes(out[0..8].try_into().unwrap()), 77);
}

/// `incsp(N)` and sp-relative offsets take any i64, i64::MIN included (its magnitude does not
/// fit in an i64), and render back to the same text.
#[test]
fn sp_i64_extremes_round_trip() {
    for line in [
        "copyb(0x0, 0x0), incsp(-9223372036854775808)",
        "copyb(0x0, 0x0), incsp(9223372036854775807)",
        "copyb(0x0, [sp - 9223372036854775808]) -> r5",
        "copyb(0x0, 0x7) -> [sp + 9223372036854775807]",
        "copyb(r9, 8[a + sp - 9223372036854775808]) -> r7",
    ] {
        let src = format!("main:\n\t{line}\n\tret\n");
        let rom = assemble(&parse_program(&src, "line").unwrap()).unwrap();
        let again: Vec<String> = rom
            .insts
            .values()
            .map(|b| &b.i)
            .filter(|i| i.uses_sp())
            .map(|i| i.to_zisk_asm())
            .collect();
        assert_eq!(again, vec![line.to_string()], "round trip of `{line}`");
        // And through ziskbin, whose signed fields are zig-zag varints.
        let decoded = zisk_core::ziskbin::decode_rom(&zisk_core::ziskbin::encode_rom(&rom))
            .unwrap_or_else(|e| panic!("ziskbin round trip of `{line}`: {e}"));
        let again: Vec<String> = decoded
            .insts
            .values()
            .map(|b| &b.i)
            .filter(|i| i.uses_sp())
            .map(|i| i.to_zisk_asm())
            .collect();
        assert_eq!(again, vec![line.to_string()], "ziskbin round trip of `{line}`");
    }
    let parse_err = |line: &str| {
        let src = format!("main:\n\t{line}\n\tret\n");
        parse_program(&src, "err").err().unwrap_or_else(|| panic!("`{line}` should not parse"))
    };
    assert!(parse_err("copyb(0, 0), incsp(9223372036854775808)").contains("i64 range"));
    assert!(parse_err("copyb(0, [sp - 9223372036854775809]) -> r5").contains("i64 range"));
    assert!(parse_err("copyb(0, 0), incsp(-0x8000000000000001)").contains("i64 range"));
}
