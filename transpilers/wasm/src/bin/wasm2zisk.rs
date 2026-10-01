//! Executable that transpiles a WebAssembly (`wasm32-wasip1`) guest to a Zisk ROM file: the
//! WebAssembly counterpart of `riscv2zisk`.

use std::{env, path::Path, process};
use zisk_core::is_wasm_file;
use zisk_core::zisk_rom_2_asm::{AsmGenerationMethod, ZiskRom2Asm};
use zisk_transpiler_wasm::wasm2rom;

/// Transpiles a WebAssembly guest and saves the ROM as an x86-64 NASM file.
/// The binary accepts 3 arguments (4 including the executable name):
/// -  the path of the input `.wasm` file
/// -  the path of the output Zisk rom file
/// -  the generation method
fn main() {
    // Get program arguments
    let args: Vec<String> = env::args().collect();

    // Check program arguments length
    if args.len() != 4 {
        eprintln!("Error parsing arguments: invalid number of arguments={}", args.len());
        for (i, arg) in args.iter().enumerate() {
            eprintln!("Argument {i}: {arg}");
        }
        eprintln!("Usage: wasm2zisk <wasm_file> <i86-64_asm_file> <generation_method>");
        process::exit(1);
    }

    // Get the 3 arguments: the input wasm file, the output ASM file and the generation method
    let wasm_file = &args[1];
    let asm_file = &args[2];
    let gen_arg = &args[3];
    println!("wasm2zisk converts a WebAssembly file ({wasm_file}) into a ZISK ASM file ({asm_file}), using generation method {gen_arg}.");

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

    // Read the wasm file bytes
    let wasm = std::fs::read(wasm_file).unwrap_or_else(|e| {
        eprintln!("Error reading wasm file: {e}");
        process::exit(1);
    });
    if !is_wasm_file(&wasm) {
        eprintln!("Error: {wasm_file} is not a WebAssembly binary (missing \\0asm magic)");
        process::exit(1);
    }

    // Convert program
    let rom = wasm2rom(&wasm).unwrap_or_else(|e| {
        println!("Application error: {e}");
        process::exit(1);
    });
    ZiskRom2Asm::save_to_asm_file(&rom, Path::new(asm_file), generation_method, true, true, false);

    // Return successfully
    process::exit(0);
}
