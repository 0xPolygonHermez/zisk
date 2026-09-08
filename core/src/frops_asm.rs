//! x86-64 generation of the FROPS counting code for the ROM-histogram assembly.
//!
//! The ROM-histogram emulation counts, for every executed operation, one increment in a
//! multiplicity table: the ROM histogram counts *instructions* (one counter per program counter),
//! and this module counts *frequent operations* (one counter per FROPS table row). Together they are
//! the multiplicity columns the ROM and FROPS AIRs need.
//!
//! # Shape of the generated code
//!
//! Membership in a FROPS box is a handful of comparisons ([`crate::frops`]), but the ops that carry
//! FROPS are also the most frequent instructions in a ROM — inlining the test at every call site
//! multiplies the size of the generated assembly by about three. So the test lives in an
//! out-of-line thunk, and each call site only loads the operands the thunk cannot know and calls it:
//!
//! ```text
//!     mov  r12, <a>            ; a: register or immediate, whatever the generator has
//!     mov  r13, <b>
//!     call frop_<op>
//! ```
//!
//! The thunk saves nothing and returns nothing: it clobbers [`FROPS_REG_A`], [`FROPS_REG_B`],
//! [`FROPS_REG_TMP`] and FLAGS, all of which are dead at the call site (see those constants), and
//! increments `frops_mult[row]` when the triple is covered.
//!
//! # Specialisation
//!
//! The generator usually knows one of the operands already — `b` for every immediate-form RISC-V
//! instruction (`addi`, `andi`, `slli`, …), `a` for a good share of the rest. That knowledge is
//! worth a lot, so each distinct `(op, known operands)` combination gets its own thunk
//! ([`FropsSpec`]), in which:
//!
//! * boxes the constant rules out are gone, often leaving one box where the generic thunk has four;
//! * the comparisons on the known axis are gone;
//! * that axis' contribution to the row is folded into the box's constant term;
//! * the call site does not have to load the operand at all.
//!
//! When the constant rules out *every* box the operation can never be a frequent operation and
//! nothing is emitted at all; when both operands are known the row is known too, and the call site
//! becomes a bare increment with no thunk.

use crate::frops::{frops_regions, frops_row, FropsRegion};

/// Holds `a` at the call site, and the row when `a` is not known at generation time. `r12` is
/// `REG_MEM_READS_ADDRESS`, which the ROM-histogram mode never uses, and `emu_start` saves it for
/// the caller.
pub const FROPS_REG_A: &str = "r12";
/// Holds `b`, and the row when `a` is known but `b` is not. `r13` is `REG_MEM_READS_SIZE`, unused in
/// ROM-histogram mode for the same reason.
pub const FROPS_REG_B: &str = "r13";
/// Scratch for immediates that do not fit an `imm32`, and for the table address. `rcx` is only ever
/// written, never read, by the operation code that follows the call site.
pub const FROPS_REG_TMP: &str = "rcx";

/// Upper bound on the specialised thunks one opcode may get. Reached only by a ROM that uses an
/// enormous number of distinct immediates with the same opcode; the rest of its call sites fall
/// back to the generic thunk, which is always correct — specialisation is only ever an optimisation.
pub const MAX_SPECIALISED_THUNKS_PER_OP: usize = 4096;

/// A FROPS operand as the ROM-to-assembly generator knows it at the point the check is emitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FropsOperand<'a> {
    /// The value is in this register.
    Reg(&'a str),
    /// The value is known at generation time.
    Const(u64),
}

impl FropsOperand<'_> {
    fn as_str(&self) -> String {
        match self {
            FropsOperand::Reg(r) => (*r).to_string(),
            FropsOperand::Const(v) => format!("0x{v:x}"),
        }
    }
    fn constant(&self) -> Option<u64> {
        match self {
            FropsOperand::Const(v) => Some(*v),
            FropsOperand::Reg(_) => None,
        }
    }
}

/// Which operands a thunk has baked in. `(None, None)` is the generic thunk, which tests both axes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FropsSpec {
    pub a: Option<u64>,
    pub b: Option<u64>,
}

