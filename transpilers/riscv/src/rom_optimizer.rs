//! ROM post-pass optimizer: rewrites of the transpiled ZisK ROM that only the ZisK instruction set
//! can express, applied once the whole program (guest, ZisK library, float library) is in the ROM.
//!
//! - **Jump threading** (`jt`): a jump or branch whose target is an unconditional jump goes
//!   straight to that jump's destination. Every ZisK instruction carries its own jump offsets, so a
//!   RISC-V `j` is absorbed by the instruction that falls through or branches to it.
//! - **Constant folding** (`cf`): `copyb imm -> rX` followed by an instruction that reads rX, where
//!   rX dies, becomes that instruction with the immediate in place of rX.
//! - **Address folding** (`af`): `add rX, base, k` followed by a load or store through rX, where rX
//!   dies, becomes the access at `[base + off + k]`.
//! - **Load + indirect jump fusion** (`lj`, opt-in): `ld t, k(base); jr t` becomes one load that
//!   writes t and jumps to it. It drops `jalr`'s `& ~1`, so it is exact only when every loaded
//!   target is even.
//!
//! A fold replaces the first instruction X of a pair X -> Y by the merged instruction, at X's
//! address and with X's ROM row; Y stays in place for its other predecessors. It only applies when
//! X's single successor is Y and the register X writes is dead after Y (or Y overwrites it), which
//! a global backward liveness analysis over the ROM decides. Every indirect jump, and every jump to
//! an address outside the ROM, makes all registers live, so calls, returns and jump tables are
//! safe.
//!
//! # Measured effect
//!
//! Change in steps and in total cost (ziskemu `-X`), against the same ROM without any pass, for
//! the ziskethone stateless validator on mainnet blocks 26235156 (91 transactions), 26235159 (532)
//! and 26235168 (260). Every configuration produced the same output as the unoptimized ROM.
//!
//! | Passes         | 91 tx: steps | cost   | 532 tx: steps | cost   | 260 tx: steps | cost   |
//! |----------------|--------------|--------|---------------|--------|---------------|--------|
//! | `jt`           | -1.26%       | -0.36% | -1.15%        | -0.46% | -1.10%        | -0.41% |
//! | `cf`           | -0.34%       | -0.10% | -0.24%        | -0.10% | -0.25%        | -0.10% |
//! | `af`           | -0.00%       | -0.00% | -0.00%        | -0.00% | -0.00%        | -0.00% |
//! | `lj` (+ all)   | -3.16%       | -1.27% | -3.74%        | -2.32% | -3.61%        | -2.07% |
//! | `jt`+`cf`+`af` | -1.76%       | -0.51% | -1.49%        | -0.61% | -1.46%        | -0.55% |
//!
//! - `jt` does most of the work: on block 26235156 it removes all of the 125,325 executed `j`.
//! - `cf` saves a step per fold; when both operands become constant the operation is evaluated at
//!   transpile time.
//! - `af` finds almost nothing (4 to 9 sites): GCC already folds constant offsets into loads and
//!   stores. It is kept because it is cheap and exact.
//! - `lj` (measured together with the other three) is the largest: most of its sites are evmone's
//!   computed-goto dispatch, a load of the next handler's address followed by a `jr`.
//!
//! # Why `lj` is disabled by default
//!
//! RISC-V `jalr` clears bit 0 of its target (`& ~1`). The fused instruction jumps straight to the
//! loaded value, so it differs from RISC-V whenever that value is odd: ZisK would jump to an odd
//! address, which is either no instruction or an internal step of a multi-step instruction. Code
//! addresses that compilers produce are always even, so on compiled code it gives the same result,
//! but the transpiler would no longer implement RISC-V exactly: hand-written assembly that relies
//! on the `& ~1`, or a guest that loads a jump target from corrupted memory, would run differently
//! and that difference would be proved. Whether to accept that is a decision for the team, so the
//! pass is opt-in (`RomOptPasses::load_jump_fusion`).

use std::collections::{BTreeMap, HashMap, HashSet};
use zisk_core::{
    zisk_ops::{OpType, ZiskOp},
    ZiskInst, ZiskInstBuilder, SRC_C, SRC_IMM, SRC_IND, SRC_REG, SRC_STEP, STORE_IND, STORE_NONE,
    STORE_REG,
};

/// Registers r1..r39, the ones the Main trace holds; liveness tracks them as a bit mask, bit r for
/// register r. Bit 0 is clear: r0 is not a register (the transpiler reads x0 as the immediate 0).
const ALL_REGS: u64 = (1u64 << 40) - 2;

/// Which passes to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RomOptPasses {
    pub jump_threading: bool,
    pub constant_folding: bool,
    pub address_folding: bool,
    pub load_jump_fusion: bool,
}

impl Default for RomOptPasses {
    /// The exact passes; load + indirect jump fusion is opt-in.
    fn default() -> Self {
        Self {
            jump_threading: true,
            constant_folding: true,
            address_folding: true,
            load_jump_fusion: false,
        }
    }
}

