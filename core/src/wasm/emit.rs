//! Low-level Zisk instruction emitter for the wasm lowering.
//!
//! A [`Code`] accumulates [`ZiskInstBuilder`]s for a single function with *symbolic* jump targets
//! (labels and function indices).  Because intra-function jumps are PC-relative and calls need
//! absolute function addresses that are only known after every function has been laid out, each
//! instruction carries an optional [`Fixup`] that is resolved in a second pass (see `lowering.rs`
//! and `mod.rs`).
//!
//! The operand stack and locals live in guest memory addressed relative to the frame pointer
//! register (`REG_FP`); see `layout.rs`.  The key efficiency trick is that a single Zisk
//! instruction can both read an indirect source (`b = mem[a + off_b]`) and write an indirect store
//! (`mem[a + off_store] = c`) sharing the base `a = REG_FP`, so a memory-to-memory slot copy is one
//! instruction.

use super::layout::*;
use crate::ZiskInstBuilder;

/// A label within a function body; resolved to an instruction index when bound.
pub type LabelId = usize;

/// How a pending instruction's symbolic operands are resolved in pass 2.
#[derive(Clone, Debug)]
pub enum Fixup {
    /// Nothing to resolve; the builder's offsets are final.
    None,
    /// Unconditional jump: both jump offsets target `label`.
    Jump(LabelId),
    /// Conditional jump taken when the comparison flag is set; fallthrough otherwise.
    JumpIfFlag(LabelId),
    /// Conditional jump taken when the comparison flag is clear; fallthrough otherwise.
    JumpIfNotFlag(LabelId),
    /// Set the `b` immediate to the absolute ROM address of function `index` (for `call`).
    FuncAddr(u32),
}

/// One emitted instruction plus its fixup.
pub struct PendingInst {
    pub zib: ZiskInstBuilder,
    pub fixup: Fixup,
}

/// Accumulator for a single function's lowered code.
#[derive(Default)]
pub struct Code {
    pub insts: Vec<PendingInst>,
    /// label id -> instruction index the label points at (the next instruction emitted after bind).
    labels: Vec<Option<usize>>,
}

impl Code {
    pub fn new() -> Self {
        Code { insts: Vec::new(), labels: Vec::new() }
    }

    /// Number of instructions emitted so far (also the index of the next instruction).
    pub fn len(&self) -> usize {
        self.insts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.insts.is_empty()
    }

    /// Allocates a fresh, unbound label.
    pub fn new_label(&mut self) -> LabelId {
        self.labels.push(None);
        self.labels.len() - 1
    }

    /// Binds `label` to the next instruction that will be emitted.
    pub fn bind(&mut self, label: LabelId) {
        self.labels[label] = Some(self.insts.len());
    }

    /// Resolved instruction index for a bound label.
    pub fn label_target(&self, label: LabelId) -> usize {
        self.labels[label].expect("wasm: label used but never bound")
    }

    /// Whether `label` has not yet been bound.
    pub fn label_is_unbound(&self, label: LabelId) -> bool {
        self.labels[label].is_none()
    }

    fn push(&mut self, zib: ZiskInstBuilder, fixup: Fixup) {
        self.insts.push(PendingInst { zib, fixup });
    }

    // -- raw building blocks -------------------------------------------------

