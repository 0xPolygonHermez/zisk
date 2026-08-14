//! zisk2lean — assemble `.zisk` source with the canonical ziskasm assembler
//! and emit the resulting `ZiskRom` as Lean 4 source, for consumption by the
//! zisk-sw-fv formal-verification project.
//!
//! The dump includes every instruction of the ROM (BIOS entry/exit and
//! end/lib blocks included) as `(paddr, instruction)` pairs. Instructions
//! whose op or addressing mode is outside the Lean model's supported subset
//! are a hard error — never silently skipped or approximated.
//!
//! Usage: zisk2lean <zisk_file_or_dir> <out.lean> [lean_def_name]

use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};
use zisk_core::{
    zisk_rom::DataSection64, ZiskInst, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG, SRC_STEP,
    STORE_IND, STORE_MEM, STORE_NONE, STORE_REG, ZISKLIB_RAM_ADDR, ZISKLIB_ROM_ADDR,
};
use ziskasm::{
    assemble_files_with_symbols, assemble_zisk_library, collect_zisk_files, ZISK_LIBRARY,
};

/// Ops modeled by zisk-sw-fv (`Zisk/Op.lean`); constructor names are the
/// zisk op-name strings, so `op_str` maps directly to `.{op_str}`.
/// TO BE REMOVED IN A FUTURE VERSION
const SUPPORTED_OPS: &[&str] = &[
    "flag",
    "copyb",
    "minu",
    "min",
    "maxu",
    "max",
    "ltu",
    "lt",
    "eq",
    "leu",
    "le",
    "add",
    "sub",
    "and",
    "or",
    "xor",
    "sll",
    "srl",
    "sra",
    "signextend_b",
    "signextend_h",
    "signextend_w",
    "pubout",
    "mulu",
    "muluh",
    "mulsuh",
    "mul",
    "mulh",
    "divu",
    "remu",
    "div",
    "rem",
];

fn sp_suffix(use_sp: bool) -> &'static str {
    if use_sp {
        " true"
    } else {
        ""
    }
}

fn a_source(i: &ZiskInst) -> Result<String, String> {
    match i.a_src {
        SRC_C => Ok(".c".into()),
        SRC_STEP => Ok(".step".into()),
        SRC_REG => {
            if i.a_offset_imm0 < 32 {
                Ok(format!("(.reg {})", i.a_offset_imm0))
            } else {
                Err(format!("a_src register {} out of range", i.a_offset_imm0))
            }
        }
        SRC_MEM => Ok(format!("(.mem 0x{:x}{})", i.a_offset_imm0, sp_suffix(i.a_use_sp_imm1 != 0))),
        SRC_IMM => Ok(format!("(.imm 0x{:x})", i.a_offset_imm0 | (i.a_use_sp_imm1 << 32))),
        n => Err(format!("unsupported a_src {n}")),
    }
}

fn b_source(i: &ZiskInst) -> Result<String, String> {
    match i.b_src {
        SRC_C => Ok(".c".into()),
        SRC_REG => {
            if i.b_offset_imm0 < 32 {
                Ok(format!("(.reg {})", i.b_offset_imm0))
            } else {
                Err(format!("b_src register {} out of range", i.b_offset_imm0))
            }
        }
        SRC_MEM => Ok(format!("(.mem 0x{:x}{})", i.b_offset_imm0, sp_suffix(i.b_use_sp_imm1 != 0))),
        SRC_IMM => Ok(format!("(.imm 0x{:x})", i.b_offset_imm0 | (i.b_use_sp_imm1 << 32))),
        SRC_IND => Ok(format!("(.ind 0x{:x}{})", i.b_offset_imm0, sp_suffix(i.b_use_sp_imm1 != 0))),
        n => Err(format!("unsupported b_src {n}")),
    }
}

fn store(i: &ZiskInst) -> Result<String, String> {
    match i.store {
        STORE_NONE => Ok(".none".into()),
        STORE_REG => {
            if (0..32).contains(&i.store_offset) {
                Ok(format!("(.reg {})", i.store_offset))
            } else {
                Err(format!("store register {} out of range", i.store_offset))
            }
        }
        STORE_MEM => {
            Ok(format!("(.mem 0x{:x}{})", i.store_offset as u64, sp_suffix(i.store_use_sp)))
        }
        STORE_IND => {
            Ok(format!("(.ind 0x{:x}{})", i.store_offset as u64, sp_suffix(i.store_use_sp)))
        }
        n => Err(format!("unsupported store {n}")),
    }
}

