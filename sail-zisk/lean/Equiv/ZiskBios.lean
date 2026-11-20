/-
  sail-zisk's semantics for the BIOS's instructions (Transpiler/Bios.lean)
  the plain-op lemmas of Equiv/ZiskSpec.lean leave out: a constant to a
  memory cell, the call to the program's entry, the reads that store
  nothing, pubout, the jump to ROM_EXIT, and the two ends: `end` and `halt`.
-/
import Transpiler.Bios
import Equiv.ZiskSpec

set_option linter.unusedSimpArgs false

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

set_option maxHeartbeats 4000000 in
/-- copyb(0, b) to the 8-byte memory cell at addr, on to pc + j. -/
theorem execute_copy_mem (s : ZState) (b addr j : BitVec 64) (pc st : BitVec 64)
    (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (hwr : writable addr 8 = true) (hnr : hits_registers addr 8 = false) :
    (zisk_execute (Transpiler.copyImm b (.ST_MEM, addr, false) j j)).run s = .ok false
      { s with
        regs := (((s.regs.insert Zc b).insert Zflag false).insert Step (BitVec.addInt st 1)).insert PC (pc + j)
        mem := Equiv.memWrite (n := 8) s.mem addr.toNat b } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  simp [zisk_execute, Transpiler.copyImm, Transpiler.mk, source_a, source_b, store_c, value_to_store, next_pc,
    hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run]
  rw [write_mem8_run s _ _ hwr hnr]; dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]

set_option maxHeartbeats 4000000 in
/-- The call to the entry: pc + 4 to ra (x1), on to `entry`. -/
theorem execute_call (s : ZState) (entry : BitVec 64) (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    (zisk_execute (Transpiler.mk (.A_IMM, 0) (.B_IMM, entry) (.ZOP_COPYB ()) (.ST_REG, 1, true) 0 4
        (setPc := true))).run s = .ok false
      { s with regs := ((((s.regs.insert Xs (vectorUpdate xs 1 (pc + 4))).insert Zc entry).insert Zflag false).insert
          Step (BitVec.addInt st 1)).insert PC (entry + 0) } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  have r1 := reg_index 1 (by omega)
  simp [zisk_execute, Transpiler.mk, source_a, source_b, store_c, value_to_store, next_pc, wX,
    hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run, r1]
  repeat (first
    | (rw [readReg_run s Xs xs hx]; try dsimp only)
    | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx]))

set_option maxHeartbeats 4000000 in
/-- copyb of the 8 bytes at a + off, stored nowhere, on to pc + j2: the
    output block's read of an output word and the input reads. -/
theorem execute_read_none (s : ZState) (ra : a_src × BitVec 64) (off j1 j2 : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (va : BitVec 64) (ha : ASrc ra xs va) (v : BitVec 64)
    (hrb : readable (va + off) 8 = true) (hnr : hits_registers (va + off) 8 = false)
    (hin : va + off ≠ INPUT_ADDR)
    (hm : Equiv.memRead s.mem 8 (va + off).toNat = some v) :
    (zisk_execute (Transpiler.mk ra (.B_IND, off) (.ZOP_COPYB ()) (.ST_NONE, 0, false) j1 j2)).run s = .ok false
      { s with regs := (((s.regs.insert Zc v).insert Zflag false).insert
          Step (BitVec.addInt st 1)).insert PC (pc + j2) } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩
  · simp [zisk_execute, Transpiler.mk, source_a, source_b, store_c, value_to_store, next_pc,
      hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run]
    rw [read_mem8_run s _ v hrb hnr hin hm]; try dsimp only
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc])
    all_goals simp [_root_.Zisk.Functions.zero_extend, Sail.BitVec.zeroExtend]
  · subst hv1
    have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩
    simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store, next_pc,
      hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run, reg_index, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only)
      | (rw [read_mem8_run s _ v (by simpa (disch := omega) using hrb) (by simpa (disch := omega) using hnr)
            (by simpa (disch := omega) using hin) (by simpa (disch := omega) using hm)]; try dsimp only)
      | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *]))
    all_goals simp (disch := omega) [_root_.Zisk.Functions.zero_extend, Sail.BitVec.zeroExtend, *]

