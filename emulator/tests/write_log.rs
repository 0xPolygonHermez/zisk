//! Diagnostic log (`write_log` in zkvm_io.h, draft zkvm-standards PR #27): the transpiler
//! turns each write_log call site into one `flag` that reads the text's pointer and length.
//! The prover sees a no-op, so the ROM is the same whether the text is printed or not; ziskemu
//! prints it only when asked to (`--write-log`).
//!
//! The committed guest `elf-regressions/write_log/write_log.elf` (built from
//! `ziskasm/lang/c/example/write_log_guest.c` and `write_log_thunk.c`) logs through six
//! inline call sites and one thunk, and outputs a sum computed between the logs (21).

use zisk_common::EmuTrace;
use zisk_core::{ZiskRom, SRC_REG};
use zisk_definitions::{zkvmcall_id, ZKVMCALL_ARG_ADDR_START};
use zisk_transpiler_common::program2rom;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator};

const WRITE_LOG_ID: u16 = zkvmcall_id("write_log");

fn elf_path() -> String {
    format!("{}/../elf-regressions/write_log/write_log.elf", env!("CARGO_MANIFEST_DIR"))
}

fn elf() -> Vec<u8> {
    std::fs::read(elf_path()).expect("committed ELF")
}

fn rom() -> ZiskRom {
    program2rom(&elf()).expect("ELF must transpile")
}

/// If `word` is `csrs <csr>, rs`, its rs
fn csrs_rs(word: u32, csr: u16) -> Option<u64> {
    (word & 0xfff0_7fff == ((csr as u32) << 20) | 0x2073).then_some(((word >> 15) & 0x1f) as u64)
}

/// Every write_log call site of `elf`: the address of its zkvmcall `csrs`, the registers
/// holding the text's pointer and length, and the length of its sequence in bytes
fn write_log_sites(elf: &[u8]) -> Vec<(u64, [u64; 2], i64)> {
    let u16_at = |o: usize| u16::from_le_bytes(elf[o..o + 2].try_into().unwrap()) as usize;
    let u32_at = |o: usize| u32::from_le_bytes(elf[o..o + 4].try_into().unwrap());
    let u64_at = |o: usize| u64::from_le_bytes(elf[o..o + 8].try_into().unwrap());
    let (phoff, phentsize, phnum) = (u64_at(0x20) as usize, u16_at(0x36), u16_at(0x38));
    let mut sites = Vec::new();
    for ph in (0..phnum).map(|k| phoff + k * phentsize) {
        // An executable PT_LOAD segment
        if u32_at(ph) != 1 || u32_at(ph + 4) & 1 == 0 {
            continue;
        }
        let (offset, vaddr, filesz) =
            (u64_at(ph + 8) as usize, u64_at(ph + 0x10), u64_at(ph + 0x20));
        let words: Vec<u32> = (0..filesz as usize / 4).map(|k| u32_at(offset + 4 * k)).collect();
        for (k, &word) in words.iter().enumerate() {
            let Some(rs) = csrs_rs(word, WRITE_LOG_ID) else { continue };
            let addr = vaddr + 4 * k as u64;
            // The inline form carries the length on the next csrs; the thunk has them in a0, a1
            match words.get(k + 1).and_then(|&w| csrs_rs(w, ZKVMCALL_ARG_ADDR_START)) {
                Some(len_rs) => sites.push((addr, [rs, len_rs], 8)),
                None => sites.push((addr, [10, 11], 4)),
            }
        }
    }
    sites
}

/// Runs the guest, printing its log or not, returning its output
fn emulate(write_log: bool) -> Vec<u8> {
    let options = EmuOptions { elf: Some(elf_path()), write_log, ..Default::default() };
    ZiskEmulator
        .emulate(&options, None::<Box<dyn Fn(EmuTrace)>>)
        .expect("the guest must run to completion")
}

#[test]
fn the_guest_logs_through_both_forms() {
    let sites = write_log_sites(&elf());
    assert_eq!(sites.iter().filter(|(_, _, len)| *len == 8).count(), 6, "inline sites");
    assert_eq!(sites.iter().filter(|(_, _, len)| *len == 4).count(), 1, "thunks");
}

#[test]
fn each_call_site_is_one_flag_that_reads_the_text() {
    let elf = elf();
    let rom = rom();
    for (addr, regs, len) in write_log_sites(&elf) {
        let inst = &rom.insts[&addr].i;
        assert!(inst.verbose.starts_with("write_log"), "0x{addr:x}");
        assert_eq!(inst.op_str, "flag", "0x{addr:x}");
        assert_eq!((inst.a_src, inst.a_offset_imm0), (SRC_REG, regs[0]), "0x{addr:x}");
        assert_eq!((inst.b_src, inst.b_offset_imm0), (SRC_REG, regs[1]), "0x{addr:x}");
        assert_eq!((inst.jmp_offset1, inst.jmp_offset2), (len, len), "0x{addr:x}");
    }
}

#[test]
fn write_log_needs_no_library() {
    // The guest still uses write_output, which does
    assert!(!rom().zkvmcalls.contains(&WRITE_LOG_ID));
}

#[test]
fn the_output_does_not_depend_on_the_log() {
    let (quiet, printed) = (emulate(false), emulate(true));
    assert_eq!(quiet[..4], 21u32.to_le_bytes());
    assert_eq!(quiet, printed);
}

#[test]
fn minimal_traces_run_the_log() {
    let options = EmuOptions { chunk_size: Some(1 << 18), ..Default::default() };
    let traces = ZiskEmulator::compute_minimal_traces(&rom(), &[], &options, 2)
        .expect("minimal traces must succeed");
    assert!(!traces.is_empty());
}
