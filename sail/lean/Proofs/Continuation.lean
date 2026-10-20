/-
  Continuation buses: segments into a chain.

  A memory AIR runs in segments, and its continuation bus links them: segment
  `i` assumes `A i` — its id, whether it is first, and the state it starts
  from — and, unless it is the last, proves `P i` — the next id, 0, and the
  state it ends in. A global update proves the opening tuple `G` — id 0,
  first — once if the area is enabled. On a balanced bus, the segments form
  one chain from `G` (`ContBal.chain`): each segment's assumption is proved by
  the global update or by a segment one id below, following them back reaches
  `G` — ids would otherwise repeat around a loop shorter than the
  characteristic — and the counts allow one segment per id.

  The setting is abstract, by position: the segments are `0, …, N-1`, and
  only the first element of the tuples, the id, is looked at.
-/
import Proofs.RegChain

namespace Correspondence

open Classical

attribute [local instance] Lean.Grind.Semiring.natCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-! ### Counting, pigeonholes, least elements -/

theorem two_le_countP {N : Nat} {q : Nat → Bool} {i i' : Nat} (hi : i < N) (hi' : i' < N) (hne : i ≠ i')
    (h : q i = true) (h' : q i' = true) : 2 ≤ (List.range N).countP q := by
  rw [List.countP_eq_length_filter]
  have m1 : i ∈ (List.range N).filter q := List.mem_filter.mpr ⟨List.mem_range.mpr hi, h⟩
  have m2 : i' ∈ ((List.range N).filter q).erase i :=
    (List.mem_erase_of_ne (Ne.symm hne)).mpr (List.mem_filter.mpr ⟨List.mem_range.mpr hi', h'⟩)
  have := List.length_erase_of_mem m1
  have := List.length_pos_of_mem m2
  omega

theorem countP_le_one {N : Nat} {q : Nat → Bool} {j : Nat} (h : ∀ t < N, q t = true → t = j) :
    (List.range N).countP q ≤ 1 := by
  rw [List.countP_eq_length_filter]
  have hnd : ((List.range N).filter q).Nodup := List.Nodup.sublist List.filter_sublist List.nodup_range
  have hall : ∀ t ∈ (List.range N).filter q, t = j := fun t ht => by
    obtain ⟨h1, h2⟩ := List.mem_filter.mp ht
    exact h t (List.mem_range.mp h1) h2
  generalize (List.range N).filter q = l at hnd hall
  match l, hnd, hall with
  | [], _, _ => simp
  | [_], _, _ => simp
  | a :: b :: _, hnd, hall =>
    exfalso
    have ha := hall a (by simp)
    have hb := hall b (by simp)
    rw [List.nodup_cons] at hnd
    exact hnd.1 (by rw [ha, ← hb]; simp)

/-- **The pigeonhole**: `M + 1` values below `M` repeat. -/
theorem pigeon : ∀ (M : Nat) (f : Nat → Nat), (∀ i ≤ M, f i < M) → ∃ i j, i < j ∧ j ≤ M ∧ f i = f j
  | 0, f, h => absurd (h 0 (Nat.le_refl _)) (by omega)
  | M + 1, f, h => by
    by_cases hv : ∃ i ≤ M, f i = f (M + 1)
    · obtain ⟨i, hi, e⟩ := hv
      exact ⟨i, M + 1, by omega, Nat.le_refl _, e⟩
    · have hne : ∀ i ≤ M, f i ≠ f (M + 1) := fun i hi e => hv ⟨i, hi, e⟩
      have hg : ∀ i ≤ M, (if f i < f (M + 1) then f i else f i - 1) < M := by
        intro i hi
        have h1 := h i (by omega)
        have h2 := h (M + 1) (Nat.le_refl _)
        have := hne i hi
        split <;> omega
      obtain ⟨i, j, hij, hj, e⟩ := pigeon M _ hg
      refine ⟨i, j, hij, by omega, ?_⟩
      have hni := hne i (by omega)
      have hnj := hne j hj
      split at e <;> split at e <;> omega

