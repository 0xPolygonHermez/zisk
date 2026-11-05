//! Lowering of a single wasm function body to Zisk instructions.
//!
//! The wasm operand stack is statically-depth-addressed relative to the frame pointer `REG_FP`
//! (wasm validation guarantees the depth is known at every instruction), so there is no runtime
//! stack pointer.  i32 values are kept sign-extended in their 64-bit slots (the RISC-V "W"
//! convention), which lets i32 ops map directly onto Zisk's `_w` opcodes.
//!
//! Control flow is resolved in two passes: this module emits PC-relative jumps and calls with
//! symbolic [`Fixup`]s, and `mod.rs` patches them once every function has an address.

use std::error::Error;

use super::emit::{Code, Fixup, LabelId};
use super::layout::*;
use super::module::{FuncSig, WasmModule};
use wasmparser::{BlockType, Operator};
use zisk_core::{ZiskInstBuilder, EXTRA_PARAMS_ADDR};

/// Worst-case operand-stack depth a function may reach.  Frames reserve this many operand slots so
/// callee frame placement is a compile-time constant (see `frame_size`).
pub const OPERAND_CAP: u32 = 1024;

/// Per-function frame size in bytes (header + locals + reserved operand slots).
pub fn func_frame_size(num_locals: u32) -> i64 {
    frame_size(num_locals, OPERAND_CAP)
}

pub(crate) fn callee_frame_size(
    module: &WasmModule,
    func_index: u32,
) -> Result<i64, Box<dyn Error>> {
    let num_locals = module.num_locals(func_index)?;
    Ok(if func_index < module.func_import_count {
        frame_size(num_locals, 0)
    } else {
        func_frame_size(num_locals)
    })
}

/// Byte count of a bounds-checked linear-memory access.
#[derive(Clone, Copy)]
enum Len {
    Imm(u64),
    Reg(u64),
}

#[derive(Clone, Copy, PartialEq)]
enum CtrlKind {
    Block,
    Loop,
    If,
}

struct CtrlFrame {
    kind: CtrlKind,
    /// Branch target for `br`: the loop head (backward) or the block/if end (forward).
    branch_label: LabelId,
    /// The end label, bound at `End`.
    end_label: LabelId,
    /// For `if`: where control jumps when the condition is false (an `else` or the end).
    else_label: Option<LabelId>,
    /// Operand depth when the block was entered.
    start_depth: u32,
    /// Number of result values the block leaves on the stack.
    result_arity: u32,
}

/// Where an operand-stack value is at compile time.  Values are only written to their operand
/// slot when something needs them there (see [`FuncGen::flush`]): a `local.get` or a constant
/// is read straight from the local or the instruction, and an operator's result stays in
/// `REG_ACC` for the next operator.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Val {
    /// In its own operand slot.
    Slot,
    /// The current value of local `k`, not copied yet.
    Local(u32),
    /// A constant, not stored yet.
    Const(i64),
    /// In `REG_ACC`.  At most one value is, counting the operands of a pending `Cmp`.
    Acc,
    /// A comparison not computed yet, so that `br_if` and `if` can branch on its flag.  Only
    /// ever the top of the stack: any operator other than `br_if`, `if` and `eqz` computes it
    /// first, so the locals it reads cannot change under it.
    Cmp(Cmp),
}

/// A pending comparison: `op(a, b)`, negated when `invert`.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Cmp {
    op: &'static str,
    a: Opnd,
    b: Opnd,
    invert: bool,
}

/// Where to read a popped operand.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Opnd {
    /// A frame slot (an operand slot or a local), as an offset from FP.
    Slot(i64),
    Const(i64),
    Acc,
}

/// Register `reg` holds the absolute address of local `local` (a wasm address) plus `off0`, and
/// `[reg, reg + hi)` has been checked to lie inside linear memory.  An access through the same
/// local at static offset `off` is to `reg + (off - off0)`, already checked when it ends by `hi`.
/// Valid until the local is written or control reaches a label; linear memory never shrinks, so
/// a check stays true.
#[derive(Clone, Copy)]
struct Ptr {
    local: u32,
    reg: u64,
    off0: u64,
    hi: i64,
}

/// Lowers `func_index` (which must be a defined function) into a [`Code`].
pub fn lower_function(module: &WasmModule, func_index: u32) -> Result<Code, Box<dyn Error>> {
    let defined_index = (func_index - module.func_import_count) as usize;
    let func = &module.defined[defined_index];
    let sig = &module.sigs[func.type_index as usize];

    let num_locals = func.num_locals;

    let mut code = Code::new();
    let func_end = code.new_label();
    let mut gen = FuncGen {
        module,
        sig,
        num_locals,
        code,
        depth: 0,
        ctrl: Vec::new(),
        unreachable: false,
        dead_nesting: 0,
        func_end,
        overflowed: false,
        vals: Vec::new(),
        ptrs: Vec::new(),
        next_ptr: 0,
    };

    // Prologue: save the return address and zero the non-parameter locals.
    gen.code.store_reg_to_slot(FRAME_RET_PC_OFF, REG_RA);
    for k in sig.params.len() as u32..num_locals {
        gen.code.store_imm_to_slot(local_offset(k), 0);
    }

    let mut op_reader = func.body.get_operators_reader()?;
    while !op_reader.eof() {
        let op = op_reader.read()?;
        gen.lower_op(op, func_end)?;
        if gen.overflowed {
            return Err(format!(
                "wasm: operand stack deeper than the {OPERAND_CAP} slots a frame reserves"
            )
            .into());
        }
    }

    // Fallthrough off the end of the body: the result is on top of the operand stack.  Every
    // other path to `func_end` (explicit or implicit `return`) has already loaded `REG_RET`.
    if !gen.unreachable {
        gen.sync();
        gen.flush();
    }
    if !gen.unreachable && !sig.results.is_empty() {
        let off = gen.slot(gen.depth - 1);
        gen.code.load_slot_to_reg(REG_RET, off);
    }
    // Epilogue (bound at func_end): restore caller FP and return.
    gen.code.bind(func_end);
    gen.emit_epilogue();

    Ok(gen.code)
}

pub(crate) struct FuncGen<'a, 'b> {
    module: &'b WasmModule<'a>,
    sig: &'b FuncSig,
    num_locals: u32,
    pub(crate) code: Code,
    pub(crate) depth: u32,
    ctrl: Vec<CtrlFrame>,
    unreachable: bool,
    /// Number of fully-dead nested blocks while `unreachable` (see module docs).
    dead_nesting: u32,
    /// Function-level end label: the target of `return`, of branches whose depth reaches past the
    /// outermost block (the function body is itself a label), and of the implicit fallthrough.
    func_end: LabelId,
    /// Set by `push` when the operand stack outgrows the frame's reserved slots.
    overflowed: bool,
    /// Where each operand-stack value is, bottom first.  Code that works on the slots directly
    /// runs after [`FuncGen::flush`] and only moves `depth`; [`FuncGen::sync`] then catches up.
    vals: Vec<Val>,
    /// The addresses held in `REG_PTRS` registers.
    ptrs: Vec<Ptr>,
    /// The `REG_PTRS` register the next new address goes to.
    next_ptr: usize,
}