impl RomOptPasses {
    pub const NONE: Self = Self {
        jump_threading: false,
        constant_folding: false,
        address_folding: false,
        load_jump_fusion: false,
    };

    /// Parses a comma-separated list of `jt`, `cf`, `af`, `lj`, or `none`.
    pub fn parse(list: &str) -> Self {
        // A pass is on when its name is one of the list's items; `none` (or anything else) matches
        // no pass, so it turns them all off.
        let has = |p: &str| list.split(',').any(|s| s.trim() == p);
        Self {
            jump_threading: has("jt"),
            constant_folding: has("cf"),
            address_folding: has("af"),
            load_jump_fusion: has("lj"),
        }
    }
}

/// How many rewrites each pass made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RomOptStats {
    pub jump_threading: usize,
    pub constant_folding: usize,
    /// Constant folds whose operands were then both constants, evaluated at transpile time.
    pub constant_evaluated: usize,
    pub address_folding: usize,
    pub load_jump_fusion: usize,
}

/// Rounds of the fixed-point iteration: each fold can expose another.
const MAX_ROUNDS: usize = 8;

/// Runs the selected passes until nothing changes, at most `MAX_ROUNDS` times.
pub fn optimize_rom(
    insts: &mut BTreeMap<u64, ZiskInstBuilder>,
    passes: RomOptPasses,
) -> RomOptStats {
    let mut st = RomOptStats::default();
    // Every pass removes executed steps, so it shifts the step number that a `step` source
    // (SRC_STEP) reads in all later instructions. Neither the transpiler nor the ZisK library
    // reads `step` today, but if any instruction in the ROM ever does, leave the ROM unchanged
    // rather than change a value the program can observe.
    if insts.values().any(|b| b.i.a_src == SRC_STEP) {
        return st;
    }
    // Each round runs every selected pass once. A rewrite can make another one possible (a fold
    // can leave a jump that threading then removes, or make a register dead for the next fold),
    // so the passes repeat until a round changes nothing, or MAX_ROUNDS is reached.
    for _ in 0..MAX_ROUNDS {
        let before = st;
        // Load + indirect jump fusion, only when asked for: it is not exact (see the module docs).
        if passes.load_jump_fusion {
            pass_load_jump(insts, &mut st);
        }
        // Constant and address folding share one pass, which computes liveness once for both.
        if passes.constant_folding || passes.address_folding {
            pass_fold(insts, passes.constant_folding, passes.address_folding, &mut st);
        }
        // Jump threading last, so it also threads the jumps the folds just merged.
        if passes.jump_threading {
            pass_jump_threading(insts, &mut st);
        }
        // No pass rewrote anything this round: the ROM is at its fixed point.
        if st == before {
            break;
        }
    }
    st
}

/// The 64-bit immediate of a, which the instruction keeps as two 32-bit halves.
fn imm_a(i: &ZiskInst) -> u64 {
    i.a_offset_imm0 | (i.a_use_sp_imm1 << 32)
}
/// The 64-bit immediate of b, which the instruction keeps as two 32-bit halves.
fn imm_b(i: &ZiskInst) -> u64 {
    i.b_offset_imm0 | (i.b_use_sp_imm1 << 32)
}
/// Makes a the 64-bit immediate `v`.
fn set_imm_a(i: &mut ZiskInst, v: u64) {
    i.a_src = SRC_IMM;
    i.a_offset_imm0 = v & 0xffff_ffff;
    i.a_use_sp_imm1 = v >> 32;
}
/// Makes b the 64-bit immediate `v`.
fn set_imm_b(i: &mut ZiskInst, v: u64) {
    i.b_src = SRC_IMM;
    i.b_offset_imm0 = v & 0xffff_ffff;
    i.b_use_sp_imm1 = v >> 32;
}

/// Main-trace registers the instruction reads (as a or b, including an indirection's base).
fn reads(i: &ZiskInst) -> u64 {
    let mut m = 0;
    // a read from a register; this is also the base of an indirect load or store.
    if i.a_src == SRC_REG {
        m |= 1 << i.a_offset_imm0;
    }
    // b read from a register (a store's value, or an operation's second operand).
    if i.b_src == SRC_REG {
        m |= 1 << i.b_offset_imm0;
    }
    m
}
/// Main-trace register the instruction writes, as a mask (empty when c goes elsewhere or nowhere).
fn writes(i: &ZiskInst) -> u64 {
    // Only a store to a register writes one; memory and indirect stores do not.
    if i.store == STORE_REG {
        1 << i.store_offset
    } else {
        0
    }
}