theorem exists_least {P : Nat → Prop} (n : Nat) (h : P n) : ∃ d ≤ n, P d ∧ ∀ k < d, ¬ P k := by
  induction n using Nat.strongRecOn with
  | ind n ih =>
    by_cases hk : ∃ k < n, P k
    · obtain ⟨k, hk, hP⟩ := hk
      obtain ⟨d, hd, h1, h2⟩ := ih k hk hP
      exact ⟨d, by omega, h1, h2⟩
    · exact ⟨n, Nat.le_refl _, h, fun k hk' hP => hk ⟨k, hk', hP⟩⟩

theorem cast_inj33 {a b : Nat} (ha : a < 2 ^ 33) (hb : b < 2 ^ 33) (h : (a : F) = (b : F)) : a = b :=
  natCast_inj (by unfold goldilocks; omega) (by unfold goldilocks; omega) h

/-! ### The bus, by position -/

/-- **A balanced continuation bus over `N` segments**: segment `i` assumes
`A i` once, and proves `P i` once if `live i` (it is not the last); a global
update proves `G`, `e` times, `e` a bit. Every tuple balances, in naturals;
the first element is the id, `G`'s 0 and `P i`'s one above `A i`'s. -/
structure ContBal (N : Nat) (A P : Nat → List F) (live : Nat → Bool) (e : Nat) (G : List F) : Prop where
  small : N < 2 ^ 32
  e1 : e ≤ 1
  bal : ∀ τ, (List.range N).countP (fun i => decide (A i = τ)) =
    e * (if τ = G then 1 else 0) + (List.range N).countP (fun i => live i && decide (P i = τ))
  hG : el 0 G = 0
  succ : ∀ i < N, el 0 (P i) = el 0 (A i) + 1

namespace ContBal

variable {N : Nat} {A P : Nat → List F} {live : Nat → Bool} {e : Nat} {G : List F}

/-- Every segment's assumption is proved: by the global update, or by a live
segment. -/
theorem proved (h : ContBal N A P live e G) {i : Nat} (hi : i < N) :
    A i = G ∨ ∃ j < N, live j = true ∧ P j = A i := by
  by_cases hr : A i = G
  · exact .inl hr
  · right
    have h1 : 0 < (List.range N).countP (fun i' => decide (A i' = A i)) :=
      List.countP_pos_iff.mpr ⟨i, List.mem_range.mpr hi, by simp⟩
    rw [h.bal, if_neg hr, Nat.mul_zero, Nat.zero_add] at h1
    obtain ⟨j, hj, hq⟩ := List.countP_pos_iff.mp h1
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hq
    exact ⟨j, List.mem_range.mp hj, hq.1, hq.2⟩

end ContBal

/-- The segment that proves `i`'s assumption, if one does. -/
noncomputable def cpred (N : Nat) (A P : Nat → List F) (live : Nat → Bool) (i : Nat) : Nat :=
  if h : ∃ j < N, live j = true ∧ P j = A i then Classical.choose h else i

/-- Following the provers back `k` times. -/
noncomputable def citer (N : Nat) (A P : Nat → List F) (live : Nat → Bool) : Nat → Nat → Nat
  | 0, i => i
  | k + 1, i => cpred N A P live (citer N A P live k i)

namespace ContBal

variable {N : Nat} {A P : Nat → List F} {live : Nat → Bool} {e : Nat} {G : List F}

theorem pred_spec (h : ContBal N A P live e G) {i : Nat} (hi : i < N) (hr : A i ≠ G) :
    cpred N A P live i < N ∧ live (cpred N A P live i) = true ∧ P (cpred N A P live i) = A i := by
  have hex : ∃ j < N, live j = true ∧ P j = A i := by
    rcases h.proved hi with h' | h'
    · exact absurd h' hr
    · exact h'
  unfold cpred
  rw [dif_pos hex]
  exact Classical.choose_spec hex

theorem iter_lt (h : ContBal N A P live e G) {i : Nat} (hi : i < N) : ∀ k, citer N A P live k i < N
  | 0 => hi
  | k + 1 => by
    have ih := iter_lt h hi k
    show cpred N A P live (citer N A P live k i) < N
    unfold cpred
    split
    · rename_i hex; exact (Classical.choose_spec hex).1
    · exact ih

