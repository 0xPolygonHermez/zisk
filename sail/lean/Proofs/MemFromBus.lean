/-
  `MemAnswers` from the memory bus.

  `MemWorld.lean` matches Main's loads and stores with the RAM's sorted log
  and InputData's rows across the memory bus; this file turns that into
  `MemAnswers` (`MemMain.lean`), the hypothesis the runs with memory take.
-/
import Proofs.MemWorld
import Proofs.MemInput

namespace Correspondence

open Pil.Zisk MemSort

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- A six-element tuple, by its elements. -/
theorem tuple6 {τ : List F} (h : τ.length = 6) : τ = [el 0 τ, el 1 τ, el 2 τ, el 3 τ, el 4 τ, el 5 τ] := by
  match τ, h with
  | [a, b, c, d, e, f], _ => rfl

/-- Main's three accesses of a lane, in its columns. -/
theorem access_cols {l : Nat} (hl : l < 4) (x : Main.Ctx F) (t : Main.Trace F) (j : Int) :
    ((PairTag.lane l 0).curArg (F := F)).reps x t j = (t j).a_src_mem l + (t j).a_src_reg l ∧
    el 0 (tupleOf ⟨(PairTag.lane l 0).curArg (F := F), x, t⟩ j) =
      (t j).a_src_mem l + 3 * (t j).a_src_reg l ∧
    el 1 (tupleOf ⟨(PairTag.lane l 0).curArg (F := F), x, t⟩ j) = (t j).a_offset_imm0 l + 0 ∧
    el 3 (tupleOf ⟨(PairTag.lane l 0).curArg (F := F), x, t⟩ j) = 8 ∧
    el 4 (tupleOf ⟨(PairTag.lane l 0).curArg (F := F), x, t⟩ j) = (t j).a l 0 + 0 ∧
    el 5 (tupleOf ⟨(PairTag.lane l 0).curArg (F := F), x, t⟩ j) = (t j).a l 1 + 0 ∧
    (tupleOf ⟨(PairTag.lane l 0).curArg (F := F), x, t⟩ j).length = 6 ∧
    ((PairTag.lane l 1).curArg (F := F)).reps x t j =
      (t j).b_src_mem l + (t j).b_src_ind l + (t j).b_src_reg l ∧
    el 0 (tupleOf ⟨(PairTag.lane l 1).curArg (F := F), x, t⟩ j) =
      (t j).b_src_mem l + (t j).b_src_ind l + 3 * (t j).b_src_reg l ∧
    el 1 (tupleOf ⟨(PairTag.lane l 1).curArg (F := F), x, t⟩ j) = (t j).addr1 l + 0 ∧
    el 3 (tupleOf ⟨(PairTag.lane l 1).curArg (F := F), x, t⟩ j) =
      (t j).b_src_ind l * ((t j).ind_width l - 8) + 8 ∧
    el 4 (tupleOf ⟨(PairTag.lane l 1).curArg (F := F), x, t⟩ j) = (t j).b l 0 + 0 ∧
    el 5 (tupleOf ⟨(PairTag.lane l 1).curArg (F := F), x, t⟩ j) = (t j).b l 1 + 0 ∧
    (tupleOf ⟨(PairTag.lane l 1).curArg (F := F), x, t⟩ j).length = 6 ∧
    el 1 (tupleOf ⟨(PairTag.lane l 2).curArg (F := F), x, t⟩ j) =
      (t j).store_offset l + (t j).store_ind l * (t j).a l 0 + 0 ∧
    el 3 (tupleOf ⟨(PairTag.lane l 2).curArg (F := F), x, t⟩ j) =
      (t j).store_ind l * ((t j).ind_width l - 8) + 8 ∧
    (tupleOf ⟨(PairTag.lane l 2).curArg (F := F), x, t⟩ j).length = 6 := by
  rcases l with _ | _ | _ | _ | l
  all_goals first
    | exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
    | omega

/-- **A load Main assumes has width 8**: its prover, a RAM lane or an
InputData row, sends 8. -/
theorem MemBusWorld.load_width {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {X st w v0 v1 : F}
    {s₀ : Sender F} (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = false)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = [1, X, st, w, v0, v1])
    (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) : w = 8 := by
  have hop : ([1, X, st, w, v0, v1] : List F).headD 0 = 1 ∨ ([1, X, st, w, v0, v1] : List F).headD 0 = 2 :=
    .inl rfl
  obtain ⟨s, hs, hp, h1, j', hj', ht', hr'⟩ := W.proved hop hs₀ hside hb hj ht hr
  rcases W.who hop hs hp h1 hj' ht' hr' with ⟨I, -, m, -, L, hL, h | ⟨-, h⟩⟩ | ⟨I, -, m, -, -, h⟩
  · rw [(ram_lane_tuple hL I.x I.t m).2] at h
    simp only [List.cons.injEq] at h
    exact h.2.2.2.1.symm
  · rw [(ram_dual_tuple hL I.x I.t m).2] at h
    simp only [List.cons.injEq] at h
    exact h.2.2.2.1.symm
  · simp only [tupleOf, InputData.bus26, List.map_cons, List.map_nil, List.cons.injEq] at h
    exact h.2.2.2.1.symm

/-! ### Main's stores -/

