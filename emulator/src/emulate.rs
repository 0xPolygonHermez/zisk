use crate::{Emu, EmuOptions, EmuTrace};
use riscv2zisk::{Riscv2zisk, ZiskRom};
use std::{
    fs,
    fs::metadata,
    path::{Path, PathBuf},
    process,
    time::Instant,
};

pub fn emulate(options: &EmuOptions, callback: Option<fn(&mut Vec<EmuTrace>)>) {
    // Log this call
    if options.verbose {
        println!("emulate()");
        println!("{}", options);
    }

    // INPUT:
    // build input data either from the provided input path, or leave it empty (default input)
    let input: Vec<u8> = if options.input.is_some() {
        // Read input data from the provided input path
        let path = std::path::PathBuf::from(options.input.clone().unwrap());
        std::fs::read(path).expect("Could not read input file")
    } else {
        // If no input data is provided, input will remain empty
        // This normally means that input data is self-contained in the program
        Vec::new()
    };

    if options.rom.is_some() && options.elf.is_some() {
        eprintln!(
            "Error parsing arguments: ROM file and ELF file are incompatible; use only one of them"
        );
        process::exit(1);
    } else if options.rom.is_some() {
        process_rom_file(options.rom.clone().unwrap(), &input, options, callback);
    } else if options.elf.is_some() {
        let elf_file = options.elf.clone().unwrap();
        let md = metadata(elf_file.clone()).unwrap();
        if md.is_file() {
            process_elf_file(elf_file, &input, options, callback);
        } else if md.is_dir() {
            process_directory(elf_file, &input, options, callback);
        }
    } else {
        eprintln!("Error parsing arguments: ROM file or ELF file must be provided");
        process::exit(1);
    }
}

fn process_directory(
    directory: String,
    input: &[u8],
    options: &EmuOptions,
    callback: Option<fn(&mut Vec<EmuTrace>)>,
) {
    if options.verbose {
        println!("process_directory() directory={}", directory);
    }

    let files = list_files(&directory);
    for file in files {
        if file.contains("dut") && file.ends_with(".elf") {
            process_elf_file(file, input, options, callback);
        }
    }
}

fn process_elf_file(
    elf_file: String,
    input: &[u8],
    options: &EmuOptions,
    callback: Option<fn(&mut Vec<EmuTrace>)>,
) {
    if options.verbose {
        println!("process_elf_file() elf_file={}", elf_file);
    }

    // Convert the ELF file to ZisK ROM
    let rom: ZiskRom = {
        // Create an instance of the RISCV -> ZisK program converter
        let rv2zk = Riscv2zisk::new(elf_file, String::new());

        // Convert program to rom
        let result = rv2zk.run();
        if result.is_err() {
            println!("Application error: {}", result.err().unwrap());
            process::exit(1);
        }

        // Get the result
        result.unwrap()
    };

    process_rom(&rom, input, options, callback);
}

fn process_rom_file(
    _rom_file: String,
    input: &[u8],
    options: &EmuOptions,
    callback: Option<fn(&mut Vec<EmuTrace>)>,
) {
    if options.verbose {
        println!("process_rom_file() rom_file={}", _rom_file);
    }

    // TODO: load from file
    let rom: ZiskRom = ZiskRom::new();
    process_rom(&rom, input, options, callback);
}

pub fn process_rom(
    rom: &ZiskRom,
    input: &[u8],
    options: &EmuOptions,
    callback: Option<fn(&mut Vec<EmuTrace>)>,
) {
    if options.verbose {
        println!("process_rom() rom size={} input size={}", rom.insts.len(), input.len());
    }

    // Create a emulator instance with this rom and input
    let mut emu = Emu::new(rom, input.to_owned(), options.clone(), callback);

    let start = Instant::now();

    // Run the emulation
    emu.run();
    if !emu.terminated() {
        println!("Emulation did not complete");
        process::exit(1);
    }

    let duration = start.elapsed();

    // Log performance metrics
    if options.log_metrics {
        let secs = duration.as_secs_f64();
        let steps = emu.number_of_steps() as f64;
        let tp = steps / secs / 1000000.0;
        println!("process_rom() steps={} duration={:.4} tp={:.4} Msteps/s", steps, secs, tp);
    }

    // OUTPUT:
    // if requested, save output to file, or log it to console
    if options.output.is_some() {
        // Get the emulation output as a u8 vector
        let output = emu.get_output_8();

        // Save the output to file
        let output_file = <Option<std::string::String> as Clone>::clone(&options.output).unwrap();
        fs::write(output_file, output).expect("Unable to write output file");
    }
    // Log output to console
    else if options.log_output {
        // Get the emulation output as a u32 vector
        let output = emu.get_output_32();

        // Log the output to console
        for o in &output {
            println!("{:08x}", o);
        }
    }
}

fn list_files(directory: &String) -> Vec<String> {
    let path = Path::new(directory);
    let paths = list_files_paths(path);
    let mut vec: Vec<String> = Vec::new();
    for p in paths {
        vec.push(p.display().to_string());
    }
    vec
}

fn list_files_paths(path: &Path) -> Vec<PathBuf> {
    let mut vec = Vec::new();
    _list_files(&mut vec, path);
    vec
}

fn _list_files(vec: &mut Vec<PathBuf>, path: &Path) {
    if metadata(path).unwrap().is_dir() {
        let paths = fs::read_dir(path).unwrap();
        for path_result in paths {
            let full_path = path_result.unwrap().path();
            if metadata(&full_path).unwrap().is_dir() {
                _list_files(vec, &full_path);
            } else {
                vec.push(full_path);
            }
        }
    }
}
