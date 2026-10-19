/-
  Offline memory checking: why a register read returns the last write.

  Main keeps the registers itself, as a linked list of accesses over the
  memory bus (10), a permutation argument (`main.pil`, "Registers on Main").
  Every access to a register at memory step `s`

  * *proves* the access before it: `(3, reg, prev, 8, pv)`, with `prev` its
    own column (`reg_pre_load`, `reg_pre_store`), and `prev < s` range-checked
    — strictly, `s - prev - 1 ≥ 0`;
  * *assumes* itself: `(3, reg, s, 8, v)` (`mem_op`), unless it is the final
    flush of the last segment, which closes the chain;

  and a global update assumes `(3, reg, 0, 8, 0)`, the initial zero. A read
  proves and assumes one value, `pv = v`; a store proves the old value and
  assumes the new one.

  A sound permutation argument makes the proved and the assumed tuples equal
  as multisets. This file proves what that buys, for one register, in the
  abstract: if the proved tuples are a permutation of the assumed ones, every
  access's `prev < step`, and the accesses' steps are distinct, then every
  access's `(prev, pv)` is the latest write before it — the last access with a
  current tuple, or the initial value if there is none (`memcheck`). That is
  the memory-checking argument; the AIR side states its hypotheses over the
  generated bus arguments.

  The proof counts. For an access at step `s`: the accesses whose `prev` is
  below `s` are at least those at or before `s` (`prev < step`), which are one
  more than those before `s` (the steps are distinct), which are at least the
  assumed tuples below `s` (the initial one, and the current ones before `s`).
  The permutation makes the first and last counts equal, so every inequality
  is tight: every access before `s` is current, and no access after `s` points
  below it. Then an access pointing past the latest write would leave that
  write's tuple to be proved by some later access, between it and `s` —
  impossible, by descent.
-/

namespace Correspondence.MemCheck

open List

/-! ### Counting -/

/-- When `p` implies `q`, `q` counts the `p`s and the rest. -/
theorem countP_split {α : Type} {p q : α → Bool} :
    ∀ {l : List α}, (∀ x ∈ l, p x → q x) →
      countP q l = countP p l + countP (fun x => q x && !p x) l
  | [], _ => by simp
  | a :: l, h => by
    have ih := countP_split (l := l) (fun y hy => h y (mem_cons_of_mem _ hy))
    have ha := h a (mem_cons_self ..)
    rw [countP_cons, countP_cons, countP_cons, ih]
    cases hp : p a <;> cases hq : q a <;> simp only [hp, hq] at ha ⊢ <;> simp at ha ⊢ <;> omega

/-- If `p` implies `q` on a list and they count the same, `q` implies `p`. -/
theorem countP_eq_imp {α : Type} {p q : α → Bool} {l : List α} (h : ∀ x ∈ l, p x → q x)
    (heq : countP p l = countP q l) : ∀ x ∈ l, q x → p x := by
  intro x hx hq
  have h0 : countP (fun x => q x && !p x) l = 0 := by have := countP_split h; omega
  rw [countP_eq_zero] at h0
  have := h0 x hx
  cases hp : p x
  · simp [hp, hq] at this
  · rfl

/-! ### One register's accesses -/

/-- One access to the register: its memory step, the previous access's step
and value that it proves, and the value it assumes at its own step — none for
the final access, which closes the chain. -/
structure Access (V : Type) where
  step : Nat
  prev : Nat
  pv : V
  cur : Option V

variable {V : Type}

/-- The proved tuples: each access's previous one. -/
def proved (es : List (Access V)) : List (Nat × V) := es.map fun e => (e.prev, e.pv)

/-- The assumed tuples: the initial value at step 0, and each access's own. -/
def assumed (v₀ : V) (es : List (Access V)) : List (Nat × V) :=
  (0, v₀) :: es.filterMap fun e => e.cur.map fun v => (e.step, v)

theorem mem_assumed {v₀ : V} {es : List (Access V)} {t : Nat} {v : V} :
    (t, v) ∈ assumed v₀ es ↔ (t = 0 ∧ v = v₀) ∨ ∃ e ∈ es, e.cur = some v ∧ e.step = t := by
  simp only [assumed, mem_cons, Prod.mk.injEq, mem_filterMap, Option.map_eq_some_iff]
  constructor
  · rintro (h | ⟨e, he, w, hw, h1, h2⟩)
    · exact .inl h
    · subst h1 h2; exact .inr ⟨e, he, hw, rfl⟩
  · rintro (h | ⟨e, he, hw, h1⟩)
    · exact .inl h
    · exact .inr ⟨e, he, v, hw, h1, rfl⟩