/// What [`emit_call`] emitted, and what the caller still owes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FropsCallSite {
    /// Nothing was emitted: the operation can never be a frequent operation here.
    Nothing,
    /// The row was known at generation time, so the increment was emitted inline. No thunk needed.
    Inline,
    /// A call to this specialisation's thunk was emitted; [`emit_thunk`] must be called for it once.
    Thunk(FropsSpec),
}

/// True when `v` can be used directly as an `imm32` operand: x86-64 sign-extends it to 64 bits, so
/// the whole top range works too (`0xFFFF_FFFF_FFFA_C847` is `-341433`).
fn fits_imm32(v: u64) -> bool {
    (v as i64) == (v as i32 as i64)
}

/// `v` as an assembler immediate, signed when it is being used through the sign-extending `imm32`
/// encoding. Only valid when [`fits_imm32`].
fn imm32(v: u64) -> String {
    let signed = v as i64;
    if (0..65536).contains(&signed) {
        format!("{signed}")
    } else {
        format!("{signed:#x}")
    }
}

/// Whether a box can still match, given what the generator already knows about the operands.
///
/// Each known coordinate is tested against its own axis, pairing it with the box's own base on the
/// other one, which always satisfies that axis. So when both are known this is exactly
/// `r.contains(a, b)`.
fn box_is_reachable(r: &FropsRegion, spec: &FropsSpec) -> bool {
    if let Some(a) = spec.a {
        if !r.contains(a, r.b_lo) {
            return false;
        }
    }
    if let Some(b) = spec.b {
        if !r.contains(r.a_lo, b) {
            return false;
        }
    }
    true
}

/// The boxes of `op` a thunk for `spec` still has to test, in order. Empty means the operation can
/// never be a frequent operation.
fn reachable_boxes(op: u8, spec: &FropsSpec) -> Vec<&'static FropsRegion> {
    frops_regions(op).iter().filter(|r| box_is_reachable(r, spec)).collect()
}

/// Whether `op` has any frequent operations at all.
pub fn op_has_frops(op: u8) -> bool {
    !frops_regions(op).is_empty()
}

/// Label-safe name of an opcode, e.g. `0x0a` -> `add`, `0x25` -> `signextend_w`.
fn op_label(op: u8) -> String {
    crate::zisk_ops::ZiskOp::try_from_code(op)
        .map(|o| o.name().replace('.', "_"))
        .unwrap_or_else(|_| format!("op_{op:02x}"))
}

/// Label of the thunk counting `op` for `spec`.
fn thunk_label(op: u8, spec: &FropsSpec) -> String {
    let mut s = format!("frop_{}", op_label(op));
    if let Some(a) = spec.a {
        s += &format!("_a{a:x}");
    }
    if let Some(b) = spec.b {
        s += &format!("_b{b:x}");
    }
    s
}

/// Emits the code that counts `(op, a, b)` as a frequent operation.
///
/// `specialised_so_far` is how many specialised thunks this opcode already has, which caps
/// specialisation (see [`MAX_SPECIALISED_THUNKS_PER_OP`]); pass 0 to always specialise.
///
/// Must be emitted *before* the operation code: the operation consumes `a` and `b` destructively.
pub fn emit_call(
    op: u8,
    a: FropsOperand,
    b: FropsOperand,
    specialised_so_far: usize,
    table_address: u64,
    comment: impl Fn(&str) -> String,
    code: &mut String,
) -> FropsCallSite {
    let known = FropsSpec { a: a.constant(), b: b.constant() };
    if reachable_boxes(op, &known).is_empty() {
        return FropsCallSite::Nothing;
    }

    // Both operands known: so is the row, and the increment needs neither a thunk nor a register.
    if let (Some(ka), Some(kb)) = (known.a, known.b) {
        let row = frops_row(op, ka, kb).expect("a reachable box must contain a known (a, b)");
        *code += &format!(
            "\tmov {FROPS_REG_TMP}, 0x{:x} {}\n",
            table_address + row * 8,
            comment(&format!("frops: &frops_mult[{row}]"))
        );
        *code +=
            &format!("\tinc qword ptr [{FROPS_REG_TMP}] {}\n", comment("frops: count, row known"));
        return FropsCallSite::Inline;
    }

    // Specialise on the known operand, unless this opcode has already had too many specialisations.
    let spec = if specialised_so_far < MAX_SPECIALISED_THUNKS_PER_OP {
        known
    } else {
        FropsSpec::default()
    };
    for (reg, operand, name, is_known) in
        [(FROPS_REG_A, a, "a", spec.a.is_some()), (FROPS_REG_B, b, "b", spec.b.is_some())]
    {
        if is_known {
            continue; // the thunk has this operand baked in
        }
        let src = operand.as_str();
        assert_ne!(src, FROPS_REG_A, "FROPS operand {name} may not live in {FROPS_REG_A}");
        assert_ne!(src, FROPS_REG_B, "FROPS operand {name} may not live in {FROPS_REG_B}");
        *code += &format!("\tmov {reg}, {src} {}\n", comment(&format!("frops: {name}")));
    }
    *code += &format!("\tcall {} {}\n", thunk_label(op, &spec), comment("frops: count"));
    FropsCallSite::Thunk(spec)
}