/// Ops whose only effect is (c, flag) = f(a, b): no precompile memory, no fcall state, and the
/// standard handler (a `write_log` flag has its own, which prints).
fn is_pure(i: &ZiskInst) -> bool {
    // An unknown opcode is taken as internal, as the emulator would run it.
    let op_type = ZiskOp::try_from_code(i.op).map(|o| o.op_type()).unwrap_or(OpType::Internal);
    // Not a precompile, not a debug print, and an operation computed from a and b alone.
    !i.is_precompiled
        && !i.write_log
        && matches!(
            op_type,
            OpType::Internal
                | OpType::Binary
                | OpType::BinaryE
                | OpType::ArithAm32
                | OpType::ArithA32
        )
}
/// Plain control flow: the next pc is pc + j1 (flag) or pc + j2, with no pc capture and no end.
fn is_plain(i: &ZiskInst) -> bool {
    // set_pc jumps to c, store_pc writes the return address (a call), end stops the program: none
    // of them may have its jump offsets or position changed.
    is_pure(i) && !i.set_pc && !i.store_pc && !i.end
}
/// An unconditional jump that does nothing else.
fn is_pure_jump(i: &ZiskInst) -> bool {
    // A `flag` (sets the flag, so it always takes j1) on two immediates that stores nothing:
    // skipping it changes nothing but the previous c, which pass_jump_threading checks. Not a
    // `write_log`, which is a flag that prints, and not an indirect jump, call or end.
    i.op == ZiskOp::FLAG
        && !i.write_log
        && i.a_src == SRC_IMM
        && i.b_src == SRC_IMM
        && i.store == STORE_NONE
        && !i.store_pc
        && !i.set_pc
        && !i.end
}

/// Static successors, or `None` when the next pc is computed (`set_pc`).
fn successors(i: &ZiskInst) -> Option<([u64; 2], usize)> {
    // Indirect jump (jalr, a return, a jump table): the target is only known at run time.
    if i.set_pc {
        return None;
    }
    // The program stops here: no successor.
    if i.end {
        return Some(([0; 2], 0));
    }
    let pc = i.paddr as i64;
    if !is_pure(i) {
        // External ops never raise the flag, and their j1 may carry an extended argument.
        return Some(([(pc + i.jmp_offset2) as u64, 0], 1));
    }
    // pc + j1 when the flag is set, pc + j2 otherwise: one successor when both are the same.
    Some((
        [(pc + i.jmp_offset1) as u64, (pc + i.jmp_offset2) as u64],
        if i.jmp_offset1 == i.jmp_offset2 { 1 } else { 2 },
    ))
}

/// The ROM's addresses in order, and each address's position among them.
struct Index {
    addrs: Vec<u64>,
    pos: HashMap<u64, usize>,
}

fn index(insts: &BTreeMap<u64, ZiskInstBuilder>) -> Index {
    let addrs: Vec<u64> = insts.keys().copied().collect();
    let pos = addrs.iter().enumerate().map(|(k, a)| (*a, k)).collect();
    Index { addrs, pos }
}

/// Backward liveness over the whole ROM; returns the registers live after each instruction. A
/// computed next pc, or a successor outside the ROM, makes every register live.
fn liveness(insts: &BTreeMap<u64, ZiskInstBuilder>, r: &Index) -> Vec<u64> {
    let n = r.addrs.len();
    let mut succ = Vec::with_capacity(n);
    let mut preds: Vec<Vec<usize>> = vec![vec![]; n];
    // Build the control-flow graph: each instruction's successors as positions, and the reverse
    // edges (predecessors) that the worklist below walks back along.
    for (k, a) in r.addrs.iter().enumerate() {
        let s = successors(&insts[a].i).and_then(|(targets, len)| {
            let mut ks = [0; 2];
            // Map each successor address to its position. A target that is not a ROM
            // instruction makes the whole result None (`?`), so the instruction is handled like
            // an indirect jump: everything live after it.
            for j in 0..len {
                ks[j] = *r.pos.get(&targets[j])?;
            }
            Some((ks, len))
        });
        // Only static successors give predecessor edges; for None there is nothing to record.
        if let Some((ks, len)) = &s {
            for &t in &ks[..*len] {
                preds[t].push(k);
            }
        }
        succ.push(s);
    }
    let rd: Vec<u64> = r.addrs.iter().map(|a| reads(&insts[a].i)).collect();
    let wr: Vec<u64> = r.addrs.iter().map(|a| writes(&insts[a].i)).collect();
    let mut live_in = vec![0u64; n];
    let mut live_out = vec![0u64; n];
    // Every instruction starts on the worklist; `queued` keeps each one on it at most once.
    let mut work: Vec<usize> = (0..n).collect();
    let mut queued = vec![true; n];
    // Standard fixed point: recompute an instruction's sets, and when its live-in grows, requeue
    // its predecessors, whose live-out depends on it. Sets only grow, so this terminates.
    while let Some(k) = work.pop() {
        queued[k] = false;
        // live_out = what any successor needs; all registers after an unknown next pc.
        let out = match &succ[k] {
            None => ALL_REGS,
            Some((ks, len)) => ks[..*len].iter().fold(0, |m, &t| m | live_in[t]),
        };
        live_out[k] = out;
        // live_in = what it reads, plus what is live after it and it does not overwrite.
        let inn = rd[k] | (out & !wr[k]);
        // Changed: the predecessors have to be looked at again.
        if inn != live_in[k] {
            live_in[k] = inn;
            for &p in &preds[k] {
                // Requeue a predecessor unless it is already waiting.
                if !queued[p] {
                    queued[p] = true;
                    work.push(p);
                }
            }
        }
    }
    live_out
}

