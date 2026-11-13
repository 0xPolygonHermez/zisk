/-
  sail-zisk's semantics for every store the transpiler emits (store_op):
  copyb of r[rs2] to r[rs1] + off, w ∈ {1, 2, 4, 8} bytes of it, as
  Equiv/ZiskStore.lean has it for sd.
-/
import Equiv.ZiskStore

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- A w-byte write inside ZisK's RAM, off the register file: the bytes
    there. -/
theorem write_mem_run (s : ZState) (a : BitVec 64) (w : Nat) (v : BitVec (8 * w))
    (hwr : writable a w = true) (hnr : hits_registers a w = false) :
    (write_mem a w v).run s = .ok () { s with mem := Equiv.memWrite (n := w) s.mem a.toNat v } := by
  have hw := Equiv.zisk_writeBytes (n := w) s a.toNat v
  simp only [write_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, hwr, hnr, if_true, run_bind',
    run_pure', Bool.not_false]
  try dsimp only
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.write_ram, run_bind']
  rw [hw]; rfl

set_option maxHeartbeats 4000000 in
/-- A store as the transpiler emits it: the low w bytes of r[rs2] go to
    r[rs1] + off, and the step moves on to pc + 4. The address is in ZisK's
    RAM, off the register file. -/
theorem execute_store (s : ZState) (rs1 rs2 : Nat) (off : BitVec 64) (w : Nat)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (h2 : 1 ≤ rs2 ∧ rs2 < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (hwr : writable (xs[(rs1 : Int)]! + off) w = true)
    (hnr : hits_registers (xs[(rs1 : Int)]! + off) w = false) :
    let b := xs[(rs2 : Int)]!
    let regs' := (((s.regs.insert Zc b).insert Zflag false).insert Step (BitVec.addInt st 1)).insert PC (pc + 4)
    let mem' := Equiv.memWrite (n := w) s.mem (xs[(rs1 : Int)]! + off).toNat (b.setWidth (8 * w))
    (zisk_execute (storeInst rs1 rs2 off w)).run s = .ok false { s with regs := regs', mem := mem' } := by
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
  have ht : BitVec.setWidth (8 * w) (Sail.BitVec.truncate xs[(rs2 : Int)]! (8 * (w : Int)).toNat)
      = xs[(rs2 : Int)]!.setWidth (8 * w) := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; have hi2 : (i : Int) < 8 * (w : Int) := by omega
    simp [Sail.BitVec.truncate, hi, hi2]
  rw [ht, write_mem_run s _ w _ hwr hnr]; dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]
  exact ⟨rfl, by simp [mem', b, BitVec.toNat_add]⟩

end Equiv.Zisk
