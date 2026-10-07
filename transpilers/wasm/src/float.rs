//! f32/f64 lowering onto the RISC-V soft-float library (`float` feature).
//!
//! Zisk has no native floating point.  The RISC-V transpiler handles the F/D extensions by
//! trapping every float instruction into a handler (`FLOAT_HANDLER_ADDR`, installed by
//! `add_end_and_lib`) that saves the integer registers, runs the SoftFloat-based `ziskfloat.elf`
//! (linked into its own ROM window by [`link_float_lib`]) on the raw RISC-V encoding stored at
//! `FREG_INST`, restores the registers and jumps back to `FREG_RA`.  The library operates on
//! memory-resident float registers `FREG_F0..`.
//!
//! The wasm lowering reuses that machinery verbatim: every wasm float operator is turned into a
//! hand-assembled RISC-V F/D instruction over a fixed set of scratch float registers (`f1`, `f2`
//! for the operands, `f3` for the result; `x5` = `REG_T0` for integer operands/results), plus a
//! few integer-side fixups where wasm and RISC-V semantics differ (NaN handling of `min`/`max`
//! and the saturating conversions, rounding of `ceil`/`floor`/`trunc`/`nearest`).
//!
//! Value representation in operand slots: an f64 is its IEEE-754 bit pattern; an f32 occupies the
//! low 32 bits and the high half is don't-care.  Values handed to the library are sign-extended
//! (the `flw` convention the library's sign tests rely on).

use std::error::Error;

use super::emit::Code;
use super::layout::*;
use super::lowering::FuncGen;
use wasmparser::Operator;
use zisk_core::mem::DataSection;
use zisk_core::rom_layout::{normalize_rw_data_sections, FLOAT_HANDLER_ADDR};
use zisk_core::zisk_rom::DataSection64;
use zisk_core::{ZiskRom, FREG_FIRST, FREG_INST, FREG_RA};
use zisk_riscv::elf_extraction::{collect_elf_payload_from_bytes, merge_ro_sections};
use zisk_riscv::{add_zisk_code, zkvmcall_ids};

const FLOAT_LIB_ELF: &[u8] = include_bytes!("../../../lib-float/c/lib/ziskfloat.elf");

/// Adds the soft-float library's code and data to `rom`, the same way `elf2rom` does for the
/// RISC-V path.  Must run after `add_end_and_lib`, which installs the handler that jumps to it.
pub fn link_float_lib(rom: &mut ZiskRom) -> Result<(), Box<dyn Error>> {
    let payload = collect_elf_payload_from_bytes(FLOAT_LIB_ELF)?;
    // The library is plain soft-float C: it must not reach for the ZisK library through
    // zkvmcalls, which the wasm machine does not link.
    let no_zkvmcalls = std::collections::HashMap::new();
    let no_inline_zkvmcalls = std::collections::HashMap::new();
    for section in &payload.exec {
        if !zkvmcall_ids(section.addr, &section.data)?.is_empty() {
            return Err("wasm: the soft-float library unexpectedly uses zkvmcalls".into());
        }
        add_zisk_code(rom, section.addr, &section.data, &no_zkvmcalls, &no_inline_zkvmcalls);
    }
    for section in merge_ro_sections(payload.ro)? {
        rom.ro_data_64.push(to_section64(section));
    }
    for section in normalize_rw_data_sections(payload.rw) {
        rom.rw_data_64.push(to_section64(section));
    }
    Ok(())
}

fn to_section64(section: DataSection) -> DataSection64 {
    let data = section.data.chunks(8).map(|c| u64::from_le_bytes(c.try_into().unwrap()));
    DataSection64 { addr: section.addr, data: data.collect() }
}

// ---------------------------------------------------------------------------
// RISC-V F/D encodings
// ---------------------------------------------------------------------------

/// Scratch float registers (indices into `FREG_F0..`).
const FA: u32 = 1;
const FB: u32 = 2;
const FR: u32 = 3;
/// Integer register used for integer operands and results (`x5` = `REG_T0`).
const XT: u32 = REG_T0 as u32;