/// Jump threading: retargets any plain jump or branch slot that lands on a pure jump.
fn pass_jump_threading(insts: &mut BTreeMap<u64, ZiskInstBuilder>, st: &mut RomOptStats) {
    // Follows a chain of pure jumps from `t` to the first address that is not one, or stops early
    // when skipping the next jump would not be exact. The bound of 16 stops a jump loop.
    let final_target = |insts: &BTreeMap<u64, ZiskInstBuilder>, mut t: u64| -> u64 {
        for _ in 0..16 {
            match insts.get(&t) {
                // t is a pure jump at an even address that is a whole instruction by itself: look
                // at where it goes (a flag always takes j1). An odd address is an internal step of
                // a multi-step instruction, which must be executed, never skipped.
                Some(u) if t & 1 == 0 && u.i.next_internal_inst.is_none() && is_pure_jump(&u.i) => {
                    let nt = (t as i64 + u.i.jmp_offset1) as u64;
                    // Skipping the jump changes what the next instruction sees as the previous c,
                    // so only skip it when the destination reads neither operand from c. A jump to
                    // itself, to an odd address (an internal step, entered only through its own
                    // instruction), or to an address outside the ROM also stops the chain.
                    match insts.get(&nt) {
                        Some(v)
                            if nt & 1 == 0
                                && v.i.a_src != SRC_C
                                && v.i.b_src != SRC_C
                                && nt != t =>
                        {
                            t = nt
                        }
                        _ => break,
                    }
                }
                // Not a pure jump (or not in the ROM): this is the final target.
                _ => break,
            }
        }
        t
    };
    let addrs: Vec<u64> = insts.keys().copied().collect();
    for a in addrs {
        let i = &insts[&a].i;
        // Only instructions whose jump offsets can be changed freely: plain ones, and pure jumps
        // themselves (which shortens chains of jumps). Never an internal step (odd address) or an
        // instruction that continues into one: their offsets are the route through the steps of
        // a multi-step instruction.
        if a & 1 != 0 || i.next_internal_inst.is_some() || (!is_plain(i) && !is_pure_jump(i)) {
            continue;
        }
        // Thread both slots: j1 (flag set, e.g. a taken branch) and j2 (flag clear).
        let (j1, j2) = (i.jmp_offset1, i.jmp_offset2);
        let t1 = final_target(insts, (a as i64 + j1) as u64);
        let t2 = final_target(insts, (a as i64 + j2) as u64);
        let (n1, n2) = (t1 as i64 - a as i64, t2 as i64 - a as i64);
        // Either slot now lands further on: write the new offsets.
        if n1 != j1 || n2 != j2 {
            let m = &mut insts.get_mut(&a).unwrap().i;
            m.jmp_offset1 = n1;
            m.jmp_offset2 = n2;
            st.jump_threading += 1;
        }
    }
}

