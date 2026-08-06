//! Executable that assembles ZisK assembly (`.zisk`) source into either an
//! x86-64 NASM assembly file (the fast-emulator source, mirroring `riscv2zisk`)
//! or a ziskbin ELF file (an embedded, already-built ZiskRom that the ELF-based
//! toolchain — `ziskemu -e`, `cargo-zisk` — can consume directly).

use zisk_core::{ziskbin, AsmGenerationMethod, ZiskRom2Asm};
use ziskasm::{assemble_files_with_defines, collect_zisk_files};

use std::{env, fs, path::Path, process};

/// Assembles one or more `.zisk` files into an output file. Arguments (4 incl. the
/// executable name):
/// -  the input `.zisk` file, or a directory containing `.zisk` files
/// -  the output file path
/// -  the mode: `--gen=0|1|2|7` for x86-64 assembly, or `--elf` for a ziskbin ELF
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
        eprintln!("Usage: zisk2zisk <zisk_file_or_dir> <output_file> <--gen=0|1|2|7 | --elf>");
        process::exit(1);
    }

    // Get the 3 arguments: the input .zisk path, the output file and the mode
    let zisk_path = args[1].clone();
    let out_file = args[2].clone();
    let mode = args[3].clone();

    // `--elf` emits a ziskbin ELF (a full-fidelity ROM); `--gen=N` emits x86-64 asm.
    let elf_mode = mode == "--elf";

    let generation_method = if elf_mode {
        None
    } else {
        Some(match mode.as_str() {
            "--gen=0" => AsmGenerationMethod::AsmFast,
            "--gen=1" => AsmGenerationMethod::AsmMinimalTraces,
            "--gen=2" => AsmGenerationMethod::AsmRomHistogram,
            "--gen=7" => AsmGenerationMethod::AsmMemOp,
            _ => {
                eprintln!("Invalid mode. Use --gen=0 (fast), =1 (minimal trace), =2 (rom histogram), =7 (mem op), or --elf (ziskbin ELF).");
                process::exit(1);
            }
        })
    };

    let kind = if elf_mode { "ziskbin ELF" } else { "ZISK ASM" };
    println!("zisk2zisk converts ZisK assembly ({zisk_path}) into a {kind} file ({out_file}), mode {mode}.");

    // Collect the input .zisk files (a single file, or every .zisk file in a directory)
    let zisk_files = collect_zisk_files(&zisk_path).unwrap_or_else(|e| {
        eprintln!("Error collecting .zisk files: {e}");
        process::exit(1);
    });

    // Assemble the .zisk source into a ZiskRom. The x86 asm target defines `ASM`
    // so a program can `ifndef ASM` out ops the generator cannot emit
    // (Zba/Zbc/Zbkx/Zicond); the ELF carries the full-fidelity ROM (no `ASM`),
    // matching what `ziskemu -z` produces.
    let defines: &[&str] = if elf_mode { &[] } else { &["ASM"] };
    let rom = assemble_files_with_defines(&zisk_files, defines).unwrap_or_else(|e| {
        eprintln!("Error assembling .zisk source: {e}");
        process::exit(1);
    });

    match generation_method {
        // Emit the ziskbin ELF (embedded ZiskRom).
        None => {
            let bytes = ziskbin::rom_to_elf(&rom);
            fs::write(&out_file, &bytes).unwrap_or_else(|e| {
                eprintln!("Error writing ELF file {out_file}: {e}");
                process::exit(1);
            });
            println!("Wrote {} bytes to {out_file}", bytes.len());
        }
        // Generate the x86-64 assembly file from the ROM (same backend as riscv2zisk).
        Some(method) => {
            ZiskRom2Asm::save_to_asm_file(&rom, Path::new(&out_file), method, true, true, false);
        }
    }

    // Return successfully
    process::exit(0);
}