section Counts

variable {v₀ : V} {es : List (Access V)}

theorem count_proved (s : Nat) :
    countP (fun x : Nat × V => decide (x.1 < s)) (proved es) =
      countP (fun e : Access V => decide (e.prev < s)) es := by
  simp only [proved, countP_map]; rfl

theorem count_assumed (s : Nat) (hs : 0 < s) :
    countP (fun x : Nat × V => decide (x.1 < s)) (assumed v₀ es) =
      countP (fun e : Access V => e.cur.isSome && decide (e.step < s)) es + 1 := by
  simp only [assumed, countP_cons, hs, decide_true, if_true, countP_filterMap]
  congr 1
  apply countP_congr
  intro e _
  cases e.cur <;> simp

/-- Steps are distinct: at most one access at a step. -/
theorem count_step_eq (hnd : (es.map (·.step)).Nodup) {e : Access V} (he : e ∈ es) :
    countP (fun e' : Access V => decide (e'.step = e.step)) es = 1 := by
  have h1 := hnd.count (a := e.step)
  rw [if_pos (mem_map.mpr ⟨e, he, rfl⟩)] at h1
  rw [count, countP_map] at h1
  rw [← h1]
  apply countP_congr
  intro x _
  simp [beq_iff_eq]

end Counts

/-! ### The argument -/

variable {v₀ : V} {es : List (Access V)}

/-- Tight counts at an access's step `s`: every access before `s` is current,
and every access whose `prev` is below `s` is at or before `s`. -/
theorem tight (hperm : (proved es).Perm (assumed v₀ es)) (hlt : ∀ e ∈ es, e.prev < e.step)
    (hnd : (es.map (·.step)).Nodup) {e : Access V} (he : e ∈ es) :
    (∀ e' ∈ es, e'.step < e.step → e'.cur.isSome) ∧
      (∀ e' ∈ es, e'.prev < e.step → e'.step ≤ e.step) := by
  have hs : 0 < e.step := Nat.lt_of_le_of_lt (Nat.zero_le _) (hlt e he)
  have hc := hperm.countP_eq (fun x : Nat × V => decide (x.1 < e.step))
  rw [count_proved, count_assumed e.step hs] at hc
  -- prev < s ⊇ step ≤ s = (step < s) + 1 ≥ (current ∧ step < s) + 1 = prev < s
  have h1 : countP (fun e' : Access V => decide (e'.step ≤ e.step)) es ≤
      countP (fun e' : Access V => decide (e'.prev < e.step)) es :=
    countP_mono_left (fun x hx h => by
      have := hlt x hx; simp only [decide_eq_true_eq] at h ⊢; omega)
  have h2 : countP (fun e' : Access V => decide (e'.step ≤ e.step)) es =
      countP (fun e' : Access V => decide (e'.step < e.step)) es + 1 := by
    rw [countP_split (p := fun e' : Access V => decide (e'.step < e.step))
      (q := fun e' : Access V => decide (e'.step ≤ e.step))
      (fun x _ h => by simp only [decide_eq_true_eq] at h ⊢; omega), ← count_step_eq hnd he]
    congr 1
    apply countP_congr
    intro x _
    simp only [Bool.and_eq_true, decide_eq_true_eq, Bool.not_eq_true', decide_eq_false_iff_not]
    omega
  have h3 : countP (fun e' : Access V => e'.cur.isSome && decide (e'.step < e.step)) es ≤
      countP (fun e' : Access V => decide (e'.step < e.step)) es :=
    countP_mono_left (fun x _ h => by simp only [Bool.and_eq_true] at h; exact h.2)
  refine ⟨fun e' he' hlt' => ?_, fun e' he' hp => ?_⟩
  · have := countP_eq_imp (p := fun e' : Access V => e'.cur.isSome && decide (e'.step < e.step))
      (q := fun e' : Access V => decide (e'.step < e.step))
      (fun x _ h => by simp only [Bool.and_eq_true] at h; exact h.2) (by omega) e' he'
      (by simpa using hlt')
    simp only [Bool.and_eq_true] at this
    exact this.1
  · have := countP_eq_imp (p := fun e' : Access V => decide (e'.step ≤ e.step))
      (q := fun e' : Access V => decide (e'.prev < e.step))
      (fun x hx h => by have := hlt x hx; simp only [decide_eq_true_eq] at h ⊢; omega)
      (by omega) e' he' (by simpa using hp)
    simpa using this

/-- Two members of a list with the same image under an injective-on-it map are
equal. -/
theorem eq_of_map_eq {α β : Type} {f : α → β} :
    ∀ {l : List α}, (l.map f).Nodup → ∀ {a b : α}, a ∈ l → b ∈ l → f a = f b → a = b
  | [], _, _, _, ha, _, _ => nomatch ha
  | x :: l, hnd, a, b, ha, hb, h => by
    rw [map_cons, nodup_cons] at hnd
    obtain ⟨hx, hnd⟩ := hnd
    rcases mem_cons.mp ha with ha' | ha' <;> rcases mem_cons.mp hb with hb' | hb'
    · rw [ha', hb']
    · subst ha'; exact absurd (by rw [h]; exact mem_map_of_mem hb') hx
    · subst hb'; exact absurd (by rw [← h]; exact mem_map_of_mem ha') hx
    · exact eq_of_map_eq hnd ha' hb' h

