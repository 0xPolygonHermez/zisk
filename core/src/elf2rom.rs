//! Reads RISC-V data from and ELF file and converts it to a ZiskRom

use crate::{
    add_end_jmp, is_elf_file,
    riscv2zisk_context::{add_entry_exit_jmp, add_zisk_code, add_zisk_init_data},
    AsmGenerationMethod, RoData, ZiskInst, ZiskRom, ZiskRom2Asm, RAM_ADDR, RAM_SIZE, ROM_ADDR,
    ROM_ADDR_MAX, ROM_ENTRY,
};
use elf::{
    abi::{SHF_EXECINSTR, SHF_WRITE, SHT_PROGBITS},
    endian::AnyEndian,
    ElfBytes,
};
use rayon::prelude::*;
use std::{error::Error, path::Path};

/// Executes the ROM transpilation process: from ELF to Zisk
pub fn elf2rom(elf_file: &Path) -> Result<ZiskRom, Box<dyn Error>> {
    // Get all data from the ELF file copied to a memory buffer
    let elf_file_path = std::path::PathBuf::from(elf_file);
    let file_data = std::fs::read(elf_file_path)?;

    match is_elf_file(&file_data) {
        Ok(is_file) => {
            if !is_file {
                panic!("ROM file is not a valid ELF file");
            }
        }
        Err(_) => {
            panic!("Error reading ROM file");
        }
    }

    // Parse the ELF data
    let elf_bytes = ElfBytes::<AnyEndian>::minimal_parse(file_data.as_slice())?;

    // Create an empty ZiskRom instance
    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    // Add the end instruction, jumping over it
    add_end_jmp(&mut rom);

    // Iterate on the available section headers of the ELF parsed data
    if let Some(section_headers) = elf_bytes.section_headers() {
        for section_header in section_headers {
            // Consider only the section headers that contain program data
            if section_header.sh_type == SHT_PROGBITS {
                // Get the section header address
                let addr = section_header.sh_addr;

                // Ignore sections with address = 0, as per ELF spec
                if addr == 0 {
                    continue;
                }

                // Get the section data
                let (data_u8, _) = elf_bytes.section_data(&section_header)?;
                let mut data = data_u8.to_vec();

                // Remove extra bytes if length is not 4-bytes aligned
                while data.len() % 4 != 0 {
                    data.pop();
                }

                // If this is a code section, add it to program
                if (section_header.sh_flags & SHF_EXECINSTR as u64) != 0 {
                    add_zisk_code(&mut rom, addr, &data);
                }

                // Add init data as a read/write memory section, initialized by code
                // If the data is a writable memory section, add it to the ROM memory using Zisk
                // copy instructions
                if (section_header.sh_flags & SHF_WRITE as u64) != 0
                    && addr >= RAM_ADDR
                    && addr + data.len() as u64 <= RAM_ADDR + RAM_SIZE
                {
                    //println! {"elf2rom() new RW from={:x} length={:x}={}", addr, data.len(),
                    //data.len()};
                    add_zisk_init_data(&mut rom, addr, &data, true);
                }
                // Add read-only data memory section
                else {
                    // Search for an existing RO section previous to this one
                    let mut found = false;
                    for rd in rom.ro_data.iter_mut() {
                        // Section data should be previous to this one
                        if (rd.from + rd.length as u64) == addr {
                            rd.length += data.len();
                            rd.data.extend(data.clone());
                            found = true;
                            //println! {"elf2rom() adding RO from={:x} length={:x}={}", rd.from,
                            // rd.length, rd.length};
                            break;
                        }
                    }

                    // If not found, create a new RO section
                    if !found {
                        //println! {"elf2rom() new RO from={:x} length={:x}={}", addr, data.len(),
                        // data.len()};
                        rom.ro_data.push(RoData::new(addr, data.len(), data));
                    }
                }
            }
        }
    }

    // Add RO data initialization code insctructions
    let ro_data_len = rom.ro_data.len();
    for i in 0..ro_data_len {
        let addr = rom.ro_data[i].from;
        let mut data = Vec::new();
        data.extend(rom.ro_data[i].data.as_slice());
        add_zisk_init_data(&mut rom, addr, &data, true);
    }

    add_entry_exit_jmp(&mut rom, elf_bytes.ehdr.e_entry);

    // Preprocess the ROM (experimental)
    // Split the ROM instructions based on their address in order to get a better performance when
    // searching for the corresponding intruction to the pc program address
    let mut max_rom_entry = 0;
    let mut max_rom_instructions = 0;
    let mut min_rom_na_unstructions = u64::MAX;
    let mut max_rom_na_unstructions = 0;

    // Prepare sorted pc list
    rom.sorted_pc_list.reserve(rom.insts.len());

    for instruction in &rom.insts {
        let addr = *instruction.0;

        // Add to pc list (still unsorted)
        rom.sorted_pc_list.push(addr);

        if addr < ROM_ENTRY {
            return Err(format!("Address out of range: {addr}").into());
        } else if addr < ROM_ADDR {
            if addr % 4 != 0 {
                // When an address is not 4 bytes aligned, it is considered a
                // na_rom_instructions We are supposed to have only one non
                // aligned instructions in > ROM_ADDRESS
                min_rom_na_unstructions = std::cmp::min(min_rom_na_unstructions, addr);
                max_rom_na_unstructions = std::cmp::max(max_rom_na_unstructions, addr);
            } else {
                max_rom_entry = std::cmp::max(max_rom_entry, addr);
            }
        } else if addr < ROM_ADDR_MAX {
            if addr % 4 != 0 {
                // When an address is not 4 bytes aligned, it is considered a
                // na_rom_instructions We are supposed to have only one non
                // aligned instructions in > ROM_ADDRESS
                min_rom_na_unstructions = std::cmp::min(min_rom_na_unstructions, addr);
                max_rom_na_unstructions = std::cmp::max(max_rom_na_unstructions, addr);
            } else {
                max_rom_instructions = max_rom_instructions.max(addr);
            }
        } else {
            return Err(format!("Address out of range: {addr}").into());
        }
    }
    rom.max_bios_pc = max_rom_entry;
    rom.max_program_pc = max_rom_instructions;

    let num_rom_entry = (max_rom_entry - ROM_ENTRY) / 4 + 1;
    let num_rom_instructions = (max_rom_instructions - ROM_ADDR) / 4 + 1;
    let num_rom_na_instructions = if u64::MAX == min_rom_na_unstructions {
        0
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

    for instruction in &rom.insts {
        let addr = *instruction.0;

        if addr % 4 != 0 {
            rom.rom_na_instructions[(addr - min_rom_na_unstructions) as usize] =
                instruction.1.i.clone();
        } else if addr < ROM_ADDR {
            rom.rom_entry_instructions[((addr - ROM_ENTRY) >> 2) as usize] =
                instruction.1.i.clone();
        } else {
            rom.rom_instructions[((addr - ROM_ADDR) >> 2) as usize] = instruction.1.i.clone();
        }
    }

    // Link every instruction with the position they occupy in the sorted pc list
    for i in 0..rom.sorted_pc_list.len() {
        let pc = rom.sorted_pc_list[i];
        rom.insts.get_mut(&pc).unwrap().i.sorted_pc_list_index = i;
        let inst = rom.get_mut_instruction(pc);
        inst.sorted_pc_list_index = i;
    }

    //println! {"elf2rom() got rom.insts.len={}", rom.insts.len()};

    Ok(rom)
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
