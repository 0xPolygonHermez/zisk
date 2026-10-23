//! Prepare a step-by-step differential test of the Sail model against ziskemu.
//!
//! From one ZisK program — ZisK assembly, or a RISC-V ELF transpiled as
//! ziskemu transpiles it — `prepare` writes everything the Sail side needs and
//! the trace it must reproduce:
//!
//! - `rom.sail`, the program as a `rom_fetch` function, to read;
//! - `mem/*.bin` and `mem.args`, the initial memory as raw sections plus the
//!   `-b addr,file` arguments that load them into the Sail C emulator, and
//!   the ROM as the table the emulator reads it from
//!   (sail-zisk/model/zisk_rom_table.sail);
//! - `ziskemu.trace`, the run on ziskemu, in the format `model/zisk_main.sail`
//!   prints;
//! - `mem/fcall-tape.bin`, ziskemu's answers to the program's fcalls, the
//!   oracle the model reads them from (sail-zisk/model/zisk_insts_fcall.sail).
//!
//! The ROM is built and the memory laid out exactly as `ziskemu -z` does, by
//! the same code: `ziskasm::assemble_files` and `Emu::create_emu_context`.
//!
//! `rom` writes, for checking the transpiler's spec in Lean against it
//! (sail-zisk/lean/TranspilerCheck), a RISC-V ELF's code sections, as the
//! transpiler reads them, and its ROM, as the table entries `prepare` writes.
//!
//! `decode-diff` compares the transpiler's decoder with sail-riscv's, in
//! ZisK's profile, on the words sail-zisk/lean/DecodeDiff lists: for each,
//! what `convert` does with it (an instruction, a halt, or a panic, which
//! rejects the whole program) against what sail-riscv's decoder makes of it.
//!
//! And `ops` writes test vectors for every operation that is a pure function
//! of `a` and `b`: `<mnemonic> <a> <b> <c> <flag>`, with `(c, flag)` what
//! `ZiskOp::call_ab` returns, the emulator's own op functions.

