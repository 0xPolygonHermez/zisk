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

/-- Code at executable addresses is off the RAM, and so are the two words after. -/
theorem noCodeRam_of_exec {C : BitVec 64 → Prop} (h : ∀ pc, C pc → ExecAt ziskRegions pc) : NoCodeRam C := by
  intro pc hc
  have ⟨e1, e2⟩ := exec_rom (h pc hc)
  have e4 : (pc + 4).toNat = pc.toNat + 4 := by rw [BitVec.toNat_add]; simp; omega
  have e8 : (pc + 8).toNat = pc.toNat + 8 := by rw [BitVec.toNat_add]; simp; omega
  refine ⟨fun i hi => ?_, fun i hi => ?_, fun i hi => ?_⟩ <;> unfold OffRam
  · omega
  · rw [e4]; omega
  · rw [e8]; omega

/-- Every ZisK region has a 16-byte misaligned-atomicity granule. -/
theorem region_gran {a : BitVec 64} {w : Nat} {r : PMA_Region}
    (hm : matching_pma_region ziskRegions (.Physaddr a) w = some r) :
    r.attributes.misaligned_atomicity_granule_size_exp = 4 := by
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions] at hm
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  · cases hm

theorem region_noex {a : BitVec 64} {w : Nat} {r : PMA_Region}
    (hm : matching_pma_region ziskRegions (.Physaddr a) w = some r) :
    r.attributes.misaligned_exceptions.load_store = none := by
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions] at hm
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  · cases hm

/-- In ZisK's regions, a load or store runs at any alignment, short of
    wrapping past the address space's end. -/
theorem splitOk_zisk {a : BitVec 64} {w : Nat} {r : PMA_Region}
    (hm : matching_pma_region ziskRegions (.Physaddr a) w = some r) (h : a.toNat + w < 2 ^ 64) : SplitOk r a w :=
  ⟨region_gran hm, region_noex hm, h⟩

theorem allowed_of_mod (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8)
    (h : a.toNat % 16 + w ≤ 16) : allowed_misaligned a w 4 = true := by
  have hb : (a &&& 15#64).toNat = a.toNat % 16 := by
    rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 4
  simp only [allowed_misaligned, access_within, Sail.BitVec.length, zero_extend, Sail.BitVec.zeroExtend, ones,
    sail_ones, BitVec.addInt]
  rcases hw with rfl | rfl | rfl | rfl
  · have hle : a &&& 15#64 ≤ 15#64 := by rw [BitVec.le_def, hb]; simp; omega
    simp
    first | decide | (refine ⟨by decide, ?_⟩; revert hle; bv_decide) | (revert hle; bv_decide)
  · have hle : a &&& 15#64 ≤ 14#64 := by rw [BitVec.le_def, hb]; simp; omega
    simp
    first | decide | (refine ⟨by decide, ?_⟩; revert hle; bv_decide) | (revert hle; bv_decide)
  · have hle : a &&& 15#64 ≤ 12#64 := by rw [BitVec.le_def, hb]; simp; omega
    simp
    first | decide | (refine ⟨by decide, ?_⟩; revert hle; bv_decide) | (revert hle; bv_decide)
  · have hle : a &&& 15#64 ≤ 8#64 := by rw [BitVec.le_def, hb]; simp; omega
    simp
    first | decide | (refine ⟨by decide, ?_⟩; revert hle; bv_decide) | (revert hle; bv_decide)

/-- In ZisK's regions, a load or store of at most 8 bytes within its 16-byte
    granule — aligned or not — passes the PMA check unsplit. -/
theorem alignOk_gran {a : BitVec 64} {w : Nat} {r : PMA_Region} {acc : MemoryAccessType mem_payload}
    (hm : matching_pma_region ziskRegions (.Physaddr a) w = some r) (hacc : acc ≠ .InstructionFetch ())
    (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8) (h : a.toNat % 16 + w ≤ 16) : AlignOk r acc a w :=
  .inr ⟨hacc, region_gran hm, by omega, allowed_of_mod a w hw h⟩

theorem slice_w (w : Nat) : get_slice_int 64 (w : Int) 0 = BitVec.ofNat 64 w := by
  apply BitVec.eq_of_toNat_eq
  simp [get_slice_int]

/-- An access ZisK's memory map reads is in a PMA region (all readable). -/
theorem readable_match (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8)
    (h : _root_.Zisk.Functions.readable a w = true) :
    ∃ r, matching_pma_region ziskRegions (.Physaddr a) w = some r ∧ r.attributes.readable = true := by
  have ha := a.isLt
  simp only [_root_.Zisk.Functions.readable, _root_.Zisk.Functions.mem_region_of, BitVec.toNatInt] at h
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions, range_subset, zopz0zIzJ_u,
    zero_extend, bits_of_physaddr, Sail.BitVec.zeroExtend, to_bits, slice_w, BitVec.toNatInt]
  split
  · exact ⟨_, rfl, rfl⟩
  split
  · exact ⟨_, rfl, rfl⟩
  split
  · exact ⟨_, rfl, rfl⟩
  exfalso
  rename_i h1 h2 h3
  simp [BitVec.toNat_sub, BitVec.toNat_add] at h1 h2 h3
  revert h
  split
  · intro h; cases h
  · intro _
    rename_i hne _
    revert hne
    split
    · intro _; rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
    split
    · intro _; rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
    split
    · intro _; rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
    · intro hne; exact absurd rfl hne

/-- ZisK's writable memory: its RAM. -/
theorem writable_ram (a : BitVec 64) (w : Nat) (h : _root_.Zisk.Functions.writable a w = true) :
    2684354560 ≤ a.toNat ∧ a.toNat + w ≤ 3221225472 := by
  simp only [_root_.Zisk.Functions.writable, _root_.Zisk.Functions.mem_region_of, BitVec.toNatInt] at h
  revert h
  split
  · next heq =>
    intro _
    revert heq
    split
    · intro hc; cases hc
    split
    · intro hc; cases hc
    split
    · intro _
      simp_all
      omega
    · intro hc; cases hc
  · intro h; cases h

/-- An access ZisK's memory map writes, in its RAM, is in a writable PMA region. -/
theorem writable_match (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8)
    (h : _root_.Zisk.Functions.writable a w = true) :
    ∃ r, matching_pma_region ziskRegions (.Physaddr a) w = some r ∧ r.attributes.writable = true := by
  have ha := a.isLt
  have hram := writable_ram a w h
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions, range_subset, zopz0zIzJ_u,
    zero_extend, bits_of_physaddr, Sail.BitVec.zeroExtend, to_bits, slice_w, BitVec.toNatInt]
  split
  · rename_i r1
    exfalso
    simp [BitVec.toNat_sub, BitVec.toNat_add] at r1
    rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
  split
  · rename_i r2
    exfalso
    simp [BitVec.toNat_sub, BitVec.toNat_add] at r2
    rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
  split
  · exact ⟨_, rfl, rfl⟩
  · rename_i r3
    exfalso
    simp [BitVec.toNat_sub, BitVec.toNat_add] at r3
    rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega

end Equiv.Sim
