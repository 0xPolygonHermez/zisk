/-
  A memory word's value at a step.

  As `RegFile.lean` does for a register, a function of the step: `memVal init
  ws a s`, the value the latest write to address `a` before step `s` wrote,
  or `init a` if there is none, over a list of writes `ws` whose steps are
  distinct. One step on, the value changes only where a write sits at that
  step (`memVal_succ`): a write at step `s` to `a` sets `a` (`memVal_set`),
  and leaves every other address as it was (`memVal_other`); a step without
  writes keeps them all (`memVal_keep`).
-/
import Proofs.MemSort

namespace Correspondence.MemSort

open Classical

variable {V : Type}

/-- The latest write to `a` before step `s`. -/
def LatestW (ws : List (Emit V)) (a s : Nat) (w : Emit V) : Prop :=
  w ∈ ws ∧ w.addr = a ∧ w.step < s ∧ ∀ w' ∈ ws, w'.addr = a → w'.step < s → w'.step ≤ w.step

theorem exists_latestW (ws : List (Emit V)) (a s : Nat)
    (h : ∃ w ∈ ws, w.addr = a ∧ w.step < s) : ∃ w, LatestW ws a s w := by
  induction ws with
  | nil => obtain ⟨w, hw, -⟩ := h; exact nomatch hw
  | cons x ws ih =>
    by_cases hx : x.addr = a ∧ x.step < s
    · by_cases hr : ∃ w ∈ ws, w.addr = a ∧ w.step < s
      · obtain ⟨w, hw, hwa, hws, hmax⟩ := ih hr
        by_cases hle : x.step ≤ w.step
        · refine ⟨w, List.mem_cons_of_mem _ hw, hwa, hws, fun e he ha hs => ?_⟩
          rcases List.mem_cons.mp he with rfl | he
          · exact hle
          · exact hmax e he ha hs
        · refine ⟨x, List.mem_cons_self .., hx.1, hx.2, fun e he ha hs => ?_⟩
          rcases List.mem_cons.mp he with rfl | he
          · exact Nat.le_refl _
          · have := hmax e he ha hs; omega
      · refine ⟨x, List.mem_cons_self .., hx.1, hx.2, fun e he ha hs => ?_⟩
        rcases List.mem_cons.mp he with rfl | he
        · exact Nat.le_refl _
        · exact absurd ⟨e, he, ha, hs⟩ hr
    · obtain ⟨e, he, ha, hs⟩ := h
      rcases List.mem_cons.mp he with rfl | he
      · exact absurd ⟨ha, hs⟩ hx
      · obtain ⟨w, hw, hwa, hws, hmax⟩ := ih ⟨e, he, ha, hs⟩
        refine ⟨w, List.mem_cons_of_mem _ hw, hwa, hws, fun e' he' ha' hs' => ?_⟩
        rcases List.mem_cons.mp he' with rfl | he'
        · exact absurd ⟨ha', hs'⟩ hx
        · exact hmax e' he' ha' hs'

