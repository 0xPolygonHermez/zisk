/-
  sail-zisk's semantics for the ZisK instructions an AMO is made of
  (Transpiler/Spec.lean, `amo`): the load and the store as for loads and
  stores (ZiskSpec.lean), but on to any address (an internal one), the load
  to any of the 40 registers; and the two that read `c`, the previous
  instruction's result: the op, with c as a, and the store of c.
-/
import Equiv.ZiskSpec

set_option linter.unusedSimpArgs false

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

set_option maxHeartbeats 4000000 in
/-- A load as the spec builds it, any registers: op of the base and the w
    bytes at base + off, zero-extended, into rd (none for x0; any of the
    40); on to pc + j. -/
theorem execute_load_mkj (s : ZState) (ra : a_src × BitVec 64) (op : zisk_op) (hop : PureOp op)
    (rd : Nat) (hd : rd < 40) (j : BitVec 64) (off : BitVec 64) (w : Nat)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va : BitVec 64) (ha : ASrc ra xs va) (v : BitVec (8 * w))
    (hrb : readable (va + off) w = true) (hnr : hits_registers (va + off) w = false)
    (hin : (va + off == INPUT_ADDR && w == 8) = false)
    (hm : Equiv.memRead s.mem w (va + off).toNat = some v) :
    let r := zisk_exec op va (v.setWidth 64)
    (zisk_execute (Transpiler.mk ra (.B_IND, off) op (Transpiler.storeReg rd) j j (w := w))).run s = .ok false
      { s with regs := ((((linkRegs s.regs xs rd r.1).insert Zc r.1).insert Zflag r.2).insert
          Step (BitVec.addInt st 1)).insert PC (pc + j) } := by
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
    value operand to base + off; on to pc + j. -/
theorem execute_store_mkj (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64)
    (off : BitVec 64) (w : Nat) (j : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va vb : BitVec 64) (ha : ASrc ra xs va) (hb : BSrc rb xs vb)
    (hwr : writable (va + off) w = true) (hnr : hits_registers (va + off) w = false) :
    (zisk_execute (Transpiler.mk ra rb (.ZOP_COPYB ()) (.ST_IND, off, false) j j (w := w))).run s = .ok false
      { s with
        regs := (((s.regs.insert Zc vb).insert Zflag false).insert Step (BitVec.addInt st 1)).insert PC (pc + j)
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

set_option maxHeartbeats 4000000 in
/-- A pure op with c, the previous result, as a, and a register or an
    immediate as b; no store; on to pc + j. -/
theorem execute_cop (s : ZState) (rb : b_src × BitVec 64) (op : zisk_op) (hop : PureOp op) (j : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st c : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (hc : s.regs.get? Zc = some c) (vb : BitVec 64) (hb : BSrc rb xs vb) :
    let r := zisk_exec op c vb
    (zisk_execute (Transpiler.mk (.A_C, 0) rb op (.ST_NONE, 0, false) j j)).run s = .ok false
      { s with regs := (((s.regs.insert Zc r.1).insert Zflag r.2).insert Step (BitVec.addInt st 1)).insert PC (pc + j) } := by
  intro r
  have hm := hop.mem
  have hp := hop.notPre
  have hh := hop.notHalt ()
  rcases hb with rfl | ⟨r2, h2a, h2b, rfl, hv2⟩ <;>
  · (try subst hv2)
    (try have e2 : r2 ≠ 0 ∧ (r2 : Int) < 40 := ⟨by omega, by omega⟩)
    simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store,
      next_pc, hm, hp, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run, reg_index, *]
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, hc, *])
    all_goals
      simp only [r]
      try rfl
    all_goals
      rw [getElem!_pos xs ((r2 : Nat) : Int) (by simp; omega)]

set_option maxHeartbeats 4000000 in
/-- The store of c, the previous result: its low w bytes to base + off; on
    to pc + j. -/
theorem execute_store_c (s : ZState) (ra : a_src × BitVec 64)
    (off : BitVec 64) (w : Nat) (j : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va vb : BitVec 64) (ha : ASrc ra xs va) (hc : s.regs.get? Zc = some vb)
    (hwr : writable (va + off) w = true) (hnr : hits_registers (va + off) w = false) :
    (zisk_execute (Transpiler.mk ra (.B_C, 0) (.ZOP_COPYB ()) (.ST_IND, off, false) j j (w := w))).run s = .ok false
      { s with
        regs := (((s.regs.insert Zc vb).insert Zflag false).insert Step (BitVec.addInt st 1)).insert PC (pc + j)
        mem := Equiv.memWrite (n := w) s.mem (va + off).toNat (vb.setWidth (8 * w)) } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  have ht : ∀ x : BitVec 64, BitVec.setWidth (8 * w) (Sail.BitVec.truncate x (8 * (w : Int)).toNat)
      = x.setWidth (8 * w) := by
    intro x; apply BitVec.eq_of_getLsbD_eq; intro i hi; have hi2 : (i : Int) < 8 * (w : Int) := by omega
    simp [Sail.BitVec.truncate, hi, hi2]
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩ <;>
  · (try subst hv1)
    (try have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩)
    simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store,
      next_pc, hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run,
      reg_index, ht, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only)
      | (rw [write_mem_run s _ w _ (by simpa (disch := omega) using hwr) (by simpa (disch := omega) using hnr)];
          try dsimp only)
      | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, hc, *]))
    all_goals simp (disch := omega) [BitVec.toNat_add, *]

end Equiv.Zisk