/// Constant and address folding of a pair X -> Y into one instruction at X (see the module docs).
fn pass_fold(insts: &mut BTreeMap<u64, ZiskInstBuilder>, cf: bool, af: bool, st: &mut RomOptStats) {
    let r = index(insts);
    let live_out = liveness(insts, &r);
    // The rewrites are collected and applied after the scan, so every decision in this round
    // sees the ROM (and the liveness) as they were when the round started.
    let mut merged: Vec<(u64, ZiskInst)> = vec![];
    // Addresses already used by a fold this round, as X or as Y: a later pair that shares one
    // waits for the next round, when liveness is recomputed.
    let mut touched: HashSet<u64> = HashSet::new();
    // Try every instruction as X.
    for &a in &r.addrs {
        let x = &insts[&a].i;
        // X must be:
        // - at an even address: odd ones are internal steps of a multi-step instruction;
        // - the last step of its instruction (no internal continuation);
        // - plain, with a single successor (j1 == j2), so Y is the only way on;
        // - writing a register (the one Y should read);
        // - not reading c, which the merged instruction (now after X's predecessor) would not see;
        // - not a load: its result comes from memory, so it cannot become an operand of Y.
        if a & 1 != 0
            || x.next_internal_inst.is_some()
            || !is_plain(x)
            || x.jmp_offset1 != x.jmp_offset2
            || x.store != STORE_REG
            || x.a_src == SRC_C
            || x.b_src == SRC_C
            || x.b_src == SRC_IND
        {
            continue;
        }
        // Y is X's only successor, which must be a ROM instruction at an even address: an odd one
        // is an internal step, owned by its own instruction and never moved out of it.
        let d = x.jmp_offset2;
        let ya = (a as i64 + d) as u64;
        if ya & 1 != 0 {
            continue;
        }
        let Some(&yk) = r.pos.get(&ya) else { continue };
        // X or Y was already folded this round: liveness for it is stale.
        if touched.contains(&a) || touched.contains(&ya) {
            continue;
        }
        let y = &insts[&ya].i;
        // Y must be a single, pure step that moves to X's address unchanged in behaviour: no
        // internal continuation, no indirect jump, no call, no end, and no read of c, because
        // at X's address the previous c would be X's predecessor's instead of X's.
        if y.next_internal_inst.is_some()
            || y.set_pc
            || y.store_pc
            || y.end
            || !is_pure(y)
            || y.a_src == SRC_C
            || y.b_src == SRC_C
        {
            continue;
        }
        let rx = x.store_offset as u64;
        let rbit = 1u64 << rx;
        // Y does not read X's register: nothing to fold.
        if reads(y) & rbit == 0 {
            continue;
        }
        // The merged instruction no longer writes rX, so rX must be dead after Y, unless Y writes
        // rX itself (then the merged one writes it too).
        if live_out[yk] & rbit != 0 && writes(y) & rbit == 0 {
            continue;
        }

        // The merged instruction: Y's operation and destination, at X's address.
        let mut f = y.clone();
        f.paddr = a;
        // The ROM trace places each instruction at the row `index`: the merged one takes X's row.
        f.index = x.index;
        // Y's jump offsets were relative to Y; from X they are d further.
        f.jmp_offset1 += d;
        f.jmp_offset2 += d;
        f.riscv_inst = x.riscv_inst.clone();
        f.verbose = format!("{} + {}", x.verbose, y.verbose);
        // Y uses a as an address base: as the base of an indirect load (b) or of an indirect store.
        let uses_ind_a = y.b_src == SRC_IND || y.store == STORE_IND;
        let mut ok = false;
        // Constant folding: X loads an immediate (copyb ignores a, so b is X's whole result).
        if cf && x.op == ZiskOp::COPYB && x.b_src == SRC_IMM {
            let v = imm_b(x);
            let a_is_r = y.a_src == SRC_REG && y.a_offset_imm0 == rx;
            let b_is_r = y.b_src == SRC_REG && y.b_offset_imm0 == rx;
            // An immediate address base is kept to 32 bits: a wider constant used as Y's base
            // is left alone.
            if !(a_is_r && uses_ind_a && v >> 32 != 0) {
                // Put the constant wherever Y read rX: a, b, or both.
                if a_is_r {
                    set_imm_a(&mut f, v);
                }
                if b_is_r {
                    set_imm_b(&mut f, v);
                }
                ok = true;
                st.constant_folding += 1;
                // Both operands constant: evaluate now when the flag cannot matter. That needs no
                // memory access (the address base stays), not a flag op (whose only effect is the
                // flag), and one successor, so nothing depends on the flag.
                if f.a_src == SRC_IMM
                    && f.b_src == SRC_IMM
                    && f.op != ZiskOp::FLAG
                    && !uses_ind_a
                    && f.jmp_offset1 == f.jmp_offset2
                {
                    // Compute c now and replace the operation by a copyb of it; the builder resets
                    // the op's handler and properties.
                    let (c, _) = ZiskOp::execute(f.op, imm_a(&f), imm_b(&f));
                    let mut zib = ZiskInstBuilder { i: f };
                    zib.op("copyb").unwrap();
                    f = zib.i;
                    set_imm_a(&mut f, 0);
                    set_imm_b(&mut f, c);
                    st.constant_evaluated += 1;
                }
            }
        } else if af
            // Address folding: X is `add rX, base, k` and Y loads or stores through rX.
            && x.op == ZiskOp::ADD
            // Replacing a must keep the operation's result, not just the address: only ops that
            // ignore a.
            && matches!(
                y.op,
                ZiskOp::COPYB | ZiskOp::SIGNEXTEND_B | ZiskOp::SIGNEXTEND_H | ZiskOp::SIGNEXTEND_W
            )
            // X adds an immediate to a register.
            && x.a_src == SRC_REG
            && x.b_src == SRC_IMM
            // Y uses rX as its address base (a), and not as its value (b).
            && uses_ind_a
            && y.a_src == SRC_REG
            && y.a_offset_imm0 == rx
            && !(y.b_src == SRC_REG && y.b_offset_imm0 == rx)
        {
            let k = imm_b(x) as i64;
            // The added constant must be a signed 32-bit offset, like the ones the transpiler
            // produces for loads and stores.
            if (-(1i64 << 31)..(1i64 << 31)).contains(&k) {
                // Y now uses X's base register, with k added to its offset.
                f.a_offset_imm0 = x.a_offset_imm0;
                // Offsets are 64-bit two's complement, as the transpiler encodes them.
                // A load: the offset is in b.
                if y.b_src == SRC_IND {
                    f.b_offset_imm0 = (y.b_offset_imm0 as i64).wrapping_add(k) as u64;
                }
                // A store: the offset is in the store.
                if y.store == STORE_IND {
                    f.store_offset = y.store_offset.wrapping_add(k);
                }
                ok = true;
                st.address_folding += 1;
            }
        }
        // One of the folds applied: record it and keep X and Y out of the rest of this round.
        if ok {
            touched.insert(a);
            touched.insert(ya);
            merged.push((a, f));
        }
    }
    // Replace each X by its merged instruction; Y keeps its own place.
    for (a, f) in merged {
        insts.get_mut(&a).unwrap().i = f;
    }
}

