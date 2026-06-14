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
use crate::ZiskInstBuilder;
use wasmparser::{BlockType, Operator};

/// Worst-case operand-stack depth a function may reach.  Frames reserve this many operand slots so
/// callee frame placement is a compile-time constant (see `frame_size`).
pub const OPERAND_CAP: u32 = 1024;

/// Per-function frame size in bytes (header + locals + reserved operand slots).
pub fn func_frame_size(num_locals: u32) -> i64 {
    frame_size(num_locals, OPERAND_CAP)
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

/// Lowers `func_index` (which must be a defined function) into a [`Code`].
pub fn lower_function(
    module: &WasmModule,
    func_index: u32,
) -> Result<Code, Box<dyn Error>> {
    let defined_index = (func_index - module.func_import_count) as usize;
    let func = &module.defined[defined_index];
    let sig = &module.sigs[func.type_index as usize];

    // Build the full local list: parameters followed by declared locals.
    let mut local_kinds: Vec<wasmparser::ValType> = Vec::new();
    {
        // Re-read params from the original wasm type via the signature kinds.
        for _ in &sig.params {
            local_kinds.push(wasmparser::ValType::I64); // kind not needed beyond count
        }
        let mut locals_reader = func.body.get_locals_reader()?;
        let count = locals_reader.get_count();
        for _ in 0..count {
            let (n, ty) = locals_reader.read()?;
            super::module::ValKind::from_valtype(ty)?; // reject unsupported local types
            for _ in 0..n {
                local_kinds.push(ty);
            }
        }
    }
    let num_locals = local_kinds.len() as u32;

    let mut gen = FuncGen {
        module,
        sig,
        num_locals,
        code: Code::new(),
        depth: 0,
        ctrl: Vec::new(),
        unreachable: false,
        dead_nesting: 0,
    };

    // Function-level end label: targets of `return` and the implicit fallthrough land here.
    let func_end = gen.code.new_label();

    // Prologue: save the return address and zero the non-parameter locals.
    gen.code.store_reg_to_slot(FRAME_RET_PC_OFF, REG_RA);
    for k in sig.params.len() as u32..num_locals {
        gen.code.store_imm_to_slot(local_offset(k), 0);
    }

    let mut op_reader = func.body.get_operators_reader()?;
    while !op_reader.eof() {
        let op = op_reader.read()?;
        gen.lower_op(op, func_end)?;
    }

    // Epilogue (bound at func_end): place the result into REG_RET, restore caller FP, return.
    gen.code.bind(func_end);
    gen.emit_epilogue();

    Ok(gen.code)
}

struct FuncGen<'a, 'b> {
    module: &'b WasmModule<'a>,
    sig: &'b FuncSig,
    num_locals: u32,
    code: Code,
    depth: u32,
    ctrl: Vec<CtrlFrame>,
    unreachable: bool,
    /// Number of fully-dead nested blocks while `unreachable` (see module docs).
    dead_nesting: u32,
}

impl<'a, 'b> FuncGen<'a, 'b> {
    /// Byte offset (from FP) of operand-stack slot at depth `d`.
    fn slot(&self, d: u32) -> i64 {
        operand_offset(self.num_locals, d)
    }

    fn push(&mut self) -> i64 {
        let off = self.slot(self.depth);
        self.depth += 1;
        if self.depth > OPERAND_CAP {
            panic!("wasm: operand stack depth exceeded OPERAND_CAP ({OPERAND_CAP})");
        }
        off
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
        // Result, if any, into REG_RET. When reached via fallthrough the result is on top of stack.
        if !self.sig.results.is_empty() {
            // depth may be unreliable if we got here only via `return`; returns load REG_RET
            // themselves, so only load here when there is a live operand.
            if self.depth >= 1 {
                let off = self.slot(self.depth - 1);
                self.code.load_slot_to_reg(REG_RET, off);
            }
        }
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
        zib.build();
        self.code.push_raw(zib, Fixup::None);
        // pc <- saved return address
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T0, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.j(0, 4);
        zib.build();
        self.code.push_raw(zib, Fixup::None);
    }

    fn lower_op(&mut self, op: Operator, func_end: LabelId) -> Result<(), Box<dyn Error>> {
        use Operator::*;

        // Dead-code skipping: only structural delimiters are processed while unreachable.
        if self.unreachable {
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

        match op {
            // -- constants -----------------------------------------------------
            I32Const { value } => {
                let off = self.push();
                self.code.store_imm_to_slot(off, value as i64);
            }
            I64Const { value } => {
                let off = self.push();
                self.code.store_imm_to_slot(off, value);
            }

            // -- locals / globals ---------------------------------------------
            LocalGet { local_index } => {
                let src = local_offset(local_index);
                let dst = self.push();
                self.code.copy_slot(dst, src);
            }
            LocalSet { local_index } => {
                let src = self.slot(self.depth - 1);
                let dst = local_offset(local_index);
                self.code.copy_slot(dst, src);
                self.depth -= 1;
            }
            LocalTee { local_index } => {
                let src = self.slot(self.depth - 1);
                let dst = local_offset(local_index);
                self.code.copy_slot(dst, src);
            }
            GlobalGet { global_index } => {
                let dst = self.push();
                self.code.load_abs_to_reg(REG_T0, global_addr(global_index));
                self.code.store_reg_to_slot(dst, REG_T0);
            }
            GlobalSet { global_index } => {
                let src = self.slot(self.depth - 1);
                self.code.load_slot_to_reg(REG_T0, src);
                self.code.store_reg_to_abs(global_addr(global_index), REG_T0, 8);
                self.depth -= 1;
            }

            // -- i32 arithmetic / logic ---------------------------------------
            I32Add => self.binop("add_w", false),
            I32Sub => self.binop("sub_w", false),
            I32Mul => self.binop("mul_w", false),
            I32And => self.binop("and", false),
            I32Or => self.binop("or", false),
            I32Xor => self.binop("xor", false),
            I32DivU => self.binop_div("divu_w", false)?,
            I32DivS => self.binop_div("div_w", false)?,
            I32RemU => self.binop_div("remu_w", false)?,
            I32RemS => self.binop_div("rem_w", false)?,
            I32Shl => self.shift_op("sll_w", 0x1f),
            I32ShrU => self.shift_op("srl_w", 0x1f),
            I32ShrS => self.shift_op("sra_w", 0x1f),
            I32Rotl => self.rotate(true, true),
            I32Rotr => self.rotate(false, true),

            // -- i64 arithmetic / logic ---------------------------------------
            I64Add => self.binop("add", false),
            I64Sub => self.binop("sub", false),
            I64Mul => self.binop("mul", false),
            I64And => self.binop("and", false),
            I64Or => self.binop("or", false),
            I64Xor => self.binop("xor", false),
            I64DivU => self.binop_div("divu", false)?,
            I64DivS => self.binop_div("div", false)?,
            I64RemU => self.binop_div("remu", false)?,
            I64RemS => self.binop_div("rem", false)?,
            I64Shl => self.shift_op("sll", 0x3f),
            I64ShrU => self.shift_op("srl", 0x3f),
            I64ShrS => self.shift_op("sra", 0x3f),
            I64Rotl => self.rotate(true, false),
            I64Rotr => self.rotate(false, false),

            // -- comparisons (push 0/1) ---------------------------------------
            I32Eqz | I64Eqz => self.unop_eqz(),
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

            // -- conversions / sign extension ---------------------------------
            I32WrapI64 => self.unop_signextend("signextend_w"),
            I64ExtendI32S => { /* canonical i32 already sign-extended: no-op */ }
            I64ExtendI32U => self.unop_and_imm(0xFFFF_FFFF),
            I32Extend8S | I64Extend8S => self.unop_signextend("signextend_b"),
            I32Extend16S | I64Extend16S => self.unop_signextend("signextend_h"),
            I64Extend32S => self.unop_signextend("signextend_w"),

            // -- bit counting (runtime loops) ---------------------------------
            I32Popcnt => self.popcnt(true),
            I64Popcnt => self.popcnt(false),
            I32Ctz => self.ctz(true),
            I64Ctz => self.ctz(false),
            I32Clz => self.clz(true),
            I64Clz => self.clz(false),

            // -- memory loads / stores ----------------------------------------
            I32Load { memarg } => self.load("signextend_w", 4, memarg.offset),
            I64Load { memarg } => self.load("copyb", 8, memarg.offset),
            I32Load8S { memarg } | I64Load8S { memarg } => self.load("signextend_b", 1, memarg.offset),
            I32Load8U { memarg } | I64Load8U { memarg } => self.load("copyb", 1, memarg.offset),
            I32Load16S { memarg } | I64Load16S { memarg } => self.load("signextend_h", 2, memarg.offset),
            I32Load16U { memarg } | I64Load16U { memarg } => self.load("copyb", 2, memarg.offset),
            I64Load32S { memarg } => self.load("signextend_w", 4, memarg.offset),
            I64Load32U { memarg } => self.load("copyb", 4, memarg.offset),
            I32Store { memarg } => self.store(4, memarg.offset),
            I64Store { memarg } => self.store(8, memarg.offset),
            I32Store8 { memarg } | I64Store8 { memarg } => self.store(1, memarg.offset),
            I32Store16 { memarg } | I64Store16 { memarg } => self.store(2, memarg.offset),
            I64Store32 { memarg } => self.store(4, memarg.offset),
            MemorySize { .. } => {
                let dst = self.push();
                self.code.load_abs_to_reg(REG_T0, WASM_MEM_PAGES_ADDR);
                self.code.store_reg_to_slot(dst, REG_T0);
            }
            MemoryGrow { .. } => self.memory_grow(),
            MemoryCopy { .. } => self.memory_copy(),
            MemoryFill { .. } => self.memory_fill(),

            // -- parametric ---------------------------------------------------
            Drop => {
                self.depth -= 1;
            }
            Select => self.select(),
            Nop => {}
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
            If { blockty } => {
                let (_p, r) = self.block_arity(blockty)?;
                let else_label = self.code.new_label();
                let end_label = self.code.new_label();
                // pop condition; if zero jump to else (or end if no else section appears).
                let cond = self.slot(self.depth - 1);
                self.depth -= 1;
                self.code.load_slot_to_reg(REG_T0, cond);
                self.code.cmp_imm_branch("eq", REG_T0, 0, else_label, true);
                self.ctrl.push(CtrlFrame {
                    kind: CtrlKind::If,
                    branch_label: end_label,
                    end_label,
                    else_label: Some(else_label),
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
            BrIf { relative_depth } => self.branch_if(relative_depth),
            BrTable { targets } => self.br_table(targets)?,
            Return => {
                if !self.sig.results.is_empty() {
                    let off = self.slot(self.depth - 1);
                    self.code.load_slot_to_reg(REG_RET, off);
                }
                self.code.jump(func_end);
                self.unreachable = true;
            }
            Call { function_index } => self.call(function_index)?,
            CallIndirect { type_index, .. } => self.call_indirect(type_index)?,

            other => {
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

    fn binop(&mut self, op: &str, swap: bool) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.load_slot_to_reg(REG_T1, b);
        if swap {
            self.code.alu_rr(op, REG_T0, REG_T1, REG_T0);
        } else {
            self.code.alu_rr(op, REG_T0, REG_T0, REG_T1);
        }
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
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
        if swap {
            self.code.alu_rr(op, REG_T0, REG_T1, REG_T0);
        } else {
            self.code.alu_rr(op, REG_T0, REG_T0, REG_T1);
        }
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
        Ok(())
    }

    fn shift_op(&mut self, op: &str, count_mask: i64) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.load_slot_to_reg(REG_T1, b);
        self.code.alu_ri("and", REG_T1, REG_T1, count_mask);
        self.code.alu_rr(op, REG_T0, REG_T0, REG_T1);
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
    }

    fn compare(&mut self, op: &str, swap: bool, invert: bool) {
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.load_slot_to_reg(REG_T1, b);
        if swap {
            self.code.alu_rr(op, REG_T0, REG_T1, REG_T0);
        } else {
            self.code.alu_rr(op, REG_T0, REG_T0, REG_T1);
        }
        if invert {
            self.code.alu_ri("xor", REG_T0, REG_T0, 1);
        }
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
    }

    fn unop_eqz(&mut self) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.alu_ri("eq", REG_T0, REG_T0, 0);
        self.code.store_reg_to_slot(a, REG_T0);
    }

    fn unop_signextend(&mut self, op: &str) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        // signextend acts on `b`; route REG_T0 through `b`.
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T0, false);
        zib.op(op).unwrap();
        zib.store("reg", REG_T0 as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.code.push_raw(zib, Fixup::None);
        self.code.store_reg_to_slot(a, REG_T0);
    }

    fn unop_and_imm(&mut self, imm: i64) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a);
        self.code.alu_ri("and", REG_T0, REG_T0, imm);
        self.code.store_reg_to_slot(a, REG_T0);
    }

    /// Rotate left/right for i32 (`is32`) or i64.  Implemented with shifts and an `or`.
    fn rotate(&mut self, left: bool, is32: bool) {
        let width: i64 = if is32 { 32 } else { 64 };
        let mask: i64 = if is32 { 0x1f } else { 0x3f };
        let (shl, shr) = if is32 { ("sll_w", "srl_w") } else { ("sll", "srl") };
        let a = self.slot(self.depth - 2);
        let b = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a); // value
        self.code.load_slot_to_reg(REG_T1, b); // count
        self.code.alu_ri("and", REG_T1, REG_T1, mask); // n
        if is32 {
            // value must be treated as unsigned 32-bit for the right half.
            self.code.alu_ri("and", REG_T0, REG_T0, 0xFFFF_FFFF);
        }
        // T2 = (width - n) & mask, so the complementary shift count is always in range
        // (handles rotate-by-zero, where width-n would otherwise equal the bit width).
        self.code.load_imm_to_reg(REG_T3, width as u64);
        self.code.alu_rr("sub", REG_T2, REG_T3, REG_T1);
        self.code.alu_ri("and", REG_T2, REG_T2, mask);
        if left {
            // (v << n) | (v >> (width-n))
            self.code.alu_rr(shl, REG_T3, REG_T0, REG_T1);
            self.code.alu_rr(shr, REG_T0, REG_T0, REG_T2);
            self.code.alu_rr("or", REG_T0, REG_T3, REG_T0);
        } else {
            // (v >> n) | (v << (width-n))
            self.code.alu_rr(shr, REG_T3, REG_T0, REG_T1);
            self.code.alu_rr(shl, REG_T0, REG_T0, REG_T2);
            self.code.alu_rr("or", REG_T0, REG_T3, REG_T0);
        }
        if is32 {
            // re-canonicalize to sign-extended 32-bit
            let mut zib = ZiskInstBuilder::new(0);
            zib.src_a("imm", 0, false);
            zib.src_b("reg", REG_T0, false);
            zib.op("signextend_w").unwrap();
            zib.store("reg", REG_T0 as i64, false, false);
            zib.j(4, 4);
            zib.build();
            self.code.push_raw(zib, Fixup::None);
        }
        self.code.store_reg_to_slot(a, REG_T0);
        self.depth -= 1;
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
        let topbit: i64 = if is32 { 0x8000_0000u64 as i64 } else { 0x8000_0000_0000_0000u64 as i64 };
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
    /// into `reg`.
    fn compute_addr(&mut self, reg: u64, addr_slot: i64, static_offset: u64) {
        self.code.load_slot_to_reg(reg, addr_slot);
        self.code.alu_ri("and", reg, reg, 0xFFFF_FFFF); // zero-extend to u32
        self.code.alu_ri("add", reg, reg, (WASM_MEM_BASE + static_offset) as i64);
    }

    fn load(&mut self, op: &str, width: u64, static_offset: u64) {
        let a = self.slot(self.depth - 1);
        self.compute_addr(REG_T0, a, static_offset);
        self.code.load_mem_to_reg(op, REG_T1, REG_T0, 0, width);
        self.code.store_reg_to_slot(a, REG_T1);
    }

    fn store(&mut self, width: u64, static_offset: u64) {
        let addr_slot = self.slot(self.depth - 2);
        let val_slot = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T1, val_slot);
        self.compute_addr(REG_T0, addr_slot, static_offset);
        self.code.store_reg_to_mem(REG_T0, 0, REG_T1, width);
        self.depth -= 2;
    }

    fn memory_grow(&mut self) {
        let a = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T0, a); // delta pages
        self.code.load_abs_to_reg(REG_T1, WASM_MEM_PAGES_ADDR); // current pages
        self.code.alu_rr("add", REG_T2, REG_T1, REG_T0); // new pages
        let success = self.code.new_label();
        let done = self.code.new_label();
        self.code.cmp_imm_branch("leu", REG_T2, WASM_MAX_PAGES as i64, success, true);
        // failure: push -1
        self.code.load_imm_to_reg(REG_T3, (-1i64) as u64);
        self.code.store_reg_to_slot(a, REG_T3);
        self.code.jump(done);
        // success: update page count, push old size
        self.code.bind(success);
        self.code.store_reg_to_abs(WASM_MEM_PAGES_ADDR, REG_T2, 8);
        self.code.store_reg_to_slot(a, REG_T1);
        self.code.bind(done);
    }

    fn memory_fill(&mut self) {
        // stack: dst, val, len
        let dst = self.slot(self.depth - 3);
        let val = self.slot(self.depth - 2);
        let len = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T2, len);
        self.code.load_slot_to_reg(REG_T1, val);
        self.compute_addr(REG_T0, dst, 0);
        let head = self.code.new_label();
        let end = self.code.new_label();
        self.code.bind(head);
        self.code.cmp_imm_branch("eq", REG_T2, 0, end, true);
        self.code.store_reg_to_mem(REG_T0, 0, REG_T1, 1);
        self.code.alu_ri("add", REG_T0, REG_T0, 1);
        self.code.alu_ri("sub", REG_T2, REG_T2, 1);
        self.code.jump(head);
        self.code.bind(end);
        self.depth -= 3;
    }

    fn memory_copy(&mut self) {
        // stack: dst, src, len. Forward byte copy (non-overlapping or dst <= src).
        let dst = self.slot(self.depth - 3);
        let src = self.slot(self.depth - 2);
        let len = self.slot(self.depth - 1);
        self.code.load_slot_to_reg(REG_T2, len);
        self.compute_addr(REG_T1, src, 0);
        self.compute_addr(REG_T0, dst, 0);
        let head = self.code.new_label();
        let end = self.code.new_label();
        self.code.bind(head);
        self.code.cmp_imm_branch("eq", REG_T2, 0, end, true);
        self.code.load_mem_to_reg("copyb", REG_T3, REG_T1, 0, 1);
        self.code.store_reg_to_mem(REG_T0, 0, REG_T3, 1);
        self.code.alu_ri("add", REG_T0, REG_T0, 1);
        self.code.alu_ri("add", REG_T1, REG_T1, 1);
        self.code.alu_ri("sub", REG_T2, REG_T2, 1);
        self.code.jump(head);
        self.code.bind(end);
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

    fn emit_trap(&mut self) {
        // Halt the machine with an error (wasm trap). `halt` (op 0xff) ends execution.
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", 0, false);
        zib.op("halt").unwrap();
        zib.j(0, 0);
        zib.build();
        self.code.push_raw(zib, Fixup::None);
    }

    /// Moves the top `arity` results to a target block's result slots and jumps to its label.
    fn branch_to(&mut self, relative_depth: u32) {
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

    fn branch_if(&mut self, relative_depth: u32) {
        // Pop condition; if nonzero, perform the branch.
        let cond = self.slot(self.depth - 1);
        self.depth -= 1;
        self.code.load_slot_to_reg(REG_T0, cond);
        let cont = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T0, 0, cont, true); // cond == 0 -> skip branch
        self.branch_to(relative_depth);
        self.code.bind(cont);
    }

    fn br_table(&mut self, targets: wasmparser::BrTable) -> Result<(), Box<dyn Error>> {
        // MVP: only arity-0 br_table (the common switch lowering).
        let cond = self.slot(self.depth - 1);
        self.depth -= 1;
        self.code.load_slot_to_reg(REG_T0, cond);
        // Validate arity 0 across targets and emit a compare chain.
        for (i, target) in targets.targets().enumerate() {
            let rel = target?;
            let frame_idx = self.ctrl.len() - 1 - rel as usize;
            if self.ctrl[frame_idx].kind != CtrlKind::Loop
                && self.ctrl[frame_idx].result_arity != 0
            {
                return Err("wasm: br_table with result values is not supported".into());
            }
            let label = self.ctrl[frame_idx].branch_label;
            self.code.cmp_imm_branch("eq", REG_T0, i as i64, label, true);
        }
        // default
        let rel = targets.default();
        let frame_idx = self.ctrl.len() - 1 - rel as usize;
        let label = self.ctrl[frame_idx].branch_label;
        self.code.jump(label);
        self.unreachable = true;
        Ok(())
    }

    fn call(&mut self, callee: u32) -> Result<(), Box<dyn Error>> {
        let sig = self.module.func_sig(callee)?.clone();
        let n_args = sig.params.len() as u32;
        let n_res = sig.results.len() as u32;
        let frame_sz = func_frame_size(self.num_locals);

        // newFP = FP - frame_size(caller)
        self.code.alu_ri("add", REG_T2, REG_FP, -frame_sz);
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
            zib.build();
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
        zib.build();
        self.code.push_raw(zib, Fixup::None);
        // FP <- newFP
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T2, false);
        zib.op("copyb").unwrap();
        zib.store("reg", REG_FP as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.code.push_raw(zib, Fixup::None);
        // CALL: pc <- callee_addr (patched), RA <- return addr (pc + 4)
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", 0, false); // patched to callee address
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.store_pc("reg", REG_RA as i64, false);
        zib.j(0, 4);
        zib.build();
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
        // entry address = WASM_TABLE_ADDR + idx*16
        self.code.alu_ri("mul", REG_T0, REG_T0, WASM_TABLE_ENTRY_BYTES as i64);
        self.code.alu_ri("add", REG_T0, REG_T0, WASM_TABLE_ADDR as i64);
        // load target pc (offset 0) and canonical type id (offset 8)
        self.code.load_mem_to_reg("copyb", REG_T3, REG_T0, 0, 8); // target pc
        self.code.load_mem_to_reg("copyb", REG_T1, REG_T0, 8, 8); // canonical type id
        // type check: trap if the table entry's type does not match the expected (structural) type
        let expected = self.module.canonical_type(type_index);
        let ok = self.code.new_label();
        self.code.cmp_imm_branch("eq", REG_T1, expected as i64, ok, true);
        self.emit_trap();
        self.code.bind(ok);

        // newFP = FP - frame_size
        self.code.alu_ri("add", REG_T2, REG_FP, -frame_sz);
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
            zib.build();
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
        zib.build();
        self.code.push_raw(zib, Fixup::None);
        // FP <- newFP
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T2, false);
        zib.op("copyb").unwrap();
        zib.store("reg", REG_FP as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.code.push_raw(zib, Fixup::None);
        // CALL via register (target pc in REG_T3)
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", REG_T3, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.store_pc("reg", REG_RA as i64, false);
        zib.j(0, 4);
        zib.build();
        self.code.push_raw(zib, Fixup::None);

        self.depth -= n_args;
        if n_res == 1 {
            let dst = self.push();
            self.code.store_reg_to_slot(dst, REG_RET);
        }
        Ok(())
    }
}
