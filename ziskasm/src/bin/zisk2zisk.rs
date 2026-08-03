//! Executable that assembles ZisK assembly (`.zisk`) source into an x86-64 NASM
//! assembly file (the fast-emulator source), mirroring `riscv2zisk` but taking
//! `.zisk` input instead of a RISC-V ELF.

use zisk_core::{AsmGenerationMethod, ZiskRom2Asm};
use ziskasm::{assemble_files_with_defines, collect_zisk_files};

use std::{env, path::Path, process};

/// Assembles one or more `.zisk` files into an x86-64 assembly file.
/// The binary accepts 3 arguments (4 including the executable name):
/// -  the path of the input `.zisk` file, or a directory containing `.zisk` files
/// -  the path of the output x86-64 assembly file
/// -  the generation method
///
/// Usage mirrors `riscv2zisk`, except the input is `.zisk` source (a single file
/// or a directory of files) rather than a single RISC-V ELF file.
fn main() {
    // Get program arguments
    let args: Vec<String> = env::args().collect();

    // Check program arguments length
    if args.len() != 4 {
        eprintln!("Error parsing arguments: invalid number of arguments={}", args.len());
        for (i, arg) in args.iter().enumerate() {
            eprintln!("Argument {i}: {arg}");
        }
        eprintln!("Usage: zisk2zisk <zisk_file_or_dir> <x86-64_asm_file> <generation_method>");
        process::exit(1);
    }

    // Get the 3 arguments: the input .zisk path, the output ASM file and the generation method
    let zisk_path = args[1].clone();
    let asm_file = args[2].clone();
    let gen_arg = args[3].clone();
    println!("zisk2zisk converts ZisK assembly ({zisk_path}) into a ZISK ASM file ({asm_file}), using generation method {gen_arg}.");

    let generation_method = match gen_arg.as_str() {
        "--gen=0" => AsmGenerationMethod::AsmFast,
        "--gen=1" => AsmGenerationMethod::AsmMinimalTraces,
        "--gen=2" => AsmGenerationMethod::AsmRomHistogram,
        "--gen=7" => AsmGenerationMethod::AsmMemOp,
        _ => {
            eprintln!("Invalid generation method. Use --gen=0 (fast), =1 (minimal trace), =2 (rom histogram), =7 (mem op).");
            process::exit(1);
        }
    };

    // Collect the input .zisk files (a single file, or every .zisk file in a directory)
    let zisk_files = collect_zisk_files(&zisk_path).unwrap_or_else(|e| {
        eprintln!("Error collecting .zisk files: {e}");
        process::exit(1);
    });

    // Assemble the .zisk source into a ZiskRom. Since this always targets the
    // x86 assembly generator, define `ASM` so a program can `ifndef ASM` out any
    // ops the generator cannot emit (e.g. the Zba/Zbc/Zbkx/Zicond ops).
    let rom = assemble_files_with_defines(&zisk_files, &["ASM"]).unwrap_or_else(|e| {
        eprintln!("Error assembling .zisk source: {e}");
        process::exit(1);
    });

    // Generate the x86-64 assembly file from the ROM (same backend as riscv2zisk)
    ZiskRom2Asm::save_to_asm_file(&rom, Path::new(&asm_file), generation_method, true, true, false);

    // Return successfully
    process::exit(0);
}