impl<'a, 'b> FuncGen<'a, 'b> {
    /// Byte offset (from FP) of operand-stack slot at depth `d`.
    pub(crate) fn slot(&self, d: u32) -> i64 {
        operand_offset(self.num_locals, d)
    }

    pub(crate) fn push(&mut self) -> i64 {
        let off = self.slot(self.depth);
        self.depth += 1;
        if self.depth > OPERAND_CAP {
            self.overflowed = true;
        }
        off
    }

    // -- the virtual operand stack ------------------------------------------------

    /// Catches `vals` up with `depth` after code that works on the slots: such code runs after a
    /// [`Self::flush`], so every value it leaves is in its slot.
    pub(crate) fn sync(&mut self) {
        self.vals.resize(self.depth as usize, Val::Slot);
    }

    fn push_val(&mut self, v: Val) {
        self.push();
        self.vals.push(v);
    }

    fn pop_val(&mut self) -> Val {
        self.depth -= 1;
        self.vals.pop().expect("wasm: operand stack underflow")
    }

    /// Where to read value `v`, which was at depth `d`.
    fn opnd_at(&self, v: Val, d: u32) -> Opnd {
        match v {
            Val::Slot => Opnd::Slot(self.slot(d)),
            Val::Local(k) => Opnd::Slot(local_offset(k)),
            Val::Const(c) => Opnd::Const(c),
            Val::Acc => Opnd::Acc,
            Val::Cmp(_) => unreachable!("a comparison is computed before its value is read"),
        }
    }

    fn pop(&mut self) -> Opnd {
        let v = self.pop_val();
        self.opnd_at(v, self.depth)
    }

    /// The register holding `o`, loading it into `scratch` unless it is already in `REG_ACC`.
    fn opnd_reg(&mut self, o: Opnd, scratch: u64) -> u64 {
        match o {
            Opnd::Slot(off) => self.code.load_slot_to_reg(scratch, off),
            Opnd::Const(c) => self.code.load_imm_to_reg(scratch, c as u64),
            Opnd::Acc => return REG_ACC,
        }
        scratch
    }

    /// Stores the value in `REG_ACC`, if any, to its slot, so that `REG_ACC` can take another.
    fn spill_acc(&mut self) {
        if let Some(i) = self.vals.iter().position(|v| *v == Val::Acc) {
            self.code.store_reg_to_slot(self.slot(i as u32), REG_ACC);
            self.vals[i] = Val::Slot;
        }
    }

    /// Copies every pending `local.get k` to its slot, before local `k` changes.
    fn materialize_local(&mut self, k: u32) {
        for i in 0..self.vals.len() {
            if self.vals[i] == Val::Local(k) {
                self.code.copy_slot(self.slot(i as u32), local_offset(k));
                self.vals[i] = Val::Slot;
            }
        }
    }

    /// Computes a pending comparison on top of the stack into `REG_ACC`.
    fn compute_cmp(&mut self) {
        let Some(&Val::Cmp(c)) = self.vals.last() else {
            return;
        };
        self.vals.pop();
        self.spill_acc();
        let ra = self.opnd_reg(c.a, REG_T0);
        match c.b {
            Opnd::Const(k) => self.code.alu_ri(c.op, REG_ACC, ra, k),
            b => {
                let rb = self.opnd_reg(b, REG_T1);
                self.code.alu_rr(c.op, REG_ACC, ra, rb);
            }
        }
        if c.invert {
            self.code.alu_ri("xor", REG_ACC, REG_ACC, 1);
        }
        self.vals.push(Val::Acc);
    }

    /// Writes every value to its operand slot, as the slot-based code, labels, branches and
    /// calls expect.  It only stores to memory, so registers survive it.
    pub(crate) fn flush(&mut self) {
        self.compute_cmp();
        for i in 0..self.vals.len() {
            let off = self.slot(i as u32);
            match self.vals[i] {
                Val::Slot => continue,
                Val::Local(k) => self.code.copy_slot(off, local_offset(k)),
                Val::Const(c) => self.code.store_imm_to_slot(off, c),
                Val::Acc => self.code.store_reg_to_slot(off, REG_ACC),
                Val::Cmp(_) => unreachable!("only the top of the stack can be a comparison"),
            }
            self.vals[i] = Val::Slot;
        }
    }

    /// Lowers the operators that work on the virtual stack; returns `false` for the others.
    fn lower_virtual(&mut self, op: &Operator) -> Result<bool, Box<dyn Error>> {
        use Operator::*;
        match *op {
            // -- constants and variables ----------------------------------------
            I32Const { value } => self.push_val(Val::Const(value as i64)),
            I64Const { value } => self.push_val(Val::Const(value)),
            LocalGet { local_index } => self.push_val(Val::Local(local_index)),
            LocalSet { local_index } => {
                let v = self.pop();
                self.set_local(local_index, v);
            }
            LocalTee { local_index } => {
                let top = *self.vals.last().expect("wasm: operand stack underflow");
                let v = self.pop();
                self.set_local(local_index, v);
                self.push_val(top);
            }
            GlobalGet { global_index } => {
                self.spill_acc();
                self.code.load_abs_to_reg(REG_ACC, global_addr(global_index));
                self.push_val(Val::Acc);
            }
            GlobalSet { global_index } => match self.pop() {
                Opnd::Const(c) => self.code.store_imm_to_abs(global_addr(global_index), c as u64),
                v => {
                    let r = self.opnd_reg(v, REG_T0);
                    self.code.store_reg_to_abs(global_addr(global_index), r, 8);
                }
            },

            // -- integer arithmetic and logic -----------------------------------
            I32Add => self.alu("add_w", true),
            I32Sub => self.alu("sub_w", false),
            I32Mul => self.alu("mul_w", true),
            I64Add => self.alu("add", true),
            I64Sub => self.alu("sub", false),
            I64Mul => self.alu("mul", true),
            I32And | I64And => self.alu("and", true),
            I32Or | I64Or => self.alu("or", true),
            I32Xor | I64Xor => self.alu("xor", true),
            I32Shl => self.shift("sll_w", 0x1f),
            I32ShrU => self.shift("srl_w", 0x1f),
            I32ShrS => self.shift("sra_w", 0x1f),
            I64Shl => self.shift("sll", 0x3f),
            I64ShrU => self.shift("srl", 0x3f),
            I64ShrS => self.shift("sra", 0x3f),
            // ZisK's rotations take the count modulo the width, and the `_w` forms rotate the low
            // 32 bits and sign-extend the result, which is the canonical i32 form.
            I32Rotl => self.alu("rol_w", false),
            I32Rotr => self.alu("ror_w", false),
            I64Rotl => self.alu("rol", false),
            I64Rotr => self.alu("ror", false),

            // -- comparisons (0/1, or a branch on the flag) ---------------------
            I32Eqz | I64Eqz => self.eqz(),
            I32Eq | I64Eq => self.compare("eq", false, false),
            I32Ne | I64Ne => self.compare("eq", false, true),
            I32LtS | I64LtS => self.compare("lt", false, false),
            I32LtU | I64LtU => self.compare("ltu", false, false),
            I32GtS | I64GtS => self.compare("lt", true, false),
            I32GtU | I64GtU => self.compare("ltu", true, false),
            I32LeS | I64LeS => self.compare("le", false, false),
            I32LeU | I64LeU => self.compare("leu", false, false),
            I32GeS | I64GeS => self.compare("le", true, false),
            I32GeU | I64GeU => self.compare("leu", true, false),

            // -- conversions and sign extension ---------------------------------
            I32WrapI64 | I64Extend32S => self.sign_extend("signextend_w"),
            I32Extend8S | I64Extend8S => self.sign_extend("signextend_b"),
            I32Extend16S | I64Extend16S => self.sign_extend("signextend_h"),
            I64ExtendI32S => { /* canonical i32 already sign-extended: no-op */ }
            I64ExtendI32U => {
                self.push_val(Val::Const(0xFFFF_FFFF));
                self.alu("and", true);
            }

            // -- memory loads and stores ----------------------------------------
            I32Load { memarg } => self.load("signextend_w", 4, memarg.offset),
            I64Load { memarg } => self.load("copyb", 8, memarg.offset),
            I32Load8S { memarg } | I64Load8S { memarg } => {
                self.load("signextend_b", 1, memarg.offset)
            }
            I32Load8U { memarg } | I64Load8U { memarg } => self.load("copyb", 1, memarg.offset),
            I32Load16S { memarg } | I64Load16S { memarg } => {
                self.load("signextend_h", 2, memarg.offset)
            }
            I32Load16U { memarg } | I64Load16U { memarg } => self.load("copyb", 2, memarg.offset),
            I64Load32S { memarg } => self.load("signextend_w", 4, memarg.offset),
            I64Load32U { memarg } => self.load("copyb", 4, memarg.offset),
            I32Store { memarg } => self.store(4, memarg.offset),
            I64Store { memarg } => self.store(8, memarg.offset),
            I32Store8 { memarg } | I64Store8 { memarg } => self.store(1, memarg.offset),
            I32Store16 { memarg } | I64Store16 { memarg } => self.store(2, memarg.offset),
            I64Store32 { memarg } => self.store(4, memarg.offset),

            // -- parametric and branches ----------------------------------------
            Drop => {
                self.pop_val();
            }
            Nop => {}
            BrIf { relative_depth } => self.br_if(relative_depth),
            If { blockty } => self.if_(blockty)?,
            BrTable { ref targets } => self.br_table(targets)?,

            _ => return Ok(false),
        }
        Ok(true)
    }

