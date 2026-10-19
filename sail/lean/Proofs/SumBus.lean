/-
  What a sound sum bus guarantees, tuple by tuple.

  `LookupHolds` (`Bus.lean`) states a lookup's guarantee in the form its uses
  need: an assumed tuple is proved somewhere. A permutation needs more — the
  memory bus's register chain counts its tuples — so this file states the
  guarantee of a sum bus (the `std` library's gsum: lookups, permutations and
  free arguments alike) in full: for every tuple, the counts proved equal the
  counts assumed.

  * A *sender* is one argument of one AIR instance. It sends its tuple at
    every row of the instance with its count (`reps`), or, if the argument is
    `once` (degree 0: constants and air values), once per instance.
  * `SumHolds b ss gp ga`: on bus `b`, for every tuple `τ`, what the senders
    `ss` prove (and the free ones send, a signed count) plus the global
    updates' proves `gp` equals what they assume plus the global assumes `ga`
    — in the field, as the gsum argument balances them.

  That is what a sound gsum argument gives (the bus id is part of the
  compressed tuple, and a random challenge separates distinct tuples); like
  the other buses' guarantees, it is a hypothesis.

  `count_eq_zero_of_never`: a sender that never sends `τ`'s first element at a
  row where its count is nonzero contributes nothing to `τ`'s count. That is
  how the memory bus's register tuples are reduced to Main's (`MemBus.lean`).
-/
import Proofs.Bus

namespace Correspondence

open Classical

/-- One argument of one AIR instance, as it sends on a bus: the prover-shaped
argument, and the instance's number of rows. -/
structure Sender (F : Type) where
  p : Prover F
  rows : Nat

variable {F : Type} [PilField F]

/-- The tuple an argument sends at row `j`. -/
def tupleOf (p : Prover F) (j : Int) : List F := p.arg.tuple.map fun f => f p.x p.t j

/-- The rows a sender sends at: every row, or row 0 for an argument sent once
(it does not depend on the row). -/
def Sender.at (s : Sender F) : List Int :=
  if s.p.arg.once then [0] else (List.range s.rows).map Int.ofNat

/-- What a sender contributes to tuple `τ`: its count at each row it sends
`τ`. -/
noncomputable def Sender.count (s : Sender F) (τ : List F) : F :=
  (s.at.map fun j => if tupleOf s.p j = τ then s.p.arg.reps s.p.x s.p.t j else 0).foldr (· + ·) 0

/-- Whether a sender counts on the proving side: proves, or a free argument,
whose count enters the sum as is. -/
def Sender.proving (s : Sender F) : Bool :=
  match s.p.arg.side with
  | .assumes => false
  | _ => true

/-- The total count the senders `ss` prove `τ` with. -/
noncomputable def provedCount (ss : List (Sender F)) (τ : List F) : F :=
  ((ss.filter (·.proving)).map (·.count τ)).foldr (· + ·) 0

/-- The total count the senders `ss` assume `τ` with. -/
noncomputable def assumedCount (ss : List (Sender F)) (τ : List F) : F :=
  ((ss.filter (! ·.proving)).map (·.count τ)).foldr (· + ·) 0

/-- How many times a list holds `τ`, in the field. -/
noncomputable def times (l : List (List F)) (τ : List F) : F :=
  ((l.map fun x => if x = τ then (1 : F) else 0)).foldr (· + ·) 0

/-- **What a sound sum bus guarantees.** On bus `b`, every sender serves `b`,
and for every tuple, what is proved — by the senders and the global proves
`gp` — equals what is assumed, by the senders and the global assumes `ga`. -/
def SumHolds (b : Nat) (ss : List (Sender F)) (gp ga : List (List F)) : Prop :=
  (∀ s ∈ ss, s.p.arg.opids = [b]) ∧
    ∀ τ : List F, provedCount ss τ + times gp τ = assumedCount ss τ + times ga τ

/-! ### Senders that never send a tuple -/

theorem foldr_add_zero {l : List F} (h : ∀ x ∈ l, x = 0) : l.foldr (· + ·) 0 = 0 := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    simp only [List.foldr_cons]
    rw [h a (List.mem_cons_self ..), ih (fun x hx => h x (List.mem_cons_of_mem _ hx))]
    exact Lean.Grind.Semiring.add_zero 0

/-- A sender whose first element is never `τ`'s at a row it sends with a
nonzero count contributes nothing to `τ`. -/
theorem Sender.count_eq_zero_of_never (s : Sender F) {τ : List F} {k : F}
    (hk : τ.headD 0 = k)
    (h : ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k) :
    s.count τ = 0 := by
  apply foldr_add_zero
  intro x hx
  obtain ⟨j, -, rfl⟩ := List.mem_map.mp hx
  split
  · rename_i heq
    apply Classical.byContradiction
    intro hne
    exact h j hne (by rw [heq, hk])
  · rfl

/-- Senders that never send a tuple contribute nothing to its counts. -/
theorem counts_eq_zero_of_never (ss : List (Sender F)) {τ : List F} {k : F}
    (hk : τ.headD 0 = k)
    (h : ∀ s ∈ ss, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k) :
    provedCount ss τ = 0 ∧ assumedCount ss τ = 0 := by
  constructor <;>
  · apply foldr_add_zero
    intro x hx
    obtain ⟨s, hs, rfl⟩ := List.mem_map.mp hx
    exact s.count_eq_zero_of_never hk (h s (List.mem_filter.mp hs).1)

theorem foldr_add_append (l l' : List F) :
    (l ++ l').foldr (· + ·) 0 = l.foldr (· + ·) 0 + l'.foldr (· + ·) 0 := by
  induction l with
  | nil => simp only [List.nil_append, List.foldr_nil]; exact (Lean.Grind.Semiring.zero_add _).symm
  | cons a l ih =>
    simp only [List.cons_append, List.foldr_cons, ih]
    exact (Lean.Grind.Semiring.add_assoc _ _ _).symm

theorem provedCount_append (ss ss' : List (Sender F)) (τ : List F) :
    provedCount (ss ++ ss') τ = provedCount ss τ + provedCount ss' τ := by
  simp only [provedCount, List.filter_append, List.map_append, foldr_add_append]

theorem assumedCount_append (ss ss' : List (Sender F)) (τ : List F) :
    assumedCount (ss ++ ss') τ = assumedCount ss τ + assumedCount ss' τ := by
  simp only [assumedCount, List.filter_append, List.map_append, foldr_add_append]

/-- **Only the senders of `τ`'s first element count it.** If the senders `ss'`
never send a tuple whose first element is `k`, then on a sound bus over
`ss ++ ss'`, every tuple starting with `k` balances among `ss` and the global
updates alone. -/
theorem SumHolds.restrict {b : Nat} {ss ss' : List (Sender F)} {gp ga : List (List F)}
    (h : SumHolds b (ss ++ ss') gp ga) {k : F}
    (hnever : ∀ s ∈ ss', ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k)
    {τ : List F} (hk : τ.headD 0 = k) :
    provedCount ss τ + times gp τ = assumedCount ss τ + times ga τ := by
  have e := h.2 τ
  obtain ⟨h1, h2⟩ := counts_eq_zero_of_never ss' hk hnever
  rw [provedCount_append, assumedCount_append, h1, h2, Lean.Grind.Semiring.add_zero,
    Lean.Grind.Semiring.add_zero] at e
  exact e

end Correspondence