fn width(i: &ZiskInst) -> Result<&'static str, String> {
    // ind_width can hold stale values on instructions with no indirection
    if i.b_src != SRC_IND && i.store != STORE_IND {
        return Ok(".w8");
    }
    match i.ind_width {
        1 => Ok(".w1"),
        2 => Ok(".w2"),
        4 => Ok(".w4"),
        8 => Ok(".w8"),
        n => Err(format!("unsupported ind_width {n}")),
    }
}

fn emit_inst(paddr: u64, i: &ZiskInst) -> Result<String, String> {
    if !SUPPORTED_OPS.contains(&i.op_str) {
        return Err(format!(
            "op `{}` (0x{:x}) is outside the zisk-sw-fv v1 subset",
            i.op_str, i.op
        ));
    }
    Ok(format!(
        "    (0x{:x}, {{ op := .{}, aSrc := {}, bSrc := {}, store := {}, \
         storePc := {}, setPc := {}, indWidth := {}, \
         jmpOff1 := {}, jmpOff2 := {}, isEnd := {} }})",
        paddr,
        i.op_str,
        a_source(i)?,
        b_source(i)?,
        store(i)?,
        i.store_pc,
        i.set_pc,
        width(i)?,
        i.jmp_offset1,
        i.jmp_offset2,
        i.end,
    ))
}

/// Display name for the generated header: just the file name, so the output
/// is machine-independent (regeneration from another checkout location must
/// not churn the checked-in files).
fn display_src(src: &str) -> &str {
    std::path::Path::new(src).file_name().and_then(|f| f.to_str()).unwrap_or(src)
}

fn raw_inst(i: &ZiskInst) -> String {
    format!(
        "    {{ paddr := 0x{:x}, op := 0x{:x}, opName := {:?}, \
         aSrc := {}, aUseSpImm1 := {}, aOffsetImm0 := 0x{:x}, \
         bSrc := {}, bUseSpImm1 := {}, bOffsetImm0 := 0x{:x}, \
         store := {}, storeUseSp := {}, storeOffset := {}, storePc := {}, \
         setPc := {}, indWidth := {}, jmpOff1 := {}, jmpOff2 := {}, \
         isExternalOp := {}, isPrecompiled := {}, inputSize := {}, m32 := {}, isEnd := {} }}",
        i.paddr,
        i.op,
        i.op_str,
        i.a_src,
        i.a_use_sp_imm1,
        i.a_offset_imm0,
        i.b_src,
        i.b_use_sp_imm1,
        i.b_offset_imm0,
        i.store,
        i.store_use_sp,
        i.store_offset,
        i.store_pc,
        i.set_pc,
        i.ind_width,
        i.jmp_offset1,
        i.jmp_offset2,
        i.is_external_op,
        i.is_precompiled,
        i.input_size,
        i.m32,
        i.end,
    )
}

fn section_contains(sections: &[DataSection64], addr: u64) -> bool {
    sections.iter().any(|section| {
        let end = section.addr.saturating_add(section.data.len() as u64 * 8);
        (section.addr..end).contains(&addr)
    })
}

fn data_entries(sections: &[DataSection64]) -> Vec<String> {
    sections
        .iter()
        .flat_map(|section| {
            section.data.iter().enumerate().map(move |(i, word)| {
                format!("    (0x{:x}, 0x{:x})", section.addr + 8 * i as u64, word)
            })
        })
        .collect()
}

fn symbol_entries(symbols: &[(String, u64)]) -> Vec<String> {
    symbols.iter().map(|(name, addr)| format!("    ({name:?}, 0x{addr:x})")).collect()
}

fn lean_symbol_name(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return format!("librarySymbol_{name}");
    }

    let encoded =
        name.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join("_");
    format!("librarySymbolEncoded_{encoded}")
}

fn symbol_defs(symbols: &[(&String, &u64)]) -> Vec<String> {
    symbols
        .iter()
        .map(|(name, addr)| {
            format!(
                "/-- Address of `{name}` in the production library artifact. -/\n\
                 def {} : Zisk.Addr := 0x{addr:x}",
                lean_symbol_name(name)
            )
        })
        .collect()
}

fn chunked_array(name: &str, element_type: &str, entries: &[String]) -> String {
    const CHUNK_SIZE: usize = 256;

    if entries.is_empty() {
        return format!("def {name} : Array {element_type} := #[]");
    }

    let mut chunks = Vec::new();
    let mut chunk_names = Vec::new();
    for (index, entries) in entries.chunks(CHUNK_SIZE).enumerate() {
        let chunk_name = format!("{name}Chunk{index}");
        chunks.push(format!(
            "private def {chunk_name} : Array {element_type} := #[\n{}\n]",
            entries.join(",\n")
        ));
        chunk_names.push(chunk_name);
    }

    format!(
        "{}\n\ndef {name} : Array {element_type} :=\n  {}",
        chunks.join("\n\n"),
        chunk_names.join(" ++\n  ")
    )
}

