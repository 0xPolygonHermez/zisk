use clap::Parser;
use std::process;
use ziskemu::{emulate, EmuOptions};

fn main() {
    // Create a emulator options instance based on arguments or default values
    let options: EmuOptions = EmuOptions::parse();

    // Log the emulator options if requested
    if options.verbose {
        println!("ziskemu converts an ELF RISCV file into a ZISK rom or loads a ZISK rom file, emulates it with the provided input, and copies the output to console or a file");
    }

    // Call emulate, with these options
    emulate(&options);

    // Return successfully
    process::exit(0);
}