use std::{
    collections::BTreeSet,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use zisk_common::EmuTrace;
use zisk_core::{
    zisk_ops::ZiskOp, ZiskInst, ZiskRom, INPUT_ADDR, SRC_C, SRC_IMM, SRC_IND, SRC_MEM, SRC_REG,
    SRC_STEP, STORE_IND, STORE_MEM, STORE_NONE, STORE_REG,
};
use ziskemu::{Emu, EmuOptions};

#[derive(Parser)]
#[command(about = "Export a ZisK program to the Sail model, and trace it on ziskemu")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write rom.sail, the initial memory and the reference trace into a directory.
    Prepare {
        /// A `.zisk` file or a directory of them, as `ziskemu -z` takes; repeat
        /// it to assemble several into one program.
        #[arg(short = 'z', long, conflicts_with = "elf", required_unless_present = "elf")]
        zisk: Vec<PathBuf>,
        /// A RISC-V ELF, transpiled as `ziskemu -e` transpiles it.
        #[arg(short = 'e', long)]
        elf: Option<PathBuf>,
        /// The program input, as `ziskemu -i` takes.
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// Output directory; created if missing.
        #[arg(short, long)]
        out: PathBuf,
        /// Stop the reference run after this many steps.
        #[arg(long, default_value_t = 10_000_000)]
        max_steps: u64,
        /// Also trace the 32-bit words of memory in [begin, end) after the run,
        /// as `<begin>,<end>` (hex): a test's signature, say.
        #[arg(long, value_parser = parse_range)]
        dump: Option<(u64, u64)>,
    },
    /// Write a RISC-V ELF's code and its transpilation, as text: `code <addr>
    /// <bytes>` for each executable segment, then `inst <pc> <w0> .. <w5>` for
    /// each instruction, its ROM table entry's first six words (all hex), or
    /// `skip <pc>` for one the model does not describe.
    Rom {
        /// The RISC-V ELF.
        #[arg(short = 'e', long)]
        elf: PathBuf,
        /// Output file.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Compare the transpiler's decoder with sail-riscv's, from what
    /// `decode_diff` (sail-zisk/lean/DecodeDiff) wrote, and print where they
    /// differ.
    DecodeDiff {
        /// decode_diff's verdicts.
        #[arg(long)]
        sail: PathBuf,
        /// decode_diff's instruction names.
        #[arg(long)]
        names: PathBuf,
    },
    /// Write test vectors for every pure operation: `<op> <a> <b> <c> <flag>`.
    Ops {
        /// Output file.
        #[arg(short, long)]
        out: PathBuf,
        /// Random vectors per operation, besides the edge cases.
        #[arg(short, long, default_value_t = 64)]
        random: usize,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Prepare { zisk, elf, input, out, max_steps, dump } => {
            let rom = match elf {
                None => {
                    let mut files = Vec::new();
                    for z in &zisk {
                        files.extend(
                            ziskasm::collect_zisk_files(&z.to_string_lossy())
                                .map_err(anyhow::Error::msg)?,
                        );
                    }
                    ziskasm::assemble_files(&files).map_err(anyhow::Error::msg)?
                }
                Some(elf) => {
                    let bytes =
                        fs::read(&elf).with_context(|| format!("reading {}", elf.display()))?;
                    zisk_transpiler_common::program2rom(&bytes).map_err(|e| anyhow!("{e}"))?
                }
            };
            prepare(rom, input.as_deref(), &out, max_steps, dump)
        }
        Command::Rom { elf, out } => rom_listing(&elf, &out),
        Command::DecodeDiff { sail, names } => decode_diff(&sail, &names),
        Command::Ops { out, random } => op_vectors(&out, random),
    }
}

/// The values every operation is tested on, pairwise: the bounds of the
/// signed and unsigned ranges at 8, 16, 32 and 64 bits, and shift amounts.
const EDGES: [u64; 16] = [
    0,
    1,
    2,
    63,
    64,
    0x7f,
    0x80,
    0xff,
    0x7fff,
    0x8000,
    0x7fff_ffff,
    0x8000_0000,
    0xffff_ffff,
    0x7fff_ffff_ffff_ffff,
    0x8000_0000_0000_0000,
    0xffff_ffff_ffff_ffff,
];

/// Writes `<mnemonic> <a> <b> <c> <flag>` for every operation `call_ab`
/// computes — every one but the precompiles, DMA and fcall, which work on
/// memory — on the edge cases pairwise and `random` random pairs.
fn op_vectors(out: &Path, random: usize) -> Result<()> {
    // xorshift64*, seeded, so the vectors are the same on every run.
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        state.wrapping_mul(0x2545_f491_4f6c_dd1d)
    };
    // The probes below panic on purpose; keep their messages off the terminal.
    std::panic::set_hook(Box::new(|_| {}));
    let mut text = String::new();
    let (mut ops, mut lines) = (0, 0);
    for code in 0..=u8::MAX {
        let Ok(op) = ZiskOp::try_from_code(code) else { continue };
        if op.is_precompiled() {
            continue;
        }
        let mut pairs: Vec<(u64, u64)> =
            EDGES.iter().flat_map(|&a| EDGES.iter().map(move |&b| (a, b))).collect();
        pairs.extend((0..random).map(|_| (next(), next())));
        // Ops that need memory or a context panic in `call_ab`; skip them.
        let probe = std::panic::catch_unwind(|| op.call_ab(1, 1));
        if probe.is_err() {
            continue;
        }
        ops += 1;
        for (a, b) in pairs {
            let Ok((c, flag)) = std::panic::catch_unwind(|| op.call_ab(a, b)) else {
                bail!("{} panics on a = {a:#x}, b = {b:#x}", op.name());
            };
            writeln!(text, "{} {a:#018x} {b:#018x} {c:#018x} {}", op.name(), flag as u8)?;
            lines += 1;
        }
    }
    fs::write(out, text)?;
    println!("wrote {}: {lines} vectors for {ops} operations", out.display());
    Ok(())
}

/// `<begin>,<end>`, in hex, with or without `0x`.
fn parse_range(s: &str) -> std::result::Result<(u64, u64), String> {
    let hex = |t: &str| {
        u64::from_str_radix(t.trim().trim_start_matches("0x"), 16).map_err(|e| e.to_string())
    };
    let (a, b) = s.split_once(',').ok_or("expected <begin>,<end>")?;
    let (a, b) = (hex(a)?, hex(b)?);
    if a > b || (b - a) % 4 != 0 {
        return Err(format!("[{a:#x}, {b:#x}) is not a range of whole 32-bit words"));
    }
    Ok((a, b))
}