theorem pubout_pure : PureOp (.ZOP_PUBOUT ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

theorem pubout_val (a b : BitVec 64) : zisk_exec (.ZOP_PUBOUT ()) a b = (b, false) := rfl

set_option maxHeartbeats 4000000 in
/-- pubout of the last result c, its a a register 1..31: nothing changes but
    the step's own registers; on to pc + 4. -/
theorem execute_pubout (s : ZState) (r : Nat) (hr : 1 ≤ r ∧ r < 32) (xs : Vector (BitVec 64) 40)
    (pc st c : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (hc : s.regs.get? Zc = some c) :
    (zisk_execute (Transpiler.mk (.A_REG, BitVec.ofNat 64 r) (.B_C, 0) (.ZOP_PUBOUT ()) (.ST_NONE, 0, false)
        0 4)).run s = .ok false
      { s with regs := (((s.regs.insert Zc c).insert Zflag false).insert
          Step (BitVec.addInt st 1)).insert PC (pc + 4) } := by
  have hm' := pubout_pure.mem
  have hp := pubout_pure.notPre
  have e1 : r ≠ 0 ∧ (r : Int) < 40 := ⟨by omega, by omega⟩
  simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store, next_pc,
    hm', hp, pubout_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run, reg_index, *]
  repeat (first
    | (rw [readReg_run s Xs xs hx]; try dsimp only)
    | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, hc, *]))
  all_goals first | rfl | simp (disch := omega) [*] | skip

set_option maxHeartbeats 4000000 in
/-- copyb(0, t) with set_pc, stored nowhere: a jump to t. -/
theorem execute_goto (s : ZState) (t st : BitVec 64) (hst : s.regs.get? Step = some st) :
    (zisk_execute (Transpiler.mk (.A_IMM, 0) (.B_IMM, t) (.ZOP_COPYB ()) (.ST_NONE, 0, false) 0 0
        (setPc := true))).run s = .ok false
      { s with regs := (((s.regs.insert Zc t).insert Zflag false).insert
          Step (BitVec.addInt st 1)).insert PC (t + 0) } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  simp [zisk_execute, Transpiler.mk, source_a, source_b, store_c, value_to_store, next_pc,
    hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run]
  repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst])

set_option maxHeartbeats 4000000 in
/-- The end at ROM_EXIT: the run ends, with no error set. -/
theorem execute_end (s : ZState) (st : BitVec 64) (hst : s.regs.get? Step = some st) :
    (zisk_execute { Transpiler.copyImm 0 (.ST_NONE, 0, false) 0 0 with is_end := true }).run s = .ok true
      { s with regs := ((s.regs.insert Zc 0).insert Zflag false).insert Step (BitVec.addInt st 1) } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  simp [zisk_execute, Transpiler.copyImm, Transpiler.mk, source_a, source_b, store_c, value_to_store,
    hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run]
  repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst])

theorem halt_mem (a b e : BitVec 64) : zisk_exec_mem (.ZOP_HALT ()) a b e = pure none := by
  first | rfl | simp

theorem halt_notPre : zisk_op_is_precompiled (.ZOP_HALT ()) = false := by
  first | rfl | decide

set_option maxHeartbeats 4000000 in
/-- The halt: the run ends, with Zerror set and the exit code r's value. -/
theorem execute_halt (s : ZState) (r : Nat) (hr : 1 ≤ r ∧ r < 32) (xs : Vector (BitVec 64) 40) (st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hst : s.regs.get? Step = some st) :
    (zisk_execute { Transpiler.mk (.A_REG, BitVec.ofNat 64 r) (.B_IMM, 0) (.ZOP_HALT ()) (.ST_NONE, 0, false) 0 0
        with is_end := true }).run s = .ok true
      { s with regs := ((((s.regs.insert Zerror true).insert Zerror_code xs[(r : Int)]!).insert Zc 0).insert
          Zflag false).insert Step (BitVec.addInt st 1) } := by
  have hm' := halt_mem
  have hp := halt_notPre
  have e1 : r ≠ 0 ∧ (r : Int) < 40 := ⟨by omega, by omega⟩
  simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store,
    hm', hp, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run, reg_index, *]
  repeat (first
    | (rw [readReg_run s Xs xs hx]; try dsimp only)
    | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hx, *]))
  all_goals first | rfl | simp (disch := omega) [*] | skip

end Equiv.Zisk
