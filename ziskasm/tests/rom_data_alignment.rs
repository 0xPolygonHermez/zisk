//! The proving ROM-trace builder (`state-machines/rom/src/custom_rom.rs`) packs
//! `ro_data_64`/`rw_data_64` into 4-u64 (32-byte) rows, so every section's length
//! must be a multiple of 4 u64s and its start 32-byte aligned. The emulator does
//! not enforce this, so this test guards the assembler's data layout.

use ziskasm::{assemble_files, collect_zisk_files};

#[test]
fn diagnostic_rom_data_sections_are_row_aligned() {
    let files = collect_zisk_files("programs/diagnostic").expect("collect diagnostic files");
    let rom = assemble_files(&files).expect("assemble diagnostic");

    for section in rom.ro_data_64.iter().chain(rom.rw_data_64.iter()) {
        assert_eq!(
            section.data.len() % 4,
            0,
            "section at 0x{:x} has {} u64s, not a multiple of 4",
            section.addr,
            section.data.len()
        );
        assert_eq!(
            section.addr % 32,
            0,
            "section start 0x{:x} is not 32-byte aligned",
            section.addr
        );
    }
}