/-- **Memory checking.** Every access proves an assumed tuple, and no write
lies strictly between the step it points to and its own: it points to the
latest write before it. -/
theorem memcheck (hperm : (proved es).Perm (assumed v₀ es)) (hlt : ∀ e ∈ es, e.prev < e.step)
    (hnd : (es.map (·.step)).Nodup) {e : Access V} (he : e ∈ es) :
    (e.prev, e.pv) ∈ assumed v₀ es ∧
      ∀ e' ∈ es, e'.cur.isSome → e'.step < e.step → e'.step ≤ e.prev := by
  have hmem : (e.prev, e.pv) ∈ assumed v₀ es := hperm.mem_iff.mp (mem_map.mpr ⟨e, he, rfl⟩)
  refine ⟨hmem, ?_⟩
  obtain ⟨hcur, hback⟩ := tight hperm hlt hnd he
  -- A write between `e.prev` and `e.step` leads to a later one, without end.
  suffices H : ∀ n, ∀ e' ∈ es, e'.cur.isSome → e.step - e'.step = n → e'.step < e.step →
      e.prev < e'.step → False by
    intro e' he' hc hl
    apply Classical.byContradiction
    intro hn
    exact H _ e' he' hc rfl hl (by omega)
  intro n
  induction n using Nat.strongRecOn with
  | _ n ih =>
  intro e' he' hc hgap hl hp
  obtain ⟨v, hv⟩ := Option.isSome_iff_exists.mp hc
  have hA : (e'.step, v) ∈ assumed v₀ es := mem_assumed.mpr (.inr ⟨e', he', hv, rfl⟩)
  -- Its tuple is proved by some access `e''`, which points to it.
  obtain ⟨e'', he'', heq⟩ := mem_map.mp (hperm.mem_iff.mpr hA)
  simp only [Prod.mk.injEq] at heq
  obtain ⟨hprev'', -⟩ := heq
  have hb := hback e'' he'' (by omega)
  have hgt := hlt e'' he''
  have hne : e''.step ≠ e.step := by
    intro h
    have := eq_of_map_eq hnd he'' he h
    subst this
    omega
  exact ih (e.step - e''.step) (by omega) e'' he'' (hcur e'' he'' (by omega)) rfl (by omega)
    (by omega)

/-- **A read returns the last write.** Every access proves the value of the
latest access before it — every one of which assumed a value — or, if it is
the first, the initial value. -/
theorem reads_last_write (hperm : (proved es).Perm (assumed v₀ es))
    (hlt : ∀ e ∈ es, e.prev < e.step) (hnd : (es.map (·.step)).Nodup) {e : Access V}
    (he : e ∈ es) :
    (e.pv = v₀ ∧ ∀ e' ∈ es, ¬ e'.step < e.step) ∨
      ∃ w ∈ es, w.cur = some e.pv ∧ w.step < e.step ∧
        ∀ e' ∈ es, e'.step < e.step → e'.step ≤ w.step := by
  obtain ⟨hmem, hlast⟩ := memcheck hperm hlt hnd he
  obtain ⟨hcur, -⟩ := tight hperm hlt hnd he
  rcases mem_assumed.mp hmem with ⟨h0, hv⟩ | ⟨w, hw, hwv, hws⟩
  · refine .inl ⟨hv, fun e' he' hl => ?_⟩
    have := hlast e' he' (hcur e' he' hl) hl
    have := hlt e' he'
    omega
  · refine .inr ⟨w, hw, hwv, by have := hlt e he; omega, fun e' he' hl => ?_⟩
    have := hlast e' he' (hcur e' he' hl) hl
    omega

end Correspondence.MemCheck
