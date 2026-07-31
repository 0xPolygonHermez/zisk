//! End-to-end test: assemble the `doubler` example (`ziskos.zisk` + `doubler.zisk`)
//! into a ZiskRom and run it on the emulator, checking that it doubles the input.

use std::path::PathBuf;

use zisk_common::EmuTrace;
use zisk_core::ROM_ADDR;
use ziskasm::assemble_files;
use ziskemu::{EmuOptions, ZiskEmulator};

fn example(dir: &str, file: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "examples", dir, file].iter().collect()
}

/// Runs an assembled ROM with the shared `[1..=8]` input and returns the first
/// eight u64 output words.
fn run_doubler(rom: &zisk_core::ZiskRom) -> Vec<u64> {
    eprintln!("--- assembled program (ROM_ADDR+) ---");
    for (addr, b) in rom.insts.range(ROM_ADDR..) {
        eprintln!("pc_{addr:08x}: {}", b.i.to_zisk_asm());
    }

    let input = std::fs::read(example("doubler", "input.bin")).expect("read input.bin");
    let out = ZiskEmulator::process_rom(rom, &input, &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");

    // Output is 64 u32 public words (256 bytes); the first 8 u64 are our results.
    out.chunks(8).take(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
}

#[test]
fn doubler_end_to_end() {
    // Explicit launcher: assemble `ziskos.zisk` (`_start`) + `doubler.zisk`.
    let rom = assemble_files(&[example("doubler", "ziskos.zisk"), example("doubler", "doubler.zisk")])
        .expect("assembly should succeed");
    assert_eq!(run_doubler(&rom), vec![2, 4, 6, 8, 10, 12, 14, 16], "doubler output mismatch");
}

#[test]
fn doubler_min_auto_launcher() {
    // No `_start`: a single `main:` file; the assembler synthesizes the launcher
    // (gp/sp, `call main`, `ret_to_bios`). Exercises jump/ret_to_bios + auto-launcher.
    let rom = assemble_files(&[example("doubler-min", "doubler.zisk")])
        .expect("assembly should succeed");
    assert_eq!(run_doubler(&rom), vec![2, 4, 6, 8, 10, 12, 14, 16], "auto-launcher output mismatch");
}
