//! Assembles parsed `.zisk` instructions into a `ZiskRom`.
//!
//! This mirrors `transpilers/common/src/elf2rom.rs`, but the program instructions
//! come from the `.zisk` parser instead of the RISC-V transpiler:
//!   1. start an empty ROM and add the BIOS end/lib block (`add_end_and_lib`),
//!   2. place each assembled instruction at `ROM_ADDR + 4*index`,
//!   3. wire the BIOS entry/exit around the `_start` label (`add_entry_exit_jmp`),
//!   4. build the pc→instruction lookup (`optimize_instruction_lookup`).
//!
//! Input memory is populated by the emulator; the program reads it and writes its
//! output to `OUTPUT_ADDR`, which the BIOS finalization reads back on return.

use std::collections::HashMap;
use std::path::Path;

use riscv::riscv2zisk_context::{add_end_and_lib, add_entry_exit_jmp};
use zisk_core::zisk_inst_builder::ZiskInstBuilder;
use zisk_core::zisk_rom::ZiskRom;
use zisk_core::{ROM_ADDR, ROM_ENTRY};

use crate::parser::{self, ASource, BSource, Control, Instruction, Kind, Op, Store, Target};

/// Bytes between two consecutive program instructions (the `.zisk` convention).
const INST_SIZE: i64 = 4;

/// RISC-V JALR target mask: clears bit 0 (2-byte instruction alignment). Used by
/// `ret` and by any `setpc`-based indirect jump through a register.
const JALR_MASK: u64 = 0xffff_ffff_ffff_fffe;

/// Reads and assembles the given `.zisk` source files (in order) into a `ZiskRom`.
/// The first file must contain the `_start` entry label (typically `ziskos.zisk`).
pub fn assemble_files<P: AsRef<Path>>(paths: &[P]) -> Result<ZiskRom, String> {
    let mut instructions = Vec::new();
    for path in paths {
        let path = path.as_ref();
        let name = path.to_string_lossy().to_string();
        let src = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read `{name}`: {e}"))?;
        instructions.extend(parser::parse_program(&src, &name)?);
    }
    assemble(&instructions)
}

/// Assembles an already-parsed program into a `ZiskRom`.
pub fn assemble(instructions: &[Instruction]) -> Result<ZiskRom, String> {
    if instructions.is_empty() {
        return Err("empty program: no instructions".into());
    }

    // The entry point (`_start`) must be the program's first instruction, so it
    // lands at ROM_ADDR. This matches the ELF convention (the entry point is the
    // program base) and the fast emulator's expectations. Source files may be
    // supplied in any order — a `-z <dir>` run collects them sorted by name — so
    // move the file that defines `_start` to the front, keeping every file's own
    // instruction order (like a linker placing the entry section first).
    let start_file = instructions
        .iter()
        .find(|i| i.label.as_deref() == Some("_start"))
        .map(|i| i.file.clone())
        .ok_or("missing `_start` label: the program has no entry point")?;

    let mut ordered: Vec<&Instruction> = Vec::with_capacity(instructions.len());
    ordered.extend(instructions.iter().filter(|i| i.file == start_file));
    ordered.extend(instructions.iter().filter(|i| i.file != start_file));

    if ordered[0].label.as_deref() != Some("_start") {
        return Err(format!(
            "the file `{start_file}` that defines `_start` must begin with it, \
             so the entry point is placed at ROM_ADDR"
        ));
    }

    // Pass 1: assign a ROM address to every instruction and collect labels.
    let addr_of = |i: usize| (ROM_ADDR as i64 + INST_SIZE * i as i64) as u64;
    let mut labels: HashMap<&str, u64> = HashMap::new();
    for (i, inst) in ordered.iter().enumerate() {
        if let Some(label) = &inst.label {
            if labels.insert(label.as_str(), addr_of(i)).is_some() {
                return Err(format!("duplicate label `{label}`"));
            }
        }
    }

    // `_start` is now instruction 0, i.e. ROM_ADDR.
    let entry = *labels
        .get("_start")
        .ok_or("missing `_start` label: the program has no entry point")?;

    // Build the ROM the same way elf2rom does.
    let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };
    add_end_and_lib(&mut rom);

    // Pass 2: encode each instruction at its address, resolving label targets.
    for (i, inst) in ordered.iter().enumerate() {
        encode(&mut rom, addr_of(i), inst, &labels)?;
    }

    // BIOS entry/exit: jumps to `entry`, leaving the return address (the output
    // finalization) in r1, so the program returns to the BIOS with `ret`.
    add_entry_exit_jmp(&mut rom, entry);

    rom.optimize_instruction_lookup().map_err(|e| e.to_string())?;
    Ok(rom)
}

