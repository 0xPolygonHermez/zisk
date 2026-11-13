/-
  Zbb's counts (clz, ctz, cpop) on both sides: sail-zisk counts in a loop
  with an Int (and, for clz and ctz, a flag: done); sail-riscv takes Lean's
  clz for clz and ctz, and counts cpop's ones in a loop of its own, in
  SailM, an assert at each turn.

  The loops are not unrolled as they stand: `loop_map` maps a loop's state
  through a function commuting with its step. clz's and ctz's flag and
  count go to one 65-bit vector (`packC`), whose loop then unrolls into
  bit-vector terms bv_decide compares with Lean's clz (Equiv/Count/, a
  module each, so that they check in parallel). cpop's count goes to 64
  bits on both sides, the same loop then: no unrolling at all. sail-riscv's
  loop runs as the Id loop while its asserts hold (`loopM_run`), which they
  do, the count no more than the turns so far.
-/
import Equiv.RiscvZbbByte
import Zisk

namespace Equiv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem Equiv.Riscv

/-- A loop in Id, its state mapped through `h` commuting with its step:
    the loop on the mapped state. -/
theorem loop_map {β γ} (r : IntRange) (g : Int → β → β) (g' : Int → γ → γ) (h : β → γ)
    (hc : ∀ i b, h (g i b) = g' i (h b)) (b : β) (i : Int) (hs : (i - r.start) % r.step = 0) :
    h (IntRange.forIn'.loop (m := Id) r (fun i _ b => (ForInStep.yield (g i b) : Id (ForInStep β))) b i hs) =
      IntRange.forIn'.loop (m := Id) r (fun i _ c => (ForInStep.yield (g' i c) : Id (ForInStep γ))) (h b) i hs := by
  induction b, i, hs using IntRange.forIn'.loop.induct r with
  | case1 b i hs hm ih =>
    rw [loop_in _ _ _ _ _ hm, loop_in _ _ _ _ _ hm]
    simp only [id_bind, id_pure]
    rw [← hc]; exact ih _
  | case2 b i hs hm =>
    rw [loop_out _ _ _ _ _ hm, loop_out _ _ _ _ _ hm]; rfl


/-- The state of a counting loop, a flag and an Int, as one bit vector. -/
def packC (p : Bool × Int) : BitVec 65 := BitVec.ofBool p.1 ++ BitVec.ofInt 64 p.2

