//! Reads RISC-V data from and ELF file and converts it to a ZiskRom

use crate::{
    add_end_and_lib,
    elf_extraction::{
        collect_elf_payload_from_bytes, get_symbol_addresses_from_bytes,
        merge_adjacent_ro_sections, ElfPayload,
    },
    riscv2zisk_context::{add_entry_exit_jmp, add_zisk_code, add_zisk_init_data},
    AsmGenerationMethod, RoData, ZiskRom, ZiskRom2Asm, ROM_ENTRY,
};
use std::{error::Error, path::Path};

/// Executes the ROM transpilation process: from ELF to Zisk
pub fn elf2rom(elf: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    // Load the embedded float library
    #[cfg(feature = "float")]
    const FLOAT_LIB_DATA: &[u8] = include_bytes!("../../lib-float/c/lib/ziskfloat.elf");

    // Extract all relevant sections from the ELF file
    #[cfg(feature = "float")]
    let payloads: Vec<ElfPayload> =
        vec![collect_elf_payload_from_bytes(elf)?, collect_elf_payload_from_bytes(FLOAT_LIB_DATA)?];
    #[cfg(not(feature = "float"))]
    let payloads: Vec<ElfPayload> = vec![collect_elf_payload_from_bytes(elf)?];

    // Without `ziskos::entrypoint!(main);` the linker can't resolve `_start`
    // to the ziskos boot thunk and emits `e_entry = 0`, which would crash the
    // emulator at PC=0 with a confusing out-of-rom error. Looking up a `main`
    // symbol is not a reliable signal: release-mode LTO inlines `main` into
    // `_zisk_main` and strips it from the symbol table.
    if payloads[0].entry_point == 0 {
        return Err("Guest ELF has no entry point (e_entry=0x0). \
                    Declare `#![no_main]` and `ziskos::entrypoint!(main);` \
                    at the guest program root."
            .into());
    }

    // Get DMA function addresses: (memcpy, memcmp, memset, memmove)
    let dma_addrs = get_dma_symbol_addresses(elf);

    // Create an empty ZiskRom instance
    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    // Add the end instruction, jumping over it
    add_end_and_lib(&mut rom);

    for (i, payload) in payloads.into_iter().enumerate() {
        // 1. Add executable code sections
        for section in &payload.exec {
            add_zisk_code(&mut rom, section.addr, &section.data, dma_addrs);
        }

        // 2. Add read-write data sections (will be copied to RAM)
        for section in &payload.rw {
            add_zisk_init_data(&mut rom, section.addr, &section.data, true);
        }

        // 3. Add read-only data sections
        // Merge adjacent read-only sections for efficiency
        let merged_ro = merge_adjacent_ro_sections(&payload.ro);
        for section in &merged_ro {
            rom.ro_data.push(RoData::new(section.addr, section.data.len(), section.data.clone()));
        }

        // Add RO data initialization code instructions
        for section in &merged_ro {
            add_zisk_init_data(&mut rom, section.addr, &section.data, true);
        }

        // Add entry and exit jump instructions, only for the main payload, i.e. for the first payload
        if i == 0 {
            add_entry_exit_jmp(&mut rom, payload.entry_point);
        }
    }

    // Preprocess the ROM
    // Split the ROM instructions based on their address to improve performance when
    // searching for the instruction corresponding to the program counter (PC) address
    rom.optimize_instruction_lookup()?;

    //println! {"elf2rom() got rom.insts.len={}", rom.insts.len()};

    Ok(rom)
}

/// Get DMA function addresses from ELF data
/// Returns (memcpy, memcmp, memset, memmove), with 0 for missing symbols
fn get_dma_symbol_addresses(elf_data: &[u8]) -> (u64, u64, u64, u64) {
    let symbols = ["memcpy", "memcmp", "memset", "memmove"];
    match get_symbol_addresses_from_bytes(elf_data, &symbols) {
        Ok(addrs) => (
            addrs.get("memcpy").copied().unwrap_or(0),
            addrs.get("memcmp").copied().unwrap_or(0),
            addrs.get("memset").copied().unwrap_or(0),
            addrs.get("memmove").copied().unwrap_or(0),
        ),
        Err(_) => (0, 0, 0, 0),
    }
}

/// Executes the ELF file data transpilation process into a Zisk ROM, and saves the result into a
/// file.  The file format can be JSON, PIL-based or binary.
pub fn elf2romfile(
    elf: &[u8],
    asm_file: &Path,
    generation_method: AsmGenerationMethod,
    log_output: bool,
    comments: bool,
    hints: bool,
) -> Result<(), Box<dyn Error>> {
    let rom = elf2rom(elf)?;
    ZiskRom2Asm::save_to_asm_file(&rom, asm_file, generation_method, log_output, comments, hints);

    Ok(())
}
