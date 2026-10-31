/-
  Sail's loops (`for i in [a:b:s]i`, an `IntRange`) with constant bounds,
  unrolled: `IntRange.forIn'.loop` one turn at a time while the index is in
  the range (decided), in any monad. Then the bit-vector facts the unrolled
  terms need for bv_decide: Sail's `access` as `getLsbD`, `BitVec.zero` as
  a literal, and `Id`'s pure and bind away.
-/
import Equiv.RiscvZbkb

namespace Equiv

open LeanZiskRV LeanZiskRV.Functions Sail

theorem loop_in {m} [Monad m] {β} (r : IntRange) (f : (i : Int) → i ∈ r → β → m (ForInStep β)) (b : β)
    (i : Int) (hs : (i - r.start) % r.step = 0) (h : i ∈ r) :
    IntRange.forIn'.loop r f b i hs = (do match (← f i h b) with
      | .done b => pure b
      | .yield b =>
        IntRange.forIn'.loop r f b (i + r.step) (by have := r.step_pos; rw [Int.add_comm, Int.add_sub_assoc]; simp_all)) := by
  rw [IntRange.forIn'.loop, dif_pos h]; rfl

theorem loop_out {m} [Monad m] {β} (r : IntRange) (f : (i : Int) → i ∈ r → β → m (ForInStep β)) (b : β)
    (i : Int) (hs : (i - r.start) % r.step = 0) (h : ¬ i ∈ r) : IntRange.forIn'.loop r f b i hs = pure b := by
  rw [IntRange.forIn'.loop, dif_neg h]

theorem bv_zero (n : Nat) : BitVec.zero n = 0#n := rfl

theorem id_pure {α} (x : α) : (pure x : Id α) = x := rfl

theorem id_bind {α β} (x : Id α) (f : α → Id β) : x >>= f = f x := rfl

theorem acc_eq {w} (x : BitVec w) (i : Nat) : BitVec.access x i = BitVec.ofBool (x.getLsbD i) := by
  by_cases h : i < w
  · simp [BitVec.access, h]
  · simp [BitVec.access, h, BitVec.getLsbD_of_ge x i (by omega)]

/-- Unroll every constant-bound Sail loop in the goal, its indices as
    literals. -/
macro "unroll" : tactic => `(tactic| (
  simp only [forIn, ForIn'.forIn', IntRange.forIn']
  simp (disch := decide) only [loop_in, loop_out]
  simp only [Id.run, id_pure, id_bind, Int.reduceAdd, Int.reduceMul, Int.reduceSub, Int.reduceToNat,
    Nat.reduceSub, Nat.reduceAdd]))

/-- A byte's bits reversed. -/
def rb8 (x : BitVec 8) : BitVec 8 :=
  BitVec.ofBool (x.getLsbD 0) ++ BitVec.ofBool (x.getLsbD 1) ++ BitVec.ofBool (x.getLsbD 2) ++
  BitVec.ofBool (x.getLsbD 3) ++ BitVec.ofBool (x.getLsbD 4) ++ BitVec.ofBool (x.getLsbD 5) ++
  BitVec.ofBool (x.getLsbD 6) ++ BitVec.ofBool (x.getLsbD 7)

theorem reverse_bits8 (x : BitVec 8) : reverse_bits x = rb8 x := by
  simp only [reverse_bits, Sail.BitVec.length, zeros]
  unroll
  simp only [rb8, BitVec.update, BitVec.updateSubrange', acc_eq, bv_zero]
  bv_decide

end Equiv
