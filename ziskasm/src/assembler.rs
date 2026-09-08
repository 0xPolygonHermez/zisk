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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use zisk_core::zisk_inst_builder::ZiskInstBuilder;
use zisk_core::zisk_rom::{DataSection64, ZiskRom};
use zisk_core::{GENERAL_RAM_ADDR, ROM_ADDR, ROM_ENTRY, SYS_ADDR};
use zisk_riscv::riscv2zisk_context::{add_end_and_lib, add_entry_exit_jmp};

use crate::parser::{
    self, ASource, BSource, Control, DataDecl, Instruction, JumpTarget, Kind, Num, Op, Program,
    Store, Target,
};

/// Bytes between two consecutive program instructions (the `.zisk` convention).
const INST_SIZE: i64 = 4;

/// Offset from the start of the BIOS entry/exit block (`add_entry_exit_jmp`) to
/// its output-finalization code (its `:0014` instruction — 5 instructions in).
/// The BIOS "CALL to entry" leaves this address in `r1`; `ret_to_bios` jumps here
/// so the BIOS reads `OUTPUT_ADDR` and ends. Kept in sync with `add_entry_exit_jmp`
/// in `transpilers/riscv/src/riscv2zisk_context.rs`.
const BIOS_FINALIZE_OFFSET: u64 = 0x14;

/// RISC-V JALR target mask: clears bit 0 (2-byte instruction alignment). Used by
/// `ret` and by any `setpc`-based indirect jump through a register.
const JALR_MASK: u64 = 0xffff_ffff_ffff_fffe;

/// Reads and assembles the given `.zisk` source files (in order) into a `ZiskRom`.
/// The first file must contain the `_start` entry label (typically `ziskos.zisk`),
/// unless the program instead defines `main` / `_zisk_main` (auto-launcher).
pub fn assemble_files<P: AsRef<Path>>(paths: &[P]) -> Result<ZiskRom, String> {
    assemble_files_with_defines(paths, &[])
}

/// Like [`assemble_files`], but with a set of externally predefined symbols that
/// the source can test with `ifdef`/`ifndef`. For example, passing `["ASM"]`
/// selects the x86-assembly target so a program can exclude ops that generator
/// cannot emit.
pub fn assemble_files_with_defines<P: AsRef<Path>>(
    paths: &[P],
    defines: &[&str],
) -> Result<ZiskRom, String> {
    let predefined: HashSet<String> = defines.iter().map(|s| s.to_string()).collect();
    let srcs = read_sources(paths)?;
    let seed = merge_public_defines(srcs.iter().map(|(_, s)| s.as_str()))?;
    let mut program = Program::default();
    for (name, src) in &srcs {
        let parsed = parser::parse_program_seeded(src, name, &predefined, &seed)?;
        program.instructions.extend(parsed.instructions);
        program.data.extend(parsed.data);
    }
    assemble(&program)
}

/// Like [`assemble_files_with_defines`], but also returns the symbol table
/// (see [`assemble_with_symbols`]).
pub fn assemble_files_with_symbols<P: AsRef<Path>>(
    paths: &[P],
    defines: &[&str],
) -> Result<(ZiskRom, HashMap<String, u64>), String> {
    let predefined: HashSet<String> = defines.iter().map(|s| s.to_string()).collect();
    let srcs = read_sources(paths)?;
    let seed = merge_public_defines(srcs.iter().map(|(_, s)| s.as_str()))?;
    let mut program = Program::default();
    for (name, src) in &srcs {
        let parsed = parser::parse_program_seeded(src, name, &predefined, &seed)?;
        program.instructions.extend(parsed.instructions);
        program.data.extend(parsed.data);
    }
    assemble_with_symbols(&program)
}

/// Reads several `.zisk` files into `(name, source)` pairs (used so a multi-file
/// assembly can gather `pub define`s before parsing any file).
fn read_sources<P: AsRef<Path>>(paths: &[P]) -> Result<Vec<(String, String)>, String> {
    paths
        .iter()
        .map(|path| {
            let path = path.as_ref();
            let name = path.to_string_lossy().to_string();
            let src =
                std::fs::read_to_string(path).map_err(|e| format!("cannot read `{name}`: {e}"))?;
            Ok((name, src))
        })
        .collect()
}

/// Gathers every `pub define` across the given sources into one value map that
/// seeds each file's parse (so a public define is visible assembly-wide). Errors
/// if a name is publicly defined twice with different values.
fn merge_public_defines<'a>(
    sources: impl Iterator<Item = &'a str>,
) -> Result<HashMap<String, String>, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for src in sources {
        for (name, value) in parser::collect_public_defines(src) {
            if let Some(prev) = map.get(&name) {
                if *prev != value {
                    return Err(format!(
                        "conflicting `pub define {name}` values: `{prev}` and `{value}`"
                    ));
                }
            } else {
                map.insert(name, value);
            }
        }
    }
    Ok(map)
}