/// Absolute address of float register `f`.
const fn freg_addr(f: u32) -> u64 {
    FREG_FIRST + 8 * f as u64
}

/// Rounding modes (the `rm` field).
const RNE: u32 = 0;
const RTZ: u32 = 1;
const RDN: u32 = 2;
const RUP: u32 = 3;

/// Assembles an R-type OP-FP instruction.
const fn op_fp(funct7: u32, rs2: u32, rs1: u32, rm: u32, rd: u32) -> u32 {
    (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (rm << 12) | (rd << 7) | 0x53
}

/// Single (`f32`) or double (`f64`) precision; selects the `.s`/`.d` encodings and bit masks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Prec {
    S,
    D,
}

impl Prec {
    /// The `.d` variant of every OP-FP funct7 is the `.s` one plus one.
    const fn fmt(self) -> u32 {
        match self {
            Prec::S => 0,
            Prec::D => 1,
        }
    }
    const fn sign_mask(self) -> u64 {
        match self {
            Prec::S => 0x8000_0000,
            Prec::D => 0x8000_0000_0000_0000,
        }
    }
    const fn abs_mask(self) -> u64 {
        match self {
            Prec::S => 0x7FFF_FFFF,
            Prec::D => 0x7FFF_FFFF_FFFF_FFFF,
        }
    }
    /// Positive infinity; `|x| > inf` (as integers) is the NaN test.
    const fn inf_bits(self) -> u64 {
        match self {
            Prec::S => 0x7F80_0000,
            Prec::D => 0x7FF0_0000_0000_0000,
        }
    }
    const fn canonical_nan(self) -> u64 {
        match self {
            Prec::S => 0x7FC0_0000,
            Prec::D => 0x7FF8_0000_0000_0000,
        }
    }
    /// `2^p` where `p` is the significand width: every finite value with `|x| >= 2^p` is already
    /// an integer, so rounding is the identity there.
    const fn integral_threshold(self) -> u64 {
        match self {
            Prec::S => 0x4B00_0000,           // 2^23
            Prec::D => 0x4330_0000_0000_0000, // 2^52
        }
    }
    /// `2^n` as a float of this precision (`n` in 0..=64).
    const fn pow2_bits(self, n: u32) -> u64 {
        match self {
            Prec::S => ((127 + n) as u64) << 23,
            Prec::D => ((1023 + n) as u64) << 52,
        }
    }
}

/// Integer destination/source widths for the conversions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum IntTy {
    I32,
    I64,
}

// ---------------------------------------------------------------------------
// Emission helpers
// ---------------------------------------------------------------------------

/// Runs RISC-V instruction `inst` in the float handler and returns to the next instruction.
fn call_handler(code: &mut Code, inst: u32) {
    code.store_imm_to_abs(FREG_INST, inst as u64);
    let ret = code.new_label();
    code.store_label_addr_to_abs(FREG_RA, ret);
    code.jump_abs(FLOAT_HANDLER_ADDR);
    code.bind(ret);
}

/// `f<f> = mem[FP+slot]`, sign-extended from 32 bits for single precision (one instruction).
fn freg_from_slot(code: &mut Code, f: u32, slot: i64, prec: Prec) {
    let op = match prec {
        Prec::D => "copyb",
        Prec::S => "signextend_w",
    };
    code.slot_to_abs(freg_addr(f), slot, op);
}

/// `mem[FP+slot] = f<f>` (one instruction).
fn slot_from_freg(code: &mut Code, slot: i64, f: u32) {
    code.abs_to_slot(slot, freg_addr(f));
}

/// `f<f> = x5` (integer operand for a conversion).
fn freg_from_int(code: &mut Code, f: u32) {
    code.store_reg_to_abs(freg_addr(f), REG_T0, 8);
}

