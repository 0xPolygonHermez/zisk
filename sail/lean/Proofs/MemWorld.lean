/-
  The memory bus, around Main: the setting for deriving `MemAnswers`.

  The other AIRs on the memory bus (`MemOthers`) split three ways: the RAM's
  instances, InputData's, and the rest. `NoOtherMem` is what the census is to
  prove: none of the rest sends a load or a store — a tuple with op 1 or 2 —
  with a nonzero count.
-/
import Proofs.MemRamBus

namespace Correspondence

open Pil.Zisk

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The RAM's instances' arguments on the memory bus. -/
def MemOthers.ramSenders (S : MemOthers F) : List (Sender F) := S.mem.flatMap MemOnMem.senders

/-- InputData's instances' arguments on the memory bus. -/
def MemOthers.inSenders (S : MemOthers F) : List (Sender F) := S.inputData.flatMap InputDataOnMem.senders

/-- Every other AIR's arguments on the memory bus. -/
def MemOthers.restSenders (S : MemOthers F) : List (Sender F) :=
  ({ S with mem := [], inputData := [] } : MemOthers F).senders

theorem MemOthers.mem_split (S : MemOthers F) {s : Sender F} (h : s ∈ S.senders) :
    s ∈ S.ramSenders ∨ s ∈ S.inSenders ∨ s ∈ S.restSenders := by
  by_cases hr : s ∈ S.ramSenders
  · exact .inl hr
  by_cases hi : s ∈ S.inSenders
  · exact .inr (.inl hi)
  right; right
  unfold MemOthers.ramSenders at hr
  unfold MemOthers.inSenders at hi
  simp only [MemOthers.restSenders, MemOthers.senders, List.mem_append, List.flatMap_nil,
    List.not_mem_nil, false_or, or_false] at h ⊢
  simpa [hr, hi] using h

/-- **No other AIR sends a load or a store**, with a nonzero count: what the
census is to prove. -/
def NoOtherMem (S : MemOthers F) : Prop :=
  ∀ s ∈ S.restSenders, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 →
    (tupleOf s.p j).headD 0 ≠ 1 ∧ (tupleOf s.p j).headD 0 ≠ 2

end Correspondence
