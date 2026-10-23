/-
  sail-zisk's semantics for the instructions the transpiler's spec builds
  with `Transpiler.mk` from register or immediate operands: any pure op, its
  a and b each a register or an immediate (a register x0 is the immediate 0,
  as the builder makes it), its result to rd or, for x0, nowhere; on to
  pc + 4. One lemma for every such form, x0 forms included.
-/
import Transpiler.Spec
import Equiv.ZiskJump
import Equiv.ZiskLoadW
import Equiv.ZiskStoreW

set_option linter.unusedSimpArgs false

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- A register's value as an operand: x0 reads 0. -/
def regv (xs : Vector (BitVec 64) 40) (r : Nat) : BitVec 64 := if r = 0 then 0 else xs[(r : Int)]!

/-- The a source `ra` reads v: an immediate v, or a register 1..31 holding v. -/
def ASrc (ra : a_src × BitVec 64) (xs : Vector (BitVec 64) 40) (v : BitVec 64) : Prop :=
  ra = (.A_IMM, v) ∨ ∃ r, 1 ≤ r ∧ r < 32 ∧ ra = (.A_REG, BitVec.ofNat 64 r) ∧ xs[(r : Int)]! = v

/-- The b source `rb` reads v. -/
def BSrc (rb : b_src × BitVec 64) (xs : Vector (BitVec 64) 40) (v : BitVec 64) : Prop :=
  rb = (.B_IMM, v) ∨ ∃ r, 1 ≤ r ∧ r < 32 ∧ rb = (.B_REG, BitVec.ofNat 64 r) ∧ xs[(r : Int)]! = v

theorem srcA_ASrc (xs : Vector (BitVec 64) 40) (r : Nat) (h : r < 32) :
    ASrc (Transpiler.srcA r) xs (regv xs r) := by
  unfold Transpiler.srcA regv
  by_cases h0 : r = 0
  · left; simp [h0]
  · right; exact ⟨r, by omega, h, by simp [h0], by simp [h0]⟩

theorem srcB_BSrc (xs : Vector (BitVec 64) 40) (r : Nat) (h : r < 32) :
    BSrc (Transpiler.srcB r) xs (regv xs r) := by
  unfold Transpiler.srcB regv
  by_cases h0 : r = 0
  · left; simp [h0]
  · right; exact ⟨r, by omega, h, by simp [h0], by simp [h0]⟩

set_option maxHeartbeats 4000000 in
/-- Any pure op on register-or-immediate operands, its result to rd (none
    for x0), on to pc + j1 if its flag is set, else pc + j2. -/