fn write_library_artifact(out_dir: &Path) -> Result<(usize, usize, usize), String> {
    let library = assemble_zisk_library()?;
    fs::create_dir_all(out_dir)
        .map_err(|e| format!("creating output directory {}: {e}", out_dir.display()))?;

    let instructions =
        library.insts.values().map(|builder| raw_inst(&builder.i)).collect::<Vec<_>>();

    let mut symbols =
        library.symbols.iter().map(|(name, addr)| (name.clone(), *addr)).collect::<Vec<_>>();
    symbols.sort_by(|(name_a, addr_a), (name_b, addr_b)| {
        addr_a.cmp(addr_b).then_with(|| name_a.cmp(name_b))
    });

    let mut code_symbols = Vec::new();
    let mut ro_symbols = Vec::new();
    let mut rw_symbols = Vec::new();
    for symbol in symbols {
        if library.insts.contains_key(&symbol.1) {
            code_symbols.push(symbol);
        } else if section_contains(&library.ro_data, symbol.1) {
            ro_symbols.push(symbol);
        } else if section_contains(&library.rw_data, symbol.1) {
            rw_symbols.push(symbol);
        } else {
            return Err(format!(
                "symbol `{}` at 0x{:x} is outside the generated code and data maps",
                symbol.0, symbol.1
            ));
        }
    }
    let all_symbols = code_symbols
        .iter()
        .chain(ro_symbols.iter())
        .chain(rw_symbols.iter())
        .map(|(name, addr)| (name, addr))
        .collect::<Vec<_>>();

    let header = "/-\n  GENERATED by zisk2lean from the canonical production ZISK_LIBRARY — DO NOT EDIT.\n-/\n\n";
    let rom = format!(
        "{header}import ZiskCore.Artifact\n\nnamespace Zisklib.Generated\n\n\
         /-- Complete production library code in program-address order. -/\n\
         {}\n\nend Zisklib.Generated\n",
        chunked_array("libraryRom", "Zisk.RawInst", &instructions)
    );
    let symbol_text = format!(
        "{header}import ZiskCore.Mem\n\nnamespace Zisklib.Generated\n\n\
         /-- Labels whose addresses point into the library code map. -/\n\
         def libraryCodeSymbols : List (String × Zisk.Addr) := [\n{}\n]\n\n\
         /-- Symbols backed by production read-only data. -/\n\
         def libraryReadOnlyDataSymbols : List (String × Zisk.Addr) := [\n{}\n]\n\n\
         /-- Symbols backed by production writable scratch/data. -/\n\
         def libraryWritableDataSymbols : List (String × Zisk.Addr) := [\n{}\n]\n\n\
         def librarySymbols : List (String × Zisk.Addr) :=\n\
           libraryCodeSymbols ++ libraryReadOnlyDataSymbols ++ libraryWritableDataSymbols\n\n\
         /-! ### Stable generated names for direct theorem references -/\n\n\
         {}\n\n\
         end Zisklib.Generated\n",
        symbol_entries(&code_symbols).join(",\n"),
        symbol_entries(&ro_symbols).join(",\n"),
        symbol_entries(&rw_symbols).join(",\n"),
        symbol_defs(&all_symbols).join("\n\n"),
    );

    let source_names =
        ZISK_LIBRARY.iter().map(|(name, _)| format!("    {name:?}")).collect::<Vec<_>>();
    let ro_data = data_entries(&library.ro_data);
    let rw_data = data_entries(&library.rw_data);
    let data = format!(
        "{header}import ZiskCore.Mem\n\nnamespace Zisklib.Generated\n\n\
         def libraryRomBase : Zisk.Addr := 0x{ZISKLIB_ROM_ADDR:x}\n\
         def libraryRamBase : Zisk.Addr := 0x{ZISKLIB_RAM_ADDR:x}\n\n\
         /-- Ordered source manifest consumed by both `elf2rom` and this generator. -/\n\
         def librarySourceOrder : List String := [\n{}\n]\n\n\
         /-- Initial read-only memory words from the production library artifact. -/\n\
         {}\n\n\
         /-- Initial writable memory words, including static scratch initialization. -/\n\
         {}\n\n\
         def libraryInitialData : Array (Zisk.Addr × Zisk.Word) :=\n\
           libraryReadOnlyData ++ libraryWritableData\n\n\
         end Zisklib.Generated\n",
        source_names.join(",\n"),
        chunked_array("libraryReadOnlyData", "(Zisk.Addr × Zisk.Word)", &ro_data),
        chunked_array("libraryWritableData", "(Zisk.Addr × Zisk.Word)", &rw_data),
    );

    let outputs: [(PathBuf, String); 3] = [
        (out_dir.join("LibraryRom.lean"), rom),
        (out_dir.join("LibrarySymbols.lean"), symbol_text),
        (out_dir.join("LibraryData.lean"), data),
    ];
    for (path, text) in outputs {
        fs::write(&path, text).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }

    Ok((
        instructions.len(),
        code_symbols.len() + ro_symbols.len() + rw_symbols.len(),
        ro_data.len() + rw_data.len(),
    ))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 && args[1] == "--library" {
        match write_library_artifact(Path::new(&args[2])) {
            Ok((instructions, symbols, words)) => {
                println!(
                    "wrote production library artifact ({instructions} instructions, \
                     {symbols} symbols, {words} initialized words)"
                );
                return;
            }
            Err(e) => {
                eprintln!("Error generating production library: {e}");
                process::exit(1);
            }
        }
    }
    if args.len() < 3 || args.len() > 4 {
        eprintln!(
            "Usage:\n  zisk2lean <zisk_file_or_dir> <out.lean> [lean_def_name]\n  \
             zisk2lean --library <output_directory>"
        );
        process::exit(1);
    }
    let (src, out) = (&args[1], &args[2]);
    let name = match args.get(3) {
        Some(n) => n.clone(),
        None => {
            let stem = std::path::Path::new(src)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .replace('-', "_");
            if stem.is_empty() || stem.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                eprintln!("cannot derive a Lean def name from `{src}`; pass one explicitly");
                process::exit(1);
            }
            stem
        }
    };

    let zisk_files = collect_zisk_files(src).unwrap_or_else(|e| {
        eprintln!("Error collecting .zisk files: {e}");
        process::exit(1);
    });

    // No defines: full fidelity, same ROM as `ziskemu -z` / `zisk2zisk --elf`.
    let (rom, symbols) = assemble_files_with_symbols(&zisk_files, &[]).unwrap_or_else(|e| {
        eprintln!("Error assembling .zisk source: {e}");
        process::exit(1);
    });

    let mut lines = Vec::new();
    for (paddr, zib) in &rom.insts {
        match emit_inst(*paddr, &zib.i) {
            Ok(l) => lines.push(l),
            Err(e) => {
                eprintln!("Error at paddr 0x{paddr:x}: {e}");
                process::exit(1);
            }
        }
    }

    // data sections (ro: after the code; rw: at GENERAL_RAM_ADDR), one
    // 8-byte word per element — these seed the initial memory
    let mut data_lines = Vec::new();
    for section in rom.ro_data_64.iter().chain(rom.rw_data_64.iter()) {
        for (i, w) in section.data.iter().enumerate() {
            data_lines.push(format!("    (0x{:x}, 0x{:x})", section.addr + 8 * i as u64, w));
        }
    }

    // programs without data sections get no `<name>Data` def at all
    let data_block = if data_lines.is_empty() {
        String::new()
    } else {
        format!(
            "\n/-- Initial memory contents from the data sections (ro + rw),\n    \
             one 8-byte word per entry (`Mem.withWords`). -/\n\
             def {name}Data : List (Addr × Word) := [\n{}\n]\n",
            data_lines.join(",\n")
        )
    };

    // symbol addresses (every label and data name), prefixed with the
    // program name: all programs share the `Zisk.Generated` namespace, so a
    // bare `main_addr` would collide across programs
    let mut syms: Vec<(&String, &u64)> = symbols.iter().collect();
    syms.sort_by_key(|&(_, a)| *a);
    let sym_defs = syms
        .iter()
        .map(|(sym, a)| {
            format!("/-- Address of `{sym}`. -/\ndef {name}_{sym}_addr : Addr := 0x{a:x}#64")
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let text = format!(
        "/-\n  GENERATED by zisk2lean from `{}` — DO NOT EDIT.\n\n  \
         The `.zisk` source is the single source of truth; this file is the\n  \
         canonical assembler's output rendered as Lean.\n-/\n\n\
         import ZiskCore.Step\n\nnamespace Zisk.Generated\n\n\
         def {name} : Rom := .fromPairs [\n{}\n]\n{data_block}\n\
         /-! ### Symbol addresses (every label and data name) -/\n\n\
         {sym_defs}\n\nend Zisk.Generated\n",
        display_src(src),
        lines.join(",\n"),
    );

    fs::write(out, &text).unwrap_or_else(|e| {
        eprintln!("Error writing {out}: {e}");
        process::exit(1);
    });
    println!("wrote {out} ({} instructions)", rom.insts.len());
}
