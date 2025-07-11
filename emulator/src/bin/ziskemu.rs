use clap::Parser;
use std::{fmt::Write, process};
use zisk_common::EmuTrace;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator};

fn main() {
    // Create a emulator options instance based on arguments or default values
    let options: EmuOptions = EmuOptions::parse();

    //println! {"options={}", options};

    // Log the emulator options if requested
    if options.verbose {
        println!("ziskemu converts an ELF RISCV file into a ZISK rom or loads a ZISK rom file, emulates it with the provided input, and copies the output to console or a file");
    }

    // Call emulate, with these options
    let emulator = ZiskEmulator;
    let result = emulator.emulate(&options, None::<Box<dyn Fn(EmuTrace)>>);

    match result {
        Ok(result) => {
            // println!("Emulation completed successfully");
            result.iter().fold(String::new(), |mut acc, byte| {
                write!(&mut acc, "{byte:02x}").unwrap();
                acc
            });
            // print!("Result: 0x{}", hex_string);
        }
        Err(e) => {
            eprintln!("Error during emulation: {e:?}");
            process::exit(1);
        }
    }
}