fn prepare(
    rom: ZiskRom,
    input: Option<&Path>,
    out: &Path,
    max_steps: u64,
    dump: Option<(u64, u64)>,
) -> Result<()> {
    let input = match input {
        Some(p) => fs::read(p).with_context(|| format!("reading {}", p.display()))?,
        None => Vec::new(),
    };

    fs::create_dir_all(out.join("mem"))?;

    let (sail, ops) = rom_to_sail(&rom)?;
    fs::write(out.join("rom.sail"), sail)?;

    let mut args = write_memory(&rom, &input, out)?;
    args.push_str(&write_rom_table(&rom, out)?);

    let (trace, steps, tape) = reference_trace(&rom, input, max_steps, dump)?;
    fs::write(out.join("ziskemu.trace"), trace)?;
    args.push_str(&write_fcall_tape(&tape, out)?);
    if let Some((begin, end)) = dump {
        let file = out.join("mem").join("dump-range.bin");
        fs::write(&file, [begin.to_le_bytes(), end.to_le_bytes()].concat())?;
        writeln!(args, "-b {DUMP_RANGE:#x},{}", fs::canonicalize(&file)?.display())?;
    }
    fs::write(out.join("mem.args"), args)?;

    println!(
        "wrote {}: {} instructions, {} opcodes ({}), {} steps on ziskemu",
        out.display(),
        rom.insts.len(),
        ops.len(),
        ops.into_iter().collect::<Vec<_>>().join(", "),
        steps
    );
    Ok(())
}

/// Sail's `bits_str` on a 64-bit vector: `0x` and sixteen uppercase digits.
fn hex(v: u64) -> String {
    format!("0x{v:016X}")
}

/// A Sail `bits(64)` literal, with the digit grouping sail itself accepts.
fn lit(v: u64) -> String {
    let h = format!("{v:016x}");
    format!("0x{}_{}_{}_{}", &h[0..4], &h[4..8], &h[8..12], &h[12..16])
}

/// The program as a `rom_fetch` over every instruction of the ROM, BIOS included.
fn rom_to_sail(rom: &ZiskRom) -> Result<(String, BTreeSet<&'static str>)> {
    let mut ops = BTreeSet::new();
    let mut s = String::new();
    s.push_str(
        "/* Generated by tools/sail-difftest from a ZiskRom. Do not edit.\n \
         * Every instruction, BIOS included, keyed by pc; see model/zisk_rom.sail. */\n\n\
         val rom_fetch : bits(64) -> zisk_inst\n\
         function rom_fetch(pc) = match pc {\n",
    );
    for (&pc, builder) in &rom.insts {
        let i = &builder.i;
        ops.insert(i.op_str);
        let r =
            inst_to_sail(i).with_context(|| format!("instruction at pc {pc:#x}: {}", i.verbose))?;
        writeln!(s, "  /* {} */", i.verbose.replace("*/", "* /"))?;
        writeln!(s, "  {} => {r},", lit(pc))?;
    }
    s.push_str(
        "  _ => {\n    assert(false, \"rom_fetch: no instruction at this pc\");\n    \
         struct { a_src = A_IMM, a_imm = zeros(), b_src = B_IMM, b_imm = zeros(), ind_width = 8,\n             \
         op = ZOP_COPYB(), store = ST_NONE, store_off = zeros(), store_pc = false,\n             \
         set_pc = false, jmp_off1 = zeros(), jmp_off2 = zeros(), is_end = true }\n  }\n}\n",
    );
    Ok((s, ops))
}

/// An instruction as the model sees it: the fields of the Sail `zisk_inst`.
struct ModelInst {
    a_src: &'static str,
    a_imm: u64,
    b_src: &'static str,
    b_imm: u64,
    ind_width: u64,
    op: &'static str,
    store: &'static str,
    store_off: i64,
    store_pc: bool,
    set_pc: bool,
    jmp_off1: i64,
    jmp_off2: i64,
    is_end: bool,
}

