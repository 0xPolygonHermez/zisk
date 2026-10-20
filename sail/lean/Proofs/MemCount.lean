/-
  Counting a tuple on a sum bus.

  `SumHolds` balances each tuple's counts in the field. For the memory bus's
  loads and stores the useful form is an integer one: if every assuming
  sender's count on a tuple is a non-negative integer, every count is an
  integer, and they are all small, then the balance is one of integers; so a
  tuple some sender assumes is sent, with a nonzero count, by some proving
  sender (`SumHolds.proved_of_assumed`), and a tuple a proving sender sends is
  assumed by some sender (`SumHolds.assumed_of_proved`). Counts are integers,
  not naturals, because one of the RAM's arguments — its padding's
  cancellation — sends with a negative count.
-/
import Proofs.RegChain

namespace Correspondence

open Classical

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

attribute [local instance] Lean.Grind.Ring.intCast

theorem foldr_intCast (l : List Int) :
    (l.map fun z => ((z : Int) : F)).foldr (· + ·) 0 = ((l.foldr (· + ·) 0 : Int) : F) := by
  induction l with
  | nil => simp [Lean.Grind.Ring.intCast_zero]
  | cons a l ih => simp only [List.map_cons, List.foldr_cons, ih, Lean.Grind.Ring.intCast_add]

/-- The integer count of the senders `ss` on the given side. -/
def sideSum (ss : List (Sender F)) (z : Sender F → Int) (side : Bool) : Int :=
  ((ss.filter (·.proving == side)).map z).foldr (· + ·) 0

theorem count_sum (ss : List (Sender F)) (τ : List F) (z : Sender F → Int)
    (hz : ∀ s ∈ ss, s.count τ = ((z s : Int) : F)) (side : Bool) :
    ((ss.filter (·.proving == side)).map (·.count τ)).foldr (· + ·) 0 = ((sideSum ss z side : Int) : F) := by
  unfold sideSum
  rw [← foldr_intCast, List.map_map]
  congr 1
  apply List.map_congr_left
  intro s hs
  exact hz s (List.mem_filter.mp hs).1

theorem filter_proving (ss : List (Sender F)) :
    ss.filter (·.proving) = ss.filter (·.proving == true) := by
  congr 1; funext s; cases s.proving <;> rfl

theorem filter_assuming (ss : List (Sender F)) :
    ss.filter (! ·.proving) = ss.filter (·.proving == false) := by
  congr 1; funext s; cases s.proving <;> rfl

theorem sideSum_le (ss : List (Sender F)) (z : Sender F → Int) (side : Bool) :
    (sideSum ss z side).natAbs ≤ ((ss.map fun s => (z s).natAbs).foldr (· + ·) 0) := by
  unfold sideSum
  induction ss with
  | nil => simp
  | cons s ss ih =>
    simp only [List.filter_cons, List.map_cons, List.foldr_cons]
    split
    · simp only [List.map_cons, List.foldr_cons]; omega
    · omega

/-- **The balance, in integers.** -/
theorem SumHolds.int_balance {b : Nat} {ss : List (Sender F)} {gp ga : List (List F)}
    (h : SumHolds b ss gp ga) {τ : List F} (hg : times gp τ = times ga τ) (z : Sender F → Int)
    (hz : ∀ s ∈ ss, s.count τ = ((z s : Int) : F))
    (hbound : ((ss.map fun s => (z s).natAbs).foldr (· + ·) 0) < 2 ^ 62) :
    sideSum ss z true = sideSum ss z false := by
  have e := h.2 τ
  unfold provedCount assumedCount at e
  rw [filter_proving, filter_assuming, count_sum ss τ z hz, count_sum ss τ z hz, hg] at e
  have e' : ((sideSum ss z true : Int) : F) = ((sideSum ss z false : Int) : F) := by
    generalize ((sideSum ss z true : Int) : F) = u at e ⊢
    generalize ((sideSum ss z false : Int) : F) = w at e ⊢
    generalize times ga τ = g at e
    grind
  have h1 := sideSum_le ss z true
  have h2 := sideSum_le ss z false
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp e'
  unfold goldilocks at this
  omega

theorem sideSum_nonneg {ss : List (Sender F)} {z : Sender F → Int} {side : Bool}
    (hpos : ∀ s ∈ ss, s.proving = side → 0 ≤ z s) : 0 ≤ sideSum ss z side := by
  unfold sideSum
  induction ss with
  | nil => simp
  | cons s ss ih =>
    have ih' := ih (fun s' hs' => hpos s' (List.mem_cons_of_mem _ hs'))
    simp only [List.filter_cons]
    split
    · rename_i hb
      simp only [List.map_cons, List.foldr_cons]
      have := hpos s (List.mem_cons_self ..) (by simpa using hb)
      omega
    · exact ih'

theorem sideSum_pos {ss : List (Sender F)} {z : Sender F → Int} {side : Bool}
    (hpos : ∀ s ∈ ss, s.proving = side → 0 ≤ z s) {s₀ : Sender F} (hs₀ : s₀ ∈ ss)
    (hside : s₀.proving = side) (h1 : 1 ≤ z s₀) : 1 ≤ sideSum ss z side := by
  induction ss with
  | nil => exact nomatch hs₀
  | cons s ss ih =>
    have hpos' : ∀ s' ∈ ss, s'.proving = side → 0 ≤ z s' := fun s' hs' => hpos s' (List.mem_cons_of_mem _ hs')
    have hrest := sideSum_nonneg hpos'
    unfold sideSum at hrest ih ⊢
    simp only [List.filter_cons]
    rcases List.mem_cons.mp hs₀ with rfl | hs₀
    · simp only [hside, beq_self_eq_true, if_true, List.map_cons, List.foldr_cons]
      omega
    · have := ih hpos' hs₀
      split
      · rename_i hb
        simp only [List.map_cons, List.foldr_cons]
        have := hpos s (List.mem_cons_self ..) (by simpa using hb)
        omega
      · exact this

