/-
  From the memory bus's balance to the register chain.

  `reg_chain_balance` (`MemBus.lean`) says that on a sound memory bus every
  register tuple balances, in the field, among Main's arguments and the
  registers' initial tuples. `MemCheck.lean` needs the chain as a permutation
  of lists. This file bridges the two, generically first:

  * `perm_of_times`: lists whose every tuple counts the same in the field, and
    whose lengths are below the characteristic, are permutations of each other
    — a count is a natural number below the characteristic, so its cast
    determines it;
  * `Sender.count_eq_times`: a sender whose count is a bit at every row it
    sends contributes the list of tuples it actually sends (`sent`), one
    each.
-/
import Proofs.MemBus
import Proofs.MemCheck

namespace Correspondence

open Classical

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

section Generic

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-! ### Counts in the field are counts -/

omit [Lean.Grind.IsCharP F goldilocks] in
theorem foldr_add_cast (l : List Nat) :
    (l.map fun n => ((n : Nat) : F)).foldr (· + ·) 0 = ((l.foldr (· + ·) 0 : Nat) : F) := by
  induction l with
  | nil => simp [Lean.Grind.Semiring.natCast_zero]
  | cons a l ih => simp only [List.map_cons, List.foldr_cons, ih, Lean.Grind.Semiring.natCast_add]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- How many times a list holds a tuple, in the field, is the cast of its count. -/
theorem times_eq_count (l : List (List F)) (τ : List F) : times l τ = ((l.count τ : Nat) : F) := by
  induction l with
  | nil => simp [times, Lean.Grind.Semiring.natCast_zero]
  | cons a l ih =>
    simp only [times, List.map_cons, List.foldr_cons] at ih ⊢
    rw [ih, List.count_cons]
    by_cases h : a = τ
    · subst h
      simp only [if_true, beq_self_eq_true, Lean.Grind.Semiring.natCast_add,
        Lean.Grind.Semiring.natCast_one]
      exact Lean.Grind.Semiring.add_comm _ _
    · have : (a == τ) = false := by simpa using h
      simp only [h, if_false, this, Bool.false_eq_true, Nat.add_zero]
      exact Lean.Grind.Semiring.zero_add _

