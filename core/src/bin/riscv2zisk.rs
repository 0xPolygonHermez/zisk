//! Executable that performs a transpilation of a RISC-V ELF file to a Zisk ROM file.

use std::{env, process};

use zisk_core::Riscv2zisk;

/// Performs a transpilation of a RISC-V ELF file to a Zisk ROM file.  
/// The binary accepts 3 arguments (4 including the executable name):
/// -  the path of the input RISC-V ELF file
/// -  the path of the output Zisk rom file  
/// -  the generation method
///
/// After parsing the arguments, the main function calls Riscv2zisk::runfile to perform the actual
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
        eprintln!("Usage: riscv2zisk <riscv_elf_file> <i86-64_asm_file> <generation_method>");
        process::exit(1);
    }

    // Get the 3 arguments: the input ELF file, the output ASM file and the generation method
    let elf_file = args[1].clone();
    let asm_file = args[2].clone();
    let gen_arg = args[3].clone();
    println!("riscv2zisk converts a RISCV ELF file ({elf_file}) into a ZISK ASM file ({asm_file}), using generation method {gen_arg}.");

    let generation_method = match gen_arg.as_str() {
        "--gen=0" => zisk_core::AsmGenerationMethod::AsmFast,
        "--gen=1" => zisk_core::AsmGenerationMethod::AsmMinimalTraces,
        "--gen=2" => zisk_core::AsmGenerationMethod::AsmRomHistogram,
        "--gen=3" => zisk_core::AsmGenerationMethod::AsmMainTrace,
        "--gen=4" => zisk_core::AsmGenerationMethod::AsmChunks,
        //"--gen=5" => zisk_core::AsmGenerationMethod::AsmBusOp,
        "--gen=6" => zisk_core::AsmGenerationMethod::AsmZip,
        "--gen=7" => zisk_core::AsmGenerationMethod::AsmMemOp,
        "--gen=8" => zisk_core::AsmGenerationMethod::AsmChunkPlayerMTCollectMem,
        "--gen=9" => zisk_core::AsmGenerationMethod::AsmMemReads,
        "--gen=10" => zisk_core::AsmGenerationMethod::AsmChunkPlayerMemReadsCollectMain,
        _ => {
            eprintln!("Invalid generation method. Use --gen=0 (fast), =1 (minimal trace), =2 (rom histogram), =3 (main trace), =4 (chunks), =5 (bus op), =6 (zip), =7 (mem op), =8 (min trace chunk player), =9 (mem reads), =10 (mem reads chunk player).");
            process::exit(1);
        }
    };

    // Read ELF file bytes
    let elf = std::fs::read(elf_file).unwrap_or_else(|e| {
        eprintln!("Error reading ELF file: {e}");
        process::exit(1);
    });

    // Create an instance of the program converter
    let rv2zk = Riscv2zisk::new(&elf);

    // Convert program
    if let Err(e) = rv2zk.runfile(asm_file, generation_method, true, true, false) {
        println!("Application error: {e}");
        process::exit(1);
    }

    // Return successfully
    process::exit(0);
}
