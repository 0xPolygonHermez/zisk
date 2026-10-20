/-
  A register's value at a memory step.

  `MemCheck.reads_last_write` says every access proves the value of the latest
  access before it. For a step-by-step correspondence the useful form is a
  register's value as a function of the memory step: `regVal es s`, the value
  the latest access before step `s` wrote, or the initial value if there is
  none. Then:

  * an access proves `regVal es` at its own step (`pv_eq_regVal`);
  * one memory step on, the value is the access at that step's, if there is
    one, and unchanged otherwise (`regVal_succ`) — so a read, which writes back
    what it proved, changes nothing, and a store sets the value it stores.
-/
import Proofs.MemCheck

namespace Correspondence.MemCheck

open Classical

variable {V : Type}

/-- The latest access before step `s` that wrote a value, if any. -/
def Latest (es : List (Access V)) (s : Nat) (w : Access V) : Prop :=
  w ∈ es ∧ w.cur.isSome ∧ w.step < s ∧ ∀ e ∈ es, e.cur.isSome → e.step < s → e.step ≤ w.step

theorem exists_latest (es : List (Access V)) (s : Nat)
    (h : ∃ e ∈ es, e.cur.isSome ∧ e.step < s) : ∃ w, Latest es s w := by
  induction es with
  | nil => obtain ⟨e, he, -⟩ := h; exact nomatch he
  | cons a es ih =>
    by_cases ha : a.cur.isSome ∧ a.step < s
    · by_cases hr : ∃ e ∈ es, e.cur.isSome ∧ e.step < s
      · obtain ⟨w, hw, hwc, hws, hmax⟩ := ih hr
        by_cases hle : a.step ≤ w.step
        · refine ⟨w, List.mem_cons_of_mem _ hw, hwc, hws, fun e he hc hs => ?_⟩
          rcases List.mem_cons.mp he with rfl | he
          · exact hle
          · exact hmax e he hc hs
        · refine ⟨a, List.mem_cons_self .., ha.1, ha.2, fun e he hc hs => ?_⟩
          rcases List.mem_cons.mp he with rfl | he
          · exact Nat.le_refl _
          · have := hmax e he hc hs; omega
      · refine ⟨a, List.mem_cons_self .., ha.1, ha.2, fun e he hc hs => ?_⟩
        rcases List.mem_cons.mp he with rfl | he
        · exact Nat.le_refl _
        · exact absurd ⟨e, he, hc, hs⟩ hr
    · obtain ⟨e, he, hc, hs⟩ := h
      rcases List.mem_cons.mp he with rfl | he
      · exact absurd ⟨hc, hs⟩ ha
      · obtain ⟨w, hw, hwc, hws, hmax⟩ := ih ⟨e, he, hc, hs⟩
        refine ⟨w, List.mem_cons_of_mem _ hw, hwc, hws, fun e' he' hc' hs' => ?_⟩
        rcases List.mem_cons.mp he' with rfl | he'
        · exact absurd ⟨hc', hs'⟩ ha
        · exact hmax e' he' hc' hs'

