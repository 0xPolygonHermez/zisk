//! Executable that performs a transpilation of a RISC-V ELF file to a Zisk ROM file.

use std::{env, process};

use zisk_core::Riscv2zisk;

/// Performs a transpilation of a RISC-V ELF file to a Zisk ROM file.  
/// The binary accepts 2 arguments: the path of the input RISC-V ELF file, and the path of the
/// output Zisk rom file.  
/// After parsing the arguments, the main function calls Riscv2zisk::runfile to perform the actual
/// work.
fn main() {
    println!("riscv2zisk converts an ELF RISCV file into a ZISK ASM file");

    // Get program arguments
    let args: Vec<String> = env::args().collect();

    // Check program arguments length
    if args.len() < 3 || args.len() > 5 {
        eprintln!("Error parsing arguments: invalid number of arguments.  Usage: riscv2zisk <elf_riscv_file> <zisk_asm_file> [<zisk_pil_file>] [<zisk_bin_file>]");
        process::exit(1);
    }

    // Get the 2 input parameters: ELF (RISCV) file name (input data) and ZisK file name (output
    // data)
    let elf_file = args[1].clone();
    let zisk_file = args[2].clone();
    let pil_file = if args.len() >= 4 { args[3].clone() } else { String::new() };
    let bin_file = if args.len() >= 5 { args[4].clone() } else { String::new() };

    println!("ELF file: {elf_file}");
    println!("ZISK file: {zisk_file}");
    if !pil_file.is_empty() {
        println!("PIL file: {pil_file}");
    }
    if !bin_file.is_empty() {
        println!("BIN file: {bin_file}");
    }

    // Create an instance of the program converter
    let rv2zk = Riscv2zisk::new(elf_file, zisk_file, pil_file, bin_file);

    // Convert program
    if let Err(e) = rv2zk.runfile() {
        println!("Application error: {e}");
        process::exit(1);
    }

    // Return successfully
    process::exit(0);
}