/-- Back `k` steps, none from the global update, the id is `k` lower. -/
theorem iter_id (h : ContBal N A P live e G) {i : Nat} (hi : i < N) :
    ∀ k, (∀ k' < k, A (citer N A P live k' i) ≠ G) →
      el 0 (A (citer N A P live k i)) + ((k : Nat) : F) = el 0 (A i)
  | 0, _ => by
    show el 0 (A i) + ((0 : Nat) : F) = el 0 (A i)
    rw [Lean.Grind.Semiring.natCast_zero, Lean.Grind.Semiring.add_zero]
  | k + 1, hk => by
    have ih := iter_id h hi k (fun k' hk' => hk k' (by omega))
    obtain ⟨hlt, -, hP⟩ := h.pred_spec (iter_lt h hi k) (hk k (by omega))
    have e1 := h.succ (cpred N A P live (citer N A P live k i)) hlt
    rw [hP] at e1
    show el 0 (A (cpred N A P live (citer N A P live k i))) + ((k + 1 : Nat) : F) = el 0 (A i)
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_one, ← ih, e1]
    generalize el 0 (A (cpred N A P live (citer N A P live k i))) = a
    generalize ((k : Nat) : F) = c
    grind

/-- Before reaching the global update, following back never repeats. -/
theorem iter_ne (h : ContBal N A P live e G) {i : Nat} (hi : i < N) {m : Nat} (hm : m ≤ N)
    (hnr : ∀ k' < m, A (citer N A P live k' i) ≠ G) {a b : Nat} (hab : a < b) (hb : b ≤ m) :
    citer N A P live a i ≠ citer N A P live b i := by
  intro e
  have ea := h.iter_id hi a (fun k' hk' => hnr k' (by omega))
  have eb := h.iter_id hi b (fun k' hk' => hnr k' (by omega))
  rw [e] at ea
  have : ((a : Nat) : F) = ((b : Nat) : F) := by
    rw [← eb] at ea
    generalize el 0 (A (citer N A P live b i)) = x at ea
    generalize ((a : Nat) : F) = u at ea ⊢
    generalize ((b : Nat) : F) = v at ea ⊢
    grind
  have := cast_inj33 (F := F) (by have := h.small; omega) (by have := h.small; omega) this
  omega

/-- **Every segment is a finite way from the global update**: its id is the
natural `d < N` of steps back to it. -/
theorem depth_spec (h : ContBal N A P live e G) {i : Nat} (hi : i < N) :
    ∃ d < N, A (citer N A P live d i) = G ∧ el 0 (A i) = ((d : Nat) : F) := by
  have hreach : ∃ d ≤ N, A (citer N A P live d i) = G := by
    apply Classical.byContradiction
    intro hno
    have hnr : ∀ k' < N, A (citer N A P live k' i) ≠ G := fun k' hk' e => hno ⟨k', by omega, e⟩
    obtain ⟨a, b, hab, hb, e⟩ := pigeon N (fun k => citer N A P live k i) (fun k _ => h.iter_lt hi k)
    exact h.iter_ne hi (Nat.le_refl _) hnr hab hb e
  obtain ⟨d0, hd0, hr0⟩ := hreach
  obtain ⟨d, hd, hr, hmin⟩ := exists_least (P := fun d => A (citer N A P live d i) = G) d0 hr0
  have hdN : d < N := by
    apply Classical.byContradiction
    intro hge
    have hdN : d = N := by omega
    obtain ⟨a, b, hab, hb, e⟩ := pigeon N (fun k => citer N A P live k i) (fun k _ => h.iter_lt hi k)
    exact h.iter_ne hi (Nat.le_refl _) (fun k' hk' => hmin k' (by omega)) hab hb e
  refine ⟨d, hdN, hr, ?_⟩
  have := h.iter_id hi d hmin
  rw [hr, h.hG, Lean.Grind.Semiring.zero_add] at this
  exact this.symm

/-- A segment's depth: how many steps back to the global update. -/
noncomputable def depth (h : ContBal N A P live e G) (i : Nat) : Nat :=
  if hi : i < N then Classical.choose (h.depth_spec hi) else 0

theorem depth_lt (h : ContBal N A P live e G) {i : Nat} (hi : i < N) : h.depth i < N := by
  unfold depth; rw [dif_pos hi]; exact (Classical.choose_spec (h.depth_spec hi)).1

theorem depth_id (h : ContBal N A P live e G) {i : Nat} (hi : i < N) :
    el 0 (A i) = ((h.depth i : Nat) : F) := by
  unfold depth; rw [dif_pos hi]; exact (Classical.choose_spec (h.depth_spec hi)).2.2

theorem depth_eq (h : ContBal N A P live e G) {i : Nat} (hi : i < N) {d : Nat} (hd : d < 2 ^ 33)
    (e' : el 0 (A i) = ((d : Nat) : F)) : h.depth i = d := by
  rw [h.depth_id hi] at e'
  exact cast_inj33 (by have := h.depth_lt hi; have := h.small; omega) hd e'

theorem root_of_depth (h : ContBal N A P live e G) {i : Nat} (hi : i < N) (h0 : h.depth i = 0) :
    A i = G := by
  have := (Classical.choose_spec (h.depth_spec hi)).2.1
  have e0 : Classical.choose (h.depth_spec hi) = 0 := by
    have := h0; unfold depth at this; rw [dif_pos hi] at this; exact this
  rw [e0] at this
  exact this

theorem depth_of_root (h : ContBal N A P live e G) {i : Nat} (hi : i < N) (hr : A i = G) :
    h.depth i = 0 :=
  h.depth_eq hi (by omega) (by rw [hr, h.hG, Lean.Grind.Semiring.natCast_zero])

/-- **One step down**: a segment at depth `d + 1` is proved by a live one at
depth `d`. -/
theorem down (h : ContBal N A P live e G) {i : Nat} (hi : i < N) {d : Nat} (hd : h.depth i = d + 1) :
    ∃ j < N, h.depth j = d ∧ live j = true ∧ P j = A i := by
  have hr : A i ≠ G := fun hr => by rw [h.depth_of_root hi hr] at hd; omega
  rcases h.proved hi with h' | ⟨j, hj, hl, hP⟩
  · exact absurd h' hr
  refine ⟨j, hj, ?_, hl, hP⟩
  have e1 := h.succ j hj
  rw [hP, h.depth_id hi, hd, h.depth_id hj] at e1
  have e2 : ((h.depth j + 1 : Nat) : F) = ((d + 1 : Nat) : F) := by
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_one]; exact e1.symm
  have := cast_inj33 (F := F) (by have := h.depth_lt hj; have := h.small; omega)
    (by have := h.depth_lt hi; have := h.small; omega) e2
  omega

/-- **One segment per depth.** -/
theorem unique (h : ContBal N A P live e G) :
    ∀ d, ∀ i < N, ∀ i' < N, h.depth i = d → h.depth i' = d → i = i'
  | 0, i, hi, i', hi', hd, hd' => by
    have r := h.root_of_depth hi hd
    have r' := h.root_of_depth hi' hd'
    apply Classical.byContradiction
    intro hne
    have h2 := two_le_countP (q := fun j => decide (A j = G)) hi hi' hne (by simp [r]) (by simp [r'])
    rw [h.bal, if_pos rfl, Nat.mul_one] at h2
    have h0 : (List.range N).countP (fun j => live j && decide (P j = G)) = 0 := by
      rw [List.countP_eq_zero]
      intro j hj hq
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hq
      have e1 := h.succ j (List.mem_range.mp hj)
      rw [hq.2, h.hG, h.depth_id (List.mem_range.mp hj)] at e1
      have e2 : ((h.depth j + 1 : Nat) : F) = ((0 : Nat) : F) := by
        rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_one,
          Lean.Grind.Semiring.natCast_zero]; exact e1.symm
      have := cast_inj33 (F := F) (by have := h.depth_lt (List.mem_range.mp hj); have := h.small; omega)
        (by omega) e2
      omega
    have := h.e1
    omega
  | d + 1, i, hi, i', hi', hd, hd' => by
    obtain ⟨j, hj, hjd, -, hP⟩ := h.down hi hd
    obtain ⟨j', hj', hjd', -, hP'⟩ := h.down hi' hd'
    have hjj := unique h d j hj j' hj' hjd hjd'
    subst hjj
    have hA : A i' = A i := hP'.symm.trans hP
    have hr : A i ≠ G := fun hr => by rw [h.depth_of_root hi hr] at hd; omega
    apply Classical.byContradiction
    intro hne
    have h2 := two_le_countP (q := fun t => decide (A t = A i)) hi hi' hne (by simp) (by simp [hA])
    rw [h.bal, if_neg hr, Nat.mul_zero, Nat.zero_add] at h2
    have h1 : (List.range N).countP (fun t => live t && decide (P t = A i)) ≤ 1 := by
      apply countP_le_one (j := j)
      intro t ht hq
      simp only [Bool.and_eq_true, decide_eq_true_eq] at hq
      apply unique h d t ht j hj _ hjd
      have e1 := h.succ t ht
      rw [hq.2, h.depth_id hi, hd, h.depth_id ht] at e1
      have e2 : ((h.depth t + 1 : Nat) : F) = ((d + 1 : Nat) : F) := by
        rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_one]; exact e1.symm
      have := cast_inj33 (F := F) (by have := h.depth_lt ht; have := h.small; omega)
        (by have := h.depth_lt hi; have := h.small; omega) e2
      omega
    omega