/// Emits the out-of-line thunk that counts one opcode's frequent operations for one specialisation.
///
/// `table_address` is the absolute address of `frops_mult[0]`, a `u64` counter per global FROPS row.
pub fn emit_thunk(op: u8, spec: &FropsSpec, table_address: u64, comments: bool, code: &mut String) {
    let boxes = reachable_boxes(op, spec);
    assert!(!boxes.is_empty(), "no FROPS thunk for op {op:#04x} spec {spec:?}");
    assert!(
        spec.a.is_none() || spec.b.is_none(),
        "a fully known row is counted inline, without a thunk"
    );
    let name = thunk_label(op, spec);
    let incr = format!(".L{name}_incr");
    let done = format!(".L{name}_done");
    // The row is built in whichever register holds the operand the thunk does not know.
    let row_reg = if spec.a.is_none() { FROPS_REG_A } else { FROPS_REG_B };

    let bodies: Vec<Vec<String>> = boxes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let fail =
                if i + 1 < boxes.len() { format!(".L{name}_b{}", i + 1) } else { done.clone() };
            // Success falls through into the increment for the last box; the others jump over.
            box_body(r, spec, row_reg, &fail, (i + 1 < boxes.len()).then_some(incr.as_str()))
        })
        .collect();

    if comments {
        let reject: usize = bodies.iter().map(|b| b.len()).sum();
        *code += &format!(
            "# {name}: {} box(es), {} instructions, {reject} worst case to reject\n",
            boxes.len(),
            reject + 3
        );
    }
    *code += &format!("{name}:\n");
    for (i, body) in bodies.iter().enumerate() {
        if i > 0 {
            *code += &format!(".L{name}_b{i}:\n");
        }
        if comments {
            *code += &format!("\t# box {i}: {}\n", box_predicate(boxes[i], spec));
        }
        for line in body {
            *code += &format!("\t{line}\n");
        }
    }
    *code += &format!("{incr}:\n");
    *code += &format!("\tmov {FROPS_REG_TMP}, 0x{table_address:x}\n");
    *code += &format!("\tinc qword ptr [{FROPS_REG_TMP} + {row_reg}*8]\n");
    *code += &format!("{done}:\n");
    *code += "\tret\n\n";
}

/// Human-readable box predicate, for the thunk comments. Axes the thunk knows are shown as the
/// constant it was specialised on.
fn box_predicate(r: &FropsRegion, spec: &FropsSpec) -> String {
    let mut s = match spec.a {
        Some(a) => format!("a == {a:#x}"),
        None if r.a_lo == 0 => match r.a_hi() {
            Some(hi) => format!("a < {hi:#x}"),
            None => "any a".to_string(),
        },
        None => match r.a_hi() {
            Some(hi) => format!("a >= {:#x} && a < {hi:#x}", r.a_lo),
            None => format!("a >= {:#x}", r.a_lo),
        },
    };
    if spec.a.is_none() && r.a_stride > 1 {
        s += &format!(" && (a & {}) == {}", r.a_stride - 1, r.a_residue());
    }
    match spec.b {
        Some(b) => s += &format!(" && b == {b:#x}"),
        None if r.b_count == 1 => s += &format!(" && b == {:#x}", r.b_lo),
        None => {
            if r.b_lo != 0 {
                s += &format!(" && b >= {:#x}", r.b_lo);
            }
            if let Some(hi) = r.b_hi() {
                s += &format!(" && b < {hi:#x}");
            }
        }
    }
    format!("{s}  -> row {}", r.base_row)
}