theorem nodup_filterMap {α β : Type _} {f : α → Option β} :
    ∀ {l : List α}, l.Nodup → (∀ a a' b, f a = some b → f a' = some b → a = a') → (l.filterMap f).Nodup
  | [], _, _ => List.nodup_nil
  | a :: l, hl, hf => by
    rw [List.nodup_cons] at hl
    have ih := nodup_filterMap hl.2 hf
    rw [List.filterMap_cons]
    cases ha : f a with
    | none => exact ih
    | some b =>
      refine List.nodup_cons.mpr ⟨fun hb => ?_, ih⟩
      obtain ⟨a', ha', hfa'⟩ := List.mem_filterMap.mp hb
      exact hl.1 (hf a a' b ha hfa' ▸ ha')

/-- A lane's global index: segment `k`, row `m`, lane `l`. -/
theorem memBase_index {k m l : Nat} (_hm : m < Main.numRows) (_hl : l < 4) :
    memBase k m l = 4 * (k * 16777216 + 4 * m + l) + 1 := by
  simp only [memBase]; omega

open Classical in
/-- The store of the lane with global index `n`, if it stores to memory: its
address, its step `4n + 3`, its value. -/
noncomputable def storeAt (M : MainSegs F) (n : Nat) : Option (Emit (F × F)) :=
  if StoresAt M (n / 16777216) ((n % 16777216) / 4) (n % 4) then
    some ⟨natOf (el 1 (tupleOf ⟨(PairTag.lane (n % 4) 2).curArg, (M.seg (n / 16777216)).1,
        (M.seg (n / 16777216)).2⟩ (((n % 16777216) / 4 : Nat) : Int))),
      4 * n + 3,
      (el 4 (tupleOf ⟨(PairTag.lane (n % 4) 2).curArg, (M.seg (n / 16777216)).1,
        (M.seg (n / 16777216)).2⟩ (((n % 16777216) / 4 : Nat) : Int)),
       el 5 (tupleOf ⟨(PairTag.lane (n % 4) 2).curArg, (M.seg (n / 16777216)).1,
        (M.seg (n / 16777216)).2⟩ (((n % 16777216) / 4 : Nat) : Int)))⟩
  else none

/-- **Main's stores**, in the order of their lanes. -/
noncomputable def mainStores (M : MainSegs F) : List (Emit (F × F)) :=
  (List.range (M.K * 16777216)).filterMap (storeAt M)

theorem storeAt_step {M : MainSegs F} {n : Nat} {w : Emit (F × F)} (h : storeAt M n = some w) :
    w.step = 4 * n + 3 ∧ StoresAt M (n / 16777216) ((n % 16777216) / 4) (n % 4) := by
  unfold storeAt at h
  split at h
  · rename_i hs; cases h; exact ⟨rfl, hs⟩
  · exact nomatch h

theorem mainStores_nodup (M : MainSegs F) : ((mainStores M).map (·.step)).Nodup := by
  unfold mainStores
  rw [List.map_filterMap]
  apply nodup_filterMap List.nodup_range
  intro a a' b ha ha'
  simp only [Option.map_eq_some_iff] at ha ha'
  obtain ⟨w, hw, rfl⟩ := ha
  obtain ⟨w', hw', e⟩ := ha'
  have := (storeAt_step hw).1
  have := (storeAt_step hw').1
  omega

/-- A store of Main's is at a lane that stores to memory, at the lane's store
step. -/
theorem mem_mainStores {M : MainSegs F} {w : Emit (F × F)} (hw : w ∈ mainStores M) :
    ∃ k < M.K, ∃ m < Main.numRows, ∃ l < 4, StoresAt M k m l ∧ w.step = memBase k m l + 2 ∧
      w = ⟨natOf (el 1 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int))),
        memBase k m l + 2,
        (el 4 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)),
         el 5 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)))⟩ := by
  unfold mainStores at hw
  obtain ⟨n, hn, hw⟩ := List.mem_filterMap.mp hw
  rw [List.mem_range] at hn
  have hN : Main.numRows = 4194304 := rfl
  have e := memBase_index (k := n / 16777216) (m := (n % 16777216) / 4) (l := n % 4)
    (by rw [hN]; omega) (by omega)
  obtain ⟨hst, hs⟩ := storeAt_step hw
  refine ⟨n / 16777216, Nat.div_lt_of_lt_mul (by omega), (n % 16777216) / 4, by rw [hN]; omega,
    n % 4, by omega, hs, by rw [hst, e]; omega, ?_⟩
  unfold storeAt at hw
  rw [if_pos hs] at hw
  cases hw
  congr 1
  rw [e]; omega

/-- **Every lane that stores to memory is among Main's stores.** -/
theorem mainStores_mem {M : MainSegs F} {k m l : Nat} (hk : k < M.K) (hm : m < Main.numRows)
    (hl : l < 4) (hs : StoresAt M k m l) :
    (⟨natOf (el 1 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int))),
      memBase k m l + 2,
      (el 4 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)),
       el 5 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)))⟩ : Emit (F × F)) ∈
      mainStores M := by
  have hN : Main.numRows = 4194304 := rfl
  unfold mainStores
  refine List.mem_filterMap.mpr ⟨k * 16777216 + 4 * m + l, List.mem_range.mpr ?_, ?_⟩
  · rw [hN] at hm
    have : k * 16777216 + 4 * m + l < (k + 1) * 16777216 := by omega
    have : (k + 1) * 16777216 ≤ M.K * 16777216 := Nat.mul_le_mul_right _ hk
    omega
  · have h1 : (k * 16777216 + 4 * m + l) / 16777216 = k := by rw [hN] at hm; omega
    have h2 : ((k * 16777216 + 4 * m + l) % 16777216) / 4 = m := by rw [hN] at hm; omega
    have h3 : (k * 16777216 + 4 * m + l) % 4 = l := by omega
    unfold storeAt
    rw [h1, h2, h3, if_pos hs, memBase_index hm hl]

end Correspondence