theorem down_to (h : ContBal N A P live e G) : ∀ n, ∀ i < N, ∀ k, h.depth i = k + n → ∃ j < N, h.depth j = k
  | 0, i, hi, k, hd => ⟨i, hi, hd⟩
  | n + 1, i, hi, k, hd => by
    obtain ⟨j, hj, hjd, -⟩ := h.down hi (d := k + n) (by rw [hd]; omega)
    exact down_to h n j hj k hjd

/-- **Every depth below `N` is a segment's.** -/
theorem hit (h : ContBal N A P live e G) {k : Nat} (hk : k < N) : ∃ i < N, h.depth i = k := by
  apply Classical.byContradiction
  intro hno
  have hall : ∀ i < N, h.depth i < k := by
    intro i hi
    apply Classical.byContradiction
    intro hge
    exact hno (h.down_to (h.depth i - k) i hi k (by omega))
  obtain ⟨a, b, hab, hb, e⟩ := pigeon k (fun i => h.depth i) (fun i hi => hall i (by omega))
  have := h.unique _ a (by omega) b (by omega) rfl e.symm
  omega

/-- **The chain.** On a balanced continuation bus, the segments in some
order `ord 0, …, ord (N-1)` — every one of them, once — form a chain: the
first assumes the global update's tuple, and each one is live and proves what
the next assumes. -/
theorem chain (h : ContBal N A P live e G) :
    ∃ ord : Nat → Nat, (∀ k < N, ord k < N) ∧ (∀ i < N, ∃ k < N, ord k = i) ∧
      (0 < N → A (ord 0) = G) ∧ (∀ k, k + 1 < N → live (ord k) = true ∧ P (ord k) = A (ord (k + 1))) := by
  let ord : Nat → Nat := fun k => if hk : k < N then Classical.choose (h.hit hk) else 0
  have hord : ∀ k (hk : k < N), ord k < N ∧ h.depth (ord k) = k := fun k hk => by
    simp only [ord, dif_pos hk]
    exact Classical.choose_spec (h.hit hk)
  refine ⟨ord, fun k hk => (hord k hk).1, fun i hi => ?_, fun hN => ?_, fun k hk => ?_⟩
  · have hd := h.depth_lt hi
    refine ⟨h.depth i, hd, ?_⟩
    exact h.unique _ _ (hord _ hd).1 i hi (hord _ hd).2 rfl
  · exact h.root_of_depth (hord 0 hN).1 (hord 0 hN).2
  · obtain ⟨j, hj, hjd, hl, hP⟩ := h.down (hord (k + 1) hk).1 (hord (k + 1) hk).2
    have := h.unique k j hj (ord k) (hord k (by omega)).1 hjd (hord k (by omega)).2
    subst this
    exact ⟨hl, hP⟩

