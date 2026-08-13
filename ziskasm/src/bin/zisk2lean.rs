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

use std::{env, fs, process};
use zisk_core::{
    ZiskInst, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG, SRC_STEP, STORE_IND, STORE_MEM,
    STORE_NONE, STORE_REG,
};
use ziskasm::{assemble_files_with_defines, collect_zisk_files};

/// Ops modeled by zisk-sw-fv (`Zisk/Op.lean`); constructor names are the
/// zisk op-name strings, so `op_str` maps directly to `.{op_str}`.
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args.len() > 4 {
        eprintln!("Usage: zisk2lean <zisk_file_or_dir> <out.lean> [lean_def_name]");
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
    let rom = assemble_files_with_defines(&zisk_files, &[]).unwrap_or_else(|e| {
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
            data_lines.push(format!(
                "    (0x{:x}, 0x{:x})",
                section.addr + 8 * i as u64,
                w
            ));
        }
    }

    let text = format!(
        "/-\n  GENERATED by zisk2lean from `{src}` — DO NOT EDIT.\n\n  \
         The `.zisk` source is the single source of truth; this file is the\n  \
         canonical assembler's output rendered as Lean.\n-/\n\n\
         import ZiskCore.Step\n\nnamespace Zisk.Generated\n\n\
         def {name} : Rom := .fromPairs [\n{}\n]\n\n\
         /-- Initial memory contents from the data sections (ro + rw),\n    \
         one 8-byte word per entry (`Mem.withWords`). -/\n\
         def {name}Data : List (Addr × Word) := [\n{}\n]\n\nend Zisk.Generated\n",
        lines.join(",\n"),
        data_lines.join(",\n")
    );

    fs::write(out, &text).unwrap_or_else(|e| {
        eprintln!("Error writing {out}: {e}");
        process::exit(1);
    });
    println!("wrote {out} ({} instructions)", rom.insts.len());
}
