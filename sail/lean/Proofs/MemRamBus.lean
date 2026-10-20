/-
  The RAM on the memory bus.

  `MemRam.lean` sees the RAM as a sorted log of lanes; on the memory bus it is
  nine arguments of each segment: lane `L`'s access, `(wr + 1, 8·addr, step,
  8, value)`, sent at every row (`bus66 + 2L`), its dual read, `(1, 8·addr,
  step_dual, 8, value)`, sent where `sel_dual` is set (`bus67 + 2L`), and the
  padding's cancellation (`bus65`), sent once with a negative count. This file
  states what each sends, row by row, in the lanes' terms.
-/
import Proofs.MemRam
import Proofs.MemCount
import Proofs.MemBus

namespace Correspondence

open Pil.Zisk MemSort

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- Lane `L`'s access, on the memory bus. -/
def ramLaneArg : Nat → BusArg F (Mem.Ctx F) (Mem.Trace F)
  | 0 => Mem.bus66 | 1 => Mem.bus68 | 2 => Mem.bus70 | _ => Mem.bus72

/-- Lane `L`'s dual read, on the memory bus. -/
def ramDualArg : Nat → BusArg F (Mem.Ctx F) (Mem.Trace F)
  | 0 => Mem.bus67 | 1 => Mem.bus69 | 2 => Mem.bus71 | _ => Mem.bus73

theorem ram_lane_tuple {L : Nat} (hL : L < 4) (x : Mem.Ctx F) (t : Mem.Trace F) (j : Int) :
    (ramLaneArg L).reps x t j = 1 ∧
      tupleOf ⟨ramLaneArg L, x, t⟩ j = [(t j).wr L + 1, (t j).addr L * 8, (t j).step L, 8,
        (t j).value L 0, (t j).value L 1] := by
  rcases L with _ | _ | _ | _ | L
  all_goals first
    | omega
    | exact ⟨rfl, by simp [tupleOf, ramLaneArg, Mem.bus66, Mem.bus68, Mem.bus70, Mem.bus72,
        Mem.Mem_mem_op_0, Mem.Mem_mem_op_1, Mem.Mem_mem_op_2, Mem.Mem_mem_op_3,
        Lean.Grind.Semiring.add_zero]⟩

theorem ram_dual_tuple {L : Nat} (hL : L < 4) (x : Mem.Ctx F) (t : Mem.Trace F) (j : Int) :
    (ramDualArg L).reps x t j = (t j).sel_dual L ∧
      tupleOf ⟨ramDualArg L, x, t⟩ j = [1, (t j).addr L * 8, (t j).step_dual L, 8,
        (t j).value L 0, (t j).value L 1] := by
  rcases L with _ | _ | _ | _ | L
  all_goals first
    | omega
    | exact ⟨by simp [ramDualArg, Mem.bus67, Mem.bus69, Mem.bus71, Mem.bus73,
          Lean.Grind.Semiring.add_zero],
        by simp [tupleOf, ramDualArg, Mem.bus67, Mem.bus69, Mem.bus71, Mem.bus73,
          Lean.Grind.Semiring.add_zero]⟩

/-! ### A lane's tuple, as an emission of the log -/

section Emit

variable {x : Mem.Ctx F} {t : Mem.Trace F} (N : RamNat x t)

open Classical in
/-- **A lane's read tuple is a read of the log.** -/
theorem RamNat.read_emit {j : Nat} (hj : j < Mem.numRows) {L : Nat} (hL : L < 4)
    (hwr : (t j).wr L * (1 - (t j).wr L) = 0) {X s v0 v1 : F}
    (htup : tupleOf ⟨ramLaneArg L, x, t⟩ j = [1, X, s, 8, v0, v1]) :
    (⟨N.A j L, N.S j L, (v0, v1)⟩ : Emit (F × F)) ∈ (N.lane j L).reads ∧
      X = ((8 * N.A j L : Nat) : F) ∧ s = ((N.S j L : Nat) : F) := by
  rw [(ram_lane_tuple hL x t j).2] at htup
  simp only [List.cons.injEq, and_true] at htup
  obtain ⟨hw, hX, hs, -, hv0, hv1⟩ := htup
  have hw0 : (t j).wr L = 0 := by
    have : (t j).wr L + 1 - 1 = 1 - 1 := by rw [hw]
    generalize (t j).wr L = w at this ⊢; grind
  have ha := N.addr j L (inRows_nat hj) hL
  have hst := N.step j L (inRows_nat hj) hL
  refine ⟨?_, ?_, ?_⟩
  · have hd : decide ((t j).wr L = 1) = false := by
      rw [hw0]; simp [Lean.Grind.Field.zero_ne_one]
    simp only [RamNat.lane, Lane.reads, hd, Bool.false_eq_true, if_false, List.mem_append,
      List.mem_singleton]
    left; rw [hv0, hv1]
  · rw [← hX, ha.2, Lean.Grind.Semiring.natCast_mul, natCast_ofNat' 8]
    exact Lean.Grind.CommSemiring.mul_comm _ _
  · rw [← hs, hst.2]

open Classical in
/-- **A lane's write tuple is a write of the log.** -/
theorem RamNat.write_emit {j : Nat} (hj : j < Mem.numRows) {L : Nat} (hL : L < 4)
    (hwr : (t j).wr L * (1 - (t j).wr L) = 0) {X s v0 v1 : F}
    (htup : tupleOf ⟨ramLaneArg L, x, t⟩ j = [2, X, s, 8, v0, v1]) :
    (⟨N.A j L, N.S j L, (v0, v1)⟩ : Emit (F × F)) ∈ (N.lane j L).writes ∧
      X = ((8 * N.A j L : Nat) : F) ∧ s = ((N.S j L : Nat) : F) := by
  rw [(ram_lane_tuple hL x t j).2] at htup
  simp only [List.cons.injEq, and_true] at htup
  obtain ⟨hw, hX, hs, -, hv0, hv1⟩ := htup
  have hw1 : (t j).wr L = 1 := by
    have : (t j).wr L + 1 - 1 = 2 - 1 := by rw [hw]
    generalize (t j).wr L = w at this ⊢; grind
  have ha := N.addr j L (inRows_nat hj) hL
  have hst := N.step j L (inRows_nat hj) hL
  refine ⟨?_, ?_, ?_⟩
  · simp only [RamNat.lane, Lane.writes, hw1, decide_true, if_true, List.mem_singleton]
    rw [hv0, hv1]
  · rw [← hX, ha.2, Lean.Grind.Semiring.natCast_mul, natCast_ofNat' 8]
    exact Lean.Grind.CommSemiring.mul_comm _ _
  · rw [← hs, hst.2]

open Classical in
/-- **A lane's dual tuple is a read of the log.** -/
theorem RamNat.dual_emit {j : Nat} (hj : j < Mem.numRows) {L : Nat} (hL : L < 4)
    (hsel : (t j).sel_dual L * (1 - (t j).sel_dual L) = 0)
    (hr : (t j).sel_dual L ≠ 0) {X s v0 v1 : F}
    (htup : tupleOf ⟨ramDualArg L, x, t⟩ j = [1, X, s, 8, v0, v1]) :
    (⟨N.A j L, N.D j L, (v0, v1)⟩ : Emit (F × F)) ∈ (N.lane j L).reads ∧
      X = ((8 * N.A j L : Nat) : F) ∧ s = ((N.D j L : Nat) : F) := by
  rw [(ram_dual_tuple hL x t j).2] at htup
  simp only [List.cons.injEq, and_true] at htup
  obtain ⟨-, hX, hs, -, hv0, hv1⟩ := htup
  have h1 : (t j).sel_dual L = 1 := by
    obtain ⟨b, hb⟩ := bool_of_idem hsel
    cases b <;> simp_all [bit]
  have ha := N.addr j L (inRows_nat hj) hL
  have hd := N.dual j L (inRows_nat hj) hL hr
  refine ⟨?_, ?_, ?_⟩
  · simp only [RamNat.lane, Lane.reads, h1, if_true, List.mem_append, List.mem_singleton]
    right; rw [hv0, hv1]
  · rw [← hX, ha.2, Lean.Grind.Semiring.natCast_mul, natCast_ofNat' 8]
    exact Lean.Grind.CommSemiring.mul_comm _ _
  · rw [← hs, hd.2]

end Emit

end Correspondence