omit [Lean.Grind.IsCharP F goldilocks] in
theorem times_append (l l' : List (List F)) (τ : List F) :
    times (l ++ l') τ = times l τ + times l' τ := by
  simp only [times, List.map_append, foldr_add_append]

/-- **Equal counts in the field make a permutation**, for lists shorter than
the characteristic. -/
theorem perm_of_times {l l' : List (List F)} (hl : l.length < goldilocks)
    (hl' : l'.length < goldilocks) (h : ∀ τ, times l τ = times l' τ) : l.Perm l' := by
  rw [List.perm_iff_count]
  intro τ
  have e := h τ
  rw [times_eq_count, times_eq_count] at e
  exact natCast_inj (F := F) (Nat.lt_of_le_of_lt (List.count_le_length) hl)
    (Nat.lt_of_le_of_lt (List.count_le_length) hl') e

/-! ### What a sender sends -/

/-- The tuples a sender actually sends: one per row it sends at with count 1. -/
noncomputable def Sender.sent (s : Sender F) : List (List F) :=
  s.at.filterMap fun j => if s.p.arg.reps s.p.x s.p.t j = 1 then some (tupleOf s.p j) else none

omit [Lean.Grind.IsCharP F goldilocks] in
theorem count_rows (s : Sender F) (js : List Int)
    (hb : ∀ j ∈ js, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (τ : List F) :
    (js.map fun j => if tupleOf s.p j = τ then s.p.arg.reps s.p.x s.p.t j else 0).foldr (· + ·) 0 =
      times (js.filterMap fun j =>
        if s.p.arg.reps s.p.x s.p.t j = 1 then some (tupleOf s.p j) else none) τ := by
  induction js with
  | nil => rfl
  | cons j js ih =>
    have hj := hb j (List.mem_cons_self ..)
    have ih := ih (fun k hk => hb k (List.mem_cons_of_mem _ hk))
    simp only [List.map_cons, List.foldr_cons, List.filterMap_cons]
    rw [ih]
    rcases hj with h0 | h1
    · have hne : s.p.arg.reps s.p.x s.p.t j ≠ 1 := by
        rw [h0]; exact fun h => Lean.Grind.Field.zero_ne_one h
      rw [if_neg hne]
      simp only [h0]
      split <;> exact Lean.Grind.Semiring.zero_add _
    · simp only [h1, if_true, times, List.map_cons, List.foldr_cons]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- **A sender of bits sends its list.** If its count is 0 or 1 at every row it
sends at, its contribution to a tuple is the number of times it sends it. -/
theorem Sender.count_eq_times (s : Sender F)
    (hb : ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (τ : List F) : s.count τ = times s.sent τ :=
  count_rows s s.at hb τ

/-- The tuples senders send, on the proving side. -/
noncomputable def provedList (ss : List (Sender F)) : List (List F) :=
  (ss.filter (·.proving)).flatMap (·.sent)

/-- The tuples senders send, on the assuming side. -/
noncomputable def assumedList (ss : List (Sender F)) : List (List F) :=
  (ss.filter (! ·.proving)).flatMap (·.sent)

omit [Lean.Grind.IsCharP F goldilocks] in
theorem counts_eq_times (ss : List (Sender F))
    (hb : ∀ s ∈ ss, ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (τ : List F) : (ss.map (·.count τ)).foldr (· + ·) 0 = times (ss.flatMap (·.sent)) τ := by
  induction ss with
  | nil => rfl
  | cons s ss ih =>
    simp only [List.map_cons, List.foldr_cons, List.flatMap_cons]
    rw [ih (fun s' hs' => hb s' (List.mem_cons_of_mem _ hs')), times_append,
      s.count_eq_times (hb s (List.mem_cons_self ..))]

omit [Lean.Grind.IsCharP F goldilocks] in
theorem provedCount_eq (ss : List (Sender F))
    (hb : ∀ s ∈ ss, ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (τ : List F) : provedCount ss τ = times (provedList ss) τ :=
  counts_eq_times _ (fun s hs => hb s (List.mem_filter.mp hs).1) τ

omit [Lean.Grind.IsCharP F goldilocks] in
theorem assumedCount_eq (ss : List (Sender F))
    (hb : ∀ s ∈ ss, ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (τ : List F) : assumedCount ss τ = times (assumedList ss) τ :=
  counts_eq_times _ (fun s hs => hb s (List.mem_filter.mp hs).1) τ

/-- **A balance of bits is a permutation.** If every sender's count is a bit,
and the lists are shorter than the characteristic, the tuples proved with first
element `k`, satisfying `q`, are a permutation of those assumed plus the extra
list `I`'s — when every such tuple balances. -/
theorem perm_of_balance (ss : List (Sender F)) (I : List (List F)) (q : List F → Bool)
    (hb : ∀ s ∈ ss, ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (hlen : (provedList ss).length < goldilocks)
    (hlen' : (assumedList ss).length + I.length < goldilocks)
    (h : ∀ τ, q τ = true → provedCount ss τ = assumedCount ss τ + times I τ) :
    ((provedList ss).filter q).Perm ((assumedList ss ++ I).filter q) := by
  apply perm_of_times
  · exact Nat.lt_of_le_of_lt (List.length_filter_le _ _) hlen
  · exact Nat.lt_of_le_of_lt (List.length_filter_le _ _) (by rw [List.length_append]; exact hlen')
  · intro τ
    rw [times_eq_count, times_eq_count]
    by_cases hq : q τ = true
    · rw [List.count_filter hq, List.count_filter hq, ← times_eq_count, ← times_eq_count,
        times_append, ← provedCount_eq ss hb, ← assumedCount_eq ss hb]
      exact h τ hq
    · have z : ∀ l : List (List F), (l.filter q).count τ = 0 := fun l =>
        List.count_eq_zero.mpr (fun hm => hq (List.mem_filter.mp hm).2)
      rw [z, z]

/-! ### Raw accesses: steps in the field, lifted -/

/-- A permutation of images, under a map injective on both lists, is a
permutation. -/
theorem perm_of_map_inj {α β : Type} [DecidableEq α] [DecidableEq β] {f : α → β} {l l' : List α}
    (hinj : ∀ a ∈ l ++ l', ∀ b ∈ l ++ l', f a = f b → a = b)
    (h : (l.map f).Perm (l'.map f)) : l.Perm l' := by
  rw [List.perm_iff_count] at h ⊢
  intro a
  by_cases ha : a ∈ l ++ l'
  · have key : ∀ m : List α, (∀ b ∈ m, b ∈ l ++ l') → m.count a = (m.map f).count (f a) := by
      intro m hm
      rw [List.count, List.count, List.countP_map]
      apply List.countP_congr
      intro b hb
      simp only [Function.comp, beq_iff_eq]
      exact ⟨fun h => h ▸ rfl, fun h => hinj b (hm b hb) a ha h⟩
    rw [key l (fun b hb => List.mem_append_left _ hb), key l' (fun b hb => List.mem_append_right _ hb)]
    exact h (f a)
  · rw [List.count_eq_zero.mpr (fun h' => ha (List.mem_append_left _ h')),
      List.count_eq_zero.mpr (fun h' => ha (List.mem_append_right _ h'))]

/-- The natural number below the characteristic a field element is the cast of,
or 0 if it is none. Irreducible: unfolding its classical choice in a
definitional check does not terminate in practice. -/
@[irreducible] noncomputable def natOf (x : F) : Nat :=
  if h : ∃ n, n < goldilocks ∧ ((n : Nat) : F) = x then Classical.choose h else 0

theorem natOf_cast {n : Nat} (hn : n < goldilocks) : natOf ((n : Nat) : F) = n := by
  have h : ∃ m, m < goldilocks ∧ ((m : Nat) : F) = ((n : Nat) : F) := ⟨n, hn, rfl⟩
  unfold natOf
  rw [dif_pos h]
  obtain ⟨hm, e⟩ := Classical.choose_spec h
  exact natCast_inj (F := F) hm hn e

/-- One register access as the trace holds it: its memory step (a natural, the
step formula's), the previous step it proves (a column: a field element), the
value it proves for the previous access, and the value it assumes at its own
step, if it does. -/
structure Raw (F : Type) where
  step : Nat
  prevF : F
  pv : F × F
  cur : Option (F × F)

/-- The tuple it proves, for register `r`. -/
def Raw.pTuple (r : F) (a : Raw F) : List F := [3, r, a.prevF, 8, a.pv.1, a.pv.2]

/-- The tuple it assumes, if any. -/
def Raw.aTuple (r : F) (a : Raw F) : Option (List F) :=
  a.cur.map fun v => [3, r, ((a.step : Nat) : F), 8, v.1, v.2]

/-- Register `r`'s initial tuple. -/
def initTuple (r : F) : List F := [3, r, 0, 8, 0, 0]

/-- As `MemCheck` sees it: the previous step lifted to a natural. -/
noncomputable def Raw.toAccess (a : Raw F) : MemCheck.Access (F × F) :=
  ⟨a.step, natOf a.prevF, a.pv, a.cur⟩

/-- The tuple a `MemCheck` step and value stand for. -/
def enc (r : F) (sv : Nat × (F × F)) : List F := [3, r, ((sv.1 : Nat) : F), 8, sv.2.1, sv.2.2]

/-- **A register's raw accesses are a `MemCheck` chain.** If the tuples they
prove are a permutation of those they assume and the initial one, their steps
are distinct, positive and far below the characteristic, and every gap
`step - prev - 1` is a range-checked 24-bit number, then, with the previous
steps lifted to naturals, they satisfy `MemCheck`'s hypotheses: the
permutation, `prev < step`, distinct steps. -/
theorem chain_of_raw (r : F) (rs : List (Raw F))
    (hperm : (rs.map (·.pTuple r)).Perm (rs.filterMap (·.aTuple r) ++ [initTuple r]))
    (hstep : ∀ a ∈ rs, 0 < a.step ∧ a.step < 4611686018427387904)
    (hgap : ∀ a ∈ rs, ∃ n : Nat, n < 16777216 ∧ ((a.step : Nat) : F) - a.prevF - 1 = ((n : Nat) : F))
    (hnd : (rs.map (·.step)).Nodup) :
    (MemCheck.proved (rs.map Raw.toAccess)).Perm (MemCheck.assumed (0, 0) (rs.map Raw.toAccess)) ∧
      (∀ e ∈ rs.map Raw.toAccess, e.prev < e.step) ∧
      ((rs.map Raw.toAccess).map (·.step)).Nodup := by
  -- Every proved tuple is an assumed one, so its previous step is a step's cast, or 0.
  have hcast : ∀ a ∈ rs, ∃ s, s < 4611686018427387904 ∧ a.prevF = ((s : Nat) : F) := by
    intro a ha
    have hm := hperm.mem_iff.mp (List.mem_map.mpr ⟨a, ha, rfl⟩)
    rcases List.mem_append.mp hm with hm | hm
    · obtain ⟨b, hb, hbt⟩ := List.mem_filterMap.mp hm
      simp only [Raw.aTuple, Option.map_eq_some_iff] at hbt
      obtain ⟨v, -, hv⟩ := hbt
      simp only [Raw.pTuple, List.cons.injEq] at hv
      exact ⟨b.step, (hstep b hb).2, hv.2.2.1.symm⟩
    · simp only [List.mem_singleton, Raw.pTuple, initTuple, List.cons.injEq] at hm
      exact ⟨0, by decide, by rw [hm.2.2.1, Lean.Grind.Semiring.natCast_zero]⟩
  have hnat : ∀ a ∈ rs, a.prevF = ((natOf a.prevF : Nat) : F) ∧ natOf a.prevF < 4611686018427387904 := by
    intro a ha
    obtain ⟨s, hs, e⟩ := hcast a ha
    rw [e, natOf_cast (by unfold goldilocks; omega)]
    exact ⟨rfl, hs⟩
  refine ⟨?_, ?_, ?_⟩
  · -- The permutation, lifted through the encoding.
    apply perm_of_map_inj (f := enc r)
    · intro x hx y hy hxy
      simp only [enc, List.cons.injEq] at hxy
      obtain ⟨-, -, h1, -, h2, h3⟩ := hxy
      have bound : ∀ z ∈ MemCheck.proved (rs.map Raw.toAccess) ++
          MemCheck.assumed (0, 0) (rs.map Raw.toAccess), z.1 < 4611686018427387904 := by
        intro z hz
        rcases List.mem_append.mp hz with hz | hz
        · simp only [MemCheck.proved, List.map_map, List.mem_map] at hz
          obtain ⟨a, ha, rfl⟩ := hz
          exact (hnat a ha).2
        · rcases (MemCheck.mem_assumed).mp (by rw [show z = (z.1, z.2) from rfl] at hz; exact hz)
            with ⟨h0, -⟩ | ⟨e, he, -, hs⟩
          · rw [h0]; omega
          · simp only [List.mem_map] at he
            obtain ⟨a, ha, rfl⟩ := he
            rw [← hs]; exact (hstep a ha).2
      have := natCast_inj (F := F) (by have := bound x hx; unfold goldilocks; omega)
        (by have := bound y hy; unfold goldilocks; omega) h1
      exact Prod.ext this (Prod.ext h2 h3.1)
    · have e1 : (MemCheck.proved (rs.map Raw.toAccess)).map (enc r) = rs.map (·.pTuple r) := by
        simp only [MemCheck.proved, List.map_map]
        apply List.map_congr_left
        intro a ha
        simp only [Function.comp, enc, Raw.toAccess, Raw.pTuple]
        rw [← (hnat a ha).1]
      have e2 : (MemCheck.assumed (0, 0) (rs.map Raw.toAccess)).map (enc r) =
          initTuple r :: rs.filterMap (·.aTuple r) := by
        simp only [MemCheck.assumed, List.map_cons, enc, initTuple, Lean.Grind.Semiring.natCast_zero,
          List.filterMap_map, List.map_filterMap]
        congr 1
        congr 1
        funext a
        cases hc : a.cur <;> simp [Raw.toAccess, Raw.aTuple, hc, Function.comp, enc]
      rw [e1, e2]
      exact hperm.trans (List.perm_append_comm.trans (List.Perm.refl _))
  · intro e he
    obtain ⟨a, ha, rfl⟩ := List.mem_map.mp he
    obtain ⟨n, hn, hg⟩ := hgap a ha
    obtain ⟨hp, hpb⟩ := hnat a ha
    have hs := hstep a ha
    have : ((a.step : Nat) : F) = ((natOf a.prevF + n + 1 : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add,
        Lean.Grind.Semiring.natCast_one, ← hp, ← hg]
      grind
    have := natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega) this
    show natOf a.prevF < a.step
    omega
  · simpa [List.map_map, Function.comp, Raw.toAccess] using hnd

/-! ### Pairs: an access kind's two arguments -/

/-- Element `k` of a tuple, 0 past its end. -/
def el (k : Nat) (τ : List F) : F := τ.getD k 0

/-- Whether a tuple is register `r`'s: op 3, address `r`. -/
noncomputable def isReg (r : F) (τ : List F) : Bool := decide (el 0 τ = 3 ∧ el 1 τ = r)

/-- What a pair needs of each row it sends at. -/
structure Pair.Good (P : Pair F) : Prop where
  same : P.cur.at = P.pre.at
  side_pre : P.pre.proving = true
  side_cur : P.cur.proving = false
  bits_pre : ∀ j ∈ P.pre.at, P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 0 ∨
    P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1
  bits_cur : ∀ j ∈ P.pre.at, P.cur.p.arg.reps P.cur.p.x P.cur.p.t j = 0 ∨
    P.cur.p.arg.reps P.cur.p.x P.cur.p.t j = 1
  /-- The proved tuple: op 3, an address, the previous step, 8 bytes, a value. -/
  shape : ∀ j ∈ P.pre.at, tupleOf P.pre.p j =
    [3, el 1 (tupleOf P.pre.p j), el 2 (tupleOf P.pre.p j), 8,
      el 4 (tupleOf P.pre.p j), el 5 (tupleOf P.pre.p j)]
  /-- Whenever the assuming side sends a register tuple, the proving side sends
  too, and the assumed tuple is the access's own: its address, its step. -/
  cur_reg : ∀ j ∈ P.pre.at, P.cur.p.arg.reps P.cur.p.x P.cur.p.t j = 1 →
    el 0 (tupleOf P.cur.p j) = 3 →
      P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1 ∧
        tupleOf P.cur.p j = [3, el 1 (tupleOf P.pre.p j), ((P.step j : Nat) : F), 8,
          el 4 (tupleOf P.cur.p j), el 5 (tupleOf P.cur.p j)]

/-- The pair's accesses to register `r`: at each row where the proving side
sends and the address is `r`. -/
noncomputable def Pair.raws (P : Pair F) (r : F) : List (Raw F) :=
  P.pre.at.filterMap fun j =>
    if P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1 ∧ el 1 (tupleOf P.pre.p j) = r then
      some ⟨P.step j, el 2 (tupleOf P.pre.p j),
        (el 4 (tupleOf P.pre.p j), el 5 (tupleOf P.pre.p j)),
        if P.cur.p.arg.reps P.cur.p.x P.cur.p.t j = 1 ∧ el 0 (tupleOf P.cur.p j) = 3 then
          some (el 4 (tupleOf P.cur.p j), el 5 (tupleOf P.cur.p j)) else none⟩
    else none

omit [Lean.Grind.IsCharP F goldilocks] in
theorem el_cons_zero (a : F) (l : List F) : el 0 (a :: l) = a := rfl
omit [Lean.Grind.IsCharP F goldilocks] in
theorem el_cons_succ (k : Nat) (a : F) (l : List F) : el (k + 1) (a :: l) = el k l := rfl

omit [Lean.Grind.IsCharP F goldilocks] in
/-- **The proved side.** A good pair's register-`r` tuples, as proved, are its
raw accesses' proved tuples. -/
theorem Pair.pre_sent (P : Pair F) (hP : P.Good) (r : F) :
    P.pre.sent.filter (isReg r) = (P.raws r).map (·.pTuple r) := by
  unfold Sender.sent Pair.raws
  have hshape := hP.shape
  generalize P.pre.at = js at hshape ⊢
  induction js with
  | nil => rfl
  | cons j js ih =>
    have hs := hshape j (List.mem_cons_self ..)
    have ih := ih (fun k hk => hshape k (List.mem_cons_of_mem _ hk))
    simp only [List.filterMap_cons]
    by_cases h1 : P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1
    · by_cases h2 : el 1 (tupleOf P.pre.p j) = r
      · have e0 : el 0 (tupleOf P.pre.p j) = 3 := by rw [hs]; rfl
        simp only [h1, h2, if_true, and_self, List.filter_cons, List.map_cons]
        have hreg : isReg r (tupleOf P.pre.p j) = true := by simp [isReg, e0, h2]
        rw [if_pos hreg, ih]
        congr 1
        conv => lhs; rw [hs]
        simp only [Raw.pTuple, h2]
      · simp only [h1, h2, if_true, and_false, if_false, List.filter_cons]
        have hreg : isReg r (tupleOf P.pre.p j) = false := by simp [isReg, h2]
        rw [if_neg (by simp [hreg]), ih]
    · simp only [h1, if_false, false_and]
      exact ih

omit [Lean.Grind.IsCharP F goldilocks] in
theorem aTuple_none_of {r : F} {a : Raw F} (h : a.cur = none) : a.aTuple r = none := by
  simp [Raw.aTuple, h]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- **The assumed side.** A good pair's register-`r` tuples, as assumed, are its
raw accesses' assumed tuples. -/
theorem Pair.cur_sent (P : Pair F) (hP : P.Good) (r : F) :
    P.cur.sent.filter (isReg r) = (P.raws r).filterMap (·.aTuple r) := by
  unfold Sender.sent Pair.raws
  rw [hP.same]
  have hreg := hP.cur_reg
  generalize P.pre.at = js at hreg ⊢
  induction js with
  | nil => rfl
  | cons j js ih =>
    have hc := hreg j (List.mem_cons_self ..)
    have ih := ih (fun k hk => hreg k (List.mem_cons_of_mem _ hk))
    simp only [List.filterMap_cons]
    by_cases h1 : P.cur.p.arg.reps P.cur.p.x P.cur.p.t j = 1
    · by_cases h3 : el 0 (tupleOf P.cur.p j) = 3
      · obtain ⟨hp, ht⟩ := hc h1 h3
        have e1 : el 1 (tupleOf P.cur.p j) = el 1 (tupleOf P.pre.p j) := by rw [ht]; rfl
        by_cases h2 : el 1 (tupleOf P.pre.p j) = r
        · simp only [h1, if_true, List.filter_cons, hp, h2, and_self, h3, List.filterMap_cons]
          have hr : isReg r (tupleOf P.cur.p j) = true := by simp [isReg, h3, e1, h2]
          rw [if_pos hr, ih]
          simp only [Raw.aTuple, Option.map_some]
          congr 1
          conv => lhs; rw [ht]
          simp only [h2]
        · simp only [h1, if_true, List.filter_cons, hp, h2, and_false, if_false]
          have hr : isReg r (tupleOf P.cur.p j) = false := by simp [isReg, e1, h2]
          rw [if_neg (by simp [hr]), ih]
      · simp only [h1, if_true, List.filter_cons]
        have hr : isReg r (tupleOf P.cur.p j) = false := by simp [isReg, h3]
        rw [if_neg (by simp [hr]), ih]
        by_cases hp : P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1 ∧ el 1 (tupleOf P.pre.p j) = r
        · simp only [hp, and_self, if_true, List.filterMap_cons, h1, h3, and_false, if_false,
            Raw.aTuple, Option.map_none]
        · simp only [hp, if_false]
    · have hne : ¬(P.cur.p.arg.reps P.cur.p.x P.cur.p.t j = 1 ∧ el 0 (tupleOf P.cur.p j) = 3) :=
        fun h => h1 h.1
      simp only [h1, if_false]
      rw [ih]
      by_cases hp : P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1 ∧ el 1 (tupleOf P.pre.p j) = r
      · simp only [hp, and_self, if_true, List.filterMap_cons]
        rw [aTuple_none_of]
        simp only [false_and, if_false]
      · simp only [hp, if_false]

/-! ### Many pairs: a register's chain -/

omit [Lean.Grind.IsCharP F goldilocks] in
theorem provedList_pairs (Ps : List (Pair F)) (hg : ∀ P ∈ Ps, P.Good) :
    provedList (pairSenders Ps) = Ps.flatMap (·.pre.sent) := by
  unfold provedList pairSenders
  induction Ps with
  | nil => rfl
  | cons P Ps ih =>
    have h := hg P (List.mem_cons_self ..)
    rw [List.flatMap_cons, List.filter_append, List.flatMap_append,
      ih (fun Q hQ => hg Q (List.mem_cons_of_mem _ hQ)), List.flatMap_cons]
    simp [List.filter, h.side_pre, h.side_cur]

omit [Lean.Grind.IsCharP F goldilocks] in
theorem assumedList_pairs (Ps : List (Pair F)) (hg : ∀ P ∈ Ps, P.Good) :
    assumedList (pairSenders Ps) = Ps.flatMap (·.cur.sent) := by
  unfold assumedList pairSenders
  induction Ps with
  | nil => rfl
  | cons P Ps ih =>
    have h := hg P (List.mem_cons_self ..)
    rw [List.flatMap_cons, List.filter_append, List.flatMap_append,
      ih (fun Q hQ => hg Q (List.mem_cons_of_mem _ hQ)), List.flatMap_cons]
    simp [List.filter, h.side_pre, h.side_cur]

omit [Lean.Grind.IsCharP F goldilocks] in
theorem pairSenders_bits (Ps : List (Pair F)) (hg : ∀ P ∈ Ps, P.Good) :
    ∀ s ∈ pairSenders Ps, ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨
      s.p.arg.reps s.p.x s.p.t j = 1 := by
  intro s hs j hj
  obtain ⟨P, hP, hs⟩ := List.mem_flatMap.mp hs
  have h := hg P hP
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with rfl | rfl
  · exact h.bits_pre j hj
  · rw [h.same] at hj; exact h.bits_cur j hj

/-- In a mapped range where exactly index `n` passes the filter, the filter
keeps exactly its image. -/
theorem filter_range_one {α : Type} (g : Nat → α) (q : α → Bool) (n : Nat) :
    ∀ k, n < k → (∀ m, m < k → (q (g m) = true ↔ m = n)) →
      ((List.range k).map g).filter q = [g n]
  | 0, h, _ => absurd h (Nat.not_lt_zero _)
  | k + 1, h, hq => by
    rw [List.range_succ, List.map_append, List.filter_append]
    by_cases hk : n < k
    · rw [filter_range_one g q n k hk (fun m hm => hq m (by omega))]
      have : q (g k) = false := by
        cases e : q (g k)
        · rfl
        · have := (hq k (by omega)).mp e; omega
      simp [this]
    · have hnk : n = k := by omega
      subst hnk
      have : ((List.range n).map g).filter q = [] := by
        rw [List.filter_eq_nil_iff]
        intro a ha
        obtain ⟨m, hm, rfl⟩ := List.mem_map.mp ha
        have hm := List.mem_range.mp hm
        intro e
        have := (hq m (by omega)).mp e
        omega
      rw [this]
      simp [(hq n (by omega)).mpr rfl]

/-- Register `x(n+1)`'s initial tuple is the only one of the registers' initial
tuples that is register `n+1`'s. -/
theorem regInits_filter {n : Nat} (hn : n < 39) :
    (regInits (F := F)).filter (isReg ((n + 1 : Nat) : F)) = [initTuple ((n + 1 : Nat) : F)] := by
  unfold regInits
  apply filter_range_one (fun r => regInit (F := F) (r + 1)) _ n 39 hn
  intro m hm
  simp only [isReg, regInit, el, List.getD_cons_zero, List.getD_cons_succ, decide_eq_true_eq,
    true_and]
  constructor
  · intro h
    have := natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega) h
    omega
  · intro h; rw [h]

theorem flatMap_congr_mem {α β : Type _} {l : List α} {f g : α → List β}
    (h : ∀ a ∈ l, f a = g a) : l.flatMap f = l.flatMap g := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    simp only [List.flatMap_cons, h a (List.mem_cons_self ..),
      ih (fun b hb => h b (List.mem_cons_of_mem _ hb))]

/-- **A register's chain, over pairs.** If the pairs are good, every register
tuple balances among their arguments and the registers' initial tuples, the
lists are far shorter than the characteristic, and register `x(n+1)`'s raw
accesses have positive, distinct steps far below it and range-checked gaps,
then they are a `MemCheck` chain. -/
theorem pairs_chain (Ps : List (Pair F)) (hg : ∀ P ∈ Ps, P.Good) {n : Nat} (hn : n < 39)
    (hbal : ∀ τ, el 0 τ = 3 →
      provedCount (pairSenders Ps) τ = assumedCount (pairSenders Ps) τ + times regInits τ)
    (hlen : (provedList (pairSenders Ps)).length < goldilocks)
    (hlen' : (assumedList (pairSenders Ps)).length + (regInits (F := F)).length < goldilocks)
    (hstep : ∀ a ∈ Ps.flatMap (·.raws ((n + 1 : Nat) : F)), 0 < a.step ∧ a.step < 4611686018427387904)
    (hgap : ∀ a ∈ Ps.flatMap (·.raws ((n + 1 : Nat) : F)), ∃ m : Nat, m < 16777216 ∧
      ((a.step : Nat) : F) - a.prevF - 1 = ((m : Nat) : F))
    (hnd : ((Ps.flatMap (·.raws ((n + 1 : Nat) : F))).map (·.step)).Nodup) :
    (MemCheck.proved ((Ps.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess)).Perm
        (MemCheck.assumed (0, 0) ((Ps.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess)) ∧
      (∀ e ∈ (Ps.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess, e.prev < e.step) ∧
      (((Ps.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess).map (·.step)).Nodup := by
  have hp := perm_of_balance (pairSenders Ps) regInits (isReg ((n + 1 : Nat) : F))
    (pairSenders_bits Ps hg) hlen hlen'
    (fun τ hq => hbal τ (by simp only [isReg, decide_eq_true_eq] at hq; exact hq.1))
  rw [List.filter_append, regInits_filter hn, provedList_pairs Ps hg, assumedList_pairs Ps hg,
    List.filter_flatMap, List.filter_flatMap] at hp
  have e1 : (Ps.flatMap fun P => P.pre.sent.filter (isReg ((n + 1 : Nat) : F))) =
      (Ps.flatMap (·.raws ((n + 1 : Nat) : F))).map (·.pTuple ((n + 1 : Nat) : F)) := by
    rw [List.map_flatMap]
    exact flatMap_congr_mem (fun P hP => P.pre_sent (hg P hP) _)
  have e2 : (Ps.flatMap fun P => P.cur.sent.filter (isReg ((n + 1 : Nat) : F))) =
      (Ps.flatMap (·.raws ((n + 1 : Nat) : F))).filterMap (·.aTuple ((n + 1 : Nat) : F)) := by
    rw [List.filterMap_flatMap]
    exact flatMap_congr_mem (fun P hP => P.cur_sent (hg P hP) _)
  rw [e1, e2] at hp
  exact chain_of_raw _ _ hp hstep hgap hnd

end Generic

end Correspondence
