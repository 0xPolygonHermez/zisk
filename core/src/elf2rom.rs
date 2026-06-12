//! Reads RISC-V data from and ELF file and converts it to a ZiskRom

use crate::{
    add_end_and_lib,
    elf_extraction::{
        collect_elf_payload_from_bytes, get_symbol_addresses_from_bytes,
        merge_adjacent_data_sections, merge_ro_sections, DataSection, ElfPayload,
    },
    riscv2zisk_context::{add_entry_exit_jmp, add_zisk_code},
    AsmGenerationMethod, DataSection64, ZiskRom, ZiskRom2Asm, RAM_ADDR, RAM_SIZE, ROM_ADDR,
    ROM_ENTRY, ROM_SIZE,
};
use std::{error::Error, path::Path};

/// Executes the ROM transpilation process: from ELF to Zisk
pub fn elf2rom(elf: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    // Load the embedded float library (enabled with the `float` feature).
    #[cfg(feature = "float")]
    const FLOAT_LIB_DATA: &[u8] = include_bytes!("../../lib-float/c/lib/ziskfloat.elf");

    // Extract all relevant sections from the ELF file
    #[cfg(feature = "float")]
    let payloads: Vec<ElfPayload> =
        vec![collect_elf_payload_from_bytes(FLOAT_LIB_DATA)?, collect_elf_payload_from_bytes(elf)?];
    #[cfg(not(feature = "float"))]
    let payloads: Vec<ElfPayload> = vec![collect_elf_payload_from_bytes(elf)?];

    // Record the ELF file index
    #[cfg(feature = "float")]
    let elf_index = 1;
    #[cfg(not(feature = "float"))]
    let elf_index = 0;

    // Without `ziskos::entrypoint!(main);` the linker can't resolve `_start`
    // to the ziskos boot thunk and emits `e_entry = 0`, which would crash the
    // emulator at PC=0 with a confusing out-of-rom error. Looking up a `main`
    // symbol is not a reliable signal: release-mode LTO inlines `main` into
    // `_zisk_main` and strips it from the symbol table.
    if payloads[elf_index].entry_point == 0 {
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

    // Store RO and RW data sections separately, as they will be treated differently when generating the ROM instructions
    let mut ro_data: Vec<DataSection> = Vec::new();
    let mut rw_data: Vec<DataSection> = Vec::new();

    for (i, payload) in payloads.into_iter().enumerate() {
        // Add executable code sections
        for section in &payload.exec {
            add_zisk_code(&mut rom, section.addr, &section.data, dma_addrs);
        }

        // Add read-only data sections.  They will be stored in ROM, but there can be some RAM
        // regions marked as read-only as well, e.g. the output region
        for section in &payload.ro {
            if section.addr >= ROM_ADDR
                && (section.addr + section.data.len() as u64) <= (ROM_ADDR + ROM_SIZE)
            {
                ro_data.push(section.clone());
            } else if section.addr >= RAM_ADDR
                && (section.addr + section.data.len() as u64) <= (RAM_ADDR + RAM_SIZE)
            {
                rw_data.push(section.clone());
            } else {
                return Err(format!(
                    "Data section at address 0x{:x} with size {} is out of ROM and RAM bounds",
                    section.addr,
                    section.data.len()
                )
                .into());
            }
        }

        // Add read-write data sections (will be stored in RAM)
        rw_data.append(&mut payload.rw.clone());

        // Add entry and exit jump instructions, only for the guest ELF payload
        // (i.e. `payloads[elf_index]`)
        if i == elf_index {
            add_entry_exit_jmp(&mut rom, payload.entry_point);
        }
    }

    // Merge adjacent read-write data sections for efficiency.
    rw_data = merge_adjacent_data_sections(&rw_data);

    // Merge and pad RO sections to a 32-byte multiple, coalescing any sections
    // that the padding would otherwise make overlap (see merge_ro_sections).
    ro_data = merge_ro_sections(&ro_data)?;

    // Delete trailing zeros in every data section of the RAM, and delete the section if needed
    rw_data = rw_data
        .into_iter()
        .filter_map(|section| {
            let mut data = section.data;
            while data.last() == Some(&0) {
                data.pop();
            }
            if data.is_empty() {
                None
            } else {
                Some(DataSection { addr: section.addr, data })
            }
        })
        .collect();

    // Ensure every data section address is aligned to 8 bytes, and data length as well
    for section in &mut ro_data {
        if section.addr % 8 != 0 {
            return Err(format!(
                "RO data section at address 0x{:x} is not aligned to 8 bytes",
                section.addr
            )
            .into());
        }
        if section.data.len() % 8 != 0 {
            return Err(format!(
                "RO data section at address 0x{:x} has size {} which is not a multiple of 8 bytes",
                section.addr,
                section.data.len()
            )
            .into());
        }
    }

    // Remove heading zeros, only for RW data sections
    for section in &mut rw_data {
        // Get number of heading zeros
        let mut heading_zeros_counter: usize = 0;
        for i in 0..section.data.len() {
            if section.data[i] == 0 {
                heading_zeros_counter += 1;
            } else {
                break;
            }
        }

        if heading_zeros_counter == section.data.len() {
            // The whole section is zeros, we can delete it
            section.data.clear();
            continue;
        }

        // Find the largest n, multiple of 8 and <= heading_zeros_counter, such that
        // (data.len() - n) % 32 == 0, so no extra trailing zeros need to be added afterwards.
        let r = section.data.len() % 32;
        heading_zeros_counter = if r % 8 == 0 && heading_zeros_counter >= r {
            r + ((heading_zeros_counter - r) / 32) * 32
        } else {
            // Cannot achieve a multiple-of-32 length with an 8-aligned removal; skip entirely.
            0
        };

        // Delete heading zeros and update the section address accordingly
        if heading_zeros_counter > 0 {
            section.data.drain(0..heading_zeros_counter);
            section.addr += heading_zeros_counter as u64;
        }
    }

    // Add trailing zeros in every data section of the RAM to make their size a multiple of 32 bytes
    rw_data = rw_data
        .into_iter()
        .map(|section| {
            let mut data = section.data;
            while data.len() % 32 != 0 {
                data.push(0);
            }
            DataSection { addr: section.addr, data }
        })
        .collect();

    for section in &mut rw_data {
        if section.addr % 8 != 0 {
            return Err(format!(
                "RW data section at address 0x{:x} is not aligned to 8 bytes",
                section.addr
            )
            .into());
        }
        if section.data.len() % 8 != 0 {
            return Err(format!(
                "RW data section at address 0x{:x} has size {} which is not a multiple of 8 bytes",
                section.addr,
                section.data.len()
            )
            .into());
        }
    }

    // Convert RO data sections to 64-bit data sections, and store them in the ROM
    rom.ro_data_64 = ro_data
        .into_iter()
        .map(|section| {
            let mut data = Vec::new();
            for chunk in section.data.chunks(8) {
                data.push(u64::from_le_bytes(chunk.try_into().unwrap()));
            }
            DataSection64 { addr: section.addr, data }
        })
        .collect();

    // Convert RW data sections to 64-bit data sections, and store them in the ROM
    rom.rw_data_64 = rw_data
        .into_iter()
        .map(|section| {
            let mut data = Vec::new();
            for chunk in section.data.chunks(8) {
                data.push(u64::from_le_bytes(chunk.try_into().unwrap()));
            }
            DataSection64 { addr: section.addr, data }
        })
        .collect();

    // println!(
    //     "Merged data sections: {} read-only sections, {} read-write sections",
    //     rom.ro_data.len(),
    //     rom.rw_data.len()
    // );
    // for i in 0..rom.ro_data.len() {
    //     println!(
    //         "RO section {}: addr=0x{:x}, size={}",
    //         i,
    //         rom.ro_data[i].addr,
    //         rom.ro_data[i].data.len()
    //     );
    // }
    // for i in 0..rom.rw_data.len() {
    //     println!(
    //         "RW section {}: addr=0x{:x}, size={}",
    //         i,
    //         rom.rw_data[i].addr,
    //         rom.rw_data[i].data.len()
    //     );
    // }

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
