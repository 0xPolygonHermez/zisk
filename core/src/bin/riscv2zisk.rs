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
    if args.len() < 3 || args.len() > 4 {
        eprintln!("Error parsing arguments: invalid number of arguments.  Usage: riscv2zisk <elf_riscv_file> [<i86-64_asm_file>] <generation_method>");
        process::exit(1);
    }

    // Get the 2 input parameters: ELF (RISCV) file name (input data) and ZisK file name (output
    // data)
    let elf_file = args[1].clone();
    let (asm_file, gen_arg) = if args.len() == 4 {
        (Some(args[2].clone()), args[3].clone())
    } else {
        (None, args[2].clone())
    };

    let generation_method = match gen_arg.as_str() {
        "--gen=1" => zisk_core::AsmGenerationMethod::AsmMinimalTraces,
        "--gen=2" => zisk_core::AsmGenerationMethod::AsmRomHistogram,
        _ => {
            eprintln!("Invalid generation method. Use --gen=1 or --gen=2.");
            process::exit(1);
        }
    };

    // Create an instance of the program converter
    let rv2zk = Riscv2zisk::new(elf_file, asm_file);

    // Convert program
    if let Err(e) = rv2zk.runfile(generation_method) {
        println!("Application error: {e}");
        process::exit(1);
    }

    // Return successfully
    process::exit(0);
}