/// Resolves a jump/call target to a pc-relative offset from the instruction at `pc`.
fn resolve(target: &Target, pc: u64, labels: &HashMap<&str, u64>) -> Result<i64, String> {
    match target {
        Target::Offset(o) => Ok(*o),
        Target::Label(l) => {
            let dst = *labels.get(l.as_str()).ok_or_else(|| format!("undefined label `{l}`"))?;
            Ok(dst as i64 - pc as i64)
        }
    }
}

fn encode(
    rom: &mut ZiskRom,
    pc: u64,
    inst: &Instruction,
    labels: &HashMap<&str, u64>,
) -> Result<(), String> {
    let mut zib = ZiskInstBuilder::new(pc);
    let loc = || format!("{}:{}", inst.file, inst.line);

    match &inst.kind {
        Kind::Ret => {
            // ret == jalr r0, r1, 0 : next pc = (r1 & ~1) via setpc; no store.
            zib.src_a("imm", JALR_MASK, false);
            zib.src_b("reg", 1, false);
            zib.op("and").unwrap();
            zib.set_pc();
            zib.j(0, INST_SIZE);
        }
        Kind::Call(target) => {
            // call LABEL == jal r1, LABEL : flag=1 forces the jump to jmp_offset1,
            // and store_pc writes the return address (pc + jmp_offset2) into r1.
            let off = resolve(target, pc, labels).map_err(|e| format!("{}: {e}", loc()))?;
            zib.src_a("imm", 0, false);
            zib.src_b("imm", 0, false);
            zib.op("flag").unwrap();
            zib.store_pc("reg", 1, false);
            zib.j(off, INST_SIZE);
        }
        Kind::Op(op) => encode_op(&mut zib, pc, op, labels).map_err(|e| format!("{}: {e}", loc()))?,
    }

    zib.verbose(&inst.verbose);
    zib.build(rom);
    Ok(())
}

fn encode_op(
    zib: &mut ZiskInstBuilder,
    pc: u64,
    op: &Op,
    labels: &HashMap<&str, u64>,
) -> Result<(), String> {
    encode_a(zib, &op.a)?;
    encode_b(zib, &op.b)?;
    zib.op(&op.op).map_err(|_| format!("unknown operation `{}`", op.op))?;
    if let Some(store) = &op.store {
        encode_store(zib, store);
    }

    match &op.control {
        Control::Fallthrough => zib.j(INST_SIZE, INST_SIZE),
        Control::Jump(j1, j2) => {
            let o1 = resolve(j1, pc, labels)?;
            let o2 = match j2 {
                Some(t) => resolve(t, pc, labels)?,
                None => INST_SIZE, // omitted jump2 == the next instruction
            };
            zib.j(o1, o2);
        }
        Control::SetPc(off) => {
            zib.set_pc();
            zib.j(*off, INST_SIZE);
        }
    }

    if op.end {
        zib.end();
    }
    Ok(())
}

fn encode_a(zib: &mut ZiskInstBuilder, a: &ASource) -> Result<(), String> {
    match a {
        ASource::C => zib.src_a("lastc", 0, false),
        ASource::Reg(n) => zib.src_a("reg", *n, false),
        ASource::Mem(n) => zib.src_a("mem", *n, false),
        ASource::Imm(n) => zib.src_a("imm", *n, false),
        ASource::Step => return Err("`step` source is not supported yet".into()),
    }
    Ok(())
}

fn encode_b(zib: &mut ZiskInstBuilder, b: &BSource) -> Result<(), String> {
    match b {
        BSource::C => zib.src_b("lastc", 0, false),
        BSource::Reg(n) => zib.src_b("reg", *n, false),
        BSource::Mem(n) => zib.src_b("mem", *n, false),
        BSource::Imm(n) => zib.src_b("imm", *n, false),
        BSource::Ind { width, offset } => {
            zib.ind_width(*width);
            zib.src_b("ind", *offset as u64, false);
        }
    }
    Ok(())
}

fn encode_store(zib: &mut ZiskInstBuilder, store: &Store) {
    match store {
        Store::Reg(n) => zib.store("reg", *n as i64, false, false),
        Store::Mem(n) => zib.store("mem", *n as i64, false, false),
        Store::Ind { width, offset } => {
            zib.ind_width(*width);
            zib.store("ind", *offset, false, false);
        }
    }
}