theorem to_bits_snd (p : Bool × Int) :
    _root_.Zisk.Functions.to_bits (l := 64) p.2 = (packC p).extractLsb' 0 64 := by
  simp only [_root_.Zisk.Functions.to_bits, get_slice_int, packC]
  have e : BitVec.extractLsb' 0 64 (BitVec.ofInt 65 p.2) = BitVec.ofInt 64 p.2 := by
    apply BitVec.eq_of_toNat_eq
    simp only [BitVec.extractLsb'_toNat, BitVec.toNat_ofInt]
    simp only [Nat.shiftRight_zero]
    omega
  simp only [Nat.zero_add] at e ⊢
  rw [e]
  generalize BitVec.ofInt 64 p.2 = y
  bv_decide

/-- clz's and ctz's step on the packed state. -/
def cntStep {w} (x : BitVec w) (i : Int) (c : BitVec 65) : BitVec 65 :=
  if !(c.getLsbD 64) then
    (if BitVec.access x i.toNat == 1#1 then 1#1 ++ c.extractLsb' 0 64 else 0#1 ++ (c.extractLsb' 0 64 + 1))
  else c

theorem cntStep_comm {w} (x : BitVec w) (i : Int) (r : Bool × Int) :
    packC ((if (!r.fst) = true then
            ((if (BitVec.access x i.toNat == 1#1) = true then (true, r.snd) else (r.fst, r.snd + 1)).fst,
              (if (BitVec.access x i.toNat == 1#1) = true then (true, r.snd) else (r.fst, r.snd + 1)).snd)
          else (r.fst, r.snd)).fst,
        (if (!r.fst) = true then
            ((if (BitVec.access x i.toNat == 1#1) = true then (true, r.snd) else (r.fst, r.snd + 1)).fst,
              (if (BitVec.access x i.toNat == 1#1) = true then (true, r.snd) else (r.fst, r.snd + 1)).snd)
          else (r.fst, r.snd)).snd) = cntStep x i (packC r) := by
  obtain ⟨d, n⟩ := r
  cases d <;> by_cases hb : (BitVec.access x i.toNat == 1#1) = true <;>
    simp only [packC, cntStep, hb, Bool.not_false, Bool.not_true, if_true, if_false, Bool.false_eq_true,
      BitVec.ofBool_false, BitVec.ofBool_true, BitVec.ofInt_add, show BitVec.ofInt 64 1 = 1#64 from rfl] <;>
    (generalize BitVec.ofInt 64 n = y; bv_decide)

theorem packC_zero : packC (false, 0) = 0#65 := by simp [packC]

macro "count_loop" : tactic => `(tactic| (
  rw [to_bits_snd]
  rw [loop_map _ _ (cntStep _) packC (cntStep_comm _)]
  simp (config := { maxSteps := 10000000 }) (disch := decide) only [loop_in, loop_out]
  simp (config := { maxSteps := 10000000 }) only [Id.run, id_pure, id_bind, Int.cast_ofNat_Int, Int.reduceAdd,
    Int.reduceSub, Int.reduceToNat, Int.reduceNeg]
  simp only [packC_zero, cntStep, acc_eq]))

/-- A Sail loop in SailM counting up by 0 or 1 per turn, step 1, its
    asserts holding while the count stays within the turns left: runs as
    the same loop in Id. -/
theorem loopM_run (r : IntRange) (hr : r.step = 1) (L : Nat)
    (f : (i : Int) → i ∈ r → Nat → SailM (ForInStep Nat)) (g : Int → Nat → Nat)
    (hg : ∀ i b, g i b ≤ b + 1)
    (hf : ∀ i h (b : Nat) s, (b : Int) + 1 ≤ L → (interpretSailEffects (f i h b)).run s = .ok (.yield (g i b)) s)
    (s : RVState) (b : Nat) (i : Int) (hs : (i - r.start) % r.step = 0)
    (hb : (b : Int) + (r.stop - i + 1) ≤ L) (hi : i ≤ r.stop + 1) :
    (interpretSailEffects (IntRange.forIn'.loop r f b i hs)).run s
      = .ok (IntRange.forIn'.loop (m := Id) r (fun i _ b => (ForInStep.yield (g i b) : Id (ForInStep Nat))) b i hs) s := by
  induction b, i, hs using IntRange.forIn'.loop.induct r with
  | case1 b i hs hm ih =>
    rw [loop_in _ _ _ _ _ hm, loop_in _ _ _ _ _ hm]
    have hm' := hm
    simp (config := { decide := true }) only [Membership.mem, hr, if_true] at hm'
    simp only [interp_bind, run_bind']
    rw [hf i hm b s (by omega)]
    simp only [id_bind, id_pure]
    exact ih _ (by have := hg i b; rw [hr]; omega) (by rw [hr]; omega)
  | case2 b i hs hm =>
    rw [loop_out _ _ _ _ _ hm, loop_out _ _ _ _ _ hm]; rfl

/-- cpop's count, as sail-riscv's count_ones loop runs it. -/
def popLoop {w} (x : BitVec w) : Nat :=
  IntRange.forIn'.loop (m := Id) ({ stop := (w : Int) - 1 } : IntRange)
    (fun i _ b => ForInStep.yield (if (BitVec.access x i.toNat == 1#1) = true then b + 1 else b)) 0 0 (by simp)

theorem count_ones_run {w} (x : BitVec w) (s : RVState) :
    (interpretSailEffects (count_ones x)).run s = .ok (popLoop x) s := by
  simp only [count_ones, Sail.BitVec.length, forIn, ForIn'.forIn', IntRange.forIn']
  first | simp only [bind_pure] | erw [interp_bind, run_bind']
  rw [loopM_run _ rfl w _ (fun i r => if (BitVec.access x i.toNat == 1#1) = true then r + 1 else r)]
  · rfl
  · intro i b; split <;> omega
  · intro i h b s hb
    split
    · simp only [interp_bind, run_bind', interp_pure, run_pure', LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
      rw [if_pos (by simp; omega)]
      try simp only [interp_pure, run_pure', if_true]
      try rfl
    · rfl
  · simp
  · simp

/-- The count, mapped to 64 bits. -/
def popStep {w} (x : BitVec w) (i : Int) (c : BitVec 64) : BitVec 64 :=
  if (BitVec.access x i.toNat == 1#1) = true then c + 1 else c

theorem riscv_cpop {w} (x : BitVec w) : (to_bits (l := 64) (popLoop x) : BitVec 64) =
    IntRange.forIn'.loop (m := Id) ({ stop := (w : Int) - 1 } : IntRange)
      (fun i _ c => ForInStep.yield (popStep x i c)) 0#64 0 (by simp) := by
  have : (to_bits (l := 64) (popLoop x) : BitVec 64) = BitVec.ofNat 64 (popLoop x) := by
    simp only [to_bits, get_slice_int]
    apply BitVec.eq_of_toNat_eq; simp
  rw [this, popLoop, loop_map _ _ (popStep x) (BitVec.ofNat 64)]
  all_goals first | rfl | (intro i b; simp only [popStep]; split <;> simp [BitVec.ofNat_add])

theorem zisk_cpop {w} (x : BitVec w) : (_root_.Zisk.Functions.to_bits (l := 64) (_root_.Zisk.Functions.cpop_bits x)) =
    IntRange.forIn'.loop (m := Id) ({ stop := (w : Int) - 1 } : IntRange)
      (fun i _ c => ForInStep.yield (popStep x i c)) 0#64 0 (by simp) := by
  simp only [_root_.Zisk.Functions.cpop_bits, Sail.BitVec.length, forIn, ForIn'.forIn', IntRange.forIn', id_bind,
    id_pure, Id.run]
  have : ∀ n : Int, _root_.Zisk.Functions.to_bits (l := 64) n = BitVec.ofInt 64 n := by
    intro n
    simp only [_root_.Zisk.Functions.to_bits, get_slice_int]
    apply BitVec.eq_of_toNat_eq
    simp only [BitVec.extractLsb'_toNat, BitVec.toNat_ofInt, Nat.shiftRight_zero]
    omega
  rw [this, loop_map _ _ (popStep x) (BitVec.ofInt 64)]
  · rfl
  · intro i b; simp only [popStep]; split <;> simp [BitVec.ofInt_add]

end Equiv