/// A hand-written `.zisk` library assembled for merging into a program ROM. Unlike
/// [`assemble`], there is no launcher / `_start` / BIOS: it is a set of callable
/// functions placed at a fixed base, plus the exported symbol table (label / data
/// name → address) used to resolve calls into it (see the RISC-V symbol-redirect in
/// `transpilers/common/src/elf2rom.rs`).
pub struct ZiskLibrary {
    /// Assembled instructions keyed by ROM address (`rom_base + 4*i`, file order).
    pub insts: BTreeMap<u64, ZiskInstBuilder>,
    /// Read-only (`const`) data sections, placed right after the code.
    pub ro_data: Vec<DataSection64>,
    /// Read-write (non-`const`) data sections, placed at `ram_base`.
    pub rw_data: Vec<DataSection64>,
    /// Exported symbols: every label and data name → its address.
    pub symbols: HashMap<String, u64>,
}

/// Reads and assembles `.zisk` source files as a [`ZiskLibrary`] (library mode:
/// no launcher, code at `rom_base`, non-`const` data at `ram_base`).
pub fn assemble_library_files<P: AsRef<Path>>(
    paths: &[P],
    rom_base: u64,
    ram_base: u64,
) -> Result<ZiskLibrary, String> {
    let srcs = read_sources(paths)?;
    let seed = merge_public_defines(srcs.iter().map(|(_, s)| s.as_str()))?;
    let mut program = Program::default();
    for (name, src) in &srcs {
        let parsed = parser::parse_program_seeded(src, name, &HashSet::new(), &seed)?;
        program.instructions.extend(parsed.instructions);
        program.data.extend(parsed.data);
    }
    assemble_library(&program, rom_base, ram_base)
}

/// Assembles several in-memory `.zisk` sources as one [`ZiskLibrary`] (library
/// mode). Each entry is `(name, source)`; the sources are concatenated in order
/// (functions placed in that order). Use this to build the library from files
/// embedded at compile time (`include_str!`), one per precompile family, without
/// touching the filesystem. Symbol names (labels + data) must be unique across
/// all sources.
pub fn assemble_library_sources(
    sources: &[(&str, &str)],
    rom_base: u64,
    ram_base: u64,
) -> Result<ZiskLibrary, String> {
    let seed = merge_public_defines(sources.iter().map(|(_, s)| *s))?;
    let mut program = Program::default();
    for (name, src) in sources {
        let parsed = parser::parse_program_seeded(src, name, &HashSet::new(), &seed)?;
        program.instructions.extend(parsed.instructions);
        program.data.extend(parsed.data);
    }
    assemble_library(&program, rom_base, ram_base)
}

/// Assembles an already-parsed program as a [`ZiskLibrary`] at the given bases.
/// Functions are placed in file order at `rom_base + 4*i`; `const` data follows the
/// code (32-byte aligned), non-`const` data goes to `ram_base`. No BIOS / launcher /
/// entry point is added, and the pc→instruction lookup is left for the host ROM to
/// rebuild after the merge.
pub fn assemble_library(
    program: &Program,
    rom_base: u64,
    ram_base: u64,
) -> Result<ZiskLibrary, String> {
    let instructions = &program.instructions;
    let addr_of = |i: usize| rom_base + INST_SIZE as u64 * i as u64;

    // `const` data right after the code (32-byte aligned, as the ROM-init trace
    // requires); non-`const` data at `ram_base`.
    let rom_data_base = addr_of(instructions.len()).next_multiple_of(32);
    let (ro_section, rw_section, data_syms) = layout_data(&program.data, rom_data_base, ram_base);

    // Symbol table: every label (function/local) and data name → address.
    let mut sym_ref: HashMap<&str, u64> = HashMap::new();
    for (i, inst) in instructions.iter().enumerate() {
        if let Some(label) = &inst.label {
            if sym_ref.insert(label.as_str(), addr_of(i)).is_some() {
                return Err(format!("duplicate symbol `{label}`"));
            }
        }
    }
    for &(name, addr) in &data_syms {
        if sym_ref.insert(name, addr).is_some() {
            return Err(format!("duplicate symbol `{name}`"));
        }
    }

    // Encode into a throwaway ROM (no BIOS / launcher / optimize); `bios_finalize`
    // is unused because library code returns via `ret`, never `ret_to_bios`.
    let mut rom = ZiskRom::default();
    rom.ro_data_64.extend(ro_section);
    rom.rw_data_64.extend(rw_section);
    for (i, inst) in instructions.iter().enumerate() {
        encode(&mut rom, addr_of(i), inst, &sym_ref, 0)?;
    }

    let symbols = sym_ref.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Ok(ZiskLibrary { insts: rom.insts, ro_data: rom.ro_data_64, rw_data: rom.rw_data_64, symbols })
}