end ContBal

/-! ### Over a list of segments -/

theorem countP_range_get {α : Type _} (l : List α) (q : α → Bool) :
    (List.range l.length).countP (fun i => (l[i]?).elim false q) = l.countP q := by
  induction l with
  | nil => rfl
  | cons x l ih =>
    rw [List.length_cons, List.range_succ_eq_map, List.countP_cons, List.countP_map, List.countP_cons, ← ih]
    simp only [List.getElem?_cons_zero, Option.elim_some, Function.comp_def, List.getElem?_cons_succ]

/-- **The bus, from counts over a list of segments**: segment `I` assumes
`aT I` and, while `live I`, proves `pT I`; the global update proves `G`, `e`
times; every tuple balances in the field. -/
theorem contBal_of_counts {α : Type _} (l : List α) (aT pT : α → List F) (live : α → Bool) (e : Nat)
    (G : List F) (hsmall : l.length < 2 ^ 32) (he : e ≤ 1)
    (hbal : ∀ τ, ((l.countP (fun I => live I && decide (pT I = τ)) : Nat) : F) +
        ((e * (if τ = G then 1 else 0) : Nat) : F) = ((l.countP (fun I => decide (aT I = τ)) : Nat) : F))
    (hG : el 0 G = 0) (hsucc : ∀ I ∈ l, el 0 (pT I) = el 0 (aT I) + 1) :
    ContBal l.length (fun i => (l[i]?).elim [] aT) (fun i => (l[i]?).elim [] pT)
      (fun i => (l[i]?).elim false live) e G where
  small := hsmall
  e1 := he
  hG := hG
  succ i hi := by
    rw [List.getElem?_eq_getElem hi]
    exact hsucc _ (List.getElem_mem hi)
  bal τ := by
    have e' := hbal τ
    rw [← Lean.Grind.Semiring.natCast_add] at e'
    have hb1 := List.countP_le_length (p := fun I => live I && decide (pT I = τ)) (l := l)
    have hb2 := List.countP_le_length (p := fun I => decide (aT I = τ)) (l := l)
    have hb3 : e * (if τ = G then 1 else 0) ≤ 1 := by split <;> omega
    have := natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega) e'
    have c1 : (List.range l.length).countP (fun i => decide ((l[i]?).elim [] aT = τ)) =
        l.countP (fun I => decide (aT I = τ)) := by
      rw [← countP_range_get l (fun I => decide (aT I = τ))]
      apply List.countP_congr
      intro i hi
      simp [List.getElem?_eq_getElem (List.mem_range.mp hi)]
    have c2 : (List.range l.length).countP (fun i => (l[i]?).elim false live && decide ((l[i]?).elim [] pT = τ)) =
        l.countP (fun I => live I && decide (pT I = τ)) := by
      rw [← countP_range_get l (fun I => live I && decide (pT I = τ))]
      apply List.countP_congr
      intro i hi
      simp [List.getElem?_eq_getElem (List.mem_range.mp hi)]
    rw [c1, c2]
    omega