/-- If the senders on side `side` all have non-positive integer counts, their
sum is not positive: a positive sum has a positive sender. -/
theorem sideSum_exists {ss : List (Sender F)} {z : Sender F → Int} {side : Bool}
    (h : 1 ≤ sideSum ss z side) : ∃ s ∈ ss, s.proving = side ∧ 1 ≤ z s := by
  unfold sideSum at h
  induction ss with
  | nil => simp at h
  | cons s ss ih =>
    simp only [List.filter_cons] at h
    split at h
    · rename_i hb
      simp only [List.map_cons, List.foldr_cons] at h
      by_cases hz : 1 ≤ z s
      · exact ⟨s, List.mem_cons_self .., by simpa using hb, hz⟩
      · obtain ⟨s', hs', h1, h2⟩ := ih (by omega)
        exact ⟨s', List.mem_cons_of_mem _ hs', h1, h2⟩
    · obtain ⟨s', hs', h1, h2⟩ := ih h
      exact ⟨s', List.mem_cons_of_mem _ hs', h1, h2⟩

/-- **A tuple assumed is proved.** On a sound sum bus, with integer counts,
the assuming senders' non-negative and the global updates balancing the tuple
among themselves: if some sender assumes `τ` with a positive count, some
proving sender sends it with a positive count. -/
theorem SumHolds.proved_of_assumed {b : Nat} {ss : List (Sender F)} {gp ga : List (List F)}
    (h : SumHolds b ss gp ga) {τ : List F} (hg : times gp τ = times ga τ) (z : Sender F → Int)
    (hz : ∀ s ∈ ss, s.count τ = ((z s : Int) : F))
    (hbound : ((ss.map fun s => (z s).natAbs).foldr (· + ·) 0) < 2 ^ 62)
    (hpos : ∀ s ∈ ss, s.proving = false → 0 ≤ z s)
    {s₀ : Sender F} (hs₀ : s₀ ∈ ss) (ha : s₀.proving = false) (h1 : 1 ≤ z s₀) :
    ∃ s ∈ ss, s.proving = true ∧ 1 ≤ z s := by
  have e := h.int_balance hg z hz hbound
  have := sideSum_pos hpos hs₀ ha h1
  exact sideSum_exists (by omega)

/-- **A tuple proved is assumed.** The converse, with the proving senders'
counts non-negative. -/
theorem SumHolds.assumed_of_proved {b : Nat} {ss : List (Sender F)} {gp ga : List (List F)}
    (h : SumHolds b ss gp ga) {τ : List F} (hg : times gp τ = times ga τ) (z : Sender F → Int)
    (hz : ∀ s ∈ ss, s.count τ = ((z s : Int) : F))
    (hbound : ((ss.map fun s => (z s).natAbs).foldr (· + ·) 0) < 2 ^ 62)
    (hpos : ∀ s ∈ ss, s.proving = true → 0 ≤ z s)
    {s₀ : Sender F} (hs₀ : s₀ ∈ ss) (hp : s₀.proving = true) (h1 : 1 ≤ z s₀) :
    ∃ s ∈ ss, s.proving = false ∧ 1 ≤ z s := by
  have e := h.int_balance hg z hz hbound
  have := sideSum_pos hpos hs₀ hp h1
  exact sideSum_exists (by omega)

/-- A sender with a positive count on `τ` sends it, at one of its rows, with a
nonzero count. -/
theorem Sender.sends_of_count {s : Sender F} {τ : List F} {z : Int}
    (hz : s.count τ = ((z : Int) : F)) (h1 : 1 ≤ z) (hb : z < 2 ^ 62) :
    ∃ j ∈ s.at, tupleOf s.p j = τ ∧ s.p.arg.reps s.p.x s.p.t j ≠ 0 := by
  apply Classical.byContradiction
  intro hno
  have h0 : s.count τ = 0 := by
    apply foldr_add_zero
    intro x hx
    obtain ⟨j, hj, rfl⟩ := List.mem_map.mp hx
    split
    · rename_i ht
      apply Classical.byContradiction
      intro hr
      exact hno ⟨j, hj, ht, hr⟩
    · rfl
  rw [hz, ← Lean.Grind.Ring.intCast_zero] at h0
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp h0
  unfold goldilocks at this
  omega

/-! ### Counts of the senders -/

/-- A sender of bits counts a tuple as a natural: how many of its rows send
it, at most its number of rows. -/
theorem Sender.count_of_bits (s : Sender F)
    (hb : ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (τ : List F) : s.count τ = ((((s.sent.count τ : Nat) : Int)) : F) ∧ s.sent.count τ ≤ s.at.length := by
  refine ⟨?_, ?_⟩
  · rw [s.count_eq_times hb, times_eq_count, Lean.Grind.Ring.intCast_natCast]
  · calc s.sent.count τ ≤ s.sent.length := List.count_le_length
      _ ≤ s.at.length := List.length_filterMap_le _ _

/-- A sender that never sends `τ`'s first element at a row it sends with a
nonzero count counts it as 0. -/
theorem Sender.count_of_never (s : Sender F) {τ : List F} {k : F} (hk : τ.headD 0 = k)
    (h : ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k) :
    s.count τ = (((0 : Int)) : F) := by
  rw [s.count_eq_zero_of_never hk h, Lean.Grind.Ring.intCast_zero]

end Correspondence