/// Assembles an already-parsed program (instructions + data) into a `ZiskRom`.
pub fn assemble(program: &Program) -> Result<ZiskRom, String> {
    Ok(assemble_with_symbols(program)?.0)
}

/// Like [`assemble`], but also returns the symbol table (every label and data
/// name → its address) — the same table used internally to resolve jumps and
/// symbolic operands.
pub fn assemble_with_symbols(program: &Program) -> Result<(ZiskRom, HashMap<String, u64>), String> {
    if program.instructions.is_empty() {
        return Err("empty program: no instructions".into());
    }

    // If the program has no explicit `_start`, synthesize a launcher around its
    // entry label (`main` or `_zisk_main`): set gp/sp, `call` the entry, and
    // `ret_to_bios`. This lets a program be just its own code plus a `main:` label,
    // with no hand-written boot file.
    let with_launcher: Vec<Instruction>;
    let instructions: &[Instruction] = if has_label(&program.instructions, "_start") {
        &program.instructions
    } else {
        let entry = program_entry(&program.instructions)?;
        let mut v = synth_launcher(entry)?;
        v.extend_from_slice(&program.instructions);
        with_launcher = v;
        &with_launcher
    };

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

    let addr_of = |i: usize| (ROM_ADDR as i64 + INST_SIZE * i as i64) as u64;

    // Lay out data: `const` goes in ROM right after the code, non-`const` in RAM
    // at GENERAL_RAM_ADDR. This yields the initialized sections and each data
    // symbol's address.
    // 32-byte align the ROM data base: the ROM-init trace commits data in 4-u64
    // (32-byte) rows anchored at the section address (state-machines/rom), matching
    // the RISC-V transpiler's aligned section starts, so proving works.
    let rom_data_base = addr_of(ordered.len()).next_multiple_of(32);
    let (ro_section, rw_section, data_syms) =
        layout_data(&program.data, rom_data_base, GENERAL_RAM_ADDR);

    // Symbol table: labels (code addresses) + data names. Used to resolve jump
    // targets and symbolic operands. Names must be unique across both.
    let mut symbols: HashMap<&str, u64> = HashMap::new();
    for (i, inst) in ordered.iter().enumerate() {
        if let Some(label) = &inst.label {
            if symbols.insert(label.as_str(), addr_of(i)).is_some() {
                return Err(format!("duplicate symbol `{label}`"));
            }
        }
    }
    for &(name, addr) in &data_syms {
        if symbols.insert(name, addr).is_some() {
            return Err(format!("duplicate symbol `{name}`"));
        }
    }

    // `_start` is now instruction 0, i.e. ROM_ADDR.
    let entry =
        *symbols.get("_start").ok_or("missing `_start` label: the program has no entry point")?;

    // Build the ROM the same way elf2rom does.
    let mut rom = ZiskRom { next_init_inst_addr: ROM_ENTRY, ..Default::default() };
    add_end_and_lib(&mut rom);

    // After `add_end_and_lib`, `next_init_inst_addr` points at the start of the
    // BIOS entry/exit block that `add_entry_exit_jmp` (called below) will emit, so
    // we can derive the BIOS finalization address that `ret_to_bios` jumps to —
    // no hard-coded constant.
    let bios_finalize = rom.next_init_inst_addr + BIOS_FINALIZE_OFFSET;

    // Initialized data sections (read by the emulator at startup).
    rom.ro_data_64.extend(ro_section);
    rom.rw_data_64.extend(rw_section);

    // Pass 2: encode each instruction at its address, resolving symbols.
    for (i, inst) in ordered.iter().enumerate() {
        encode(&mut rom, addr_of(i), inst, &symbols, bios_finalize)?;
    }

    // BIOS entry/exit: jumps to `entry`, leaving the return address (the output
    // finalization) in r1, so the program returns to the BIOS with `ret`.
    add_entry_exit_jmp(&mut rom, entry);

    rom.optimize_instruction_lookup().map_err(|e| e.to_string())?;
    let symbols = symbols.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Ok((rom, symbols))
}