/-- With distinct steps, the latest access is unique. -/
theorem Latest.unique {es : List (Access V)} (hnd : (es.map (·.step)).Nodup) {s : Nat}
    {w w' : Access V} (h : Latest es s w) (h' : Latest es s w') : w = w' := by
  obtain ⟨hw, hwc, hws, hmax⟩ := h
  obtain ⟨hw', hwc', hws', hmax'⟩ := h'
  have := hmax w' hw' hwc' hws'
  have := hmax' w hw hwc hws
  exact eq_of_map_eq hnd hw hw' (by omega)

/-- The value the latest access before step `s` wrote, or `v₀`. -/
noncomputable def regVal (v₀ : V) (es : List (Access V)) (s : Nat) : V :=
  if h : ∃ w, Latest es s w then ((Classical.choose h).cur.getD v₀) else v₀

theorem regVal_of_latest {v₀ : V} {es : List (Access V)} (hnd : (es.map (·.step)).Nodup)
    {s : Nat} {w : Access V} (hw : Latest es s w) : regVal v₀ es s = w.cur.getD v₀ := by
  have h : ∃ w, Latest es s w := ⟨w, hw⟩
  unfold regVal
  rw [dif_pos h, Latest.unique hnd (Classical.choose_spec h) hw]

theorem regVal_of_none {v₀ : V} {es : List (Access V)} {s : Nat}
    (h : ∀ e ∈ es, e.cur.isSome → ¬ e.step < s) : regVal v₀ es s = v₀ := by
  unfold regVal
  rw [dif_neg]
  rintro ⟨w, hw, hwc, hws, -⟩
  exact h w hw hwc hws

/-- **An access proves the register's value at its step.** -/
theorem pv_eq_regVal {v₀ : V} {es : List (Access V)} (hperm : (proved es).Perm (assumed v₀ es))
    (hlt : ∀ e ∈ es, e.prev < e.step) (hnd : (es.map (·.step)).Nodup) {e : Access V}
    (he : e ∈ es) : e.pv = regVal v₀ es e.step := by
  obtain ⟨hcur, -⟩ := tight hperm hlt hnd he
  rcases reads_last_write hperm hlt hnd he with ⟨hv, hnone⟩ | ⟨w, hw, hwc, hws, hmax⟩
  · rw [hv, regVal_of_none (fun e' he' _ hs => hnone e' he' hs)]
  · have hl : Latest es e.step w := ⟨hw, by rw [hwc]; rfl, hws, fun e' he' _ hs => hmax e' he' hs⟩
    rw [regVal_of_latest hnd hl, hwc]
    rfl

/-- **One memory step on.** The value just after step `s` is the one the access
at `s` wrote, if there is such an access; otherwise it is unchanged. -/
theorem regVal_succ {v₀ : V} {es : List (Access V)} (hnd : (es.map (·.step)).Nodup) (s : Nat) :
    regVal v₀ es (s + 1) =
      if h : ∃ e ∈ es, e.step = s ∧ e.cur.isSome then
        (Classical.choose h).cur.getD v₀ else regVal v₀ es s := by
  split
  · rename_i h
    obtain ⟨he, hs, hc⟩ := Classical.choose_spec h
    apply regVal_of_latest hnd
    refine ⟨he, hc, by omega, fun e' he' hc' hs' => ?_⟩
    omega
  · rename_i h
    by_cases hx : ∃ w, Latest es s w
    · obtain ⟨w, hw⟩ := hx
      rw [regVal_of_latest hnd hw]
      obtain ⟨hwm, hwc, hws, hmax⟩ := hw
      apply regVal_of_latest hnd
      refine ⟨hwm, hwc, by omega, fun e' he' hc' hs' => ?_⟩
      by_cases heq : e'.step = s
      · exact absurd ⟨e', he', heq, hc'⟩ h
      · exact hmax e' he' hc' (by omega)
    · have hnone : ∀ e ∈ es, e.cur.isSome → ¬ e.step < s := by
        intro e he hc hs
        exact hx (exists_latest es s ⟨e, he, hc, hs⟩)
      rw [regVal_of_none hnone, regVal_of_none]
      intro e he hc hs
      by_cases heq : e.step = s
      · exact h ⟨e, he, heq, hc⟩
      · exact hnone e he hc (by omega)

/-- An access at step `s` that writes back what it proved — a read — leaves the
value unchanged. -/
theorem regVal_read {v₀ : V} {es : List (Access V)} (hperm : (proved es).Perm (assumed v₀ es))
    (hlt : ∀ e ∈ es, e.prev < e.step) (hnd : (es.map (·.step)).Nodup) {s : Nat}
    (hread : ∀ e ∈ es, e.step = s → e.cur = some e.pv ∨ e.cur = none) :
    regVal v₀ es (s + 1) = regVal v₀ es s := by
  rw [regVal_succ hnd s]
  split
  · rename_i h
    obtain ⟨he, hs, hc⟩ := Classical.choose_spec h
    rcases hread _ he hs with hr | hr
    · rw [hr, Option.getD_some, pv_eq_regVal hperm hlt hnd he, hs]
    · rw [hr] at hc; exact absurd hc (by simp)
  · rfl

/-- A store at step `s`, writing `v`, sets the value. -/
theorem regVal_store {v₀ : V} {es : List (Access V)} (hnd : (es.map (·.step)).Nodup) {s : Nat}
    {e : Access V} (he : e ∈ es) (hs : e.step = s) {v : V} (hv : e.cur = some v) :
    regVal v₀ es (s + 1) = v := by
  rw [regVal_succ hnd s, dif_pos ⟨e, he, hs, by rw [hv]; rfl⟩]
  have h : ∃ e ∈ es, e.step = s ∧ e.cur.isSome := ⟨e, he, hs, by rw [hv]; rfl⟩
  obtain ⟨he', hs', -⟩ := Classical.choose_spec h
  have := eq_of_map_eq hnd he' he (by rw [hs, hs'])
  rw [this, hv]
  rfl

end Correspondence.MemCheck
