use riscv2zisk::Riscv2zisk;
use std::{env, process};

fn main() {
    println!("riscv2zisk converts an ELF RISCV file into a ZISK ASM file");

    // Get program arguments
    let args: Vec<String> = env::args().collect();

    // Check program arguments length
    if args.len() != 3 {
        eprintln!("Error parsing arguments: number of arguments should be 2.  Usage: riscv2zisk <elf_riscv_file> <zisk_asm_file>");
        process::exit(1);
    }

    // Get the 2 input parameters: ELF (RISCV) file name (input data) and ZisK file name (output
    // data)
    let elf_file = args[1].clone();
    let zisk_file = args[2].clone();

    println!("ELF file: {elf_file}");
    println!("ZISK file: {zisk_file}");

    // Create an instance of the program converter
    let rv2zk = Riscv2zisk::new(elf_file, zisk_file);

    // Convert program
    if let Err(e) = rv2zk.runfile() {
        println!("Application error: {e}");
        process::exit(1);
    }

    // Return successfully
    process::exit(0);
}