/// The model's view of one instruction. Fails on what the model does not
/// describe, rather than exporting something it would silently get wrong.
fn model_inst(i: &ZiskInst) -> Result<ModelInst> {
    // The model has no stack pointer; see "Registers and memory" in sail-zisk/README.md.
    let a_sp = i.a_src != SRC_IMM && i.a_use_sp_imm1 != 0;
    let b_sp = i.b_src != SRC_IMM && i.b_use_sp_imm1 != 0;
    if a_sp || b_sp || i.store_use_sp {
        bail!("uses the stack pointer, which the model leaves out");
    }

    let (a_src, a_imm) = match i.a_src {
        SRC_C => ("A_C", 0),
        SRC_MEM => ("A_MEM", i.a_offset_imm0),
        SRC_IMM => ("A_IMM", i.a_offset_imm0 | (i.a_use_sp_imm1 << 32)),
        SRC_STEP => ("A_STEP", 0),
        SRC_REG => ("A_REG", i.a_offset_imm0),
        s => bail!("a_src {s} has no Sail counterpart"),
    };
    let (b_src, b_imm) = match i.b_src {
        SRC_C => ("B_C", 0),
        SRC_MEM => ("B_MEM", i.b_offset_imm0),
        SRC_IMM => ("B_IMM", i.b_offset_imm0 | (i.b_use_sp_imm1 << 32)),
        SRC_IND => ("B_IND", i.b_offset_imm0),
        SRC_REG => ("B_REG", i.b_offset_imm0),
        s => bail!("b_src {s} has no Sail counterpart"),
    };
    let store = match i.store {
        STORE_NONE => "ST_NONE",
        STORE_MEM => "ST_MEM",
        STORE_IND => "ST_IND",
        STORE_REG => "ST_REG",
        s => bail!("store {s} has no Sail counterpart"),
    };
    // The width only matters to an indirect access; elsewhere the encoder may
    // leave anything there, and the model's type admits only 1, 2, 4 and 8.
    let uses_width = i.b_src == SRC_IND || i.store == STORE_IND;
    let ind_width = match i.ind_width {
        w @ (1 | 2 | 4 | 8) => w,
        _ if !uses_width => 8,
        w => bail!("indirect width {w}"),
    };

    Ok(ModelInst {
        a_src,
        a_imm,
        b_src,
        b_imm,
        ind_width,
        op: i.op_str,
        store,
        store_off: i.store_offset,
        store_pc: i.store_pc,
        set_pc: i.set_pc,
        jmp_off1: i.jmp_offset1,
        jmp_off2: i.jmp_offset2,
        is_end: i.end,
    })
}

/// One `zisk_inst` record literal.
fn inst_to_sail(i: &ZiskInst) -> Result<String> {
    let m = model_inst(i)?;
    Ok(format!(
        "struct {{ a_src = {}, a_imm = {}, b_src = {}, b_imm = {}, ind_width = {},\n    \
         op = ZOP_{}(), store = {}, store_off = {}, store_pc = {},\n    \
         set_pc = {}, jmp_off1 = {}, jmp_off2 = {}, is_end = {} }}",
        m.a_src,
        lit(m.a_imm),
        m.b_src,
        lit(m.b_imm),
        m.ind_width,
        m.op.to_uppercase(),
        m.store,
        lit(m.store_off as u64),
        m.store_pc,
        m.set_pc,
        lit(m.jmp_off1 as u64),
        lit(m.jmp_off2 as u64),
        m.is_end,
    ))
}

/// Where the ROM table starts: `ROM_TABLE` in sail-zisk/model/zisk_rom_table.sail.
const ROM_TABLE: u64 = 0x0001_0000_0000_0000;

/// Where the model reads fcall results from: `FCALL_TAPE` in
/// sail-zisk/model/zisk_insts_fcall.sail.
const FCALL_TAPE: u64 = 0x0002_0000_0000_0000;

/// Where the C emulator finds the range `--dump` asks for: `DUMP_RANGE` in
/// sail-zisk/model/main.sail.
const DUMP_RANGE: u64 = 0x0003_0000_0000_0000;

/// The ROM table entry of one instruction, as sail-zisk/model/zisk_rom_table.sail
/// decodes it: eight little-endian words.
fn rom_entry(i: &ZiskInst) -> Result<[u64; 8]> {
    let m = model_inst(i)?;
    let code = |name: &str, names: &[&str]| -> Result<u64> {
        names.iter().position(|n| *n == name).map(|k| k as u64).ok_or_else(|| anyhow!("{name}"))
    };
    let w0 = 1
        | code(m.a_src, &["A_C", "A_MEM", "A_IMM", "A_STEP", "A_REG"])? << 8
        | code(m.b_src, &["B_C", "B_MEM", "B_IMM", "B_IND", "B_REG"])? << 16
        | code(m.store, &["ST_NONE", "ST_MEM", "ST_IND", "ST_REG"])? << 24
        | m.ind_width << 32
        | (m.store_pc as u64) << 40
        | (m.set_pc as u64) << 41
        | (m.is_end as u64) << 42
        | (i.op as u64) << 48;
    Ok([w0, m.a_imm, m.b_imm, m.store_off as u64, m.jmp_off1 as u64, m.jmp_off2 as u64, 0, 0])
}

