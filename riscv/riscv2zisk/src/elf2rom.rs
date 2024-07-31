//use core::num;
use crate::{
    zv2zisk::{add_entry_exit_jmp, add_zisk_code, add_zisk_init_data},
    RoData, ZiskRom, RAM_ADDR, RAM_SIZE, ROM_ENTRY,
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
                let (data_u8, _) = elf_bytes.section_data(&section_header)?;
                let mut data = data_u8.to_vec();

                while data.len() % 4 != 0 {
                    data.pop();
                }

                let addr = section_header.sh_addr;

                if (section_header.sh_flags & SHF_EXECINSTR as u64) != 0 {
                    add_zisk_code(&mut rom, addr, &data);
                }

                if (section_header.sh_flags & SHF_WRITE as u64) != 0 &&
                    addr >= RAM_ADDR &&
                    addr + data.len() as u64 <= RAM_ADDR + RAM_SIZE
                {
                    add_zisk_init_data(&mut rom, addr, &data);
                } else {
                    rom.ro_data.push(RoData::new(addr, data.len(), data));
                }
            }
        }
    }

    add_entry_exit_jmp(&mut rom, elf_bytes.ehdr.e_entry);

    Ok(rom)
}

/// Executes the file conversion process, and saves result into a file
pub fn elf2romfile(elf_file: String, rom_file: String) -> Result<(), Box<dyn Error>> {
    let rom = elf2rom(elf_file)?;
    rom.save_to_file(&rom_file);
    Ok(())
}