    /// `local.set k`: pending reads of local `k` get the old value first, and an address made
    /// from it is forgotten.
    fn set_local(&mut self, k: u32, v: Opnd) {
        self.materialize_local(k);
        self.ptrs.retain(|p| p.local != k);
        let dst = local_offset(k);
        match v {
            Opnd::Slot(off) if off == dst => {}
            Opnd::Slot(off) => self.code.copy_slot(dst, off),
            Opnd::Const(c) => self.code.store_imm_to_slot(dst, c),
            Opnd::Acc => self.code.store_reg_to_slot(dst, REG_ACC),
        }
    }

    /// `REG_ACC = op(a, b)`; a constant operand becomes the instruction's immediate.
    fn alu(&mut self, op: &'static str, commutative: bool) {
        let mut b = self.pop();
        let mut a = self.pop();
        if commutative && matches!(a, Opnd::Const(_)) && !matches!(b, Opnd::Const(_)) {
            std::mem::swap(&mut a, &mut b);
        }
        self.spill_acc();
        let ra = self.opnd_reg(a, REG_T0);
        match b {
            Opnd::Const(c) => self.code.alu_ri(op, REG_ACC, ra, c),
            _ => {
                let rb = self.opnd_reg(b, REG_T1);
                self.code.alu_rr(op, REG_ACC, ra, rb);
            }
        }
        self.push_val(Val::Acc);
    }

    /// A shift: wasm takes the count modulo the width (`count_mask`).
    fn shift(&mut self, op: &'static str, count_mask: i64) {
        let b = self.pop();
        let a = self.pop();
        self.spill_acc();
        let ra = self.opnd_reg(a, REG_T0);
        match b {
            Opnd::Const(c) => self.code.alu_ri(op, REG_ACC, ra, c & count_mask),
            _ => {
                let rb = self.opnd_reg(b, REG_T1);
                self.code.alu_ri("and", REG_T1, rb, count_mask);
                self.code.alu_rr(op, REG_ACC, ra, REG_T1);
            }
        }
        self.push_val(Val::Acc);
    }

    /// Pushes the comparison `op(a, b)` (operands swapped when `swap`, negated when `invert`)
    /// without computing it.
    fn compare(&mut self, op: &'static str, swap: bool, invert: bool) {
        let b = self.pop();
        let a = self.pop();
        let (mut a, mut b) = if swap { (b, a) } else { (a, b) };
        if op == "eq" && matches!(a, Opnd::Const(_)) {
            std::mem::swap(&mut a, &mut b);
        }
        self.push_val(Val::Cmp(Cmp { op, a, b, invert }));
    }

    /// `eqz`: negates a pending comparison, or compares with zero.
    fn eqz(&mut self) {
        if let Some(Val::Cmp(c)) = self.vals.last_mut() {
            c.invert = !c.invert;
            return;
        }
        let a = self.pop();
        self.push_val(Val::Cmp(Cmp { op: "eq", a, b: Opnd::Const(0), invert: false }));
    }

    /// `signextend_{b,h,w}`, folded for a constant.
    pub(crate) fn sign_extend(&mut self, op: &'static str) {
        let a = self.pop();
        if let Opnd::Const(c) = a {
            let v = match op {
                "signextend_b" => c as i8 as i64,
                "signextend_h" => c as i16 as i64,
                _ => c as i32 as i64,
            };
            self.push_val(Val::Const(v));
            return;
        }
        self.spill_acc();
        let ra = self.opnd_reg(a, REG_T0);
        // signextend acts on `b`.
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", ra, false);
        zib.op(op).unwrap();
        zib.store("reg", REG_ACC as i64, false, false);
        zib.j(4, 4);
        self.code.push_raw(zib, Fixup::None);
        self.push_val(Val::Acc);
    }

    /// Jumps to `label` when the popped condition `cond` (which was at depth `self.depth`) is
    /// nonzero (`when`) or zero (`!when`).  A pending comparison branches on its own flag.
    fn branch_on(&mut self, cond: Val, label: LabelId, when: bool) {
        if let Val::Cmp(c) = cond {
            let jump_if_flag = when != c.invert;
            let ra = self.opnd_reg(c.a, REG_T0);
            match c.b {
                Opnd::Const(k) => self.code.cmp_imm_branch(c.op, ra, k, label, jump_if_flag),
                b => {
                    let rb = self.opnd_reg(b, REG_T1);
                    self.code.cmp_reg_branch(c.op, ra, rb, label, jump_if_flag);
                }
            }
            return;
        }
        match self.opnd_at(cond, self.depth) {
            Opnd::Const(c) => {
                if (c != 0) == when {
                    self.code.jump(label);
                }
            }
            o => {
                let r = self.opnd_reg(o, REG_T0);
                self.code.cmp_imm_branch("eq", r, 0, label, !when);
            }
        }
    }

