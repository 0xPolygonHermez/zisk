/-
  ZisK's PMA regions (Equiv/Regions.lean), concretely: only the ROM is
  executable. So the exit address and the trap vector are not, and code is
  never in the RAM — what the theorems take of the regions.
-/
import Equiv.Regions
import Equiv.SpecRom

namespace Equiv.Sim

open Equiv.Riscv Equiv.Machine
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- An executable address is in the ROM, [0x9000_0000, 0x9800_0000). -/
theorem exec_rom {a : BitVec 64} (h : ExecAt ziskRegions a) :
    0x9000_0000 ≤ a.toNat ∧ a.toNat + 2 ≤ 0x9800_0000 := by
  obtain ⟨r, hm, hx⟩ := h
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions] at hm
  split at hm
  · cases hm; simp at hx
  split at hm
  · rename_i _ hr
    have g : get_slice_int 64 (2 : Int) 0 = 2#64 := by rfl
    have z : Sail.BitVec.zeroExtend a 64 = a := by simp [Sail.BitVec.zeroExtend]
    have u : ∀ x y : BitVec 64, (Sail.BitVec.toNatInt x ≤ Sail.BitVec.toNatInt y) ↔ x ≤ y := by
      intro x y; simp [Sail.BitVec.toNatInt, BitVec.le_def]
    simp only [range_subset, zopz0zIzJ_u, zero_extend, bits_of_physaddr, to_bits, g, z, Bool.and_eq_true,
      decide_eq_true_eq, u] at hr
    obtain ⟨h1, h2, h3⟩ := hr
    have e : 0x9000_0000#64 ≤ a ∧ a + 2#64 ≤ 0x9800_0000#64 ∧ a ≤ a + 2#64 := by
      revert h1 h2 h3; bv_decide
    obtain ⟨e1, e2, e3⟩ := e
    simp only [BitVec.le_def, BitVec.toNat_add] at e1 e2 e3
    simp at e1 e2 e3
    omega
  split at hm
  · cases hm; simp at hx
  · cases hm

theorem exit_not_exec : ¬ ExecAt ziskRegions EXIT_ADDR := by
  intro h; have := exec_rom h; simp [EXIT_ADDR] at this

theorem trap_not_exec : ¬ ExecAt ziskRegions TRAP_VEC := by
  intro h; have := exec_rom h; simp [TRAP_VEC] at this

/-- Code at executable addresses is off the RAM, and so is the word after. -/
theorem noCodeRam_of_exec {C : BitVec 64 → Prop} (h : ∀ pc, C pc → ExecAt ziskRegions pc) : NoCodeRam C := by
  intro pc hc
  have ⟨e1, e2⟩ := exec_rom (h pc hc)
  have e4 : (pc + 4).toNat = pc.toNat + 4 := by rw [BitVec.toNat_add]; simp; omega
  refine ⟨fun i hi => ?_, fun i hi => ?_⟩ <;> unfold OffRam
  · omega
  · rw [e4]; omega

end Equiv.Sim
