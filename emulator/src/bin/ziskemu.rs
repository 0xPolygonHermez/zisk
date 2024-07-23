use clap::Parser;
use riscv2zisk::{Riscv2zisk, ZiskRom};
use std::{fs, process};
use ziskemu::{Emu, EmuOptions};

fn main() {
    // Create a emulator options instance based on arguments or default values
    let emu_options: EmuOptions = EmuOptions::parse();

    // Log the emulator options if requested
    if emu_options.verbose {
        println!("ziskemu converts an ELF RISCV file into a ZISK rom or loads a ZISK rom file, emulates it with the provided input, and copies the output to console or a file");
        println!("{}", emu_options);
    }

    // INPUT:
    // build input data either from the provided input path, or leave it empty (default input)
    let input: Vec<u8> = if emu_options.input.is_some() {
        // Read input data from the provided input path
        let path = std::path::PathBuf::from(emu_options.input.clone().unwrap());
        std::fs::read(path).expect("Could not read input file")
    } else {
        // If no input data is provided, input will remain empty
        // This normally means that input data is self-contained in the program
        Vec::new()
    };

    // ROM:
    // convert it from the ELF file (if provided) or get it from ROM file (if provided)
    let rom: ZiskRom = if emu_options.elf.is_some() {
        if emu_options.rom.is_some() {
            eprintln!("Error parsing arguments: ROM file and ELF file are incompatible; use only one of them");
            process::exit(1);
        }
        // Create an instance of the RISCV -> ZisK program converter
        let rv2zk = Riscv2zisk::new(emu_options.elf.clone().unwrap(), String::new());

        // Convert program to rom
        let result = rv2zk.run();
        if result.is_err() {
            println!("Application error: {}", result.err().unwrap());
            process::exit(1);
        }

        // Get the result
        result.unwrap()
    } else if emu_options.rom.is_some() {
        // TODO: load rom from file
        ZiskRom::new()
    } else {
        eprintln!("Error parsing arguments: either a ROM file or an ELF file must be provided");
        process::exit(1);
    };

    // Create a emulator instance with this rom and input
    let mut emu = Emu::new(rom, input.clone());

    // Run the emulation
    emu.run(&emu_options);
    if !emu.terminated() {
        println!("Emuulation did not complete");
        process::exit(1);
    }

    // OUTPUT:
    // if requested, save output to file, or log it to console
    if emu_options.output.is_some() {
        // Get the emulation output as a u8 vector
        let output = emu.get_output_8();

        // Save the output to file
        fs::write("/tmp/foo", output).expect("Unable to write output file");
    }
    // Log output to console
    else {
        // Get the emulation output as a u32 vector
        let output = emu.get_output_32();

        // Log the output to console
        for o in &output {
            println!("{:08x}", o);
        }
    }

    // Return successfully
    process::exit(0);
}