/// Lays out the `const` (ROM, at `rom_data_base`) and non-`const` (RAM, at
/// `GENERAL_RAM_ADDR`) data declarations, packing each element into one 8-byte
/// slot in declaration order. Returns the two initialized sections (if non-empty)
/// and each symbol's address.
fn layout_data(
    data: &[DataDecl],
    rom_data_base: u64,
    ram_data_base: u64,
) -> (Option<DataSection64>, Option<DataSection64>, Vec<(&str, u64)>) {
    let mut ro: Vec<u64> = Vec::new();
    let mut rw: Vec<u64> = Vec::new();
    let mut syms: Vec<(&str, u64)> = Vec::new();
    for d in data {
        let base = if d.is_const { rom_data_base } else { ram_data_base };
        let buf = if d.is_const { &mut ro } else { &mut rw };
        syms.push((d.name.as_str(), base + buf.len() as u64 * 8));
        for k in 0..d.count {
            buf.push(d.values.get(k).copied().unwrap_or(0));
        }
    }
    // Pad each section to a multiple of 4 u64s (32 bytes). The ROM-init trace packs
    // data into 4-u64 rows (state-machines/rom/src/custom_rom.rs), so a section's
    // length must be a multiple of 4 for the ROM to be provable — mirroring the
    // RISC-V transpiler's `RO_SECTION_ALIGN`. Padding appends zeros after all
    // symbols, so it does not shift any data address. (Harmless for emulation.)
    ro.resize(ro.len().next_multiple_of(4), 0);
    rw.resize(rw.len().next_multiple_of(4), 0);

    let ro_section = (!ro.is_empty()).then_some(DataSection64 { addr: rom_data_base, data: ro });
    let rw_section = (!rw.is_empty()).then_some(DataSection64 { addr: ram_data_base, data: rw });
    (ro_section, rw_section, syms)
}

/// Whether any instruction carries the given label.
fn has_label(instructions: &[Instruction], label: &str) -> bool {
    instructions.iter().any(|i| i.label.as_deref() == Some(label))
}

/// Finds the program entry label when there is no explicit `_start`.
fn program_entry(instructions: &[Instruction]) -> Result<&'static str, String> {
    ["main", "_zisk_main"]
        .into_iter()
        .find(|name| has_label(instructions, name))
        .ok_or_else(|| "program has no `_start`, `main` or `_zisk_main` entry label".to_string())
}

/// Builds the launcher (`_start`) a program without an explicit `_start` gets
/// automatically: set gp (r3) and sp (r2), `call` the entry, then `ret_to_bios`.
/// Mirrors `ziskos::_start`.
fn synth_launcher(entry: &str) -> Result<Vec<Instruction>, String> {
    let src = format!(
        "_start:\n\
         \tcopyb(0, 0) -> r3\n\
         \tcopyb(0, 0x{sys:x}) -> r2\n\
         \tcall {entry}\n\
         \tret_to_bios\n",
        sys = SYS_ADDR,
    );
    Ok(parser::parse_program(&src, "<launcher>")?.instructions)
}

/// Emits an unconditional static jump to an absolute address: `copyb(0, addr)`
/// puts the constant target in `c`, and `setpc(0)` sets the next pc to `c`.
/// Because `c` is a constant, the x86 generator compiles this to a direct `jmp`,
/// which works for any address (unlike a register-based dynamic jump).
fn emit_static_jump(zib: &mut ZiskInstBuilder, addr: u64) {
    zib.src_a("imm", 0, false);
    zib.src_b("imm", addr, false);
    zib.op("copyb").unwrap();
    zib.set_pc();
    zib.j(0, INST_SIZE);
}

/// Resolves a jump/call target to a pc-relative offset from the instruction at `pc`.
fn resolve(target: &Target, pc: u64, symbols: &HashMap<&str, u64>) -> Result<i64, String> {
    match target {
        Target::Offset(o) => Ok(*o),
        Target::Label(l) => {
            let dst = *symbols.get(l.as_str()).ok_or_else(|| format!("undefined label `{l}`"))?;
            Ok(dst as i64 - pc as i64)
        }
    }
}

/// Resolves a number operand to its `u64` value: a literal as-is, a symbol to its
/// address.
fn resolve_num(n: &Num, symbols: &HashMap<&str, u64>) -> Result<u64, String> {
    match n {
        Num::Lit(v) => Ok(*v),
        Num::Sym(name) => {
            symbols.get(name.as_str()).copied().ok_or_else(|| format!("undefined symbol `{name}`"))
        }
    }
}

