/-
  An execution: the runs and the register file, over one chain of segments.

  `Segments.lean` and `End.lean` run the model along a chain of Main segments;
  `Registers.lean` proves the register file Main keeps over its segments. This
  file joins them over one chain.

  * `ChainHolds.initial_step`: the continuation chain itself puts segment `k`'s
    first step at `k·2^24` — the boot tuple starts at 0, and every segment but
    the last hands on `initial_step + 2^24` (`segment_next_step`, with
    `main_last_segment` 0, which the next segment's assumed tuple forces). So
    `MainSegOk`'s first step is no longer an assumption.
-/
import Proofs.End
import Proofs.Registers

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]
variable {xr : Rom.Ctx F} {tr : Rom.Trace F} {ps : List (Prover F)}

omit [Lean.Grind.IsCharP F goldilocks] in
/-- No segment before the last is marked last: the next one assumes 0 there. -/
theorem ChainHolds.not_last {boot : BitVec 64} {segs : List (Segment xr tr ps)}
    (hc : ChainHolds boot segs) {k : Nat} (h : k + 1 < segs.length) :
    segs[k].x.Main_main_last_segment = 0 := by
  have e1 := hc.next k h 1
  simp only [tupleAt, Main.bus714, Main.bus715, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at e1
  exact e1.symm

/-- **Segment `k` starts at step `k·2^24`**, by the continuation chain. -/
theorem ChainHolds.initial_step {boot : BitVec 64} {segs : List (Segment xr tr ps)}
    (hc : ChainHolds boot segs) :
    ∀ k (h : k < segs.length), segs[k].x.Main_segment_initial_step = ((k * 16777216 : Nat) : F)
  | 0, h => by
    have e2 := hc.first h 2
    simp only [tupleAt, Main.bus714, bootTuple, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at e2
    rw [e2, Nat.zero_mul, Lean.Grind.Semiring.natCast_zero]
  | k + 1, h => by
    have ih := hc.initial_step k (by omega)
    have hl := hc.not_last h
    have e2 := hc.next k h 2
    simp only [tupleAt, Main.bus714, Main.bus715, Main.Main_segment_next_step, List.getD_cons_zero,
      List.getD_cons_succ, Lean.Grind.Semiring.add_zero] at e2
    rw [e2, ih, hl, Nat.succ_mul, Lean.Grind.Semiring.natCast_add, ← natCast_ofNat' 16777216]
    grind

end Correspondence