/// The `rom` listing: the ELF's code sections and its ROM's table entries.
fn rom_listing(elf: &Path, out: &Path) -> Result<()> {
    let bytes = fs::read(elf).with_context(|| format!("reading {}", elf.display()))?;
    let payload = zisk_riscv::elf_extraction::collect_elf_payload_from_bytes(&bytes)
        .map_err(|e| anyhow!("{e}"))?;
    let rom = zisk_transpiler_common::program2rom(&bytes).map_err(|e| anyhow!("{e}"))?;
    let mut s = String::new();
    for section in &payload.exec {
        write!(s, "code {:x} ", section.addr)?;
        for b in &section.data {
            write!(s, "{b:02x}")?;
        }
        s.push('\n');
    }
    for (&pc, builder) in &rom.insts {
        match rom_entry(&builder.i) {
            Ok(e) => writeln!(
                s,
                "inst {pc:x} {:x} {:x} {:x} {:x} {:x} {:x}",
                e[0], e[1], e[2], e[3], e[4], e[5]
            )?,
            Err(_) => writeln!(s, "skip {pc:x}")?,
        }
    }
    fs::write(out, s).with_context(|| format!("writing {}", out.display()))?;
    Ok(())
}

/// What `convert` does with a word: emits instructions, or a halt (the
/// run fails there), or panics (the transpiler rejects the program).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Conversion {
    Translates,
    Halts,
    /// With the panic's message.
    Panics(String),
}

fn convert_word(w: u32) -> (&'static str, Conversion) {
    let code = [(w & 0xffff) as u16, (w >> 16) as u16];
    let insts = zisk_riscv::riscv_interpreter(0x8000_0000, &code);
    let inst = &insts[0];
    let name = inst.inst_name.as_str();
    let result = std::panic::catch_unwind(|| {
        let mut rom = ZiskRom::default();
        let mut ctx = zisk_riscv::Riscv2ZiskContext::new(&mut rom);
        ctx.convert(inst, &[]);
        rom.insts.values().any(|b| b.i.op_str == "halt" && b.i.end)
    });
    match result {
        Ok(true) => (name, Conversion::Halts),
        Ok(false) => (name, Conversion::Translates),
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|m| m.to_string()))
                .unwrap_or_default();
            // The first line, without the instruction's own fields.
            let msg =
                msg.lines().next().unwrap_or("").split(" at 0x").next().unwrap_or("").to_owned();
            (name, Conversion::Panics(msg))
        }
    }
}

/// The words decode_diff lists, in its order.
fn decode_diff_words() -> Vec<u32> {
    let regs = [0u32, 1, 5];
    let mut ws = Vec::with_capacity(32 * 8 * 4096 * 9);
    for o in 0..32u32 {
        for f3 in 0..8u32 {
            for hi in 0..4096u32 {
                for rd in regs {
                    for rs1 in regs {
                        ws.push((hi << 20) | (rs1 << 15) | (f3 << 12) | (rd << 7) | (o << 2) | 3);
                    }
                }
            }
        }
    }
    ws
}

/// A kind of word where the decoders differ: what differs, the opcode and
/// funct3, the transpiler's name for it and sail-riscv's, and why the
/// transpiler panics, if it does.
type DiffKey = (&'static str, u32, u32, &'static str, String, String);

