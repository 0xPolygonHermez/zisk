/-
  sail-zisk's semantics for the instruction the transpiler emits for a
  branch (create_branch_op): op(r[rs1], r[rs2]) sets the flag, no store, and
  the step goes to pc + jmp_off1 when the flag is set, pc + jmp_off2 when it
  is not (a negated branch swaps the two).
-/
import Equiv.ZiskLoad

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- The instruction the transpiler emits for a branch on rs1, rs2 (not x0). -/
def branchInst (op : zisk_op) (rs1 rs2 : Nat) (j1 j2 : BitVec 64) : zisk_inst :=
  { a_src := .A_REG, a_imm := .ofNat 64 rs1, b_src := .B_REG, b_imm := .ofNat 64 rs2,
    ind_width := 8, op, store := .ST_NONE, store_off := 0,
    store_pc := false, set_pc := false, jmp_off1 := j1, jmp_off2 := j2, is_end := false }

theorem eq_pure : PureOp (.ZOP_EQ ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

set_option maxHeartbeats 4000000 in
/-- Any pure op as a branch: the flag chooses the next pc; no register of
    the program changes. -/
theorem execute_branch (s : ZState) (op : zisk_op) (hop : PureOp op) (rs1 rs2 : Nat) (j1 j2 : BitVec 64)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (h2 : 1 ≤ rs2 ∧ rs2 < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    let r := zisk_exec op xs[(rs1 : Int)]! xs[(rs2 : Int)]!
    (zisk_execute (branchInst op rs1 rs2 j1 j2)).run s = .ok false
      { s with regs := ((((s.regs.insert Zc r.1).insert Zflag r.2).insert Step (BitVec.addInt st 1)).insert
          PC (if r.2 then pc + j1 else pc + j2)) } := by
  intro r
  have n1 : rs1 ≠ 0 := by omega
  have n2 : rs2 ≠ 0 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have l2 : (rs2 : Int) < 40 := by omega
  have hm := hop.mem
  have hp := hop.notPre
  simp [zisk_execute, branchInst, source_a, source_b, rX, store_c, value_to_store, next_pc,
    hm, hp, reg_index rs1 (by omega), reg_index rs2 (by omega),
    Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, n2, l1, l2]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run s Xs xs hx]; try dsimp only
  split
  · exact absurd rfl (hop.notHalt ())
  · simp only [run_bind', run_map', writeReg_run]
    rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
    cases hf : (zisk_exec op xs[(rs1 : Int)]! xs[(rs2 : Int)]!).snd <;>
    · simp only [Bool.false_eq_true, if_true, if_false, run_map']
      rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc, r]

end Equiv.Zisk
