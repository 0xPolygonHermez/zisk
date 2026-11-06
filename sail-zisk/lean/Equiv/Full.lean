/-
  Memory as a ZisK run starts it: every byte of ZisK's regions defined,
  the unwritten ones 0 (`Full`). A run keeps it so (sail-riscv never forgets
  a byte, Equiv/MemGrow.lean), and it gives what the BIOS's end reads: the
  32 output words and the 15 input words (`OutBytes`, `InBytes`).
-/
import Equiv.BiosEnd
import Equiv.MemGrow
import ZiskRiscv.Machine

namespace Equiv

/-- Every byte of ZisK's memory regions defined. -/
def Full (m : Mem) : Prop := ∀ a, ZiskRiscv.inRegions a = true → (m.get? a).isSome

theorem Full.grow {m m' : Mem} (h : Full m) (g : Grows m m') : Full m' := fun a ha => g a (h a ha)

/-- n bytes there, read. -/
theorem memRead_some (m : Mem) : ∀ (n a : Nat), (∀ i, i < n → (m.get? (a + i)).isSome) →
    ∃ v, memRead m n a = some v
  | 0, a, _ => ⟨_, rfl⟩
  | 1, a, h => by
    obtain ⟨b, hb⟩ := Option.isSome_iff_exists.1 (h 0 (by omega))
    exact ⟨b, by simpa [memRead] using hb⟩
  | n + 2, a, h => by
    obtain ⟨b, hb⟩ := Option.isSome_iff_exists.1 (h 0 (by omega))
    obtain ⟨v, hv⟩ := memRead_some m (n + 1) (a + 1) (fun i hi => by
      have := h (i + 1) (by omega); rwa [show a + (i + 1) = a + 1 + i by omega] at this)
    have hb' : m[a]? = some b := by simpa using hb
    simp [memRead, hb', hv]

theorem Full.outBytes {m : Mem} (h : Full m) : Sim.OutBytes m := by
  intro j hj
  apply memRead_some
  intro i hi
  apply h
  have e0 : Transpiler.OUTPUT_ADDR.toNat = 0xA041_0000 := rfl
  have : (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)).toNat = 0xA041_0000 + 8 * j := by
    rw [BitVec.toNat_add, BitVec.toNat_ofNat, e0, Nat.mod_eq_of_lt (a := 8 * j) (by omega),
      Nat.mod_eq_of_lt (by omega)]
  rw [this]
  simp only [ZiskRiscv.inRegions, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq]
  omega

theorem Full.inBytes {m : Mem} (h : Full m) : Sim.InBytes m := by
  intro k hk
  apply memRead_some
  intro i hi
  apply h
  have e0 : Transpiler.INPUT_ADDR.toNat = 0x1000_0000 := rfl
  have e1 : Transpiler.INPUT_ANCHOR_STEP.toNat = 0x800_0000 := rfl
  have : (Sim.inAddr k).toNat = 0x1000_0000 + (k + 1) * 0x800_0000 := by
    rw [Sim.inAddr, BitVec.toNat_add, BitVec.toNat_mul, BitVec.toNat_ofNat, e0, e1,
      Nat.mod_eq_of_lt (a := k + 1) (by omega), Nat.mod_eq_of_lt (a := (k + 1) * 0x800_0000) (by omega),
      Nat.mod_eq_of_lt (by omega)]
  rw [this]
  simp only [ZiskRiscv.inRegions, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq]
  omega

end Equiv