/// Load + indirect jump: `ld t, k(base); jr t` (jalr with rd = x0) becomes one instruction at the
/// load that writes t and jumps to it. Drops jalr's `& ~1`.
fn pass_load_jump(insts: &mut BTreeMap<u64, ZiskInstBuilder>, st: &mut RomOptStats) {
    let addrs: Vec<u64> = insts.keys().copied().collect();
    let mut merged = vec![];
    // Try every instruction as the load X.
    for a in addrs {
        let x = &insts[&a].i;
        // X must be an `ld t, k(base)` at an even address, the last step of its instruction,
        // plain and with a single successor: a copyb of 8 bytes from [base + k] into a register.
        if a & 1 != 0
            || x.next_internal_inst.is_some()
            || !is_plain(x)
            || x.jmp_offset1 != x.jmp_offset2
            || x.op != ZiskOp::COPYB
            || x.b_src != SRC_IND
            || x.ind_width != 8
            || x.store != STORE_REG
            || x.a_src != SRC_REG
        {
            continue;
        }
        // Y is X's successor, which must be a ROM instruction.
        let ya = (a as i64 + x.jmp_offset2) as u64;
        let Some(y) = insts.get(&ya).map(|b| &b.i) else { continue };
        let t = x.store_offset as u64;
        // Y must be exactly a `jr t`, as the transpiler lowers it: `and(!1, t)` with set_pc, which
        // stores nothing and is not a call (store_pc), in one step.
        let is_jr = y.set_pc
            && !y.store_pc
            && y.store == STORE_NONE
            && y.op == ZiskOp::AND
            && y.a_src == SRC_IMM
            && imm_a(y) == 0xffff_ffff_ffff_fffe
            && y.b_src == SRC_REG
            && y.b_offset_imm0 == t
            && y.next_internal_inst.is_none();
        if !is_jr {
            continue;
        }
        // The fused instruction: X's load into t, then a jump to c (the loaded value). Y's offsets
        // move to X's address.
        let mut f = x.clone();
        f.set_pc = true;
        f.jmp_offset1 = y.jmp_offset1;
        f.jmp_offset2 = y.jmp_offset2 + (ya as i64 - a as i64);
        merged.push((a, f));
    }
    // Replace each load by its fused instruction; the jr keeps its own place.
    for (a, f) in merged {
        insts.get_mut(&a).unwrap().i = f;
        st.load_jump_fusion += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pure instruction at `pc` that falls through to pc + 4.
    fn inst(pc: u64, op: &str) -> ZiskInstBuilder {
        let mut b = ZiskInstBuilder::default();
        b.i.paddr = pc;
        b.i.index = pc;
        b.src_a("imm", 0, false);
        b.src_b("imm", 0, false);
        b.op(op).unwrap();
        b.j(4, 4);
        b
    }
    fn end(pc: u64) -> ZiskInstBuilder {
        let mut b = inst(pc, "copyb");
        b.i.end = true;
        b
    }
    /// An end that reads `reg`, so the register is live up to it.
    fn end_reading(pc: u64, reg: u64) -> ZiskInstBuilder {
        let mut b = end(pc);
        b.src_b("reg", reg, false);
        b
    }

    /// A plain jump (`j`) is absorbed by the instruction before it: the branch goes straight to
    /// the jump's target, both from its taken and its fall-through slot.
    #[test]
    fn jump_threading_skips_a_plain_jump() {
        let mut br = inst(100, "eq");
        br.src_a("reg", 5, false);
        br.j(8, 4); // taken -> 108, else -> 104
        let mut j = inst(104, "flag");
        j.j(96, 96); // -> 200
        let mut insts = BTreeMap::from([(100, br), (104, j), (108, end(108)), (200, end(200))]);
        let mut st = RomOptStats::default();
        pass_jump_threading(&mut insts, &mut st);
        assert_eq!(st.jump_threading, 1);
        assert_eq!((insts[&100].i.jmp_offset1, insts[&100].i.jmp_offset2), (8, 100));
    }

    /// Jump threading leaves multi-step instructions alone: an instruction that continues into an
    /// internal step keeps its offsets, and an internal (odd) pure jump is never skipped.
    #[test]
    fn jump_threading_keeps_internal_steps() {
        // 100 continues into an internal flag at 101, which jumps on to 104.
        let mut x = inst(100, "add");
        x.j(1, 1);
        x.i.next_internal_inst = Some(101);
        let mut internal = inst(101, "flag");
        internal.j(3, 3);
        let mut insts = BTreeMap::from([(100, x.clone()), (101, internal), (104, end(104))]);
        let mut st = RomOptStats::default();
        pass_jump_threading(&mut insts, &mut st);
        assert_eq!(st.jump_threading, 0);
        assert_eq!((insts[&100].i.jmp_offset1, insts[&100].i.jmp_offset2), (1, 1));

        // An even plain instruction whose successor is an odd pure jump does not skip it either.
        let mut odd_flag = inst(201, "flag");
        odd_flag.j(3, 3);
        let mut y = inst(196, "add");
        y.j(5, 5);
        let mut insts = BTreeMap::from([(196, y), (201, odd_flag), (204, end(204))]);
        pass_jump_threading(&mut insts, &mut st);
        assert_eq!(st.jump_threading, 0);
        assert_eq!(insts[&196].i.jmp_offset1, 5);

        // An even pure jump whose target is odd is not skipped: the caller would otherwise jump
        // straight into an internal step.
        let mut to_odd = inst(304, "flag");
        to_odd.j(-3, -3); // -> 301
        let mut insts = BTreeMap::from([
            (300, inst(300, "add")),
            (301, end(301)),
            (304, to_odd),
            (296, {
                let mut z = inst(296, "add");
                z.j(8, 8); // -> 304
                z
            }),
        ]);
        pass_jump_threading(&mut insts, &mut st);
        assert_eq!(insts[&296].i.jmp_offset1, 8);
    }

    /// A ROM that reads the step counter is left unchanged: removing a step would change the
    /// value every later `step` read observes.
    #[test]
    fn a_rom_that_reads_step_is_not_optimized() {
        let mut br = inst(100, "eq");
        br.src_a("reg", 5, false);
        br.j(8, 4);
        let mut j = inst(104, "flag");
        j.j(96, 96);
        let mut reads_step = inst(200, "add");
        reads_step.i.a_src = SRC_STEP;
        reads_step.i.end = true;
        let mut insts = BTreeMap::from([(100, br), (104, j), (108, end(108)), (200, reads_step)]);
        let st = optimize_rom(&mut insts, RomOptPasses::default());
        assert_eq!(st, RomOptStats::default());
        assert_eq!((insts[&100].i.jmp_offset1, insts[&100].i.jmp_offset2), (8, 4));
    }

    /// A jump that is followed by an instruction reading the previous c, an instruction that
    /// stores the pc (a call), or a `write_log` flag is not threaded.
    #[test]
    fn jump_threading_keeps_what_it_must() {
        // The target reads c: skipping the jump would change what it sees.
        let mut j = inst(104, "flag");
        j.j(96, 96);
        let mut reads_c = end(200);
        reads_c.i.b_src = SRC_C;
        let mut insts =
            BTreeMap::from([(100, inst(100, "copyb")), (104, j.clone()), (200, reads_c)]);
        let mut st = RomOptStats::default();
        pass_jump_threading(&mut insts, &mut st);
        assert_eq!(st.jump_threading, 0);

        // A call stores its own pc + j2: retargeting it would change the return address.
        let mut call = inst(100, "flag");
        call.i.store_pc = true;
        let mut insts = BTreeMap::from([(100, call), (104, j.clone()), (200, end(200))]);
        pass_jump_threading(&mut insts, &mut st);
        assert_eq!(st.jump_threading, 0);

        // write_log with two immediate arguments looks like a jump but prints.
        let mut log = inst(104, "flag");
        log.i.write_log = true;
        log.j(96, 96);
        let mut insts = BTreeMap::from([(100, inst(100, "copyb")), (104, log), (200, end(200))]);
        pass_jump_threading(&mut insts, &mut st);
        assert_eq!(st.jump_threading, 0);
    }

    /// `li r5, 7; add r6, r5, r4` becomes `add r6, 7, r4` at the li's address and row, when r5 is
    /// dead afterwards; with r5 still live it stays.
    #[test]
    fn constant_folding_needs_the_register_dead() {
        for live in [false, true] {
            let mut li = inst(100, "copyb");
            li.src_b("imm", 7, false);
            li.store("reg", 5, false, false);
            let mut add = inst(104, "add");
            add.src_a("reg", 5, false);
            add.src_b("reg", 4, false);
            add.store("reg", 6, false, false);
            let tail = if live { end_reading(108, 5) } else { end_reading(108, 6) };
            let mut insts = BTreeMap::from([(100, li), (104, add), (108, tail)]);
            let mut st = RomOptStats::default();
            pass_fold(&mut insts, true, false, &mut st);
            assert_eq!(st.constant_folding, usize::from(!live), "live={live}");
            let x = &insts[&100].i;
            if live {
                assert_eq!(x.op, ZiskOp::COPYB);
            } else {
                assert_eq!((x.op, x.a_src, imm_a(x)), (ZiskOp::ADD, SRC_IMM, 7));
                assert_eq!((x.b_src, x.b_offset_imm0), (SRC_REG, 4));
                assert_eq!((x.store, x.store_offset), (STORE_REG, 6));
                assert_eq!((x.paddr, x.index), (100, 100));
                assert_eq!((x.jmp_offset1, x.jmp_offset2), (8, 8));
                // The add itself stays for any other predecessor.
                assert_eq!(insts[&104].i.op, ZiskOp::ADD);
            }
        }
    }

    /// A successor at an odd address is an internal step: it is never folded into X.
    #[test]
    fn constant_folding_skips_an_odd_successor() {
        let mut li = inst(100, "copyb");
        li.src_b("imm", 7, false);
        li.store("reg", 5, false, false);
        li.j(5, 5); // -> 105
        let mut add = inst(105, "add");
        add.src_a("reg", 5, false);
        add.src_b("reg", 4, false);
        add.store("reg", 6, false, false);
        add.j(3, 3); // -> 108
        let mut insts = BTreeMap::from([(100, li), (105, add), (108, end_reading(108, 6))]);
        let mut st = RomOptStats::default();
        pass_fold(&mut insts, true, false, &mut st);
        assert_eq!(st.constant_folding, 0);
        assert_eq!(insts[&100].i.op, ZiskOp::COPYB);
    }

    /// Two constants: the operation is evaluated at transpile time into a copyb.
    #[test]
    fn constant_folding_evaluates_two_constants() {
        let mut li = inst(100, "copyb");
        li.src_b("imm", 0x30, false);
        li.store("reg", 5, false, false);
        let mut sub = inst(104, "sub");
        sub.src_a("reg", 5, false);
        sub.src_b("imm", 0x8, false);
        sub.store("reg", 6, false, false);
        let mut insts = BTreeMap::from([(100, li), (104, sub), (108, end_reading(108, 6))]);
        let mut st = RomOptStats::default();
        pass_fold(&mut insts, true, false, &mut st);
        assert_eq!((st.constant_folding, st.constant_evaluated), (1, 1));
        let x = &insts[&100].i;
        assert_eq!((x.op, x.b_src, imm_b(x)), (ZiskOp::COPYB, SRC_IMM, 0x28));
        assert_eq!((x.store, x.store_offset), (STORE_REG, 6));
    }

    /// Address folding keeps the loaded or stored value and the address, and only applies to the
    /// ops that ignore a.
    #[test]
    fn address_folding_preserves_operation_and_address() {
        for op in ["copyb", "signextend_b", "signextend_h", "signextend_w", "add", "eq"] {
            for load in [false, true] {
                for offset in [-8i64, 8] {
                    let mut x = inst(100, "add");
                    x.src_a("reg", 1, false);
                    x.src_b("imm", offset as u64, false);
                    x.store("reg", 2, false, false);
                    let mut y = inst(104, op);
                    y.src_a("reg", 2, false);
                    if load {
                        y.src_b("ind", 16, false);
                        y.store("reg", 3, false, false);
                    } else {
                        y.src_b("imm", 0x8000_0080, false);
                        y.store("ind", 16, false, false);
                    }
                    y.ind_width(8);
                    let original = y.i.clone();
                    let mut insts = BTreeMap::from([(100, x), (104, y), (108, end(108))]);
                    let mut st = RomOptStats::default();
                    pass_fold(&mut insts, false, true, &mut st);

                    let should_fold = !matches!(op, "add" | "eq");
                    assert_eq!(st.address_folding, usize::from(should_fold), "{op} load={load}");
                    if should_fold {
                        let f = &insts[&100].i;
                        let base = 1000u64;
                        let original_a = base.wrapping_add(offset as u64);
                        let off = if load { f.b_offset_imm0 } else { f.store_offset as u64 };
                        assert_eq!(f.a_offset_imm0, 1);
                        assert_eq!(base.wrapping_add(off), original_a + 16);
                        assert_eq!(
                            ZiskOp::execute(original.op, original_a, 0x8000_0080),
                            ZiskOp::execute(f.op, base, 0x8000_0080)
                        );
                    } else {
                        assert_eq!(insts[&100].i.op, ZiskOp::ADD);
                    }
                }
            }
        }
    }

    /// Liveness follows both branch slots, makes everything live after an indirect jump or a jump
    /// out of the ROM, and follows only j2 for an external op.
    #[test]
    fn liveness_follows_the_successors() {
        for (j1, j2, indirect, precompiled, expected) in [
            (4, 4, false, false, 1 << 1),
            (4, 8, false, false, (1 << 1) | (1 << 2)),
            (4, 8, true, false, ALL_REGS),
            (4, 12, false, false, ALL_REGS),
            (12345, 8, false, true, 1 << 2),
        ] {
            let mut branch = inst(100, "eq");
            branch.j(j1, j2);
            branch.i.set_pc = indirect;
            branch.i.is_precompiled = precompiled;
            let insts = BTreeMap::from([
                (100, branch),
                (104, end_reading(104, 1)),
                (108, end_reading(108, 2)),
            ]);
            assert_eq!(liveness(&insts, &index(&insts)), vec![expected, 0, 0]);
        }
    }
}