fn encode(
    rom: &mut ZiskRom,
    pc: u64,
    inst: &Instruction,
    symbols: &HashMap<&str, u64>,
    bios_finalize: u64,
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
        Kind::Jump(target) => {
            // jump(target) : unconditional static jump to an absolute address.
            let addr = match target {
                JumpTarget::Addr(a) => *a,
                JumpTarget::Label(l) => *symbols
                    .get(l.as_str())
                    .ok_or_else(|| format!("{}: undefined label `{l}`", loc()))?,
            };
            emit_static_jump(&mut zib, addr);
        }
        Kind::RetToBios => {
            // ret_to_bios : static jump to the BIOS output-finalization address.
            // A dynamic `ret` cannot be used: the x86 asm generator's dynamic-jump
            // path assumes high (>= ROM_ADDR) targets, and this is a low address.
            emit_static_jump(&mut zib, bios_finalize);
        }
        Kind::Call(target) => {
            // call LABEL == jal r1, LABEL : flag=1 forces the jump to jmp_offset1,
            // and store_pc writes the return address (pc + jmp_offset2) into r1.
            let off = resolve(target, pc, symbols).map_err(|e| format!("{}: {e}", loc()))?;
            zib.src_a("imm", 0, false);
            zib.src_b("imm", 0, false);
            zib.op("flag").unwrap();
            zib.store_pc("reg", 1, false);
            zib.j(off, INST_SIZE);
        }
        Kind::Op(op) => {
            encode_op(&mut zib, pc, op, symbols).map_err(|e| format!("{}: {e}", loc()))?
        }
    }

    zib.verbose(&inst.verbose);
    zib.build(rom);
    Ok(())
}

fn encode_op(
    zib: &mut ZiskInstBuilder,
    pc: u64,
    op: &Op,
    symbols: &HashMap<&str, u64>,
) -> Result<(), String> {
    encode_a(zib, &op.a, symbols)?;
    encode_b(zib, &op.b, symbols)?;
    zib.op(&op.op).map_err(|_| format!("unknown operation `{}`", op.op))?;
    if let Some(store) = &op.store {
        encode_store(zib, store, symbols)?;
    }

    match &op.control {
        // Fall-through: pc advances by jmp_offset2 (flag is false). For a regular op
        // jmp_offset1 is also the instruction size, but precompiles must have
        // jmp_offset1 == 0 for proof generation: they never raise the register flag,
        // so jmp_offset1 carries no control flow, and the constraint system requires
        // it to be 0. (DMA precompiles that pass a third parameter in jmp_offset1 are
        // written with an explicit `j(...)`, i.e. the `Control::Jump` arm below.)
        Control::Fallthrough => {
            let jmp1 = if zib.i.is_precompiled { 0 } else { INST_SIZE };
            zib.j(jmp1, INST_SIZE);
        }
        Control::Jump(j1, j2) => {
            let o1 = resolve(j1, pc, symbols)?;
            let o2 = match j2 {
                Some(t) => resolve(t, pc, symbols)?,
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

fn encode_a(
    zib: &mut ZiskInstBuilder,
    a: &ASource,
    symbols: &HashMap<&str, u64>,
) -> Result<(), String> {
    match a {
        ASource::C => zib.src_a("lastc", 0, false),
        ASource::Reg(n) => zib.src_a("reg", *n, false),
        ASource::Mem(n) => zib.src_a("mem", resolve_num(n, symbols)?, false),
        ASource::Imm(n) => zib.src_a("imm", resolve_num(n, symbols)?, false),
        ASource::Step => zib.src_a("step", 0, false),
    }
    Ok(())
}

fn encode_b(
    zib: &mut ZiskInstBuilder,
    b: &BSource,
    symbols: &HashMap<&str, u64>,
) -> Result<(), String> {
    match b {
        BSource::C => zib.src_b("lastc", 0, false),
        BSource::Reg(n) => zib.src_b("reg", *n, false),
        BSource::Mem(n) => zib.src_b("mem", resolve_num(n, symbols)?, false),
        BSource::Imm(n) => zib.src_b("imm", resolve_num(n, symbols)?, false),
        BSource::Ind { width, offset } => {
            zib.ind_width(*width);
            zib.src_b("ind", *offset as u64, false);
        }
    }
    Ok(())
}

fn encode_store(
    zib: &mut ZiskInstBuilder,
    store: &Store,
    symbols: &HashMap<&str, u64>,
) -> Result<(), String> {
    match store {
        Store::Reg(n) => zib.store("reg", *n as i64, false, false),
        Store::Mem(n) => zib.store("mem", resolve_num(n, symbols)? as i64, false, false),
        Store::Ind { width, offset } => {
            zib.ind_width(*width);
            zib.store("ind", *offset, false, false);
        }
    }
    Ok(())
}
