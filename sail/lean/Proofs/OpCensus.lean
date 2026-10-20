/-
  The operation bus, from Main's side: what Main assumes on it.

  Main hands an op to another AIR over the operation bus (5000) at each lane
  whose `is_external_op` is set, with the lane's `op` column as the tuple's
  op. The ROM lookup holds at every lane, so the lane holds an instruction of
  the program (`rom_pc32`, `rom_lane'`), and the model has eight ops: Main
  assumes only add, ltu, eq, and and pubout (`main_op_modeled`). No AIR's
  tuple with another op is ever assumed by Main — the first step to showing
  that the precompiles and the DMA AIRs, which only Main's calls start, stay
  idle.
-/
import Proofs.Rom
import Proofs.ExtPilot

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The ops Main hands over the operation bus: ltu, eq, add, and, pubout. -/
def mainExtOps : List Nat := [0x06, 0x09, 0x0a, 0x0e, 0xa0]

/-- **Main assumes only the model's external ops.** At any lane of any Main
row that assumes an operation-bus tuple, the tuple's op is one of
`mainExtOps`: the lane holds an instruction of the program, by the ROM lookup,
and its op is external. -/
theorem main_op_modeled (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (hm : ∀ i, Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    {l : Nat} (hl : l < 4) (hbus : LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr) (i : Int) (hr0 : (opArg l).reps xm tm i ≠ 0) :
    ∃ n ∈ mainExtOps, tupleAt (opArg l).tuple xm tm i 0 = ((n : Nat) : F) := by
  obtain ⟨q, hq, hpc⟩ := rom_pc32 prog xm tm i xr tr hr hl hbus hrom
  obtain ⟨inst, -, hslot, -⟩ := rom_lane' prog xm tm i (hm i) xr tr hr hl hbus hrom q hq hpc
  rw [opArg_reps hl, hslot.is_external_op, Lean.Grind.Semiring.add_zero] at hr0
  rw [(opArg_tuple hl xm tm i).1, hslot.op, Lean.Grind.Semiring.add_zero]
  have hext : isExternal inst.op = true := by
    cases h : isExternal inst.op
    · rw [h] at hr0; exact absurd rfl hr0
    · rfl
  cases hop : inst.op with
  | ZOP_FLAG => rw [hop] at hext; exact nomatch hext
  | ZOP_COPYB => rw [hop] at hext; exact nomatch hext
  | ZOP_HALT => rw [hop] at hext; exact nomatch hext
  | ZOP_LTU => exact ⟨0x06, by simp [mainExtOps], rfl⟩
  | ZOP_EQ => exact ⟨0x09, by simp [mainExtOps], rfl⟩
  | ZOP_ADD => exact ⟨0x0a, by simp [mainExtOps], rfl⟩
  | ZOP_AND => exact ⟨0x0e, by simp [mainExtOps], rfl⟩
  | ZOP_PUBOUT => exact ⟨0xa0, by simp [mainExtOps], rfl⟩

end Correspondence