    /// The label a branch to `relative_depth` can jump to directly, when it moves no value.
    fn plain_branch_target(&self, relative_depth: u32) -> Option<LabelId> {
        if relative_depth as usize == self.ctrl.len() {
            return None; // a return
        }
        let frame = &self.ctrl[self.ctrl.len() - 1 - relative_depth as usize];
        (frame.kind == CtrlKind::Loop || frame.result_arity == 0).then_some(frame.branch_label)
    }

    fn br_if(&mut self, relative_depth: u32) {
        let cond = self.pop_val();
        self.flush();
        if let Some(label) = self.plain_branch_target(relative_depth) {
            self.branch_on(cond, label, true);
        } else {
            let cont = self.code.new_label();
            self.branch_on(cond, cont, false);
            self.branch_to(relative_depth);
            self.code.bind(cont);
        }
    }

    fn if_(&mut self, blockty: BlockType) -> Result<(), Box<dyn Error>> {
        let (_p, r) = self.block_arity(blockty)?;
        let else_label = self.code.new_label();
        let end_label = self.code.new_label();
        // Pop the condition; when zero, jump to the else arm (or the end, if there is none).
        let cond = self.pop_val();
        self.flush();
        self.branch_on(cond, else_label, false);
        self.ctrl.push(CtrlFrame {
            kind: CtrlKind::If,
            branch_label: end_label,
            end_label,
            else_label: Some(else_label),
            start_depth: self.depth,
            result_arity: r,
        });
        Ok(())
    }

    fn block_arity(&self, blockty: BlockType) -> Result<(u32, u32), Box<dyn Error>> {
        match blockty {
            BlockType::Empty => Ok((0, 0)),
            BlockType::Type(_) => Ok((0, 1)),
            BlockType::FuncType(idx) => {
                let s = &self.module.sigs[idx as usize];
                if !s.params.is_empty() {
                    return Err("wasm: multi-value blocks with parameters are not supported".into());
                }
                if s.results.len() > 1 {
                    return Err("wasm: multi-value block results are not supported".into());
                }
                Ok((0, s.results.len() as u32))
            }
        }
    }

    fn emit_epilogue(&mut self) {
        // Restore caller FP and jump to the saved return address.
        self.code.load_slot_to_reg(REG_T0, FRAME_RET_PC_OFF);
        self.code.load_slot_to_reg(REG_T2, FRAME_CALLER_FP_OFF);
        // FP <- caller FP
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T2, false);
        zib.op("copyb").unwrap();
        zib.store("reg", REG_FP as i64, false, false);
        zib.j(4, 4);

        self.code.push_raw(zib, Fixup::None);
        // pc <- saved return address
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T0, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.j(0, 4);
        // Profiling metadata: a return through t0, like a RISC-V `jalr x0, t0`
        zib.set_meta_rs1_rd(REG_T0 as u8, 0);

