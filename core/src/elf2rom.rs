//use core::num;
use crate::{
    zv2zisk::{add_entry_exit_jmp, add_zisk_code, add_zisk_init_data},
    RoData, ZiskInst, ZiskRom, RAM_ADDR, RAM_SIZE, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY,
};
use elf::{
    abi::{SHF_EXECINSTR, SHF_WRITE, SHT_PROGBITS},
    endian::AnyEndian,
    ElfBytes,
};
use std::error::Error;

/// Executes the file conversion process
pub fn elf2rom(elf_file: String) -> Result<ZiskRom, Box<dyn Error>> {
    let elf_file_path = std::path::PathBuf::from(elf_file.clone());
    let file_data = std::fs::read(elf_file_path)?;

    let elf_bytes = ElfBytes::<AnyEndian>::minimal_parse(file_data.as_slice())?;

    let mut rom: ZiskRom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };

    if let Some(section_headers) = elf_bytes.section_headers() {
        for section_header in section_headers {
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

                while data.len() % 4 != 0 {
                    data.pop();
                }

                // If this is a code section, add it to program
                if (section_header.sh_flags & SHF_EXECINSTR as u64) != 0 {
                    add_zisk_code(&mut rom, addr, &data);
                }

                // Add init data as a read/write memory section, initialized by code
                if (section_header.sh_flags & SHF_WRITE as u64) != 0 &&
                    addr >= RAM_ADDR &&
                    addr + data.len() as u64 <= RAM_ADDR + RAM_SIZE
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
    let mut max_rom_entry = 0;
    let mut max_rom_instructions = 0;

    let mut min_rom_na_unstructions = u64::MAX;
    let mut max_rom_na_unstructions = 0;
    for instruction in &rom.insts {
        let addr = *instruction.0;

        if addr < ROM_ENTRY {
            return Err(format!("Address out of range: {}", addr).into());
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
            return Err(format!("Address out of range: {}", addr).into());
        }
    }

    let num_rom_entry = (max_rom_entry - ROM_ENTRY) / 4 + 1;
    let num_rom_instructions = (max_rom_instructions - ROM_ADDR) / 4 + 1;
    let num_rom_na_instructions = if u64::MAX == min_rom_na_unstructions {
        0
    } else {
        max_rom_na_unstructions - min_rom_na_unstructions + 1
    };

    rom.rom_entry_instructions = vec![ZiskInst::default(); num_rom_entry as usize];
    rom.rom_instructions = vec![ZiskInst::default(); num_rom_instructions as usize];
    rom.rom_na_instructions = vec![ZiskInst::default(); num_rom_na_instructions as usize];
    rom.offset_rom_na_unstructions = min_rom_na_unstructions;

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

    //println! {"elf2rom() got rom.insts.len={}", rom.insts.len()};

    Ok(rom)
}

/// Executes the file conversion process, and saves result into a file
pub fn elf2romfile(
    elf_file: String,
    rom_file: String,
    pil_file: String,
    bin_file: String,
) -> Result<(), Box<dyn Error>> {
    let rom = elf2rom(elf_file)?;
    rom.save_to_json_file(&rom_file);
    if !pil_file.is_empty() {
        rom.save_to_pil_file(&pil_file);
    }
    if !bin_file.is_empty() {
        rom.save_to_bin_file(&bin_file);
    }
    Ok(())
}