fn decode_diff(sail: &Path, names: &Path) -> Result<()> {
    let verdicts = fs::read(sail).with_context(|| format!("reading {}", sail.display()))?;
    let names: Vec<String> = fs::read_to_string(names)?.lines().map(str::to_owned).collect();
    let words = decode_diff_words();
    if verdicts.len() != 2 * words.len() {
        bail!("{} has {} verdicts for {} words", sail.display(), verdicts.len() / 2, words.len());
    }
    std::panic::set_hook(Box::new(|_| {}));
    // A kind of word -> how many, and one of them.
    let mut groups: std::collections::BTreeMap<DiffKey, (u64, u32)> = Default::default();
    let mut agree = 0u64;
    for (k, &w) in words.iter().enumerate() {
        let v = u16::from_le_bytes([verdicts[2 * k], verdicts[2 * k + 1]]);
        let (name, conv) = convert_word(w);
        let sail_name = match v {
            0 => "ILLEGAL".to_owned(),
            0xffff => "?".to_owned(),
            n => names[n as usize - 1].clone(),
        };
        let why = match &conv {
            Conversion::Panics(m) if v != 0 => m.clone(),
            _ => String::new(),
        };
        let what = match (conv, v) {
            (_, 0xffff) => "sail-riscv's executable model cannot tell",
            (Conversion::Translates, 0) => "LENIENT: the transpiler runs a word RISC-V rejects",
            (Conversion::Halts, 0) => {
                agree += 1;
                continue;
            }
            (Conversion::Panics(_), 0) => "rejected: RISC-V rejects it too, but only if it runs",
            (Conversion::Halts, _) => "STRICT: the transpiler halts on a word RISC-V decodes",
            (Conversion::Panics(_), _) => "STRICT: the transpiler rejects a word RISC-V decodes",
            (Conversion::Translates, _) => {
                agree += 1;
                continue;
            }
        };
        let e =
            groups.entry((what, w & 0x7f, (w >> 12) & 7, name, sail_name, why)).or_insert((0, w));
        e.0 += 1;
    }
    println!(
        "{} words; {agree} on which both decoders agree, accepting or rejecting.",
        words.len()
    );
    let mut last = "";
    for ((what, op, f3, name, sail_name, why), (n, w)) in &groups {
        if *what != last {
            println!("\n{what}");
            last = what;
        }
        let why = if why.is_empty() { String::new() } else { format!(" ({why})") };
        println!(
            "  opcode {op:#04x} funct3 {f3}: {name} / {sail_name}: {n} words, e.g. {w:#010x}{why}"
        );
    }
    // What the contract allows (sail-zisk/doc/riscv_contract.md): on opcode
    // SYSTEM, ebreak, the privileged instructions and the CSRs are ZisK's own
    // (§§ 1, 5-9), and words the executable model cannot tell are left to the
    // reader. Anything else fails the check.
    let bad: Vec<_> = groups
        .keys()
        .filter(|(what, op, ..)| {
            !(what.starts_with("STRICT") && *op == 0x73) && !what.starts_with("sail-riscv")
        })
        .collect();
    if !bad.is_empty() {
        bail!(
            "{} kinds of word where the decoders differ outside what the contract allows",
            bad.len()
        );
    }
    Ok(())
}

/// Write the ROM as the table the model's C emulator reads it from
/// (sail-zisk/model/zisk_rom_table.sail): an entry of 64 bytes at
/// `ROM_TABLE + 64 * pc` for every instruction, in one section per run of
/// nearby pcs, and return the `-b` arguments that load them.
fn write_rom_table(rom: &ZiskRom, out: &Path) -> Result<String> {
    let mut sections: Vec<(u64, Vec<u8>)> = Vec::new();
    for (&pc, builder) in &rom.insts {
        let entry = rom_entry(&builder.i)
            .with_context(|| format!("instruction at pc {pc:#x}: {}", builder.i.verbose))?;
        let addr = ROM_TABLE + 64 * pc;
        // A gap of up to 4 KB is filled with zeros, entries the model rejects
        // as invalid; a wider one starts a new section.
        match sections.last_mut() {
            Some((start, bytes)) if addr <= *start + bytes.len() as u64 + 4096 => {
                bytes.resize((addr - *start) as usize, 0);
                bytes.extend(entry.iter().flat_map(|w| w.to_le_bytes()));
            }
            _ => sections.push((addr, entry.iter().flat_map(|w| w.to_le_bytes()).collect())),
        }
    }
    let mut args = String::new();
    for (addr, bytes) in sections {
        let file = out.join("mem").join(format!("rom-{addr:016x}.bin"));
        fs::write(&file, bytes)?;
        let file = fs::canonicalize(&file)?;
        writeln!(args, "-b {addr:#x},{}", file.display())?;
    }
    Ok(args)
}