        self.code.push_raw(zib, Fixup::None);
    }

    fn lower_op(&mut self, op: Operator, func_end: LabelId) -> Result<(), Box<dyn Error>> {
        use Operator::*;

        // Dead-code skipping: only structural delimiters are processed while unreachable.
        if self.unreachable {
            self.forget_ptrs(); // `else` and `end` bind labels
            match op {
                Block { .. } | Loop { .. } | If { .. } => {
                    self.dead_nesting += 1;
                    return Ok(());
                }
                Else => {
                    if self.dead_nesting > 0 {
                        return Ok(());
                    }
                    // Reactivate the else arm of the current `if`.
                    let frame = self.ctrl.last().unwrap();
                    let else_label = frame.else_label.expect("else without if");
                    let start_depth = frame.start_depth;
                    self.code.bind(else_label);
                    self.depth = start_depth;
                    self.unreachable = false;
                    return Ok(());
                }
                End => {
                    if self.dead_nesting > 0 {
                        self.dead_nesting -= 1;
                        return Ok(());
                    }
                    // The function-level End: nothing follows, the body stays unreachable (the
                    // epilogue must not try to read a result that was never pushed).
                    if self.ctrl.is_empty() {
                        return Ok(());
                    }
                    // Closing the block that was open when we became unreachable: the code that
                    // follows it is reachable again (it is a branch/merge target).  Conservatively
                    // restoring reachability is safe — at worst it emits some dead instructions,
                    // whereas leaving it set would wrongly skip live code after the block.
                    self.lower_end(func_end)?;
                    self.unreachable = false;
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }

        // Bring `vals` up to date with whatever the slot-based code did to `depth`, and compute
        // a pending comparison unless this operator branches on it or negates it.
        self.sync();
        if !matches!(op, BrIf { .. } | If { .. } | I32Eqz | I64Eqz) {
            self.compute_cmp();
        }
        if self.lower_virtual(&op)? {
            return Ok(());
        }
        // Everything below works on the operand slots, and may bind a label or call.
        self.flush();
        self.forget_ptrs();

        match op {
            // -- integer division (trap guards) -------------------------------
            I32DivU => self.binop_div("divu_w", false)?,
            I32DivS => self.binop_div("div_w", false)?,
            I32RemU => self.binop_div("remu_w", false)?,
            I32RemS => self.binop_div("rem_w", false)?,
            I64DivU => self.binop_div("divu", false)?,
            I64DivS => self.binop_div("div", false)?,
            I64RemU => self.binop_div("remu", false)?,
            I64RemS => self.binop_div("rem", false)?,

            // wide arithmetic (128-bit values as two i64 halves, low half first)
            I64MulWideU => self.mul_wide("muluh"),
            I64MulWideS => self.mul_wide("mulh"),
            I64Add128 => self.add_sub128(true),
            I64Sub128 => self.add_sub128(false),

            // -- bit counting (runtime loops) ---------------------------------
            I32Popcnt => self.popcnt(true),
            I64Popcnt => self.popcnt(false),
            I32Ctz => self.ctz(true),
            I64Ctz => self.ctz(false),
            I32Clz => self.clz(true),
            I64Clz => self.clz(false),

            // -- memory -------------------------------------------------------
            MemorySize { .. } => {
                let dst = self.push();
                self.code.load_abs_to_reg(REG_T0, WASM_MEM_PAGES_ADDR);
                self.code.store_reg_to_slot(dst, REG_T0);
            }
            MemoryGrow { .. } => self.memory_grow(),
            MemoryCopy { .. } => self.memory_copy(),
            MemoryFill { .. } => self.memory_fill(),

            // -- parametric ---------------------------------------------------
            Select => self.select(),
            Unreachable => {
                self.emit_trap();
                self.unreachable = true;
            }

            // -- control flow -------------------------------------------------
            Block { blockty } => {
                let (_p, r) = self.block_arity(blockty)?;
                let end_label = self.code.new_label();
                self.ctrl.push(CtrlFrame {
                    kind: CtrlKind::Block,
                    branch_label: end_label,
                    end_label,
                    else_label: None,
                    start_depth: self.depth,
                    result_arity: r,
                });
            }
            Loop { blockty } => {
                let (_p, r) = self.block_arity(blockty)?;
                let head = self.code.new_label();
                let end_label = self.code.new_label();
                self.code.bind(head);
                self.ctrl.push(CtrlFrame {
                    kind: CtrlKind::Loop,
                    branch_label: head,
                    end_label,
                    else_label: None,
                    start_depth: self.depth,
                    result_arity: r,
                });
            }
            Else => {
                let frame = self.ctrl.last().unwrap();
                let end_label = frame.end_label;
                let else_label = frame.else_label.expect("else without if");
                let start_depth = frame.start_depth;
                // End of the `then` arm jumps over the `else` arm.
                self.code.jump(end_label);
                self.code.bind(else_label);
                self.depth = start_depth;
            }
            End => self.lower_end(func_end)?,
            Br { relative_depth } => {
                self.branch_to(relative_depth);
                self.unreachable = true;
            }
            Return => {
                self.emit_return();
                self.unreachable = true;
            }
            Call { function_index } => self.call(function_index)?,
            CallIndirect { type_index, .. } => self.call_indirect(type_index)?,

            other => {
                #[cfg(feature = "float")]
                if self.lower_float_op(&other)? {
                    return Ok(());
                }
                return Err(format!("wasm: unsupported operator: {other:?}").into());
            }
        }
        Ok(())
    }

    fn lower_end(&mut self, _func_end: LabelId) -> Result<(), Box<dyn Error>> {
        if let Some(frame) = self.ctrl.pop() {
            self.code.bind(frame.end_label);
            // If this is an `if` with no `else`, its else_label also lands here.
            if frame.kind == CtrlKind::If {
                if let Some(else_label) = frame.else_label {
                    // Bind else only if it was never bound by an explicit Else.
                    if self.code.label_is_unbound(else_label) {
                        self.code.bind(else_label);
                    }
                }
            }
            self.depth = frame.start_depth + frame.result_arity;
        }
        // The function-level End (empty ctrl stack) just falls into the epilogue.
        Ok(())
    }

    // -- operator helpers ----------------------------------------------------

    /// `i64.mul_wide_{u,s}`: [a, b] -> [low, high] of the 128-bit product, `high` from ZisK's
    /// `muluh` (unsigned) or `mulh` (signed).
    fn mul_wide(&mut self, high_op: &str) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.load_slot_to_reg(REG_T1, b);
        self.code.alu_rr("mul", REG_T2, REG_T0, REG_T1);
        self.code.alu_rr(high_op, REG_T3, REG_T0, REG_T1);
        self.code.store_reg_to_slot(a, REG_T2);
        self.code.store_reg_to_slot(b, REG_T3);
    }

    /// `i64.add128` / `i64.sub128`: [a_lo, a_hi, b_lo, b_hi] -> [lo, hi], the carry (or borrow)
    /// out of the low halves taken with `ltu`.
    fn add_sub128(&mut self, add: bool) {
        let a_lo = self.slot(self.depth - 4);
        let a_hi = self.slot(self.depth - 3);
        let b_lo = self.slot(self.depth - 2);
        let b_hi = self.slot(self.depth - 1);
        let op = if add { "add" } else { "sub" };
        self.code.load_slot_to_reg(REG_T0, a_lo);
        self.code.load_slot_to_reg(REG_T1, b_lo);
        self.code.alu_rr(op, REG_T2, REG_T0, REG_T1);
        if add {
            self.code.alu_rr("ltu", REG_T3, REG_T2, REG_T0); // lo < a_lo: carry
        } else {
            self.code.alu_rr("ltu", REG_T3, REG_T0, REG_T1); // a_lo < b_lo: borrow
        }
        self.code.store_reg_to_slot(a_lo, REG_T2);
        self.code.load_slot_to_reg(REG_T0, a_hi);
        self.code.load_slot_to_reg(REG_T1, b_hi);
        self.code.alu_rr(op, REG_T0, REG_T0, REG_T1);
        self.code.alu_rr(op, REG_T0, REG_T0, REG_T3);
        self.code.store_reg_to_slot(a_hi, REG_T0);
        self.depth -= 2;
    }

    /// Division/remainder with a wasm divide-by-zero trap guard.
    fn binop_div(&mut self, op: &str, swap: bool) -> Result<(), Box<dyn Error>> {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.load_slot_to_reg(REG_T1, b);
        // Trap if divisor (REG_T1) == 0.
        let ok = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T1, 0, ok, false); // jump to ok when divisor != 0
        self.emit_trap();
        self.code.bind(ok);
        let signed_min = match op {
            "div_w" => Some(i32::MIN as i64),
            "div" => Some(i64::MIN),
            _ => None,
        };
        if let Some(min) = signed_min {
            let ok = self.code.new_label();
            self.code.cmp_imm_branch("eq", REG_T1, -1, ok, false); // divisor != -1
            self.code.cmp_imm_branch("eq", REG_T0, min, ok, false); // dividend != MIN
            self.emit_trap();
            self.code.bind(ok);
        }
        if swap {
            self.code.alu_rr(op, REG_T0, REG_T1, REG_T0);
        } else {
            self.code.alu_rr(op, REG_T0, REG_T0, REG_T1);
        }
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
        Ok(())
    }

    /// Population count via a simple shift-and-add loop.
    fn popcnt(&mut self, is32: bool) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a); // value
        if is32 {
            self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        }
        self.code.load_imm_to_reg(REG_T2, 0); // counter
        let head = self.code.new_label();
        let end = self.code.new_label();
        self.code.bind(head);
        self.code.cmp_imm_branch("eq", REG_T0, 0, end, true); // value == 0 -> done
        self.code.alu_ri("and", REG_T3, REG_T0, 1); // low bit
        self.code.alu_rr("add", REG_T2, REG_T2, REG_T3);
        self.code.alu_ri("srl", REG_T0, REG_T0, 1);
        self.code.jump(head);
        self.code.bind(end);
        self.code.store_reg_to_slot(a, REG_T2);
    }

    /// Count trailing zeros.
    fn ctz(&mut self, is32: bool) {
        let width: u64 = if is32 { 32 } else { 64 };
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        if is32 {
            self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        }
        self.code.load_imm_to_reg(REG_T2, 0);
        let zero_case = self.code.new_label();
        let end = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T0, 0, zero_case, true);
        let head = self.code.new_label();
        self.code.bind(head);
        self.code.alu_ri("and", REG_T3, REG_T0, 1);
        self.code.cmp_imm_branch("eq", REG_T3, 0, end, false); // low bit set -> done
        self.code.alu_ri("add", REG_T2, REG_T2, 1);
        self.code.alu_ri("srl", REG_T0, REG_T0, 1);
        self.code.jump(head);
        self.code.bind(zero_case);
        self.code.load_imm_to_reg(REG_T2, width);
        self.code.bind(end);
        self.code.store_reg_to_slot(a, REG_T2);
    }

    /// Count leading zeros.
    fn clz(&mut self, is32: bool) {
        let width: u64 = if is32 { 32 } else { 64 };
        let topbit: i64 =
            if is32 { 0x8000_0000u64 as i64 } else { 0x8000_0000_0000_0000u64 as i64 };
        let (shl, _shr) = if is32 { ("sll_w", "srl_w") } else { ("sll", "srl") };
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        if is32 {
            self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        }
        self.code.load_imm_to_reg(REG_T2, 0);
        let zero_case = self.code.new_label();
        let end = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T0, 0, zero_case, true);
        let head = self.code.new_label();
        self.code.bind(head);
        self.code.alu_ri("and", REG_T3, REG_T0, topbit);
        self.code.cmp_imm_branch("eq", REG_T3, 0, end, false); // top bit set -> done
        self.code.alu_ri("add", REG_T2, REG_T2, 1);
        self.code.alu_ri(shl, REG_T0, REG_T0, 1);
        if is32 {
            self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        }
        self.code.jump(head);
        self.code.bind(zero_case);
        self.code.load_imm_to_reg(REG_T2, width);
        self.code.bind(end);
        self.code.store_reg_to_slot(a, REG_T2);
    }

    /// Computes the absolute linear-memory address of a wasm i32 address operand + static offset
    /// into `reg`.  The sum cannot wrap: `u32 + u32 + WASM_MEM_BASE` fits in 34 bits.
    fn compute_addr(&mut self, reg: u64, addr_slot: i64, static_offset: u64) {
        self.code.load_slot_to_reg(reg, addr_slot);
        self.code.alu_ri("and", reg, reg, 0xFFFF_FFFF); // zero-extend to u32
        self.code.alu_ri("add", reg, reg, (WASM_MEM_BASE + static_offset) as i64);
    }

    /// Traps unless the `len` bytes starting at the absolute address in `addr_reg` lie inside the
    /// current linear memory (`addr + len <= REG_MEM_END`).  `len` is a register for the bulk
    /// operations (`scratch` receives the end address) and an access width for plain accesses,
    /// which compare the start address with that width's limit register in one instruction.
    fn check_bounds(&mut self, addr_reg: u64, len: Len, scratch: u64) {
        let ok = self.code.new_label();
        match len {
            Len::Imm(w) => self.code.cmp_reg_branch("leu", addr_reg, mem_limit_reg(w), ok, true),
            Len::Reg(r) => {
                self.code.alu_rr("add", scratch, addr_reg, r);
                self.code.cmp_reg_branch("leu", scratch, REG_MEM_END, ok, true);
            }
        }
        self.emit_trap();
        self.code.bind(ok);
    }

    /// A linear-memory load; the value goes to `REG_ACC`.
    pub(crate) fn load(&mut self, op: &str, width: u64, static_offset: u64) {
        let (reg, off) = self.access_addr(static_offset, width);
        self.spill_acc();
        self.code.load_mem_to_reg(op, REG_ACC, reg, off, width);
        self.push_val(Val::Acc);
    }

    /// A linear-memory store.
    pub(crate) fn store(&mut self, width: u64, static_offset: u64) {
        let val = self.pop();
        let val_reg = self.opnd_reg(val, REG_T1);
        let (reg, off) = self.access_addr(static_offset, width);
        self.code.store_reg_to_mem(reg, off, val_reg, width);
    }

    /// Pops the address operand of a `width`-byte access and returns `(reg, off)`: the access is
    /// to `reg + off`, which has been checked to lie inside linear memory.  Only `REG_T0` and the
    /// `REG_PTRS` registers are written.
    fn access_addr(&mut self, static_offset: u64, width: u64) -> (u64, i64) {
        let v = self.pop_val();
        if let Val::Local(k) = v {
            if let Some(access) = self.local_addr(k, static_offset, width) {
                return access;
            }
        }
        let addr = self.opnd_at(v, self.depth);
        self.addr_to_reg(REG_T0, addr, static_offset);
        self.check_bounds(REG_T0, Len::Imm(width), REG_T1);
        (REG_T0, 0)
    }

    /// An access through local `k`, whose absolute address this straight-line code may already
    /// have in a `REG_PTRS` register (see [`Ptr`]).  `None` for an offset too far from the one
    /// the register was made for to be an instruction's offset.
    fn local_addr(&mut self, k: u32, static_offset: u64, width: u64) -> Option<(u64, i64)> {
        let w = width as i64;
        if let Some(i) = self.ptrs.iter().position(|p| p.local == k) {
            let p = self.ptrs[i];
            let rel = static_offset as i64 - p.off0 as i64;
            i32::try_from(rel).ok()?;
            if rel + w <= p.hi {
                return Some((p.reg, rel)); // already checked
            }
            self.code.alu_ri("add", REG_T0, p.reg, rel);
            self.check_bounds(REG_T0, Len::Imm(width), REG_T1);
            self.ptrs[i].hi = rel + w;
            return Some((REG_T0, 0));
        }
        let reg = REG_PTRS[self.next_ptr];
        self.next_ptr = (self.next_ptr + 1) % REG_PTRS.len();
        self.ptrs.retain(|p| p.reg != reg);
        self.code.load_slot_to_reg(reg, local_offset(k));
        self.code.alu_ri("and", reg, reg, 0xFFFF_FFFF); // zero-extend to u32
        self.code.alu_ri("add", reg, reg, (WASM_MEM_BASE + static_offset) as i64);
        self.check_bounds(reg, Len::Imm(width), REG_T1);
        self.ptrs.push(Ptr { local: k, reg, off0: static_offset, hi: w });
        Some((reg, 0))
    }

    /// Forgets every address in a `REG_PTRS` register: at a label (where control can arrive from
    /// code that left other values there) and around anything that may write those registers.
    fn forget_ptrs(&mut self) {
        self.ptrs.clear();
    }

    /// `reg` = the absolute address of wasm address `addr` + `static_offset`, as
    /// [`Self::compute_addr`]; a constant address costs one instruction.
    fn addr_to_reg(&mut self, reg: u64, addr: Opnd, static_offset: u64) {
        let base = WASM_MEM_BASE + static_offset;
        if let Opnd::Const(c) = addr {
            self.code.load_imm_to_reg(reg, (c as u64 & 0xFFFF_FFFF) + base);
            return;
        }
        let src = self.opnd_reg(addr, reg);
        self.code.alu_ri("and", reg, src, 0xFFFF_FFFF); // zero-extend to u32
        self.code.alu_ri("add", reg, reg, base as i64);
    }

    fn memory_grow(&mut self) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a); // delta pages (u32)
        self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        self.code.load_abs_to_reg(REG_T1, WASM_MEM_PAGES_ADDR); // current pages
        self.code.alu_rr("add", REG_T2, REG_T1, REG_T0); // new pages
        let success = self.code.new_label();
        let done = self.code.new_label();
        self.code.cmp_imm_branch("leu", REG_T2, self.module.mem_max_pages as i64, success, true);
        // failure: push -1
        self.code.load_imm_to_reg(REG_T3, (-1i64) as u64);
        self.code.store_reg_to_slot(a, REG_T3);
        self.code.jump(done);
        // success: update page count and the bounds register, push old size
        self.code.bind(success);
        self.code.store_reg_to_abs(WASM_MEM_PAGES_ADDR, REG_T2, 8);
        self.code.alu_ri("sll", REG_T3, REG_T2, WASM_PAGE_SIZE.trailing_zeros() as i64);
        self.code.alu_ri("add", REG_MEM_END, REG_T3, WASM_MEM_BASE as i64);
        self.code.set_mem_limits();
        self.code.store_reg_to_slot(a, REG_T1);
        self.code.bind(done);
    }

    /// `memory.fill` on the DMA `memset` precompile, as a RISC-V guest's `memset` does.  The
    /// precompile takes the byte as a constant: zero, by far the common case, is one inline
    /// instruction, and any other byte calls its entry of the module's table (see
    /// [`build_memset_table`]).
    fn memory_fill(&mut self) {
        // stack: dst, val, len
        let dst = self.slot(self.depth - 3);
        let val = self.slot(self.depth - 2);
        let len = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T2, len);
        self.code.alu_ri("and", REG_T2, REG_T2, 0xFFFF_FFFF);
        self.code.load_slot_to_reg(REG_T1, val);
        self.compute_addr(REG_T0, dst, 0);
        self.check_bounds(REG_T0, Len::Reg(REG_T2), REG_T3);
        self.code.alu_ri("and", REG_T1, REG_T1, 0xFF);
        let nonzero = self.code.new_label();
        let end = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T1, 0, nonzero, false);
        self.code.dma_xmemset(REG_T0, REG_T2, 0);
        self.code.jump(end);
        // Entry `byte` of the table is MEMSET_TABLE_ENTRY instructions in.
        self.code.bind(nonzero);
        self.code.load_func_addr_to_reg(REG_T3, self.module.func_count());
        self.code.alu_ri("mul", REG_T1, REG_T1, 4 * MEMSET_TABLE_ENTRY as i64);
        self.code.alu_rr("add", REG_T3, REG_T3, REG_T1);
        self.code.call_reg(REG_T3);
        self.code.uses_memset_table = true;
        self.code.bind(end);
        self.depth -= 3;
    }

    /// `memory.copy` on the DMA `memcpy` precompile, as a RISC-V guest's `memmove` does: the
    /// precompile has `memmove` semantics, so the ranges may overlap either way.  The length
    /// goes through `EXTRA_PARAMS_ADDR`, where the precompile reads it.
    fn memory_copy(&mut self) {
        // stack: dst, src, len
        let dst = self.slot(self.depth - 3);
        let src = self.slot(self.depth - 2);
        let len = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T2, len);
        self.code.alu_ri("and", REG_T2, REG_T2, 0xFFFF_FFFF);
        self.compute_addr(REG_T1, src, 0);
        self.compute_addr(REG_T0, dst, 0);
        self.check_bounds(REG_T1, Len::Reg(REG_T2), REG_T3);
        self.check_bounds(REG_T0, Len::Reg(REG_T2), REG_T3);
        self.code.store_reg_to_abs(EXTRA_PARAMS_ADDR, REG_T2, 8);
        self.code.dma_memcpy(REG_T0, REG_T1);
        self.depth -= 3;
    }

    fn select(&mut self) {
        // stack: a, b, cond -> cond != 0 ? a : b
        let a = self.slot(self.depth - 3);
        let b = self.slot(self.depth - 2);
        let cond = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T2, cond);
        let use_b = self.code.new_label();
        let done = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T2, 0, use_b, true); // cond == 0 -> b
                                                                // use a: already at slot a, nothing to move
        self.code.jump(done);
        self.code.bind(use_b);
        self.code.load_slot_to_reg(REG_T0, b);
        self.code.store_reg_to_slot(a, REG_T0);
        self.code.bind(done);
        self.depth -= 2;
    }

    pub(crate) fn emit_trap(&mut self) {
        self.code.trap();
    }

    /// Emits a `return`: the result (if any) goes to `REG_RET`, then jump to the epilogue.
    fn emit_return(&mut self) {
        if !self.sig.results.is_empty() {
            let off = self.slot(self.depth - 1);
            self.code.load_slot_to_reg(REG_RET, off);
        }
        self.code.jump(self.func_end);
    }

    /// Moves the top `arity` results to a target block's result slots and jumps to its label.
    /// A depth one past the outermost block targets the function body itself, i.e. returns.
    fn branch_to(&mut self, relative_depth: u32) {
        if relative_depth as usize == self.ctrl.len() {
            self.emit_return();
            return;
        }
        let frame_idx = self.ctrl.len() - 1 - relative_depth as usize;
        let frame = &self.ctrl[frame_idx];
        let is_loop = frame.kind == CtrlKind::Loop;
        let arity = if is_loop { 0 } else { frame.result_arity };
        let target_start = frame.start_depth;
        let branch_label = frame.branch_label;
        if arity == 1 {
            let dst = operand_offset(self.num_locals, target_start);
            let src = self.slot(self.depth - 1);
            self.code.copy_slot(dst, src);
        }
        self.code.jump(branch_label);
    }

    /// `br_table`: a jump table for `BR_TABLE_JUMP_MIN` targets or more, a compare chain below.
    /// Only arity-0 targets (the common switch lowering).
    fn br_table(&mut self, targets: &wasmparser::BrTable) -> Result<(), Box<dyn Error>> {
        let cond = self.pop_val();
        self.flush();
        let cond = self.opnd_at(cond, self.depth);
        let index = self.opnd_reg(cond, REG_T0);
        // A target past the outermost block is the function body: those cases jump to a shared
        // return stub emitted after the dispatch.
        let mut return_stub: Option<LabelId> = None;
        let mut target_label = |this: &mut Self, rel: u32| -> Result<LabelId, Box<dyn Error>> {
            if rel as usize == this.ctrl.len() {
                if this.sig.results.len() > 1 {
                    return Err("wasm: br_table with result values is not supported".into());
                }
                return Ok(*return_stub.get_or_insert_with(|| this.code.new_label()));
            }
            let frame_idx = this.ctrl.len() - 1 - rel as usize;
            if this.ctrl[frame_idx].kind != CtrlKind::Loop && this.ctrl[frame_idx].result_arity != 0
            {
                return Err("wasm: br_table with result values is not supported".into());
            }
            Ok(this.ctrl[frame_idx].branch_label)
        };
        let mut labels = Vec::new();
        for target in targets.targets() {
            labels.push(target_label(self, target?)?);
        }
        let default = target_label(self, targets.default())?;
        if labels.len() < BR_TABLE_JUMP_MIN {
            for (i, &label) in labels.iter().enumerate() {
                self.code.cmp_imm_branch("eq", index, i as i64, label, true);
            }
            self.code.jump(default);
        } else {
            // An i32 index is sign-extended, so one at 2^31 or above compares as huge here too.
            self.code.cmp_imm_branch("ltu", index, labels.len() as i64, default, false);
            self.code.alu_ri("sll", REG_T0, index, 2); // one 4-byte jump per entry
            let table = self.code.new_label();
            self.code.jump_reg_plus_label(REG_T0, table);
            self.code.bind(table);
            for label in labels {
                self.code.jump(label);
            }
        }
        if let Some(stub) = return_stub {
            self.code.bind(stub);
            self.emit_return();
        }
        self.unreachable = true;
        Ok(())
    }

    fn check_stack(&mut self, new_fp_reg: u64, callee_frame: i64, scratch: u64) {
        self.code.alu_ri("add", scratch, new_fp_reg, -callee_frame);
        let ok = self.code.new_label();
        self.code.cmp_imm_branch("ltu", scratch, WASM_STACK_LIMIT as i64, ok, false);
        self.emit_trap();
        self.code.bind(ok);
    }

    fn call(&mut self, callee: u32) -> Result<(), Box<dyn Error>> {
        let sig = self.module.func_sig(callee)?.clone();
        let n_args = sig.params.len() as u32;
        let n_res = sig.results.len() as u32;
        let frame_sz = func_frame_size(self.num_locals);

        // newFP = FP - frame_size(caller)
        self.code.alu_ri("add", REG_T2, REG_FP, -frame_sz);
        let callee_frame = callee_frame_size(self.module, callee)?;
        self.check_stack(REG_T2, callee_frame, REG_T3);
        // Copy arguments into callee locals.
        for i in 0..n_args {
            let src = self.slot(self.depth - n_args + i);
            self.code.load_slot_to_reg(REG_T0, src);
            // mem[newFP + local_offset(i)] = REG_T0
            let mut zib = ZiskInstBuilder::new(0);
            zib.src_a("reg", REG_T2, false);
            zib.src_b("reg", REG_T0, false);
            zib.op("copyb").unwrap();
            zib.ind_width(8);
            zib.store("ind", local_offset(i), false, false);
            zib.j(4, 4);

            self.code.push_raw(zib, Fixup::None);
        }
        // Save caller FP into callee header: mem[newFP - 16] = FP
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", REG_T2, false);
        zib.src_b("reg", REG_FP, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", FRAME_CALLER_FP_OFF, false, false);
        zib.j(4, 4);

        self.code.push_raw(zib, Fixup::None);
        // FP <- newFP
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T2, false);
        zib.op("copyb").unwrap();
        zib.store("reg", REG_FP as i64, false, false);
        zib.j(4, 4);

        self.code.push_raw(zib, Fixup::None);
        // CALL: pc <- callee_addr (patched), RA <- return addr (pc + 4)
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", 0, false); // patched to callee address
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.store_pc("reg", REG_RA as i64, false);
        zib.j(0, 4);
        // Profiling metadata: a call linking through ra, like a RISC-V `jal ra`
        zib.set_meta_rd(REG_RA as u8);

        self.code.push_raw(zib, Fixup::FuncAddr(callee));

        // Pop arguments; push result.
        self.depth -= n_args;
        if n_res == 1 {
            let dst = self.push();
            self.code.store_reg_to_slot(dst, REG_RET);
        }
        Ok(())
    }

    fn call_indirect(&mut self, type_index: u32) -> Result<(), Box<dyn Error>> {
        let sig = self.module.sigs[type_index as usize].clone();
        let n_args = sig.params.len() as u32;
        let n_res = sig.results.len() as u32;
        let frame_sz = func_frame_size(self.num_locals);

        // table index is on top of stack
        let idx_slot = self.slot(self.depth - 1);
        self.depth -= 1;
        self.code.load_slot_to_reg(REG_T0, idx_slot);
        self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        let in_bounds = self.code.new_label();
        self.code.cmp_imm_branch("ltu", REG_T0, self.module.table_initial as i64, in_bounds, true);
        self.emit_trap();
        self.code.bind(in_bounds);
        // entry address = WASM_TABLE_ADDR + idx*16
        self.code.alu_ri("mul", REG_T0, REG_T0, WASM_TABLE_ENTRY_BYTES as i64);
        self.code.alu_ri("add", REG_T0, REG_T0, WASM_TABLE_ADDR as i64);
        // load target pc (offset 0) and canonical type id (offset 8)
        self.code.load_mem_to_reg("copyb", REG_T3, REG_T0, 0, 8); // target pc
        self.code.load_mem_to_reg("copyb", REG_T1, REG_T0, 8, 8); // canonical type id
        let non_null = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T3, 0, non_null, false);
        self.emit_trap();
        self.code.bind(non_null);
        let expected = self.module.canonical_type(type_index);
        let ok = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T1, expected as i64, ok, true);
        self.emit_trap();
        self.code.bind(ok);

        // newFP = FP - frame_size
        self.code.alu_ri("add", REG_T2, REG_FP, -frame_sz);
        let worst_frame = func_frame_size(self.module.max_num_locals());
        self.check_stack(REG_T2, worst_frame, REG_T1);
        for i in 0..n_args {
            let src = self.slot(self.depth - n_args + i);
            self.code.load_slot_to_reg(REG_T1, src);
            let mut zib = ZiskInstBuilder::new(0);
            zib.src_a("reg", REG_T2, false);
            zib.src_b("reg", REG_T1, false);
            zib.op("copyb").unwrap();
            zib.ind_width(8);
            zib.store("ind", local_offset(i), false, false);
            zib.j(4, 4);

            self.code.push_raw(zib, Fixup::None);
        }
        // save caller FP
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", REG_T2, false);
        zib.src_b("reg", REG_FP, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", FRAME_CALLER_FP_OFF, false, false);
        zib.j(4, 4);

        self.code.push_raw(zib, Fixup::None);
        // FP <- newFP
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T2, false);
        zib.op("copyb").unwrap();
        zib.store("reg", REG_FP as i64, false, false);
        zib.j(4, 4);

        self.code.push_raw(zib, Fixup::None);
        // CALL via register (target pc in REG_T3)
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T3, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.store_pc("reg", REG_RA as i64, false);
        zib.j(0, 4);
        // Profiling metadata: a call linking through ra, like a RISC-V `jal ra`
        zib.set_meta_rd(REG_RA as u8);

        self.code.push_raw(zib, Fixup::None);

        self.depth -= n_args;
        if n_res == 1 {
            let dst = self.push();
            self.code.store_reg_to_slot(dst, REG_RET);
        }
        Ok(())
    }
}

/// Fewest `br_table` targets dispatched through a jump table (4 steps for any index); fewer go
/// through a compare chain, which takes `i + 1` steps for index `i`.
const BR_TABLE_JUMP_MIN: usize = 4;

/// Instructions per entry of the `memory.fill` table.
pub(crate) const MEMSET_TABLE_ENTRY: u64 = 2;

/// The module's `memory.fill` table, laid out right after its functions (at function index
/// `func_count`): entry `b` fills `REG_T2` bytes at `REG_T0` with byte `b` and returns through
/// `REG_RA`.  The DMA `memset` precompile takes its byte as a constant, so a byte known only at
/// run time selects its entry; RISC-V guests' `memset` uses the same table (`ziskos`'s
/// `memset.s`).
pub(crate) fn build_memset_table() -> Code {
    let mut code = Code::new();
    for byte in 0..=255u8 {
        code.dma_xmemset(REG_T0, REG_T2, byte);
        code.ret_reg(REG_RA);
    }
    debug_assert_eq!(code.len() as u64, 256 * MEMSET_TABLE_ENTRY);
    code
}
