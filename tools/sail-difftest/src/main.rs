//! Prepare a step-by-step differential test of the Sail model against ziskemu.
//!
//! From one ZisK program — ZisK assembly, or a RISC-V ELF transpiled as
//! ziskemu transpiles it — `prepare` writes everything the Sail side needs and
//! the trace it must reproduce:
//!
//! - `rom.sail`, the program as a `rom_fetch` function, for `make ROM=…`;
//! - `mem/*.bin` and `mem.args`, the initial memory as raw sections plus the
//!   `-b addr,file` arguments that load them into the Sail C emulator;
//! - `ziskemu.trace`, the run on ziskemu, in the format `model/zisk_main.sail`
//!   prints.
//!
//! The ROM is built and the memory laid out exactly as `ziskemu -z` does, by
//! the same code: `ziskasm::assemble_files` and `Emu::create_emu_context`.
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
        /// A `.zisk` file or a directory of them, as `ziskemu -z` takes.
        #[arg(short = 'z', long, conflicts_with = "elf", required_unless_present = "elf")]
        zisk: Option<PathBuf>,
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
        Command::Prepare { zisk, elf, input, out, max_steps } => {
            let rom = match (zisk, elf) {
                (Some(zisk), _) => {
                    let files = ziskasm::collect_zisk_files(&zisk.to_string_lossy())
                        .map_err(anyhow::Error::msg)?;
                    ziskasm::assemble_files(&files).map_err(anyhow::Error::msg)?
                }
                (None, Some(elf)) => {
                    let bytes =
                        fs::read(&elf).with_context(|| format!("reading {}", elf.display()))?;
                    zisk_transpiler_common::program2rom(&bytes).map_err(|e| anyhow!("{e}"))?
                }
                (None, None) => bail!("a program is needed: -z or -e"),
            };
            prepare(rom, input.as_deref(), &out, max_steps)
        }
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

fn prepare(rom: ZiskRom, input: Option<&Path>, out: &Path, max_steps: u64) -> Result<()> {
    let input = match input {
        Some(p) => fs::read(p).with_context(|| format!("reading {}", p.display()))?,
        None => Vec::new(),
    };

    fs::create_dir_all(out.join("mem"))?;

    let (sail, ops) = rom_to_sail(&rom)?;
    fs::write(out.join("rom.sail"), sail)?;

    let args = write_memory(&rom, &input, out)?;
    fs::write(out.join("mem.args"), args)?;

    let (trace, steps) = reference_trace(&rom, input, max_steps)?;
    fs::write(out.join("ziskemu.trace"), trace)?;

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
    if i.input_size > 0 || i.is_precompiled {
        bail!("op {} takes extended arguments, which the model does not describe", i.op_str);
    }
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

/// Run the program on ziskemu one step at a time, and print what
/// `model/zisk_main.sail` prints: per step, the pc it ran at, c, flag, and
/// every register the step changed; then how the run ended, and the output
/// region.
fn reference_trace(rom: &ZiskRom, input: Vec<u8>, max_steps: u64) -> Result<(String, u64)> {
    let options = EmuOptions::default();
    let mut emu = Emu::new(rom);
    emu.ctx = emu.create_emu_context(input, &options);

    let mut s = String::new();
    let mut n = 0u64;
    while !emu.ctx.inst_ctx.end {
        if n >= max_steps {
            bail!("ziskemu did not end within {max_steps} steps");
        }
        let pc = emu.ctx.inst_ctx.pc;
        let before = emu.ctx.inst_ctx.regs;
        emu.step(&options, &None::<fn(EmuTrace)>);
        let ctx = &emu.ctx.inst_ctx;
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
    Ok((s, n))
}