impl<'a, 'b> FuncGen<'a, 'b> {
    /// Lowers `op` if it is a float operator; returns `Ok(false)` for anything else.
    pub(crate) fn lower_float_op(&mut self, op: &Operator) -> Result<bool, Box<dyn Error>> {
        use Operator::*;
        match op {
            // -- constants / memory --------------------------------------------
            F32Const { value } => {
                let off = self.push();
                self.code.store_imm_to_slot(off, value.bits() as i64);
            }
            F64Const { value } => {
                let off = self.push();
                self.code.store_imm_to_slot(off, value.bits() as i64);
            }
            F32Load { memarg } => self.load("copyb", 4, memarg.offset),
            F64Load { memarg } => self.load("copyb", 8, memarg.offset),
            F32Store { memarg } => self.store(4, memarg.offset),
            F64Store { memarg } => self.store(8, memarg.offset),

            // -- arithmetic ----------------------------------------------------
            F32Add => self.f_binop(Prec::S, 0x00),
            F64Add => self.f_binop(Prec::D, 0x00),
            F32Sub => self.f_binop(Prec::S, 0x04),
            F64Sub => self.f_binop(Prec::D, 0x04),
            F32Mul => self.f_binop(Prec::S, 0x08),
            F64Mul => self.f_binop(Prec::D, 0x08),
            F32Div => self.f_binop(Prec::S, 0x0C),
            F64Div => self.f_binop(Prec::D, 0x0C),
            F32Sqrt => self.f_sqrt(Prec::S),
            F64Sqrt => self.f_sqrt(Prec::D),
            F32Min => self.f_minmax(Prec::S, RNE),
            F64Min => self.f_minmax(Prec::D, RNE),
            F32Max => self.f_minmax(Prec::S, RTZ),
            F64Max => self.f_minmax(Prec::D, RTZ),

            // -- sign manipulation (pure bit operations) -----------------------
            F32Abs => self.f_bitop_imm("and", Prec::S.abs_mask()),
            F64Abs => self.f_bitop_imm("and", Prec::D.abs_mask()),
            F32Neg => self.f_bitop_imm("xor", Prec::S.sign_mask()),
            F64Neg => self.f_bitop_imm("xor", Prec::D.sign_mask()),
            F32Copysign => self.f_copysign(Prec::S),
            F64Copysign => self.f_copysign(Prec::D),

            // -- rounding to integral --------------------------------------------
            F32Ceil => self.f_round(Prec::S, RUP),
            F64Ceil => self.f_round(Prec::D, RUP),
            F32Floor => self.f_round(Prec::S, RDN),
            F64Floor => self.f_round(Prec::D, RDN),
            F32Trunc => self.f_round(Prec::S, RTZ),
            F64Trunc => self.f_round(Prec::D, RTZ),
            F32Nearest => self.f_round(Prec::S, RNE),
            F64Nearest => self.f_round(Prec::D, RNE),

            // -- comparisons (push i32 0/1) ---------------------------------------
            F32Eq => self.f_compare(Prec::S, 2, false, false),
            F64Eq => self.f_compare(Prec::D, 2, false, false),
            F32Ne => self.f_compare(Prec::S, 2, false, true),
            F64Ne => self.f_compare(Prec::D, 2, false, true),
            F32Lt => self.f_compare(Prec::S, 1, false, false),
            F64Lt => self.f_compare(Prec::D, 1, false, false),
            F32Gt => self.f_compare(Prec::S, 1, true, false),
            F64Gt => self.f_compare(Prec::D, 1, true, false),
            F32Le => self.f_compare(Prec::S, 0, false, false),
            F64Le => self.f_compare(Prec::D, 0, false, false),
            F32Ge => self.f_compare(Prec::S, 0, true, false),
            F64Ge => self.f_compare(Prec::D, 0, true, false),

            // -- conversions -----------------------------------------------------
            F32ConvertI32S => self.f_from_int(Prec::S, IntTy::I32, true),
            F32ConvertI32U => self.f_from_int(Prec::S, IntTy::I32, false),
            F32ConvertI64S => self.f_from_int(Prec::S, IntTy::I64, true),
            F32ConvertI64U => self.f_from_int(Prec::S, IntTy::I64, false),
            F64ConvertI32S => self.f_from_int(Prec::D, IntTy::I32, true),
            F64ConvertI32U => self.f_from_int(Prec::D, IntTy::I32, false),
            F64ConvertI64S => self.f_from_int(Prec::D, IntTy::I64, true),
            F64ConvertI64U => self.f_from_int(Prec::D, IntTy::I64, false),
            I32TruncF32S => self.int_from_f(Prec::S, IntTy::I32, true, true),
            I32TruncF32U => self.int_from_f(Prec::S, IntTy::I32, false, true),
            I32TruncF64S => self.int_from_f(Prec::D, IntTy::I32, true, true),
            I32TruncF64U => self.int_from_f(Prec::D, IntTy::I32, false, true),
            I64TruncF32S => self.int_from_f(Prec::S, IntTy::I64, true, true),
            I64TruncF32U => self.int_from_f(Prec::S, IntTy::I64, false, true),
            I64TruncF64S => self.int_from_f(Prec::D, IntTy::I64, true, true),
            I64TruncF64U => self.int_from_f(Prec::D, IntTy::I64, false, true),
            I32TruncSatF32S => self.int_from_f(Prec::S, IntTy::I32, true, false),
            I32TruncSatF32U => self.int_from_f(Prec::S, IntTy::I32, false, false),
            I32TruncSatF64S => self.int_from_f(Prec::D, IntTy::I32, true, false),
            I32TruncSatF64U => self.int_from_f(Prec::D, IntTy::I32, false, false),
            I64TruncSatF32S => self.int_from_f(Prec::S, IntTy::I64, true, false),
            I64TruncSatF32U => self.int_from_f(Prec::S, IntTy::I64, false, false),
            I64TruncSatF64S => self.int_from_f(Prec::D, IntTy::I64, true, false),
            I64TruncSatF64U => self.int_from_f(Prec::D, IntTy::I64, false, false),
            F32DemoteF64 => self.f_unary_lib(Prec::D, op_fp(0x20, 1, FA, RNE, FR)),
            F64PromoteF32 => self.f_unary_lib(Prec::S, op_fp(0x21, 0, FA, RNE, FR)),

            // -- reinterpretations: same bits, only the i32 canonical form changes ---
            I32ReinterpretF32 => self.unop_signextend("signextend_w"),
            I64ReinterpretF64 | F32ReinterpretI32 | F64ReinterpretI64 => {}

            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `a op b` through the library: `fR = fA <op> fB`.
    fn f_binop(&mut self, prec: Prec, funct7_s: u32) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        freg_from_slot(&mut self.code, FA, a, prec);
        freg_from_slot(&mut self.code, FB, b, prec);
        call_handler(&mut self.code, op_fp(funct7_s + prec.fmt(), FB, FA, RNE, FR));
        slot_from_freg(&mut self.code, a, FR);
        self.depth -= 1;
    }

    /// Unary library op on the top of stack: `fR = <inst>(fA)`, result replaces the operand.
    fn f_unary_lib(&mut self, in_prec: Prec, inst: u32) {
        let a = self.slot(self.depth - 1);
        freg_from_slot(&mut self.code, FA, a, in_prec);
        call_handler(&mut self.code, inst);
        slot_from_freg(&mut self.code, a, FR);
    }

    fn f_sqrt(&mut self, prec: Prec) {
        self.f_unary_lib(prec, op_fp(0x2C + prec.fmt(), 0, FA, RNE, FR));
    }

    /// `min` (`rm` = 0) / `max` (`rm` = 1).  RISC-V returns the non-NaN operand when only one is
    /// NaN; wasm requires NaN, so NaN inputs are short-circuited on the integer side.
    fn f_minmax(&mut self, prec: Prec, rm: u32) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        let nan = self.code.new_label();
        let done = self.code.new_label();
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.alu_ri("and", REG_T0, REG_T0, prec.abs_mask() as i64);
        self.code.cmp_imm_branch("ltu", REG_T0, prec.inf_bits() as i64 + 1, nan, false);
        self.code.load_slot_to_reg(REG_T0, b);
        self.code.alu_ri("and", REG_T0, REG_T0, prec.abs_mask() as i64);
        self.code.cmp_imm_branch("ltu", REG_T0, prec.inf_bits() as i64 + 1, nan, false);
        freg_from_slot(&mut self.code, FA, a, prec);
        freg_from_slot(&mut self.code, FB, b, prec);
        call_handler(&mut self.code, op_fp(0x14 + prec.fmt(), FB, FA, rm, FR));
        slot_from_freg(&mut self.code, a, FR);
        self.code.jump(done);
        self.code.bind(nan);
        self.code.store_imm_to_slot(a, prec.canonical_nan() as i64);
        self.code.bind(done);
        self.depth -= 1;
    }