/// Write each initial memory section as a raw file, and return the `-b`
/// arguments that load them. The sections are the ones
/// `Emu::create_emu_context` sets up: the ROM's read-only and read-write data,
/// and the input, behind its 8-byte header at `INPUT_ADDR`. RAM beyond those
/// starts zeroed in both emulators, so it needs no file.
fn write_memory(rom: &ZiskRom, input: &[u8], out: &Path) -> Result<String> {
    let mut sections: Vec<(u64, Vec<u8>)> = Vec::new();
    for s in rom.ro_data_64.iter().chain(&rom.rw_data_64) {
        sections.push((s.addr, s.data.iter().flat_map(|w| w.to_le_bytes()).collect()));
    }
    let mut inp = vec![0u8; 8];
    inp.extend_from_slice(input);
    sections.push((INPUT_ADDR, inp));

    let mut args = String::new();
    for (addr, bytes) in sections {
        let file = out.join("mem").join(format!("{addr:016x}.bin"));
        fs::write(&file, bytes)?;
        let file = fs::canonicalize(&file)?;
        writeln!(args, "-b {addr:#x},{}", file.display())?;
    }
    Ok(args)
}

/// Write the fcall tape (sail-zisk/model/zisk_insts_fcall.sail) at
/// `FCALL_TAPE`, and return the `-b` argument that loads it.
fn write_fcall_tape(tape: &[u64], out: &Path) -> Result<String> {
    let file = out.join("mem").join("fcall-tape.bin");
    fs::write(&file, tape.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>())?;
    Ok(format!("-b {FCALL_TAPE:#x},{}\n", fs::canonicalize(&file)?.display()))
}

/// Run the program on ziskemu one step at a time, and print what
/// `model/zisk_main.sail` prints: per step, the pc it ran at, c, flag, and
/// every register the step changed; then how the run ended, and the output
/// region, and the words of `dump`; or, if a step panics, the steps before
/// it and `fault <message>`. Also returns the fcall tape: each fcall's
/// result, as its length and its words, in the order the run made them.
fn reference_trace(
    rom: &ZiskRom,
    input: Vec<u8>,
    max_steps: u64,
    dump: Option<(u64, u64)>,
) -> Result<(String, u64, Vec<u64>)> {
    let options = EmuOptions::default();
    let mut emu = Emu::new(rom);
    emu.ctx = emu.create_emu_context(input, &options);
    // A panic is a fault the trace records, not an error of this tool.
    std::panic::set_hook(Box::new(|_| {}));

    let mut s = String::new();
    let mut tape = Vec::new();
    let mut n = 0u64;
    while !emu.ctx.inst_ctx.end {
        if n >= max_steps {
            bail!("ziskemu did not end within {max_steps} steps");
        }
        let pc = emu.ctx.inst_ctx.pc;
        let before = emu.ctx.inst_ctx.regs;
        let is_fcall = rom.get_instruction(pc).op == ZiskOp::Fcall.code();
        // A step ZisK refuses — an access outside its memory map, say —
        // panics. The trace ends there, with why: the model must stop at
        // the same step.
        if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            emu.step(&options, &None::<fn(EmuTrace)>)
        })) {
            let why = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|m| m.to_string()))
                .unwrap_or_default();
            writeln!(s, "fault {}", why.lines().next().unwrap_or(""))?;
            return Ok((s, n, tape));
        }
        let ctx = &emu.ctx.inst_ctx;
        if is_fcall {
            let n = ctx.fcall.result_size as usize;
            tape.push(n as u64);
            tape.extend_from_slice(&ctx.fcall.result[..n]);
        }
        write!(s, "{n} {} {} {}", hex(pc), hex(ctx.c), ctx.flag as u8)?;
        for (r, (now, was)) in ctx.regs.iter().zip(&before).enumerate().skip(1) {
            if now != was {
                write!(s, " x{r}={}", hex(*now))?;
            }
        }
        s.push('\n');
        n += 1;
    }
    let ctx = &emu.ctx.inst_ctx;
    writeln!(s, "end {} {}", ctx.error as u8, hex(if ctx.error { ctx.error_code } else { 0 }))?;
    for (k, w) in emu.get_output_8().chunks(8).enumerate() {
        let w = u64::from_le_bytes(w.try_into().context("output is not a whole number of u64")?);
        writeln!(s, "out {k} {}", hex(w))?;
    }
    if let Some((begin, end)) = dump {
        for addr in (begin..end).step_by(4) {
            let w = emu.ctx.inst_ctx.mem.read(addr, 4);
            writeln!(s, "dump {} 0x{w:08X}", hex(addr))?;
        }
    }
    Ok((s, n, tape))
}
