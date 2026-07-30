//! End-to-end test: assemble the `doubler` example (`ziskos.zisk` + `doubler.zisk`)
//! into a ZiskRom and run it on the emulator, checking that it doubles the input.

use std::path::PathBuf;

use zisk_common::EmuTrace;
use zisk_core::ROM_ADDR;
use ziskasm::assemble_files;
use ziskemu::{EmuOptions, ZiskEmulator};

fn example(file: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "examples", "doubler", file].iter().collect()
}

#[test]
fn doubler_end_to_end() {
    // Assemble the launcher first (so `_start` is at ROM_ADDR) then the program.
    let rom = assemble_files(&[example("ziskos.zisk"), example("doubler.zisk")])
        .expect("assembly should succeed");

    // Dump the assembled program instructions (decoded back to ZisK asm) so the
    // round-trip is visible with `cargo test -- --nocapture`.
    eprintln!("--- assembled program (ROM_ADDR+) ---");
    for (addr, b) in rom.insts.range(ROM_ADDR..) {
        eprintln!("pc_{addr:08x}: {}", b.i.to_zisk_asm());
    }

    // Run it with the sample input [1..=8] (leading length = 8).
    let input = std::fs::read(example("input.bin")).expect("read input.bin");
    let out = ZiskEmulator::process_rom(&rom, &input, &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");

    println!("out={:?}", out);

    // Output is 64 u32 public words (256 bytes); the first 8 u64 are our results.
    let results: Vec<u64> =
        out.chunks(8).take(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
    assert_eq!(results, vec![2, 4, 6, 8, 10, 12, 14, 16], "doubler output mismatch");
}
