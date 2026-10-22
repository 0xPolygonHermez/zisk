/-
  sail-zisk's semantics for the instruction the transpiler emits for an
  immediate instruction (immediate_op): op(r[rs1], imm) -> r[rd], on to
  pc + 4 — as Equiv/Zisk.lean's `execute_regop` for the register form.
-/
import Equiv.Zisk

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- The instruction the transpiler emits for an immediate instruction whose
    rs1 is not x0 (immediate_op): op(r[rs1], b) -> r[rd], on to pc + 4. -/
def immInst (op : zisk_op) (rd rs1 : Nat) (b : BitVec 64) : zisk_inst :=
  { a_src := .A_REG, a_imm := .ofNat 64 rs1, b_src := .B_IMM, b_imm := b,
    ind_width := 8, op, store := .ST_REG, store_off := .ofNat 64 rd,
    store_pc := false, set_pc := false, jmp_off1 := 4, jmp_off2 := 4, is_end := false }

/-- Any pure op, as the transpiler emits it for an immediate instruction: rd
    gets its c, and the step moves on to pc + 4 whatever its flag. -/
theorem execute_immop (s : ZState) (op : zisk_op) (hop : PureOp op) (rd rs1 : Nat) (b : BitVec 64)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (hd : 1 ≤ rd ∧ rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    let r := zisk_exec op xs[(rs1 : Int)]! b
    (zisk_execute (immInst op rd rs1 b)).run s = .ok false
      { s with regs := (((((s.regs.insert Xs (vectorUpdate xs rd r.1)).insert
          Zc r.1).insert Zflag r.2).insert Step (BitVec.addInt st 1)).insert
          PC (pc + 4)) } := by
  intro r
  have n1 : rs1 ≠ 0 := by omega
  have nd : rd ≠ 0 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have ld : (rd : Int) < 40 := by omega
  have hm := hop.mem
  have hp := hop.notPre
  simp [zisk_execute, immInst, source_a, source_b, rX, wX, store_c, value_to_store, next_pc,
    hm, hp, reg_index rs1 (by omega), reg_index rd (by omega),
    Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, nd, l1, ld, writeReg_run]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run s Xs xs hx]; try dsimp only
  split
  · exact absurd rfl (hop.notHalt ())
  · simp only [run_bind', run_map', writeReg_run]
    rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
    rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]; try dsimp only
    all_goals rfl

end Equiv.Zisk
