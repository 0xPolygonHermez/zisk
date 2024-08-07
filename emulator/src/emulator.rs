use crate::{Emu, EmuOptions, EmuTrace, ErrWrongArguments, ZiskEmulatorErr};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use zisk_core::{Riscv2zisk, ZiskInst, ZiskRom, ROM_ADDR, ROM_ADDR_MAX, ROM_ENTRY};

pub trait Emulator<ET> {
    fn emulate(
        &self,
        options: &EmuOptions,
        callback: Option<Box<dyn Fn(ET)>>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr>;
}

pub struct ZiskEmulator;

impl ZiskEmulator {
    fn process_directory(
        directory: String,
        inputs: &[u8],
        options: &EmuOptions,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_directory() directory={}", directory);
        }

        let files = Self::list_files(&directory).unwrap();
        for file in files {
            if file.contains("dut") && file.ends_with(".elf") {
                Self::process_elf_file(file, inputs, options, None)?;
            }
        }

        Ok(Vec::new())
    }

    fn process_elf_file(
        elf_filename: String,
        inputs: &[u8],
        options: &EmuOptions,
        callback: Option<Box<dyn Fn(EmuTrace)>>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_elf_file() elf_file={}", elf_filename);
        }

        // Convert the ELF file to ZisK ROM
        // Create an instance of the RISCV -> ZisK program converter
        let riscv2zisk = Riscv2zisk::new(elf_filename, String::new());

        // Convert program to rom
        let zisk_rom = riscv2zisk.run();
        if zisk_rom.is_err() {
            return Err(ZiskEmulatorErr::Unknown(zisk_rom.err().unwrap().to_string()));
        }

        Self::process_rom(&mut zisk_rom.unwrap(), inputs, options, callback)
    }

    fn process_rom_file(
        rom_filename: String,
        inputs: &[u8],
        options: &EmuOptions,
        callback: Option<Box<dyn Fn(EmuTrace)>>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_rom_file() rom_file={}", rom_filename);
        }

        // TODO: load from file
        let mut rom: ZiskRom = ZiskRom::new();
        Self::process_rom(&mut rom, inputs, options, callback)
    }

    pub fn process_rom(
        rom: &mut ZiskRom,
        inputs: &[u8],
        options: &EmuOptions,
        callback: Option<Box<dyn Fn(EmuTrace)>>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        if options.verbose {
            println!("process_rom() rom size={} inputs size={}", rom.insts.len(), inputs.len());
        }

        // Preprocess the ROM (experimental)
        let mut max_rom_entry = 0;
        let mut max_rom_instructions = 0;

        let mut min_rom_na_unstructions = u64::MAX;
        let mut max_rom_na_unstructions = 0;
        for instruction in &rom.insts {
            let addr = *instruction.0;

            if addr < ROM_ENTRY {
                return Err(ZiskEmulatorErr::AddressOutOfRange(addr));
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
                return Err(ZiskEmulatorErr::AddressOutOfRange(addr));
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

        // Create a emulator instance with this rom and inputs
        let mut emu = Emu::new(rom);
        let start = Instant::now();

        // Run the emulation
        emu.run(inputs.to_owned(), options, callback);

        if !emu.terminated() {
            return Err(ZiskEmulatorErr::EmulationNoCompleted)
        }

        let duration = start.elapsed();

        // Log performance metrics
        if options.log_metrics {
            let secs = duration.as_secs_f64();
            let steps = emu.number_of_steps();
            let tp = steps as f64 / secs / 1_000_000.0;
            let cpus = cpu_freq::get();
            let cpu_frequency: f64 = cpus[0].max.unwrap() as f64;
            let clocks_per_step = cpu_frequency / tp;
            println!(
                "process_rom() steps={} duration={:.4} tp={:.4} Msteps/s freq={:.4} {:.4} clocks/step",
                steps, secs, tp, cpu_frequency, clocks_per_step
            );
        }

        // Get the emulation output
        let output = emu.get_output_8();

        // OUTPUT:
        // Save output to a file if requested
        if options.output.is_some() {
            fs::write(options.output.as_ref().unwrap(), &output)
                .map_err(|e| ZiskEmulatorErr::Unknown(e.to_string()))?
        }

        // Log output to console if requested
        if options.log_output {
            // Get the emulation output as a u32 vector
            let output = emu.get_output_32();

            // Log the output to console
            for o in &output {
                println!("{:08x}", o);
            }
        }

        Ok(output)
    }

    fn list_files(directory: &str) -> std::io::Result<Vec<String>> {
        fn _list_files(vec: &mut Vec<PathBuf>, path: &Path) -> std::io::Result<()> {
            if path.is_dir() {
                for entry in fs::read_dir(path)? {
                    let entry = entry?;
                    let full_path = entry.path();
                    if full_path.is_dir() {
                        _list_files(vec, &full_path)?;
                    } else {
                        vec.push(full_path);
                    }
                }
            }
            Ok(())
        }

        let mut paths = Vec::new();
        _list_files(&mut paths, Path::new(directory))?;
        Ok(paths.into_iter().map(|p| p.display().to_string()).collect())
    }
}

impl Emulator<EmuTrace> for ZiskEmulator {
    fn emulate(
        &self,
        options: &EmuOptions,
        callback: Option<Box<dyn Fn(EmuTrace)>>,
    ) -> Result<Vec<u8>, ZiskEmulatorErr> {
        // Log this call
        if options.verbose {
            println!("emulate()\n{}", options);
        }

        // Check options are valid
        if options.rom.is_some() && options.elf.is_some() {
            return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                "ROM file and ELF file are incompatible; use only one of them",
            )))
        } else if options.rom.is_none() && options.elf.is_none() {
            return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                "ROM file or ELF file must be provided",
            )))
        }

        // INPUTs:
        // build inputs data either from the provided inputs path, or leave it empty (default
        // inputs)
        let mut inputs = Vec::new();
        if options.inputs.is_some() {
            // Read inputs data from the provided inputs path
            let path = PathBuf::from(options.inputs.clone().unwrap());
            inputs = fs::read(path).expect("Could not read inputs file");
        }

        if options.rom.is_some() {
            let rom_filename = options.rom.clone().unwrap();

            let metadata = fs::metadata(&rom_filename);
            if metadata.is_err() {
                return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                    "ROM file does not exist",
                )))
            }

            let metadata = metadata.unwrap();
            if metadata.is_dir() {
                return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                    "ROM file must be a file",
                )))
            }

            Self::process_rom_file(rom_filename, &inputs, options, callback)
        } else {
            let elf_filename = options.elf.clone().unwrap();

            let metadata = fs::metadata(&elf_filename);
            if metadata.is_err() {
                return Err(ZiskEmulatorErr::WrongArguments(ErrWrongArguments::new(
                    "ELF file does not exist",
                )))
            }

            let metadata = metadata.unwrap();
            if metadata.is_dir() {
                Self::process_directory(elf_filename, &inputs, options)
            } else {
                Self::process_elf_file(elf_filename, &inputs, options, callback)
            }
        }
    }
}