/-- Writes with the same step are the same write. -/
theorem eq_of_step {ws : List (Emit V)} (hnd : (ws.map (·.step)).Nodup) {w w' : Emit V}
    (hw : w ∈ ws) (hw' : w' ∈ ws) (h : w.step = w'.step) : w = w' := by
  induction ws with
  | nil => exact nomatch hw
  | cons x ws ih =>
    rw [List.map_cons, List.nodup_cons] at hnd
    rcases List.mem_cons.mp hw with h1 | h1 <;> rcases List.mem_cons.mp hw' with h2 | h2
    · rw [h1, h2]
    · subst h1
      exact absurd (List.mem_map.mpr ⟨w', h2, h.symm⟩ : w.step ∈ ws.map (·.step)) hnd.1
    · subst h2
      exact absurd (List.mem_map.mpr ⟨w, h1, h⟩ : w'.step ∈ ws.map (·.step)) hnd.1
    · exact ih hnd.2 h1 h2

theorem LatestW.unique {ws : List (Emit V)} (hnd : (ws.map (·.step)).Nodup) {a s : Nat}
    {w w' : Emit V} (h : LatestW ws a s w) (h' : LatestW ws a s w') : w = w' := by
  obtain ⟨hw, hwa, hws, hmax⟩ := h
  obtain ⟨hw', hwa', hws', hmax'⟩ := h'
  have := hmax w' hw' hwa' hws'
  have := hmax' w hw hwa hws
  exact eq_of_step hnd hw hw' (by omega)

/-- The value at `a` just before step `s`. -/
noncomputable def memVal (init : Nat → V) (ws : List (Emit V)) (a s : Nat) : V :=
  if h : ∃ w, LatestW ws a s w then (Classical.choose h).val else init a

theorem memVal_of_latest {init : Nat → V} {ws : List (Emit V)} (hnd : (ws.map (·.step)).Nodup)
    {a s : Nat} {w : Emit V} (hw : LatestW ws a s w) : memVal init ws a s = w.val := by
  have h : ∃ w, LatestW ws a s w := ⟨w, hw⟩
  unfold memVal
  rw [dif_pos h, LatestW.unique hnd (Classical.choose_spec h) hw]

theorem memVal_of_none {init : Nat → V} {ws : List (Emit V)} {a s : Nat}
    (h : ∀ w ∈ ws, w.addr = a → ¬ w.step < s) : memVal init ws a s = init a := by
  unfold memVal
  rw [dif_neg]
  rintro ⟨w, hw, ha, hs, -⟩
  exact h w hw ha hs

/-- **One step on.** -/
theorem memVal_succ {init : Nat → V} {ws : List (Emit V)} (hnd : (ws.map (·.step)).Nodup)
    (a s : Nat) :
    memVal init ws a (s + 1) =
      if h : ∃ w ∈ ws, w.addr = a ∧ w.step = s then (Classical.choose h).val
      else memVal init ws a s := by
  split
  · rename_i h
    obtain ⟨hw, ha, hs⟩ := Classical.choose_spec h
    apply memVal_of_latest hnd
    exact ⟨hw, ha, by omega, fun w' _ _ hs' => by omega⟩
  · rename_i h
    by_cases hx : ∃ w, LatestW ws a s w
    · obtain ⟨w, hw⟩ := hx
      rw [memVal_of_latest hnd hw]
      obtain ⟨hwm, hwa, hws, hmax⟩ := hw
      apply memVal_of_latest hnd
      refine ⟨hwm, hwa, by omega, fun w' hw' ha' hs' => ?_⟩
      by_cases heq : w'.step = s
      · exact absurd ⟨w', hw', ha', heq⟩ h
      · exact hmax w' hw' ha' (by omega)
    · have hnone : ∀ w ∈ ws, w.addr = a → ¬ w.step < s := fun w hw ha hs =>
        hx (exists_latestW ws a s ⟨w, hw, ha, hs⟩)
      rw [memVal_of_none hnone, memVal_of_none]
      intro w hw ha hs
      by_cases heq : w.step = s
      · exact h ⟨w, hw, ha, heq⟩
      · exact hnone w hw ha (by omega)

/-- A step without writes keeps every address. -/
theorem memVal_keep {init : Nat → V} {ws : List (Emit V)} (hnd : (ws.map (·.step)).Nodup)
    {s : Nat} (h : ∀ w ∈ ws, w.step ≠ s) (a : Nat) :
    memVal init ws a (s + 1) = memVal init ws a s := by
  rw [memVal_succ hnd, dif_neg]
  rintro ⟨w, hw, -, hs⟩
  exact h w hw hs

/-- A write at step `s` sets its address. -/
theorem memVal_set {init : Nat → V} {ws : List (Emit V)} (hnd : (ws.map (·.step)).Nodup)
    {w : Emit V} (hw : w ∈ ws) : memVal init ws w.addr (w.step + 1) = w.val := by
  apply memVal_of_latest hnd
  exact ⟨hw, rfl, by omega, fun w' _ _ _ => by omega⟩

/-- A write at step `s` leaves every other address. -/
theorem memVal_other {init : Nat → V} {ws : List (Emit V)} (hnd : (ws.map (·.step)).Nodup)
    {w : Emit V} (hw : w ∈ ws) {a : Nat} (ha : a ≠ w.addr) :
    memVal init ws a (w.step + 1) = memVal init ws a w.step := by
  rw [memVal_succ hnd, dif_neg]
  rintro ⟨w', hw', ha', hs'⟩
  have := eq_of_step hnd hw' hw hs'
  subst this
  exact ha ha'.symm

/-- Before every write, the initial value. -/
theorem memVal_init {init : Nat → V} {ws : List (Emit V)} {s : Nat} (h : ∀ w ∈ ws, s ≤ w.step)
    (a : Nat) : memVal init ws a s = init a :=
  memVal_of_none fun w hw _ hs => by have := h w hw; omega

end Correspondence.MemSort