theorem execute_mkj (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64) (op : zisk_op)
    (hop : PureOp op) (rd : Nat) (hd : rd < 32) (j1 j2 : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va vb : BitVec 64) (ha : ASrc ra xs va) (hb : BSrc rb xs vb) :
    let r := zisk_exec op va vb
    (zisk_execute (Transpiler.mk ra rb op (Transpiler.storeReg rd) j1 j2)).run s = .ok false
      { s with regs := ((((linkRegs s.regs xs rd r.1).insert Zc r.1).insert Zflag r.2).insert
          Step (BitVec.addInt st 1)).insert PC (if r.2 then pc + j1 else pc + j2) } := by
  intro r
  have hm := hop.mem
  have hp := hop.notPre
  have hh := hop.notHalt ()
  have ld : (rd : Int) < 40 := by omega
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩ <;> rcases hb with rfl | ⟨r2, h2a, h2b, rfl, hv2⟩ <;>
  by_cases h0 : rd = 0 <;>
  · (try subst hv1)
    (try subst hv2)
    (try have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩)
    (try have e2 : r2 ≠ 0 ∧ (r2 : Int) < 40 := ⟨by omega, by omega⟩)
    simp (disch := omega) [zisk_execute, Transpiler.mk, Transpiler.storeReg, h0, source_a, source_b, rX, wX,
      store_c, value_to_store, next_pc, hm, hp, linkRegs, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert,
      writeReg_run, ld, reg_index, *]
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *])
    all_goals
      simp only [r]
      cases hf : (zisk_exec op _ _).snd <;> simp only [hf, if_true, if_false, Bool.false_eq_true, run_map'] <;>
        rw [readReg_run'] <;> simp [Std.ExtDHashMap.get?_insert, hpc] <;> simp (disch := omega) [hf]

/-- Any pure op on register-or-immediate operands, its result to rd (none
    for x0), on to pc + 4. -/
theorem execute_mk (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64) (op : zisk_op)
    (hop : PureOp op) (rd : Nat) (hd : rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va vb : BitVec 64) (ha : ASrc ra xs va) (hb : BSrc rb xs vb) :
    let r := zisk_exec op va vb
    (zisk_execute (Transpiler.mk ra rb op (Transpiler.storeReg rd) 4 4)).run s = .ok false
      { s with regs := ((((linkRegs s.regs xs rd r.1).insert Zc r.1).insert Zflag r.2).insert
          Step (BitVec.addInt st 1)).insert PC (pc + 4) } := by
  intro r
  have := execute_mkj s ra rb op hop rd hd 4 4 xs pc st hx hpc hst va vb ha hb
  simp only [ite_self] at this
  exact this

set_option maxHeartbeats 4000000 in
/-- A load as the spec builds it, any registers: op of the base and the w
    bytes at base + off, zero-extended, into rd (none for x0); on to pc + 4. -/
theorem execute_load_mk (s : ZState) (ra : a_src × BitVec 64) (op : zisk_op) (hop : PureOp op)
    (rd : Nat) (hd : rd < 32) (off : BitVec 64) (w : Nat)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va : BitVec 64) (ha : ASrc ra xs va) (v : BitVec (8 * w))
    (hrb : readable (va + off) w = true) (hnr : hits_registers (va + off) w = false)
    (hin : (va + off == INPUT_ADDR && w == 8) = false)
    (hm : Equiv.memRead s.mem w (va + off).toNat = some v) :
    let r := zisk_exec op va (v.setWidth 64)
    (zisk_execute (Transpiler.mk ra (.B_IND, off) op (Transpiler.storeReg rd) 4 4 (w := w))).run s = .ok false
      { s with regs := ((((linkRegs s.regs xs rd r.1).insert Zc r.1).insert Zflag r.2).insert
          Step (BitVec.addInt st 1)).insert PC (pc + 4) } := by
  intro r
  have hm' := hop.mem
  have hp := hop.notPre
  have hh := hop.notHalt ()
  have ld : (rd : Int) < 40 := by omega
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩ <;> by_cases h0 : rd = 0 <;>
  · (try subst hv1)
    (try have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩)
    simp (disch := omega) [zisk_execute, Transpiler.mk, Transpiler.storeReg, h0, source_a, source_b, rX, wX,
      store_c, value_to_store, next_pc, hm', hp, linkRegs, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert,
      writeReg_run, ld, reg_index, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only)
      | (rw [read_mem_run s _ w v (by simpa (disch := omega) using hrb) (by simpa (disch := omega) using hnr)
            (by simpa (disch := omega) using hin) (by simpa (disch := omega) using hm)]; try dsimp only)
      | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *]))
    all_goals simp (disch := omega) [r, _root_.Zisk.Functions.zero_extend, Sail.BitVec.zeroExtend, *]

set_option maxHeartbeats 4000000 in
/-- A store as the spec builds it, any registers: the low w bytes of the
    value operand to base + off; on to pc + 4. -/
theorem execute_store_mk (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64)
    (off : BitVec 64) (w : Nat)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va vb : BitVec 64) (ha : ASrc ra xs va) (hb : BSrc rb xs vb)
    (hwr : writable (va + off) w = true) (hnr : hits_registers (va + off) w = false) :
    (zisk_execute (Transpiler.mk ra rb (.ZOP_COPYB ()) (.ST_IND, off, false) 4 4 (w := w))).run s = .ok false
      { s with
        regs := (((s.regs.insert Zc vb).insert Zflag false).insert Step (BitVec.addInt st 1)).insert PC (pc + 4)
        mem := Equiv.memWrite (n := w) s.mem (va + off).toNat (vb.setWidth (8 * w)) } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  have ht : ∀ x : BitVec 64, BitVec.setWidth (8 * w) (Sail.BitVec.truncate x (8 * (w : Int)).toNat)
      = x.setWidth (8 * w) := by
    intro x; apply BitVec.eq_of_getLsbD_eq; intro i hi; have hi2 : (i : Int) < 8 * (w : Int) := by omega
    simp [Sail.BitVec.truncate, hi, hi2]
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩ <;> rcases hb with rfl | ⟨r2, h2a, h2b, rfl, hv2⟩ <;>
  · (try subst hv1)
    (try subst hv2)
    (try have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩)
    (try have e2 : r2 ≠ 0 ∧ (r2 : Int) < 40 := ⟨by omega, by omega⟩)
    simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store,
      next_pc, hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run,
      reg_index, ht, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only)
      | (rw [write_mem_run s _ w _ (by simpa (disch := omega) using hwr) (by simpa (disch := omega) using hnr)];
          try dsimp only)
      | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *]))
    all_goals simp (disch := omega) [BitVec.toNat_add, *]

end Equiv.Zisk
