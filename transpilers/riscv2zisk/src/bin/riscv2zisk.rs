//! Executable that performs a transpilation of a RISC-V ELF file to a Zisk ROM file.

use riscv2zisk::Riscv2zisk;
use std::{env, process};

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
    let lang_c = args.iter().any(|a| a == "--lang=c");
    // Debugging: trace every executed instruction through the runtime's _print_pc()
    let print_pc = args.iter().any(|a| a == "--print-pc");
    // Target number of instructions per generated C function, 0 to emit the whole ROM as one
    let chunk_size: u64 = args
        .iter()
        .find_map(|a| a.strip_prefix("--chunk=").and_then(|v| v.parse().ok()))
        .unwrap_or(0);
    let args: Vec<String> = args
        .into_iter()
        .filter(|a| a != "--lang=c" && a != "--print-pc" && !a.starts_with("--chunk="))
        .collect();
    if args.len() != 4 {
        eprintln!("Error parsing arguments: invalid number of arguments={}", args.len());
        for (i, arg) in args.iter().enumerate() {
            eprintln!("Argument {i}: {arg}");
        }
        eprintln!(
            "Usage: riscv2zisk <riscv_elf_file> <output_file> <generation_method> \
             [--lang=c] [--chunk=<instructions_per_function>] [--print-pc]"
        );
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
        "--gen=7" => zisk_core::AsmGenerationMethod::AsmMemOp,
        _ => {
            eprintln!("Invalid generation method. Use --gen=0 (fast), =1 (minimal trace), =2 (rom histogram), =7 (mem op).");
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

    // Convert program, into assembly or into C
    let result = if lang_c {
        rv2zk.runfile_c(asm_file, generation_method, true, true, false, chunk_size, print_pc)
    } else {
        rv2zk.runfile(asm_file, generation_method, true, true, false)
    };
    if let Err(e) = result {
        println!("Application error: {e}");
        process::exit(1);
    }

    // Return successfully
    process::exit(0);
}
