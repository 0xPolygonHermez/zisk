//! Executable that performs a transpilation of a guest program (RISC-V ELF, ziskbin ELF or
//! WebAssembly) to a Zisk ROM file.  The name is historical: it predates WebAssembly support.

use std::{env, process};
use zisk_transpiler::ZiskTranspiler;

/// Performs a transpilation of a guest program to a Zisk ROM file.  
/// The binary accepts 3 arguments (4 including the executable name):
/// -  the path of the input program file
/// -  the path of the output Zisk rom file  
/// -  the generation method
///
/// After parsing the arguments, the main function calls ZiskTranspiler::runfile to perform the actual
/// work.
fn main() {
    // Get program arguments
    let args: Vec<String> = env::args().collect();

    // Check program arguments length
    if args.len() != 4 {
        eprintln!("Error parsing arguments: invalid number of arguments={}", args.len());
        for (i, arg) in args.iter().enumerate() {
            eprintln!("Argument {i}: {arg}");
        }
        eprintln!(
            "Usage: zisk-transpiler-riscv <program_file> <x86-64_asm_file> <generation_method>"
        );
        process::exit(1);
    }

    // Get the 3 arguments: the input program file, the output ASM file and the generation method
    let elf_file = args[1].clone();
    let asm_file = args[2].clone();
    let gen_arg = args[3].clone();
    println!("zisk-transpiler-riscv converts a guest program ({elf_file}) into a ZISK ASM file ({asm_file}), using generation method {gen_arg}.");

    let generation_method = match gen_arg.as_str() {
        "--gen=0" => zisk_core::AsmGenerationMethod::AsmFast,
        "--gen=1" => zisk_core::AsmGenerationMethod::AsmMinimalTraces,
        "--gen=2" => zisk_core::AsmGenerationMethod::AsmRomHistogram,
        "--gen=7" => zisk_core::AsmGenerationMethod::AsmMemOp,
        _ => {
            eprintln!("Invalid generation method. Use --gen=0 (fast), =1 (minimal trace), =2 (rom histogram), =7 (mem op).");
            process::exit(1);
        }
    };

    // Read the program file bytes
    let elf = std::fs::read(elf_file).unwrap_or_else(|e| {
        eprintln!("Error reading program file: {e}");
        process::exit(1);
    });

    // Create an instance of the program converter
    let transpiler = ZiskTranspiler::new(&elf);

    // Convert program
    if let Err(e) = transpiler.runfile(asm_file, generation_method, true, true, false) {
        println!("Application error: {e}");
        process::exit(1);
    }

    // Return successfully
    process::exit(0);
}