    /// `copyb a=FP, b=imm value` -> store ind `slot_off` (width 8).  Pushes an immediate to a slot.
    pub fn store_imm_to_slot(&mut self, slot_off: i64, value: i64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", REG_FP, false);
        zib.src_b("imm", value as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", slot_off, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// Memory-to-memory slot copy: `mem[FP+dst] = mem[FP+src]` in one instruction (width 8).
    pub fn copy_slot(&mut self, dst_off: i64, src_off: i64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", REG_FP, false);
        zib.src_b("ind", src_off as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", dst_off, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// Loads `mem[FP+slot_off]` into register `reg` (width 8).
    pub fn load_slot_to_reg(&mut self, reg: u64, slot_off: i64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", REG_FP, false);
        zib.src_b("ind", slot_off as u64, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("reg", reg as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// Stores register `reg` into `mem[FP+slot_off]` (width 8).
    pub fn store_reg_to_slot(&mut self, slot_off: i64, reg: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", REG_FP, false);
        zib.src_b("reg", reg, false);
        zib.op("copyb").unwrap();
        zib.ind_width(8);
        zib.store("ind", slot_off, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `reg_dst = op(reg_a, reg_b)`.  `op` is a Zisk op name.
    pub fn alu_rr(&mut self, op: &str, reg_dst: u64, reg_a: u64, reg_b: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", reg_a, false);
        zib.src_b("reg", reg_b, false);
        zib.op(op).unwrap();
        zib.store("reg", reg_dst as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `reg_dst = op(reg_a, imm)`.
    pub fn alu_ri(&mut self, op: &str, reg_dst: u64, reg_a: u64, imm: i64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", reg_a, false);
        zib.src_b("imm", imm as u64, false);
        zib.op(op).unwrap();
        zib.store("reg", reg_dst as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `reg = copyb(0, imm)` — load an immediate into a register.
    pub fn load_imm_to_reg(&mut self, reg: u64, imm: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", imm, false);
        zib.op("copyb").unwrap();
        zib.store("reg", reg as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `mem[reg_addr + off] = reg_val` with the given width (a linear-memory store).
    pub fn store_reg_to_mem(&mut self, reg_addr: u64, off: i64, reg_val: u64, width: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", reg_addr, false);
        zib.src_b("reg", reg_val, false);
        zib.op("copyb").unwrap();
        zib.ind_width(width);
        zib.store("ind", off, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `reg_dst = op(mem[reg_addr + off])` where `op` is a load op (copyb / signextend_*).
    pub fn load_mem_to_reg(&mut self, op: &str, reg_dst: u64, reg_addr: u64, off: i64, width: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", reg_addr, false);
        zib.src_b("ind", off as u64, false);
        zib.op(op).unwrap();
        zib.ind_width(width);
        zib.store("reg", reg_dst as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `mem[abs_addr] = reg` (absolute system address).
    pub fn store_reg_to_abs(&mut self, abs_addr: u64, reg: u64, _width: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", reg, false);
        zib.op("copyb").unwrap();
        zib.store("mem", abs_addr as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `reg = mem[abs_addr]` (absolute system address).
    pub fn load_abs_to_reg(&mut self, reg: u64, abs_addr: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("mem", abs_addr, false);
        zib.op("copyb").unwrap();
        zib.store("reg", reg as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// `reg_dst = reg_src`.
    pub fn mov_reg(&mut self, reg_dst: u64, reg_src: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", reg_src, false);
        zib.op("copyb").unwrap();
        zib.store("reg", reg_dst as i64, false, false);
        zib.j(4, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    /// Sets the program counter to the address held in `reg` (an indirect jump / return).
    pub fn ret_reg(&mut self, reg: u64) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("reg", reg, false);
        zib.op("copyb").unwrap();
        zib.set_pc();
        zib.j(0, 4);
        zib.build();
        self.push(zib, Fixup::None);
    }

    // -- control flow --------------------------------------------------------

    /// Unconditional jump to `label`.
    pub fn jump(&mut self, label: LabelId) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("imm", 0, false);
        zib.src_b("imm", 0, false);
        zib.op("flag").unwrap();
        zib.j(0, 0);
        zib.build();
        self.push(zib, Fixup::Jump(label));
    }

    /// Compares `reg` against immediate `imm` using `cmp_op` (a flag-setting op such as `eq`),
    /// then jumps to `label` when the flag is set (`jump_if_flag`) or clear.
    pub fn cmp_imm_branch(
        &mut self,
        cmp_op: &str,
        reg: u64,
        imm: i64,
        label: LabelId,
        jump_if_flag: bool,
    ) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", reg, false);
        zib.src_b("imm", imm as u64, false);
        zib.op(cmp_op).unwrap();
        zib.store("none", 0, false, false);
        zib.j(0, 0);
        zib.build();
        let fixup = if jump_if_flag { Fixup::JumpIfFlag(label) } else { Fixup::JumpIfNotFlag(label) };
        self.push(zib, fixup);
    }

    /// Compares two registers with `cmp_op` and branches on the flag.
    pub fn cmp_reg_branch(
        &mut self,
        cmp_op: &str,
        reg_a: u64,
        reg_b: u64,
        label: LabelId,
        jump_if_flag: bool,
    ) {
        let mut zib = ZiskInstBuilder::new(0);
        zib.src_a("reg", reg_a, false);
        zib.src_b("reg", reg_b, false);
        zib.op(cmp_op).unwrap();
        zib.store("none", 0, false, false);
        zib.j(0, 0);
        zib.build();
        let fixup = if jump_if_flag { Fixup::JumpIfFlag(label) } else { Fixup::JumpIfNotFlag(label) };
        self.push(zib, fixup);
    }

    /// Emits a raw builder with a custom fixup (escape hatch for calls etc.).
    pub fn push_raw(&mut self, zib: ZiskInstBuilder, fixup: Fixup) {
        self.push(zib, fixup);
    }
}
