/-
  sail-zisk's semantics for the instruction the transpiler emits for a store
  (store_op): copyb(r[rs1], r[rs2]) -> mem[r[rs1] + off], `ind_width` bytes
  wide (ST_IND), on to pc + 4.
-/
import Equiv.ZiskLoad
import Equiv.MemWrite

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- The instruction the transpiler emits for a store of rs2 to rs1 + off, w
    bytes wide, rs1, rs2 not x0 (store_op). -/
def storeInst (rs1 rs2 : Nat) (off : BitVec 64) (w : Nat) : zisk_inst :=
  { a_src := .A_REG, a_imm := .ofNat 64 rs1, b_src := .B_REG, b_imm := .ofNat 64 rs2,
    ind_width := w, op := .ZOP_COPYB (), store := .ST_IND, store_off := off,
    store_pc := false, set_pc := false, jmp_off1 := 4, jmp_off2 := 4, is_end := false }

/-- An 8-byte write inside ZisK's RAM, off the register file: the bytes
    there. -/
theorem write_mem8_run (s : ZState) (a : BitVec 64) (v : BitVec 64)
    (hwr : writable a 8 = true) (hnr : hits_registers a 8 = false) :
    (write_mem a 8 v).run s = .ok () { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hw := Equiv.zisk_writeBytes (n := 8) s a.toNat v
  simp only [write_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, hwr, hnr, if_true, run_bind',
    run_pure', Bool.not_false]
  try dsimp only
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.write_ram, run_bind']
  rw [hw]; rfl

set_option maxHeartbeats 4000000 in
/-- `sd` as the transpiler emits it: the 8 bytes of r[rs2] go to
    r[rs1] + off, and the step moves on to pc + 4. The address is in ZisK's
    RAM, off the register file. -/
theorem execute_sd (s : ZState) (rs1 rs2 : Nat) (off : BitVec 64)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (h2 : 1 ≤ rs2 ∧ rs2 < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (hwr : writable (xs[(rs1 : Int)]! + off) 8 = true)
    (hnr : hits_registers (xs[(rs1 : Int)]! + off) 8 = false) :
    let b := xs[(rs2 : Int)]!
    let regs' := (((s.regs.insert Zc b).insert Zflag false).insert Step (BitVec.addInt st 1)).insert PC (pc + 4)
    let mem' := Equiv.memWrite (n := 8) s.mem (xs[(rs1 : Int)]! + off).toNat b
    (zisk_execute (storeInst rs1 rs2 off 8)).run s = .ok false { s with regs := regs', mem := mem' } := by
  intro b regs' mem'
  have n1 : rs1 ≠ 0 := by omega
  have n2 : rs2 ≠ 0 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have l2 : (rs2 : Int) < 40 := by omega
  have hm' := copyb_pure.mem
  have hp' := copyb_pure.notPre
  simp [zisk_execute, storeInst, source_a, source_b, rX, store_c, value_to_store, next_pc,
    hm', hp', copyb_val, reg_index rs1 (by omega), reg_index rs2 (by omega),
    Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, n2, l1, l2, writeReg_run]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run s Xs xs hx]; try dsimp only
  have ht : Sail.BitVec.truncate xs[(rs2 : Int)]! 64 = xs[(rs2 : Int)]! := by
    simp [Sail.BitVec.truncate]
  rw [ht, write_mem8_run s _ _ hwr hnr]; dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]
  exact ⟨rfl, by simp [mem', b, BitVec.toNat_add]⟩

end Equiv.Zisk