    /// `top = top <op> imm` on the raw bits (abs / neg).
    fn f_bitop_imm(&mut self, op: &str, imm: u64) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.alu_ri(op, REG_T0, REG_T0, imm as i64);
        self.code.store_reg_to_slot(a, REG_T0);
    }

    /// `copysign(a, b) = (a & ~sign) | (b & sign)`.
    fn f_copysign(&mut self, prec: Prec) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.load_slot_to_reg(REG_T1, b);
        self.code.alu_ri("and", REG_T0, REG_T0, prec.abs_mask() as i64);
        self.code.alu_ri("and", REG_T1, REG_T1, prec.sign_mask() as i64);
        self.code.alu_rr("or", REG_T0, REG_T0, REG_T1);
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
    }

    /// Rounds to an integral value in rounding mode `rm` via `fcvt.l.<p>` + `fcvt.<p>.l`.  Values
    /// with `|x| >= 2^precision` (all integral), infinities and NaNs pass through unchanged; the
    /// sign of `x` is re-applied so that e.g. `ceil(-0.5)` yields `-0`.
    fn f_round(&mut self, prec: Prec, rm: u32) {
        let a = self.slot(self.depth - 1);
        let done = self.code.new_label();
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.alu_ri("and", REG_T1, REG_T0, prec.abs_mask() as i64);
        // |x| >= 2^p (includes inf/NaN): leave the operand as is.
        self.code.cmp_imm_branch("ltu", REG_T1, prec.integral_threshold() as i64, done, false);
        freg_from_slot(&mut self.code, FA, a, prec);
        // x5 = (int64) round(fA);  fR = (float) x5
        call_handler(&mut self.code, op_fp(0x60 + prec.fmt(), 2, FA, rm, XT));
        freg_from_int(&mut self.code, FA);
        call_handler(&mut self.code, op_fp(0x68 + prec.fmt(), 2, XT, RNE, FR));
        // Re-apply the sign of the input (only matters when the result is zero).
        self.code.load_abs_to_reg(REG_T0, freg_addr(FR));
        self.code.load_slot_to_reg(REG_T1, a);
        self.code.alu_ri("and", REG_T1, REG_T1, prec.sign_mask() as i64);
        self.code.alu_rr("or", REG_T0, REG_T0, REG_T1);
        self.code.store_reg_to_slot(a, REG_T0);
        self.code.bind(done);
    }

    /// `feq` (`rm` = 2) / `flt` (1) / `fle` (0); `swap` evaluates `b <op> a`, `invert` negates.
    fn f_compare(&mut self, prec: Prec, rm: u32, swap: bool, invert: bool) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        freg_from_slot(&mut self.code, FA, a, prec);
        freg_from_slot(&mut self.code, FB, b, prec);
        let (rs1, rs2) = if swap { (FB, FA) } else { (FA, FB) };
        call_handler(&mut self.code, op_fp(0x50 + prec.fmt(), rs2, rs1, rm, XT));
        if invert {
            self.code.alu_ri("xor", REG_T0, REG_T0, 1);
        }
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
    }

    /// Integer -> float.  i32 operands are widened first (they are canonical sign-extended, so an
    /// unsigned one just needs masking), then converted with the 64-bit `fcvt.<p>.l[u]`.  The
    /// widening is exact, so the library sees the same integer the 32-bit `fcvt.<p>.w[u]` would
    /// and performs the same single correctly-rounded conversion — for an f32 target that rounding
    /// is inherent (e.g. 16777217 -> 16777216), not an artifact of the widening.
    fn f_from_int(&mut self, prec: Prec, ty: IntTy, signed: bool) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        if ty == IntTy::I32 && !signed {
            self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        }
        freg_from_int(&mut self.code, FA);
        let rs2 = if signed { 2 } else { 3 };
        call_handler(&mut self.code, op_fp(0x68 + prec.fmt(), rs2, XT, RNE, FR));
        slot_from_freg(&mut self.code, a, FR);
    }

    /// Float -> integer, truncating.  The library's `fcvt.<w|l>[u].<p>` already saturates, so
    /// only the NaN case (wasm: 0) needs fixing for the `_sat` forms; the trapping forms check the
    /// domain on the integer side first (see `trunc_domain`).
    fn int_from_f(&mut self, prec: Prec, ty: IntTy, signed: bool, trapping: bool) {
        let a = self.slot(self.depth - 1);
        let done = self.code.new_label();
        self.code.load_slot_to_reg(REG_T0, a);
        if prec == Prec::S {
            self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        }
        self.code.alu_ri("and", REG_T1, REG_T0, prec.abs_mask() as i64);
        if trapping {
            self.trunc_domain(prec, ty, signed);
        } else {
            // NaN -> 0.
            let convert = self.code.new_label();
            self.code.cmp_imm_branch("ltu", REG_T1, prec.inf_bits() as i64 + 1, convert, true);
            self.code.store_imm_to_slot(a, 0);
            self.code.jump(done);
            self.code.bind(convert);
        }
        freg_from_slot(&mut self.code, FA, a, prec);
        let rs2 = match (ty, signed) {
            (IntTy::I32, true) => 0,
            (IntTy::I32, false) => 1,
            (IntTy::I64, true) => 2,
            (IntTy::I64, false) => 3,
        };
        call_handler(&mut self.code, op_fp(0x60 + prec.fmt(), rs2, FA, RTZ, XT));
        if ty == IntTy::I32 {
            self.code.signextend_w_reg(REG_T0);
        }
        self.code.store_reg_to_slot(a, REG_T0);
        self.code.bind(done);
    }

    /// Traps unless the value in `REG_T0` (its bits, masked to the precision's width; `|x|` in
    /// `REG_T1`) truncates into the target
    /// range.  Signed: `-2^n - 1 < x < 2^n` (with `n` the width minus one); unsigned:
    /// `-1 < x < 2^n`.  NaN fails every `<` test.  The lower bound `-2^n - 1` is only
    /// representable as f64 for i32; where it is not, no float lies strictly between it and
    /// `-2^n`, so the test becomes `x >= -2^n`, i.e. `|x| < 2^n || x == -2^n`.
    fn trunc_domain(&mut self, prec: Prec, ty: IntTy, signed: bool) {
        let n = match ty {
            IntTy::I32 => 31,
            IntTy::I64 => 63,
        };
        let ok = self.code.new_label();
        if signed {
            // Positive side or |x| < 2^n (either sign).
            self.code.cmp_imm_branch("ltu", REG_T1, prec.pow2_bits(n) as i64, ok, true);
            if prec == Prec::D && ty == IntTy::I32 {
                // Negative and |x| < 2^31 + 1 (2147483649.0 is exact in f64).
                let lo = 0x41E0_0000_0020_0000u64; // 2147483649.0
                let neg = self.code.new_label();
                self.code.cmp_imm_branch("ltu", REG_T0, Prec::D.sign_mask() as i64, neg, true);
                self.code.cmp_imm_branch("ltu", REG_T1, lo as i64, ok, true);
                self.code.bind(neg);
            } else {
                let min = prec.sign_mask() | prec.pow2_bits(n);
                self.code.cmp_imm_branch("eq", REG_T0, min as i64, ok, true);
            }
        } else {
            let neg = self.code.new_label();
            self.code.cmp_imm_branch("ltu", REG_T0, prec.sign_mask() as i64, neg, false);
            // Positive: x < 2^(n+1).
            self.code.cmp_imm_branch("ltu", REG_T1, prec.pow2_bits(n + 1) as i64, ok, true);
            self.emit_trap();
            self.code.bind(neg);
            // Negative: |x| < 1.
            self.code.cmp_imm_branch("ltu", REG_T1, prec.pow2_bits(0) as i64, ok, true);
        }
        self.emit_trap();
        self.code.bind(ok);
    }
}