/-- **The chain, as a list**: the segments in chain order, each once. -/
theorem chain_list {α : Type _} {l : List α} {aT pT : α → List F} {live : α → Bool} {e : Nat} {G : List F}
    (h : ContBal l.length (fun i => (l[i]?).elim [] aT) (fun i => (l[i]?).elim [] pT)
      (fun i => (l[i]?).elim false live) e G) :
    ∃ order : List α, (∀ x ∈ order, x ∈ l) ∧ (∀ x ∈ l, x ∈ order) ∧
      (∀ h0 : 0 < order.length, aT order[0] = G) ∧
      ∀ k (hk : k + 1 < order.length), live order[k] = true ∧ pT order[k] = aT order[k + 1] := by
  obtain ⟨ord, hlt, hsurj, h0, hnext⟩ := h.chain
  refine ⟨List.ofFn (n := l.length) fun k => l[ord k.1]'(hlt k.1 k.2), ?_, ?_, ?_, ?_⟩
  · intro x hx
    obtain ⟨k, rfl⟩ := List.mem_ofFn.mp hx
    exact List.getElem_mem _
  · intro x hx
    obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem hx
    obtain ⟨k, hk, hki⟩ := hsurj i hi
    exact List.mem_ofFn.mpr ⟨⟨k, hk⟩, by simp [hki]⟩
  · intro hpos
    rw [List.length_ofFn] at hpos
    have := h0 hpos
    simp only [List.getElem_ofFn]
    simpa [List.getElem?_eq_getElem (hlt 0 hpos)] using this
  · intro k hk
    rw [List.length_ofFn] at hk
    obtain ⟨a, b⟩ := hnext k hk
    simp only [List.getElem_ofFn]
    simp only [List.getElem?_eq_getElem (hlt k (by omega)), List.getElem?_eq_getElem (hlt (k + 1) hk),
      Option.elim_some] at a b
    exact ⟨a, b⟩

end Correspondence
