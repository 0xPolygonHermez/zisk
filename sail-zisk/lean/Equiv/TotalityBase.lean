/-
  Totality's common facts: the word at pc as its four bytes, and pc + 2
  executable where pc is, 4-aligned, in ZisK's regions.
-/
import Equiv.Totality

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- A word read from memory: its four bytes there. -/
theorem readWord_bytes {m : Equiv.Mem} {a : Nat} {w : BitVec 32} (h : ZiskRiscv.readWord m a = some w) :
    m.get? a = some (byte w 0) ∧ m.get? (a + 1) = some (byte w 1) ∧ m.get? (a + 2) = some (byte w 2) ∧
      m.get? (a + 3) = some (byte w 3) := by
  simp only [ZiskRiscv.readWord] at h
  cases h0 : m.get? a <;> cases h1 : m.get? (a + 1) <;> cases h2 : m.get? (a + 2) <;>
    cases h3 : m.get? (a + 3) <;> simp_all [Option.bind]
  subst h
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp only [byte] <;> bv_decide

/-- An address in the ROM, with a parcel after it there: executable. -/
theorem exec_of_rom {a : BitVec 64} (h1 : 0x9000_0000 ≤ a.toNat) (h2 : a.toNat + 2 ≤ 0x9800_0000) :
    ExecAt ziskRegions a := by
  have e : 0x9000_0000#64 ≤ a ∧ a + 2#64 ≤ 0x9800_0000#64 ∧ a ≤ a + 2#64 := by
    simp only [BitVec.le_def, BitVec.toNat_add]; simp; omega
  simp only [ExecAt, matching_pma_region, matching_pma_region_bits_range, ziskRegions]
  have g : get_slice_int 64 (2 : Int) 0 = 2#64 := by rfl
  have z : Sail.BitVec.zeroExtend a 64 = a := by simp [Sail.BitVec.zeroExtend]
  have u : ∀ x y : BitVec 64, (Sail.BitVec.toNatInt x ≤ Sail.BitVec.toNatInt y) ↔ x ≤ y := by
    intro x y; simp [Sail.BitVec.toNatInt, BitVec.le_def]
  split
  · next hr =>
    exfalso
    simp only [range_subset, zopz0zIzJ_u, zero_extend, bits_of_physaddr, to_bits, g, z, Bool.and_eq_true,
      decide_eq_true_eq, u] at hr
    have g' : get_slice_int 64 ((2 : Nat) : Int) 0 = 2#64 := by rfl
    simp only [z, g'] at hr
    revert hr e; bv_decide
  split
  · exact ⟨_, rfl, rfl⟩
  · next hr =>
    exfalso; apply hr
    simp only [range_subset, zopz0zIzJ_u, zero_extend, bits_of_physaddr, to_bits, g, z, Bool.and_eq_true,
      decide_eq_true_eq, u]
    have g' : get_slice_int 64 ((2 : Nat) : Int) 0 = 2#64 := by rfl
    try simp only [z, g']
    revert e; bv_decide

/-- pc + 2 executable where pc is, 4-aligned. -/
theorem exec_add2 {pc : BitVec 64} (h : ExecAt ziskRegions pc) (hal : pc.toNat % 4 = 0) :
    ExecAt ziskRegions (pc + 2) := by
  have := exec_rom h
  apply exec_of_rom <;> rw [BitVec.toNat_add] <;> simp <;> omega

end Equiv.Final
