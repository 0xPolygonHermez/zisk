//! Reads RISC-V data from an ELF file and converts it to a ZiskRom

use crate::elf_extraction::{
    collect_elf_payload_from_bytes, merge_ro_sections, validate_entry_point, ElfPayload,
};
use crate::riscv2zisk_context::{add_zisk_code, zkvmcall_ids as zkvmcall_ids_in};
use std::collections::HashMap;
use std::error::Error;
use zisk_core::mem::DataSection;
use zisk_core::mem::{RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_ENTRY, ROM_SIZE};
use zisk_core::rom_layout::{
    add_end_and_lib, add_entry_exit_jmp, normalize_rw_data_sections, InlineBody,
};
use zisk_core::zisk_rom::{DataSection64, ZiskRom};
use zisk_core::{FLOAT_LIB_RAM_ADDR, FLOAT_LIB_ROM_ADDR};

/// Executes the ROM transpilation process: from a RISC-V ELF to Zisk.
///
/// A ziskbin ELF (`e_machine == EM_ZISK`, a prebuilt ROM) is not RISC-V; the guest-format
/// dispatcher (`zisk_transpiler::program2rom`) decodes those before calling this.
pub fn elf2rom(elf: &[u8]) -> Result<ZiskRom, Box<dyn Error>> {
    // Load the embedded float library (enabled with the `float` feature).
    #[cfg(feature = "float")]
    const FLOAT_LIB_DATA: &[u8] = include_bytes!("../../../lib-float/c/lib/ziskfloat.elf");

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

    // Validate the guest entry point: instruction-aligned (2 bytes, ZisK decodes
    // compressed instructions) and inside a loaded executable segment (ZisK reads
    // e_entry rather than booting from a fixed address).
    validate_entry_point(&payloads[elf_index])?;

    // zkvmcalls (`csrs <id>, x0`, see zisk_definitions::ZKVMCALLS) used by the guest:
    // the only way a guest reaches the ZisK library. They are found by instruction, not
    // by symbol, so they also work on stripped ELFs.
    let mut zkvmcall_ids = std::collections::BTreeSet::new();
    for payload in &payloads {
        for section in &payload.exec {
            zkvmcall_ids.extend(zkvmcall_ids_in(section.addr, &section.data)?);
        }
    }
    // zkvmcall ID → library-entry map, filled in below once the library is assembled.
    let mut zkvmcalls: HashMap<u16, u64> = HashMap::new();
    // zkvmcall ID → routine body, for every used inline zkvmcall.
    let mut inline_zkvmcalls: HashMap<u16, InlineBody> = HashMap::new();

    // A guest with no zkvmcall never reaches the library, so it is neither assembled
    // nor merged into the ROM (see the merge below). `None` = nothing to link.
    let library = {
        if zkvmcall_ids.is_empty() {
            None
        } else {
            let library = ziskasm::assemble_zisk_library()
                .map_err(|e| format!("assembling ZisK library: {e}"))?;

            // Report how much of the reserved ZISKLIB ROM/RAM windows the library
            // occupies (it is fit-checked inside assemble_zisk_library, so this only
            // ever prints a value within budget).
            let (rom_used, ram_used) = library.footprint();
            let rom_pct = rom_used as f64 * 100.0 / zisk_core::ZISKLIB_ROM_SIZE as f64;
            let ram_pct = ram_used as f64 * 100.0 / zisk_core::ZISKLIB_RAM_SIZE as f64;
            println!(
                "ZisK library footprint: ROM {rom_used}/{} bytes ({rom_pct:.1}%), RAM {ram_used}/{} bytes ({ram_pct:.1}%)",
                zisk_core::ZISKLIB_ROM_SIZE,
                zisk_core::ZISKLIB_RAM_SIZE
            );

            for id in &zkvmcall_ids {
                let call = zisk_definitions::zkvmcall_by_id(*id).unwrap();
                let lib_addr = *library.symbols.get(call.target).ok_or_else(|| {
                    format!(
                        "ZisK library has no function `{}` (zkvmcall 0x{id:X}, `{}`)",
                        call.target, call.name
                    )
                })?;
                zkvmcalls.insert(*id, lib_addr);
                if call.inline_args > 0 {
                    let body = library
                        .inline_body(call.target, call.inline_args)
                        .map_err(|e| format!("inline zkvmcall 0x{id:X} (`{}`): {e}", call.name))?;
                    inline_zkvmcalls.insert(*id, body);
                }
            }
            Some(library)
        }
    };

    // Create an empty ZiskRom instance
    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    // Add the end instruction, jumping over it
    add_end_and_lib(&mut rom);

    // Store RO and RW data sections separately, as they will be treated differently when generating the ROM instructions
    let mut ro_data: Vec<DataSection> = Vec::new();
    let mut rw_data: Vec<DataSection> = Vec::new();

    for (i, payload) in payloads.into_iter().enumerate() {
        let ElfPayload { entry_point, exec, ro, rw } = payload;

        // Add executable code sections (zkvmcalls become jumps into the library).
        for section in &exec {
            add_zisk_code(&mut rom, section.addr, &section.data, &zkvmcalls, &inline_zkvmcalls);
        }

        // Add read-only data sections.  They will be stored in ROM, but there can be some RAM
        // regions marked as read-only as well, e.g. the output region
        for section in ro {
            if section.addr >= ROM_ADDR
                && (section.addr + section.data.len() as u64) <= (ROM_ADDR + ROM_SIZE)
            {
                // If this is a program section, it should not overlap with the float library region
                // If this is a float library section, it should not overlap with the program region
                if i == elf_index {
                    if section.addr + section.data.len() as u64 >= FLOAT_LIB_ROM_ADDR {
                        return Err(format!(
                            "ROM program data section at address 0x{:x} with size {} overlaps with the ZisK float library region",
                            section.addr,
                            section.data.len()
                        )
                        .into());
                    }
                } else if section.addr < FLOAT_LIB_ROM_ADDR {
                    return Err(format!(
                        "ROM float library data section at address 0x{:x} with size {} overlaps with the ZisK program region",
                        section.addr,
                        section.data.len()
                    )
                    .into());
                }
                ro_data.push(section);
            } else if section.addr >= RAM_ADDR
                && (section.addr + section.data.len() as u64) <= (RAM_ADDR + RAM_SIZE)
            {
                // If this is a program section, it should not overlap with the float library region
                // If this is a float library section, it should not overlap with the program region
                if i == elf_index {
                    if section.addr + section.data.len() as u64 >= FLOAT_LIB_RAM_ADDR {
                        return Err(format!(
                            "RAM program data section at address 0x{:x} with size {} overlaps with the ZisK float library region",
                            section.addr,
                            section.data.len()
                        )
                        .into());
                    }
                } else if section.addr < FLOAT_LIB_RAM_ADDR {
                    return Err(format!(
                        "RAM float library data section at address 0x{:x} with size {} overlaps with the ZisK program region",
                        section.addr,
                        section.data.len()
                    )
                    .into());
                }
                rw_data.push(section);
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
        rw_data.extend(rw);

        // Add entry and exit jump instructions, only for the guest ELF payload
        // (i.e. `payloads[elf_index]`)
        if i == elf_index {
            add_entry_exit_jmp(&mut rom, entry_point);
        }
    }

    // Merge and pad RO sections to a 32-byte multiple, coalescing any sections
    // that the padding would otherwise make overlap (see merge_ro_sections).
    ro_data = merge_ro_sections(ro_data)?;

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

    // Normalize RW data sections for ROM-data initialization: trim zero padding,
    // merge sections that collide once expanded to 32-byte blocks, and carve out
    // every internal all-zero region >= 1 KiB (see normalize_rw_data_sections).
    rw_data = normalize_rw_data_sections(rw_data);

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

    // Merge the ZisK library (only assembled when a zkvmcall uses it): its
    // instructions and data live in the reserved region, disjoint from the guest.
    if let Some(library) = library {
        merge_library(&mut rom, library)?;
    }

    // Preprocess the ROM
    // Split the ROM instructions based on their address to improve performance when
    // searching for the instruction corresponding to the program counter (PC) address
    rom.optimize_instruction_lookup()?;

    Ok(rom)
}

/// Merges the assembled ZisK library into the guest ROM: its instructions and data
/// live in the reserved region, disjoint from the guest.
fn merge_library(rom: &mut ZiskRom, library: ziskasm::ZiskLibrary) -> Result<(), Box<dyn Error>> {
    // The guest linker script reserves ZISKLIB_RAM but not ZISKLIB_ROM, and unlike
    // the float-library region above nothing has fenced these off yet. `extend`
    // would silently overwrite a colliding guest instruction (BTreeMap) or leave
    // overlapping data sections, so reject the collision instead.
    use zisk_core::{
        ZISKLIB_RAM_ADDR, ZISKLIB_RAM_ADDR_MAX, ZISKLIB_ROM_ADDR, ZISKLIB_ROM_ADDR_MAX,
    };
    if let Some((&addr, _)) = rom.insts.range(ZISKLIB_ROM_ADDR..=ZISKLIB_ROM_ADDR_MAX).next() {
        return Err(format!(
            "guest instruction at 0x{addr:x} overlaps the reserved ZisK library ROM region (0x{ZISKLIB_ROM_ADDR:x}..0x{ZISKLIB_ROM_ADDR_MAX:x})"
        )
        .into());
    }
    for (what, sections, lo, hi) in [
        ("ROM", &rom.ro_data_64, ZISKLIB_ROM_ADDR, ZISKLIB_ROM_ADDR_MAX),
        ("RAM", &rom.rw_data_64, ZISKLIB_RAM_ADDR, ZISKLIB_RAM_ADDR_MAX),
    ] {
        for s in sections {
            let end = s.addr + (s.data.len() * 8) as u64;
            if s.addr <= hi && end > lo {
                return Err(format!(
                    "guest data section at 0x{:x} (size {}) overlaps the reserved ZisK library {what} region (0x{lo:x}..0x{hi:x})",
                    s.addr,
                    s.data.len() * 8
                )
                .into());
            }
        }
    }
    // The library was assembled into a ROM of its own, so its instruction indexes
    // start at 0 like the guest's. The index is the instruction's row in the ROM
    // trace (and in the ROM state machine's multiplicities), so renumber the
    // library after the guest: a shared index would put two instructions on one row.
    let mut library_insts = library.insts;
    let first_index = rom.build_counter;
    for zib in library_insts.values_mut() {
        zib.i.index += first_index;
    }
    rom.build_counter += library_insts.len() as u64;
    rom.insts.extend(library_insts);
    rom.ro_data_64.extend(library.ro_data);
    rom.rw_data_64.extend(library.rw_data);
    Ok(())
}

#[cfg(test)]
mod zkvmcall_tests {
    use zisk_definitions::{ZKVMCALLS, ZKVMCALL_ADDR_END, ZKVMCALL_ADDR_START};

    /// The table itself: IDs in range, strictly increasing (so none is reused).
    #[test]
    fn zkvmcall_ids_are_in_range_and_unique() {
        for call in ZKVMCALLS {
            assert!(
                (ZKVMCALL_ADDR_START..=ZKVMCALL_ADDR_END).contains(&call.id),
                "{} has out-of-range ID 0x{:X}",
                call.name,
                call.id
            );
        }
        for pair in ZKVMCALLS.windows(2) {
            assert!(
                pair[0].id < pair[1].id,
                "{} and {} are out of order",
                pair[0].name,
                pair[1].name
            );
        }
    }

    /// The C thunks (`ZKVMCALL <name>, <id>` lines) match the table, so a C guest can
    /// never call one routine and get another: every thunk is a table entry, and every
    /// entry has a thunk but the inline zkvmcalls, which may (a thunk calls them too,
    /// see `inline_zkvmcall_site`) or may not.
    #[test]
    fn c_thunks_match_zkvmcall_table() {
        const ASM: &str = include_str!("../../../ziskasm/lang/c/src/zkvm_calls.s");
        let thunks: Vec<(String, u16)> = ASM
            .lines()
            .filter_map(|line| line.trim().strip_prefix("ZKVMCALL "))
            .map(|rest| {
                let (name, id) = rest.split_once(',').expect("ZKVMCALL <name>, <id>");
                let id = id.trim().strip_prefix("0x").expect("hex ID");
                (name.trim().to_string(), u16::from_str_radix(id, 16).expect("hex ID"))
            })
            .collect();
        for (name, id) in &thunks {
            assert!(
                ZKVMCALLS.iter().any(|c| c.name == name && c.id == *id),
                "thunk {name} 0x{id:X} is not in the table"
            );
        }
        for c in ZKVMCALLS.iter().filter(|c| c.inline_args == 0) {
            assert!(
                thunks.iter().any(|(name, id)| name == c.name && *id == c.id),
                "{} 0x{:X} has no thunk",
                c.name,
                c.id
            );
        }
    }

    /// Every zkvmcall target exists in the assembled ZisK library.
    #[test]
    fn zkvmcall_targets_exist_in_library() {
        let library = ziskasm::assemble_zisk_library().unwrap();
        for call in ZKVMCALLS {
            assert!(
                library.symbols.contains_key(call.target),
                "{} targets missing library routine {}",
                call.name,
                call.target
            );
        }
    }

    /// The library, assembled on its own, numbers its instructions from 0 like the guest;
    /// merged, every instruction must still have its own index (its ROM trace row).
    #[test]
    fn merged_library_gets_indexes_after_the_guest() {
        use super::{add_end_and_lib, merge_library, ZiskRom, ROM_ENTRY};
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };
        add_end_and_lib(&mut rom);
        let guest = rom.insts.len() as u64;
        assert!(guest > 0 && rom.build_counter == guest);

        let library = ziskasm::assemble_zisk_library().unwrap();
        let lib = library.insts.len() as u64;
        assert!(library.insts.values().any(|zib| zib.i.index == 0), "the library starts at 0");
        merge_library(&mut rom, library).unwrap();

        let mut indexes: Vec<u64> = rom.insts.values().map(|zib| zib.i.index).collect();
        indexes.sort_unstable();
        assert_eq!(indexes, (0..guest + lib).collect::<Vec<_>>(), "indexes are unique and dense");
        assert_eq!(rom.build_counter, guest + lib);
    }
}
