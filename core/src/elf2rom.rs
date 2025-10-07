//! Reads RISC-V data from and ELF file and converts it to a ZiskRom

use crate::{
    add_end_and_lib,
    elf_extraction::{
        collect_elf_payload, collect_elf_payload_from_bytes, merge_adjacent_ro_sections, ElfPayload,
    },
    riscv2zisk_context::{add_entry_exit_jmp, add_zisk_code, add_zisk_init_data},
    AsmGenerationMethod, RoData, ZiskInst, ZiskRom, ZiskRom2Asm, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY,
};
use rayon::prelude::*;
use std::{error::Error, path::Path};

/// Executes the ROM transpilation process: from ELF to Zisk
pub fn elf2rom(elf_file: &Path) -> Result<ZiskRom, Box<dyn Error>> {
    // Load the embedded float library
    const FLOAT_LIB_DATA: &[u8] = include_bytes!("../../lib-float/c/lib/ziskfloat.elf");

    // Extract all relevant sections from the ELF file
    let payloads: Vec<ElfPayload> =
        vec![collect_elf_payload(elf_file)?, collect_elf_payload_from_bytes(FLOAT_LIB_DATA)?];

    // Create an empty ZiskRom instance
    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    // Add the end instruction, jumping over it
    add_end_and_lib(&mut rom);

    for payload in payloads.into_iter() {
        // 1. Add executable code sections
        for section in &payload.exec {
            add_zisk_code(&mut rom, section.addr, &section.data);
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

        // Add entry and exit jump instructions
        add_entry_exit_jmp(&mut rom, payload.entry_point);
    }

    // Preprocess the ROM (experimental)
    // Split the ROM instructions based on their address in order to get a better performance when
    // searching for the corresponding intruction to the pc program address
    optimize_instruction_lookup(&mut rom)?;

    //println! {"elf2rom() got rom.insts.len={}", rom.insts.len()};

    Ok(rom)
}

/// Optimizes instruction lookup by organizing instructions into direct-access arrays.
///
/// ## Problem it solves:
///
/// Instead of using a HashMap for every instruction fetch (key is the `pc`),
/// this creates three separate arrays where instructions can be accessed by direct
/// index/PC calculations.
///
/// Instructions are split into three categories:
///
/// 1. Entry/BIOS instructions:
///    - Address range: `[ROM_ENTRY, ROM_ADDR)`
///    - 4-byte aligned instructions in the startup/BIOS area
///    - Accessed via: `array[(addr - ROM_ENTRY) / 4]`
///
/// 2. Main program instructions:
///    - Address range: `[ROM_ADDR, ROM_ADDR_MAX)`
///    - 4-byte aligned instructions in the main ROM area
///    - Accessed via: `array[(addr - ROM_ADDR) / 4]`
///
/// 3. Non-aligned instructions:
///    - Any instruction NOT on a 4-byte boundary
///    - TODO: previous comment says there should only be one -- what is it?
///    - Accessed via: `array[addr - min_na_addr]`
///
/// There are two places where this optimization is used:
///     - When building traces for proof generation, we iterate through all instructions in address order
///     - When running the emulator, each iteration of emulator need to fetch an instruction based on the `pc`
///       Using an array vs a hashmap here will be faster due to instructions being next to each other and array cache locality.
fn optimize_instruction_lookup(rom: &mut ZiskRom) -> Result<(), Box<dyn Error>> {
    // 1. Find the address ranges for each instruction category
    let mut max_rom_entry = 0;
    let mut min_rom_instructions = u64::MAX;
    let mut max_rom_instructions = 0;
    let mut min_rom_na_unstructions = u64::MAX;
    let mut max_rom_na_unstructions = 0;

    // Prepare sorted pc list
    rom.sorted_pc_list.reserve(rom.insts.len());

    // Scan all instructions to categorize them and find ranges
    for instruction in &rom.insts {
        let addr = *instruction.0;

        // Add to pc list (still unsorted)
        rom.sorted_pc_list.push(addr);

        if addr < ROM_ENTRY {
            return Err(format!("Address out of range: {addr}").into());
        } else if addr < ROM_ADDR {
            // Entry/BIOS area
            if addr % 4 != 0 {
                // Non-aligned instruction in entry area
                //
                // When an address is not 4 bytes aligned, it is considered a
                // na_rom_instructions We are supposed to have only one non
                // aligned instructions in > ROM_ADDRESS
                // TODO: Where is this only one claim checked?
                min_rom_na_unstructions = std::cmp::min(min_rom_na_unstructions, addr);
                max_rom_na_unstructions = std::cmp::max(max_rom_na_unstructions, addr);
            } else {
                // Aligned instruction in entry area
                max_rom_entry = std::cmp::max(max_rom_entry, addr);
            }
        } else if addr < ROM_ADDR_MAX {
            // Main ROM area
            if addr % 4 != 0 {
                // Non-aligned instruction in main area
                //
                // When an address is not 4 bytes aligned, it is considered a
                // na_rom_instructions We are supposed to have only one non
                // aligned instructions in > ROM_ADDRESS
                // TODO: Where is this only one claim checked?
                min_rom_na_unstructions = std::cmp::min(min_rom_na_unstructions, addr);
                max_rom_na_unstructions = std::cmp::max(max_rom_na_unstructions, addr);
            } else {
                // Aligned instruction in main area
                min_rom_instructions = min_rom_instructions.min(addr);
                max_rom_instructions = max_rom_instructions.max(addr);
            }
        } else {
            return Err(format!("Address out of range: {addr}").into());
        }
    }
    rom.max_bios_pc = max_rom_entry;
    rom.max_program_pc = max_rom_instructions;
    rom.min_program_pc =
        if min_rom_instructions == u64::MAX { ROM_ADDR } else { min_rom_instructions };

    let num_rom_entry = if max_rom_entry > 0 { (max_rom_entry - ROM_ENTRY) / 4 + 1 } else { 0 };
    let num_rom_instructions = if max_rom_instructions > 0 {
        (max_rom_instructions - rom.min_program_pc) / 4 + 1
    } else {
        0
    };
    let num_rom_na_instructions = if u64::MAX == min_rom_na_unstructions {
        0 // No non-aligned instructions found
    } else {
        max_rom_na_unstructions - min_rom_na_unstructions + 1
    };

    // Initialize in parallel to increase performance
    rom.rom_entry_instructions =
        (0..num_rom_entry).into_par_iter().map(|_| ZiskInst::default()).collect();
    rom.rom_instructions =
        (0..num_rom_instructions).into_par_iter().map(|_| ZiskInst::default()).collect();
    rom.rom_na_instructions =
        (0..num_rom_na_instructions).into_par_iter().map(|_| ZiskInst::default()).collect();
    rom.offset_rom_na_unstructions = min_rom_na_unstructions;

    // Sort pc list
    rom.sorted_pc_list.sort();

    // 2. Populate the arrays with instructions at their calculated indices
    for instruction in &rom.insts {
        let addr = *instruction.0;

        if addr % 4 != 0 {
            // Non-aligned: store at offset from minimum non-aligned address
            rom.rom_na_instructions[(addr - min_rom_na_unstructions) as usize] =
                instruction.1.i.clone();
        } else if addr < ROM_ADDR {
            // Entry/BIOS area: divide by 4 for index (using shift for efficiency)
            rom.rom_entry_instructions[((addr - ROM_ENTRY) >> 2) as usize] =
                instruction.1.i.clone();
        } else {
            // Main ROM: divide by 4 for index (using shift for efficiency)
            rom.rom_instructions[((addr - rom.min_program_pc) >> 2) as usize] =
                instruction.1.i.clone();
        }
    }

    // 3. Link every instruction with the position they occupy in the sorted pc list
    //
    // The index is stored in two places because instructions exist in:
    // - rom.insts: The original HashMap for random access by PC
    // - rom.*_instructions arrays: The optimized arrays for fast indexed access
    for i in 0..rom.sorted_pc_list.len() {
        let pc = rom.sorted_pc_list[i];
        rom.insts.get_mut(&pc).unwrap().i.sorted_pc_list_index = i;

        let inst = rom.get_mut_instruction(pc);
        inst.sorted_pc_list_index = i;
    }

    Ok(())
}

/// Executes the ELF file data transpilation process into a Zisk ROM, and saves the result into a
/// file.  The file format can be JSON, PIL-based or binary.
pub fn elf2romfile(
    elf_file: &Path,
    asm_file: &Path,
    generation_method: AsmGenerationMethod,
    log_output: bool,
    comments: bool,
) -> Result<(), Box<dyn Error>> {
    let rom = elf2rom(elf_file)?;
    ZiskRom2Asm::save_to_asm_file(&rom, asm_file, generation_method, log_output, comments);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ZiskInstBuilder, ZiskRom};

    // Helper to create a test instruction with a given opcode
    fn create_test_inst_builder(addr: u64, op: u8) -> ZiskInstBuilder {
        let mut builder = ZiskInstBuilder::new(addr);
        builder.i.op = op;
        builder
    }

    #[test]
    fn test_optimize_empty_rom() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        assert!(optimize_instruction_lookup(&mut rom).is_ok());
        assert_eq!(rom.sorted_pc_list.len(), 0);
        assert_eq!(rom.rom_entry_instructions.len(), 0);
        assert_eq!(rom.rom_instructions.len(), 0);
        assert_eq!(rom.rom_na_instructions.len(), 0);
    }

    #[test]
    fn test_optimize_entry_instructions_only() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add some entry area instructions, but none in main area
        let entry_base = ROM_ENTRY;
        rom.insts.insert(entry_base, create_test_inst_builder(entry_base, 1));
        rom.insts.insert(entry_base + 4, create_test_inst_builder(entry_base + 4, 2));
        rom.insts.insert(entry_base + 8, create_test_inst_builder(entry_base + 8, 3));

        assert!(optimize_instruction_lookup(&mut rom).is_ok());

        // Check arrays are correctly sized
        assert_eq!(rom.rom_entry_instructions.len(), 3);
        assert_eq!(rom.rom_instructions.len(), 0);
        assert_eq!(rom.rom_na_instructions.len(), 0);

        // Check sorted PC list
        assert_eq!(rom.sorted_pc_list, vec![entry_base, entry_base + 4, entry_base + 8]);

        // Verify instructions are at correct indices
        assert_eq!(rom.rom_entry_instructions[0].op, 1);
        assert_eq!(rom.rom_entry_instructions[1].op, 2);
        assert_eq!(rom.rom_entry_instructions[2].op, 3);

        // Check max values
        assert_eq!(rom.max_bios_pc, entry_base + 8);
        assert_eq!(rom.max_program_pc, 0);
    }

    #[test]
    fn test_optimize_main_rom_instructions() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add main ROM area instructions, but none in BIOS area
        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 10));
        rom.insts.insert(ROM_ADDR + 4, create_test_inst_builder(ROM_ADDR + 4, 11));
        rom.insts.insert(ROM_ADDR + 12, create_test_inst_builder(ROM_ADDR + 12, 12)); // Gap in addresses

        assert!(optimize_instruction_lookup(&mut rom).is_ok());

        // Check arrays
        assert_eq!(rom.rom_entry_instructions.len(), 0);
        assert_eq!(rom.rom_instructions.len(), 4); // Includes the gap at ROM_ADDR + 8
        assert_eq!(rom.rom_na_instructions.len(), 0);

        // Check instructions are at correct indices
        assert_eq!(rom.rom_instructions[0].op, 10); // (ROM_ADDR - ROM_ADDR) / 4 = 0
        assert_eq!(rom.rom_instructions[1].op, 11); // (ROM_ADDR + 4 - ROM_ADDR) / 4 = 1
        assert_eq!(rom.rom_instructions[3].op, 12); // (ROM_ADDR + 12 - ROM_ADDR) / 4 = 3

        assert_eq!(rom.max_program_pc, ROM_ADDR + 12);
    }

    #[test]
    fn test_optimize_non_aligned_instructions() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add non-aligned instructions (not on 4-byte boundary)
        rom.insts.insert(ROM_ADDR + 1, create_test_inst_builder(ROM_ADDR + 1, 20));
        rom.insts.insert(ROM_ADDR + 5, create_test_inst_builder(ROM_ADDR + 5, 21));
        rom.insts.insert(ROM_ADDR + 7, create_test_inst_builder(ROM_ADDR + 7, 22));

        assert!(optimize_instruction_lookup(&mut rom).is_ok());

        // Check arrays
        assert_eq!(rom.rom_entry_instructions.len(), 0);
        assert_eq!(rom.rom_instructions.len(), 0);
        // Since the smallest non-aligned instruction adress(`offset_rom_na_unstructions`) is ROM_ADDR+1
        // This will be the gap between each non-aligned instruction.

        // The memory layout will look like the following:
        /*
            Address         | Array Index | Content
            ----------------|-------------|----------
            0x80001001      | 0           | Instruction (op=20)
            0x80001002      | 1           | Empty
            0x80001003      | 2           | Empty
            0x80001004      | 3           | Empty
            0x80001005      | 4           | Instruction (op=21)
            0x80001006      | 5           | Empty
            0x80001007      | 6           | Instruction (op=22)
        */
        assert_eq!(rom.rom_na_instructions.len(), 7); // ROM_ADDR+7 - (ROM_ADDR+1) + 1

        // Check offset is set correctly
        assert_eq!(rom.offset_rom_na_unstructions, ROM_ADDR + 1);

        // Check instructions are at correct indices
        assert_eq!(rom.rom_na_instructions[0].op, 20); // ROM_ADDR+1 - offset = 0
        assert_eq!(rom.rom_na_instructions[4].op, 21); // ROM_ADDR+5 - offset = 4
        assert_eq!(rom.rom_na_instructions[6].op, 22); // ROM_ADDR+7 - offset = 6
    }

    #[test]
    fn test_optimize_mixed_instructions() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Mix of all three types
        rom.insts.insert(ROM_ENTRY + 4, create_test_inst_builder(ROM_ENTRY + 4, 1));
        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 2));
        rom.insts.insert(ROM_ADDR + 3, create_test_inst_builder(ROM_ADDR + 3, 3));

        assert!(optimize_instruction_lookup(&mut rom).is_ok());

        // All three arrays should have content
        assert!(!rom.rom_entry_instructions.is_empty());
        assert!(!rom.rom_instructions.is_empty());
        assert!(!rom.rom_na_instructions.is_empty());

        // Check sorted list has all PCs
        assert_eq!(rom.sorted_pc_list.len(), 3);
        assert_eq!(rom.sorted_pc_list, vec![ROM_ENTRY + 4, ROM_ADDR, ROM_ADDR + 3]);
    }

    #[test]
    fn test_optimize_sorted_pc_indices() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add instructions out of order
        rom.insts.insert(ROM_ADDR + 8, create_test_inst_builder(ROM_ADDR + 8, 3));
        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 1));
        rom.insts.insert(ROM_ADDR + 4, create_test_inst_builder(ROM_ADDR + 4, 2));

        assert!(optimize_instruction_lookup(&mut rom).is_ok());

        // Check sorted order
        assert_eq!(rom.sorted_pc_list, vec![ROM_ADDR, ROM_ADDR + 4, ROM_ADDR + 8]);

        // Verify each instruction knows its position in sorted list
        assert_eq!(rom.insts.get(&ROM_ADDR).unwrap().i.sorted_pc_list_index, 0);
        assert_eq!(rom.insts.get(&(ROM_ADDR + 4)).unwrap().i.sorted_pc_list_index, 1);
        assert_eq!(rom.insts.get(&(ROM_ADDR + 8)).unwrap().i.sorted_pc_list_index, 2);

        // Also check in the arrays
        assert_eq!(rom.rom_instructions[0].sorted_pc_list_index, 0);
        assert_eq!(rom.rom_instructions[1].sorted_pc_list_index, 1);
        assert_eq!(rom.rom_instructions[2].sorted_pc_list_index, 2);
    }

    #[test]
    fn test_optimize_sorted_pc_indices_with_gaps() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        rom.insts.insert(ROM_ADDR, create_test_inst_builder(ROM_ADDR, 10));
        rom.insts.insert(ROM_ADDR + 4, create_test_inst_builder(ROM_ADDR + 4, 11));
        rom.insts.insert(ROM_ADDR + 12, create_test_inst_builder(ROM_ADDR + 12, 12));
        rom.insts.insert(ROM_ADDR + 100, create_test_inst_builder(ROM_ADDR + 100, 13));

        assert!(optimize_instruction_lookup(&mut rom).is_ok());

        // Check sorted list has 4 instructions (no gaps in sorted list)
        assert_eq!(rom.sorted_pc_list.len(), 4);
        assert_eq!(rom.sorted_pc_list, vec![ROM_ADDR, ROM_ADDR + 4, ROM_ADDR + 12, ROM_ADDR + 100]);

        // Array has space for all addresses including gaps
        // Array size = (100 - 0) / 4 + 1 = 26 slots
        assert_eq!(rom.rom_instructions.len(), 26);

        // rom_instructions[0] is at ROM_ADDR
        assert_eq!(rom.rom_instructions[0].op, 10);
        assert_eq!(rom.rom_instructions[0].sorted_pc_list_index, 0);

        // rom_instructions[1] is at ROM_ADDR + 4
        assert_eq!(rom.rom_instructions[1].op, 11);
        assert_eq!(rom.rom_instructions[1].sorted_pc_list_index, 1);

        // rom_instructions[3] is at ROM_ADDR + 12
        assert_eq!(rom.rom_instructions[3].op, 12);
        assert_eq!(rom.rom_instructions[3].sorted_pc_list_index, 2);

        // rom_instructions[25] is at ROM_ADDR + 100
        assert_eq!(rom.rom_instructions[25].op, 13);
        assert_eq!(rom.rom_instructions[25].sorted_pc_list_index, 3);
    }

    #[test]
    fn test_optimize_address_below_rom_entry_err() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add instruction below ROM_ENTRY
        rom.insts.insert(ROM_ENTRY - 4, create_test_inst_builder(ROM_ENTRY - 4, 1));
        assert!(optimize_instruction_lookup(&mut rom).is_err());
    }

    #[test]
    fn test_optimize_address_above_rom_max_err() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        // Add instruction above ROM_ADDR_MAX.
        rom.insts.insert(ROM_ADDR_MAX + 4, create_test_inst_builder(ROM_ADDR_MAX + 4, 1));
        assert!(optimize_instruction_lookup(&mut rom).is_err());
    }

    #[test]
    fn test_basic_optimize_preserves_instruction_data() {
        let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

        let mut builder = ZiskInstBuilder::new(ROM_ADDR);
        builder.i.op = 42;
        builder.i.a_src = 1;
        builder.i.b_src = 2;
        builder.i.store = 3;

        rom.insts.insert(ROM_ADDR, builder);

        assert!(optimize_instruction_lookup(&mut rom).is_ok());

        // Verify all fields are preserved
        let stored = &rom.rom_instructions[0];
        assert_eq!(stored.op, 42);
        assert_eq!(stored.a_src, 1);
        assert_eq!(stored.b_src, 2);
        assert_eq!(stored.store, 3);
    }
}