/// `cmp <reg>, <imm>` + `<jcc> <fail>`, going through the scratch register for immediates that do
/// not fit an `imm32`.
fn push_cmp(out: &mut Vec<String>, reg: &str, imm: u64, jcc: &str, fail: &str) {
    if imm == 0 && jcc == "jne" {
        out.push(format!("test {reg}, {reg}"));
    } else if fits_imm32(imm) {
        out.push(format!("cmp {reg}, {}", imm32(imm)));
    } else {
        out.push(format!("mov {FROPS_REG_TMP}, 0x{imm:x}"));
        out.push(format!("cmp {reg}, {FROPS_REG_TMP}"));
    }
    out.push(format!("{jcc} {fail}"));
}

/// `add <reg>, <imm>`, through the scratch register when needed. Nothing for a zero.
fn push_add(out: &mut Vec<String>, reg: &str, imm: u64) {
    if imm == 0 {
    } else if fits_imm32(imm) {
        out.push(format!("add {reg}, {}", imm32(imm)));
    } else {
        out.push(format!("mov {FROPS_REG_TMP}, 0x{imm:x}"));
        out.push(format!("add {reg}, {FROPS_REG_TMP}"));
    }
}

/// One box: the membership test for the axes the thunk does not know, jumping to `fail` on any
/// mismatch, followed by the row computation, which leaves the global row in `row_reg`. `success` is
/// the increment label to jump to, or `None` when the increment follows immediately.
///
/// The `b` axis is tested first so that [`FROPS_REG_A`] is still free as scratch, and neither test
/// modifies its operand — only the row computation does, once the box has matched for good.
///
/// # Row computation
///
/// ```text
/// row = base_row + ((a - a_lo) / stride) * b_count + (b - b_lo)
/// ```
///
/// Every term the thunk knows is a constant, and so are `a_lo`, `b_lo` and `base_row`. Because the
/// box has already been shown to hold `a` at the right alignment, `(a - a_lo) / stride` is
/// `(a >> log2(stride)) - (a_lo >> log2(stride))`, so *nothing* has to be subtracted from the
/// operands: the whole constant part collapses into one `add`, evaluated modulo 2^64 (the
/// intermediate products may wrap, the final row cannot — it is bounded by the table size).
fn box_body(
    r: &FropsRegion,
    spec: &FropsSpec,
    row_reg: &str,
    fail: &str,
    success: Option<&str>,
) -> Vec<String> {
    let mut out = Vec::new();
    let shift = r.a_stride.trailing_zeros();

    // b axis, when the thunk does not know it.
    if spec.b.is_none() {
        if r.b_count == 1 {
            push_cmp(&mut out, FROPS_REG_B, r.b_lo, "jne", fail);
        } else {
            if r.b_lo != 0 {
                push_cmp(&mut out, FROPS_REG_B, r.b_lo, "jb", fail);
            }
            if let Some(hi) = r.b_hi() {
                push_cmp(&mut out, FROPS_REG_B, hi, "jae", fail);
            }
        }
    }

    // a axis, when the thunk does not know it.
    if spec.a.is_none() {
        if let Some(hi) = r.a_hi() {
            push_cmp(&mut out, FROPS_REG_A, hi, "jae", fail);
        }
        if r.a_lo != 0 {
            push_cmp(&mut out, FROPS_REG_A, r.a_lo, "jb", fail);
        }
        if r.a_stride > 1 {
            let mask = r.a_stride - 1;
            if r.a_residue() == 0 {
                out.push(format!("test {FROPS_REG_A}, {mask}"));
                out.push(format!("jnz {fail}"));
            } else {
                out.push(format!("mov {FROPS_REG_TMP}, {FROPS_REG_A}"));
                out.push(format!("and {FROPS_REG_TMP}, {mask}"));
                out.push(format!("cmp {FROPS_REG_TMP}, {}", r.a_residue()));
                out.push(format!("jne {fail}"));
            }
        }
    }

    // Row. `constant` accumulates every term known at generation time, modulo 2^64.
    let mut constant = r.base_row;
    match spec.a {
        // a is scaled in place, and its `a_lo` offset moves into the constant.
        None => {
            if shift > 0 {
                out.push(format!("shr {FROPS_REG_A}, {shift}"));
            }
            if r.b_count != 1 {
                out.push(format!("imul {FROPS_REG_A}, {FROPS_REG_A}, {}", r.b_count));
            }
            constant = constant.wrapping_sub((r.a_lo >> shift).wrapping_mul(r.b_count));
        }
        Some(a) => constant = constant.wrapping_add(((a - r.a_lo) >> shift) * r.b_count),
    }
    match spec.b {
        None => {
            // `b` contributes its value to the row register when that register *is* the one holding
            // it (`a` is known), or when it has to be added in. A single-valued axis with the row
            // elsewhere contributes nothing at all: `b == b_lo` is already proven.
            let b_reg_is_row = row_reg == FROPS_REG_B;
            if b_reg_is_row || r.b_count != 1 {
                if !b_reg_is_row {
                    out.push(format!("add {FROPS_REG_A}, {FROPS_REG_B}"));
                }
                constant = constant.wrapping_sub(r.b_lo);
            }
        }
        Some(b) => constant = constant.wrapping_add(b - r.b_lo),
    }
    push_add(&mut out, row_reg, constant);

    if let Some(label) = success {
        out.push(format!("jmp {label}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frops::FROPS_REGIONS;

    const TABLE: u64 = 0xd000_1000;
    /// Loaded into the register of an operand the thunk claims to know: if the thunk reads it after
    /// all, the row comes out wrong and the test fails.
    const POISON: u64 = 0xbaad_f00d_dead_beef;

    /// Minimal interpreter of the instruction subset [`emit_thunk`] produces, used to check the
    /// emitted text against [`frops_row`]. Returns the row the thunk would increment, or `None`
    /// when it returns without counting.
    ///
    /// Anything the emitter is not supposed to produce panics, so a change that emits an instruction
    /// this does not model fails the tests instead of silently going unchecked.
    fn run(asm: &str, spec: &FropsSpec, a: u64, b: u64) -> Option<u64> {
        let lines: Vec<&str> = asm
            .lines()
            .map(|l| l.split('#').next().unwrap().trim())
            .filter(|l| !l.is_empty())
            .collect();
        let mut labels = std::collections::HashMap::new();
        for (i, l) in lines.iter().enumerate() {
            if let Some(name) = l.strip_suffix(':') {
                labels.insert(name.to_string(), i);
            }
        }
        let row_reg = if spec.a.is_none() { FROPS_REG_A } else { FROPS_REG_B };
        let mut reg = std::collections::HashMap::from([
            (FROPS_REG_A.to_string(), if spec.a.is_none() { a } else { POISON }),
            (FROPS_REG_B.to_string(), if spec.b.is_none() { b } else { POISON }),
            (FROPS_REG_TMP.to_string(), POISON),
        ]);
        let val = |reg: &std::collections::HashMap<String, u64>, t: &str| -> u64 {
            if let Some(v) = reg.get(t) {
                *v
            } else if let Some(hex) = t.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).unwrap()
            } else if let Some(hex) = t.strip_prefix("-0x") {
                (-i64::from_str_radix(hex, 16).unwrap()) as u64
            } else {
                t.parse::<i64>().unwrap_or_else(|_| panic!("operand {t:?}")) as u64
            }
        };
        let (mut zf, mut cf) = (false, false);
        let mut pc = 0usize;
        let mut steps = 0;
        while pc < lines.len() {
            steps += 1;
            assert!(steps < 10_000, "thunk does not terminate");
            let line = lines[pc];
            pc += 1;
            if line.ends_with(':') {
                continue;
            }
            let (mnemonic, rest) = line.split_once(' ').unwrap_or((line, ""));
            let ops: Vec<&str> = rest.split(',').map(str::trim).filter(|o| !o.is_empty()).collect();
            let jump = |target: &str| labels[target];
            match mnemonic {
                "cmp" => {
                    let (x, y) = (val(&reg, ops[0]), val(&reg, ops[1]));
                    zf = x == y;
                    cf = x < y;
                }
                "test" => {
                    zf = val(&reg, ops[0]) & val(&reg, ops[1]) == 0;
                    cf = false;
                }
                "jae" => {
                    if !cf {
                        pc = jump(ops[0]);
                    }
                }
                "jb" => {
                    if cf {
                        pc = jump(ops[0]);
                    }
                }
                "jne" | "jnz" => {
                    if !zf {
                        pc = jump(ops[0]);
                    }
                }
                "jmp" => pc = jump(ops[0]),
                "mov" => {
                    let v = val(&reg, ops[1]);
                    *reg.get_mut(ops[0]).unwrap() = v;
                }
                "add" => {
                    let v = val(&reg, ops[1]);
                    let r = reg.get_mut(ops[0]).unwrap();
                    *r = r.wrapping_add(v);
                }
                "and" => {
                    let v = val(&reg, ops[1]);
                    let r = reg.get_mut(ops[0]).unwrap();
                    *r &= v;
                }
                "shr" => {
                    let v = val(&reg, ops[1]);
                    let r = reg.get_mut(ops[0]).unwrap();
                    *r >>= v;
                }
                "imul" => {
                    assert_eq!(ops[0], ops[1], "only the 3-operand self form is emitted");
                    let v = val(&reg, ops[2]);
                    let r = reg.get_mut(ops[0]).unwrap();
                    *r = r.wrapping_mul(v);
                }
                "inc" => {
                    assert_eq!(
                        rest.trim(),
                        format!("qword ptr [{FROPS_REG_TMP} + {row_reg}*8]"),
                        "unexpected increment operand"
                    );
                    assert_eq!(reg[FROPS_REG_TMP], TABLE, "increment through the wrong base");
                    return Some(reg[row_reg]);
                }
                "ret" => return None,
                other => panic!("unmodelled instruction {other:?} in {line:?}"),
            }
        }
        panic!("fell off the end of the thunk");
    }

    /// Values that exercise every box of an opcode on one axis: each corner, the middle, and the
    /// near misses just outside the bounds (including the stride alignment).
    fn axis_samples(op: u8, a_axis: bool) -> Vec<u64> {
        let mut v = vec![0, 1, u64::MAX];
        for r in FROPS_REGIONS[op as usize] {
            let (lo, count, stride) =
                if a_axis { (r.a_lo, r.a_count, r.a_stride) } else { (r.b_lo, r.b_count, 1) };
            let last = lo + (count - 1) * stride;
            v.extend([
                lo,
                lo + (count / 2) * stride,
                last,
                lo.wrapping_sub(1),
                last.wrapping_add(stride),
                lo.wrapping_add(1),
            ]);
        }
        v
    }

    fn samples(op: u8) -> Vec<(u64, u64)> {
        let mut v = Vec::new();
        for a in axis_samples(op, true) {
            for b in axis_samples(op, false) {
                v.push((a, b));
            }
        }
        v
    }

    /// The emitted assembly must count exactly the rows the reference implementation says, for the
    /// generic thunk and for every specialisation the samples can produce.
    #[test]
    fn thunks_count_the_reference_rows() {
        let mut checked = 0;
        for op in 0u8..=255 {
            if !op_has_frops(op) {
                continue;
            }
            let pairs = samples(op);
            // Every specialisation a call site could ask for: generic, b known, a known.
            let mut specs = vec![FropsSpec::default()];
            specs.extend(pairs.iter().map(|&(_, b)| FropsSpec { a: None, b: Some(b) }));
            specs.extend(pairs.iter().map(|&(a, _)| FropsSpec { a: Some(a), b: None }));
            specs.sort();
            specs.dedup();

            for spec in &specs {
                if reachable_boxes(op, spec).is_empty() {
                    continue; // no thunk is emitted for a specialisation that can never match
                }
                let mut asm = String::new();
                emit_thunk(op, spec, TABLE, true, &mut asm);
                for &(a, b) in &pairs {
                    // The thunk only sees the operands it does not know.
                    let (a, b) = (spec.a.unwrap_or(a), spec.b.unwrap_or(b));
                    assert_eq!(
                        run(&asm, spec, a, b),
                        frops_row(op, a, b),
                        "op {op:#04x} spec {spec:?} a={a:#x} b={b:#x}\n{asm}"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000, "only {checked} triples checked");
    }

    /// A specialised thunk must be smaller than the generic one whenever the constant rules a box
    /// out or removes a comparison — that is the whole point of specialising.
    #[test]
    fn specialising_never_grows_the_thunk() {
        let mut smaller = 0;
        for op in 0u8..=255 {
            if !op_has_frops(op) {
                continue;
            }
            let mut generic = String::new();
            emit_thunk(op, &FropsSpec::default(), TABLE, false, &mut generic);
            let generic_len = generic.lines().count();
            for r in FROPS_REGIONS[op as usize] {
                for spec in
                    [FropsSpec { a: None, b: Some(r.b_lo) }, FropsSpec { a: Some(r.a_lo), b: None }]
                {
                    let mut special = String::new();
                    emit_thunk(op, &spec, TABLE, false, &mut special);
                    let len = special.lines().count();
                    assert!(
                        len <= generic_len,
                        "op {op:#04x} spec {spec:?}: {len} lines vs {generic_len} generic"
                    );
                    smaller += (len < generic_len) as usize;
                }
            }
        }
        assert!(smaller > 0, "specialisation never helped, which cannot be right");
    }

    /// What a call site emits: nothing when it cannot be a FROP, an inline increment when the row is
    /// known, a call otherwise. And the cap must fall back to the generic thunk.
    #[test]
    fn call_sites_match_what_the_generator_knows() {
        let nop = |_: &str| String::new();
        for op in 0u8..=255 {
            let mut code = String::new();
            let site = emit_call(
                op,
                FropsOperand::Reg("rbx"),
                FropsOperand::Reg("rax"),
                0,
                TABLE,
                nop,
                &mut code,
            );
            if !op_has_frops(op) {
                assert_eq!(site, FropsCallSite::Nothing, "op {op:#04x}");
                assert!(code.is_empty());
                continue;
            }
            assert_eq!(site, FropsCallSite::Thunk(FropsSpec::default()), "op {op:#04x}");

            for r in FROPS_REGIONS[op as usize] {
                // Both operands known: the row is known, so no thunk and no register.
                let mut code = String::new();
                let site = emit_call(
                    op,
                    FropsOperand::Const(r.a_lo),
                    FropsOperand::Const(r.b_lo),
                    0,
                    TABLE,
                    nop,
                    &mut code,
                );
                assert_eq!(site, FropsCallSite::Inline, "op {op:#04x}");
                assert!(!code.contains("call"), "an inline count must not call a thunk");
                let row = frops_row(op, r.a_lo, r.b_lo).unwrap();
                assert!(
                    code.contains(&format!("0x{:x}", TABLE + row * 8)),
                    "op {op:#04x}: inline count does not address row {row}\n{code}"
                );

                // One operand known: a specialised thunk, and only the other operand is loaded.
                let mut code = String::new();
                let site = emit_call(
                    op,
                    FropsOperand::Reg("rbx"),
                    FropsOperand::Const(r.b_lo),
                    0,
                    TABLE,
                    nop,
                    &mut code,
                );
                assert_eq!(site, FropsCallSite::Thunk(FropsSpec { a: None, b: Some(r.b_lo) }));
                assert!(
                    !code.contains(FROPS_REG_B),
                    "b is baked in, it must not be loaded\n{code}"
                );

                // Past the cap, the same call site falls back to the generic thunk.
                let mut code = String::new();
                let site = emit_call(
                    op,
                    FropsOperand::Reg("rbx"),
                    FropsOperand::Const(r.b_lo),
                    MAX_SPECIALISED_THUNKS_PER_OP,
                    TABLE,
                    nop,
                    &mut code,
                );
                assert_eq!(site, FropsCallSite::Thunk(FropsSpec::default()));
                assert!(code.contains(FROPS_REG_B), "the generic thunk needs b loaded\n{code}");
            }

            // A constant no box can cover: never a FROP, nothing emitted.
            let wild = 0xdead_beef_0bad_f00d;
            let reachable = FROPS_REGIONS[op as usize]
                .iter()
                .any(|r| box_is_reachable(r, &FropsSpec { a: None, b: Some(wild) }));
            let mut code = String::new();
            let site = emit_call(
                op,
                FropsOperand::Reg("rbx"),
                FropsOperand::Const(wild),
                0,
                TABLE,
                nop,
                &mut code,
            );
            assert_eq!(site == FropsCallSite::Nothing, !reachable, "op {op:#04x}");
        }
    }
}
