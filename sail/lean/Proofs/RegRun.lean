/-
  The register file at a position.

  `MainSegs.reads` and `RegFile.lean` say what a register holds at a memory
  step. A lane at global step `G = k·2^24 + 4·j + l` (segment `k`, row `j`,
  lane `l`) owns the memory steps `b = 1 + 4·G` to `b + 3`: its `a`, `b` and
  store accesses, and a flush at the end of a window. This file reads the
  register chain at those steps:

  * `MainSegs.raw_from`: an access comes from a pair of a segment, at a row;
  * `MainSegs.lane_read`: a lane's `a` or `b` access to register `r` reads
    `r`'s value at its step;
  * `MainSegs.reads_keep`: the reads leave the value unchanged, so at `b + 1`,
    `b + 2` and `b + 4` it is as at `b`, `b + 1` and `b + 3`;
  * `MainSegs.store_set`, `MainSegs.store_none`: the lane's store sets the
    value at `b + 3`, or, if it does not store to `r`, leaves it.
-/
import Proofs.Registers
import Proofs.RegFile

namespace Correspondence

open Pil.Zisk MemCheck Classical

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- Register `x(n+1)`'s accesses, over every segment. -/
noncomputable def MainSegs.es (M : MainSegs F) (n : Nat) : List (Access (F × F)) :=
  (M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess

/-- The raw access a pair makes at a row. -/
noncomputable def Pair.rawAt (P : Pair F) (j : Int) : Raw F :=
  ⟨P.step j, el 2 (tupleOf P.pre.p j), (el 4 (tupleOf P.pre.p j), el 5 (tupleOf P.pre.p j)),
    if P.cur.p.arg.reps P.cur.p.x P.cur.p.t j = 1 ∧ el 0 (tupleOf P.cur.p j) = 3 then
      some (el 4 (tupleOf P.cur.p j), el 5 (tupleOf P.cur.p j)) else none⟩

theorem Pair.mem_raws_iff {P : Pair F} {r : F} {a : Raw F} :
    a ∈ P.raws r ↔ ∃ j ∈ P.pre.at, P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1 ∧
      el 1 (tupleOf P.pre.p j) = r ∧ a = P.rawAt j := by
  unfold Pair.raws
  rw [List.mem_filterMap]
  constructor
  · rintro ⟨j, hj, he⟩
    split at he
    · rename_i hc
      exact ⟨j, hj, hc.1, hc.2, (Option.some.inj he).symm⟩
    · exact nomatch he
  · rintro ⟨j, hj, h1, h2, rfl⟩
    exact ⟨j, hj, by rw [if_pos ⟨h1, h2⟩]; rfl⟩

/-- **Where an access comes from**: a pair of a segment, at a row its proving
side sends at, to the register. -/
theorem MainSegs.raw_from {M : MainSegs F} {r : F} {a : Raw F}
    (ha : a ∈ M.pairs.flatMap (·.raws r)) :
    ∃ k < M.K, ∃ g ∈ mainTags, ∃ j ∈ (g.pair k (M.seg k).1 (M.seg k).2).pre.at,
      (g.pair k (M.seg k).1 (M.seg k).2).pre.p.arg.reps (M.seg k).1 (M.seg k).2 j = 1 ∧
      el 1 (tupleOf (g.pair k (M.seg k).1 (M.seg k).2).pre.p j) = r ∧
      a = (g.pair k (M.seg k).1 (M.seg k).2).rawAt j := by
  obtain ⟨P, hP, ha⟩ := List.mem_flatMap.mp ha
  obtain ⟨k, hk, g, hg, rfl⟩ := M.mem_pairs hP
  obtain ⟨j, hj, h1, h2, rfl⟩ := Pair.mem_raws_iff.mp ha
  exact ⟨k, hk, g, hg, j, hj, h1, h2, rfl⟩

/-- A pair's access at a row it sends at, to the register, is in the chain. -/
theorem MainSegs.raw_mem {M : MainSegs F} {r : F} {k : Nat} (hk : k < M.K) {g : PairTag}
    (hg : g ∈ mainTags) {j : Int} (hj : j ∈ (g.pair k (M.seg k).1 (M.seg k).2).pre.at)
    (h1 : (g.pair k (M.seg k).1 (M.seg k).2).pre.p.arg.reps (M.seg k).1 (M.seg k).2 j = 1)
    (h2 : el 1 (tupleOf (g.pair k (M.seg k).1 (M.seg k).2).pre.p j) = r) :
    (g.pair k (M.seg k).1 (M.seg k).2).rawAt j ∈ M.pairs.flatMap (·.raws r) := by
  apply List.mem_flatMap.mpr
  refine ⟨g.pair k (M.seg k).1 (M.seg k).2, ?_, Pair.mem_raws_iff.mpr ⟨j, hj, h1, h2, rfl⟩⟩
  simp only [MainSegs.pairs, List.mem_flatMap, List.mem_range, mainPairs, List.mem_map]
  exact ⟨k, hk, g, hg, rfl⟩

/-- A pair's step at a row it sends at, by kind: a lane access at a row below
`2^22`, or a flush. -/
theorem step_shape {M : MainSegs F} {k : Nat} {g : PairTag} (hg : g ∈ mainTags) {j : Int}
    (hj : j ∈ (g.pair k (M.seg k).1 (M.seg k).2).pre.at) :
    (∃ l κ m : Nat, g = .lane l κ ∧ l < 4 ∧ κ < 3 ∧ m < 4194304 ∧ j = (m : Int) ∧
      (g.pair k (M.seg k).1 (M.seg k).2).step j = 1 + 4 * (k * 16777216 + 4 * m + l) + κ) ∨
    (∃ f ρ, g = .flush f ρ ∧ f < 4 ∧ ρ < 39 ∧
      (g.pair k (M.seg k).1 (M.seg k).2).step j = 4 * (k * 16777216) + 4 * ((f + 1) * 4194304)) := by
  have hwf := mainTags_wf g hg
  cases g with
  | lane l κ =>
    obtain ⟨hl, hκ⟩ := hwf
    have honce : ((PairTag.lane l κ).preArg (F := F)).once = false := (mainTags_once (F := F)).mem _ hg
    obtain ⟨m, hm, rfl⟩ := mem_at_rows
      (s := ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre) honce hj
    refine .inl ⟨l, κ, m, rfl, hl, hκ, by simpa [PairTag.pair, Main.numRows] using hm, rfl, ?_⟩
    simp [PairTag.pair, PairTag.step, laneStep]
  | flush f ρ =>
    obtain ⟨hf, hρ⟩ := hwf
    refine .inr ⟨f, ρ, rfl, hf, hρ, ?_⟩
    simp only [PairTag.pair, PairTag.step, flushStep]
    have : 1 ≤ (f + 1) * 4194304 := by omega
    omega

/-- **Reads keep the value.** At a memory step that is not a store's — every
step but those `≡ 3 (mod 4)` — every access to a register reads, writing back
what it proved, so the register's value one step on is unchanged. -/
theorem MainSegs.reads_keep {M : MainSegs F} {n : Nat}
    (hperm : (proved (M.es n)).Perm (assumed (0, 0) (M.es n)))
    (hlt : ∀ e ∈ M.es n, e.prev < e.step) (hnd : ((M.es n).map (·.step)).Nodup)
    {s : Nat} (hs : s % 4 ≠ 3) : regVal (0, 0) (M.es n) (s + 1) = regVal (0, 0) (M.es n) s := by
  apply regVal_read hperm hlt hnd
  intro e he hes
  obtain ⟨a, ha, rfl⟩ := List.mem_map.mp he
  obtain ⟨k, hk, g, hg, j, hj, -, -, rfl⟩ := M.raw_from ha
  have hread : g.isRead = true := by
    rcases step_shape hg hj with ⟨l, κ, m, rfl, -, hκ, -, -, hst⟩ | ⟨f, ρ, rfl, -⟩
    · simp only [Raw.toAccess, Pair.rawAt] at hes
      rw [hst] at hes
      have : κ ≠ 2 := by omega
      simp [PairTag.isRead, this]
    · rfl
  obtain ⟨h4, h5⟩ := (mainTags_read (F := F)).mem g hg hread (M.seg k).1 (M.seg k).2 j
  simp only [Raw.toAccess, Pair.rawAt, PairTag.pair] at h4 h5 ⊢
  split
  · left; rw [h4, h5]
  · right; rfl

theorem lane_at {M : MainSegs F} {k l κ m : Nat} (hm : m < Main.numRows)
    (honce : ((PairTag.lane l κ).preArg (F := F)).once = false) :
    (m : Int) ∈ ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre.at := by
  simp only [PairTag.pair, Sender.at, honce, Bool.false_eq_true, if_false, List.mem_map,
    List.mem_range]
  exact ⟨m, hm, rfl⟩

theorem lane_step_eq {M : MainSegs F} {k l κ m : Nat} :
    ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).step (m : Int) =
      1 + 4 * (k * 16777216 + 4 * m + l) + κ := by
  simp [PairTag.pair, PairTag.step, laneStep]

/-- **A lane's access reads the register's value at its step.** -/
theorem MainSegs.lane_read {M : MainSegs F} {n : Nat}
    (hperm : (proved (M.es n)).Perm (assumed (0, 0) (M.es n)))
    (hlt : ∀ e ∈ M.es n, e.prev < e.step) (hnd : ((M.es n).map (·.step)).Nodup)
    {k l κ m : Nat} (hk : k < M.K) (hg : PairTag.lane l κ ∈ mainTags) (hm : m < Main.numRows)
    (h1 : ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre.p.arg.reps
      (M.seg k).1 (M.seg k).2 (m : Int) = 1)
    (h2 : el 1 (tupleOf ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre.p (m : Int)) =
      ((n + 1 : Nat) : F)) :
    (el 4 (tupleOf ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre.p (m : Int)),
      el 5 (tupleOf ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre.p (m : Int))) =
      regVal (0, 0) (M.es n) (1 + 4 * (k * 16777216 + 4 * m + l) + κ) := by
  have honce : ((PairTag.lane l κ).preArg (F := F)).once = false := (mainTags_once (F := F)).mem _ hg
  have hraw := M.raw_mem hk hg (lane_at hm honce) h1 h2
  have he : ((((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).rawAt (m : Int)).toAccess)
      ∈ M.es n := List.mem_map.mpr ⟨_, hraw, rfl⟩
  have := pv_eq_regVal hperm hlt hnd he
  simp only [Raw.toAccess, Pair.rawAt] at this
  rw [this, lane_step_eq]

/-- **A lane's store sets the register.** -/
theorem MainSegs.store_set {M : MainSegs F} {n : Nat} (hnd : ((M.es n).map (·.step)).Nodup)
    {k l m : Nat} (hk : k < M.K) (hg : PairTag.lane l 2 ∈ mainTags) (hm : m < Main.numRows)
    (h1 : ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).pre.p.arg.reps
      (M.seg k).1 (M.seg k).2 (m : Int) = 1)
    (h2 : el 1 (tupleOf ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).pre.p (m : Int)) =
      ((n + 1 : Nat) : F))
    (h3 : ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).cur.p.arg.reps
      (M.seg k).1 (M.seg k).2 (m : Int) = 1)
    (h4 : el 0 (tupleOf ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).cur.p (m : Int)) = 3) :
    regVal (0, 0) (M.es n) (1 + 4 * (k * 16777216 + 4 * m + l) + 2 + 1) =
      (el 4 (tupleOf ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).cur.p (m : Int)),
        el 5 (tupleOf ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).cur.p (m : Int))) := by
  have honce : ((PairTag.lane l 2).preArg (F := F)).once = false := (mainTags_once (F := F)).mem _ hg
  have hraw := M.raw_mem hk hg (lane_at hm honce) h1 h2
  have he : ((((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).rawAt (m : Int)).toAccess)
      ∈ M.es n := List.mem_map.mpr ⟨_, hraw, rfl⟩
  apply regVal_store hnd he
  · simp only [Raw.toAccess, Pair.rawAt]; exact lane_step_eq
  · simp only [Raw.toAccess, Pair.rawAt]; rw [if_pos ⟨h3, h4⟩]

/-- **A lane that does not store to the register keeps it.** -/
theorem MainSegs.store_none {M : MainSegs F} (hK : M.K < 1048576) {n : Nat}
    (hnd : ((M.es n).map (·.step)).Nodup)
    {k l m : Nat} (hk : k < M.K) (hl : l < 4) (hm : m < Main.numRows)
    (hno : ¬ (((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).pre.p.arg.reps
      (M.seg k).1 (M.seg k).2 (m : Int) = 1 ∧
      el 1 (tupleOf ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).pre.p (m : Int)) =
        ((n + 1 : Nat) : F))) :
    regVal (0, 0) (M.es n) (1 + 4 * (k * 16777216 + 4 * m + l) + 2 + 1) =
      regVal (0, 0) (M.es n) (1 + 4 * (k * 16777216 + 4 * m + l) + 2) := by
  rw [regVal_succ hnd, dif_neg]
  rintro ⟨e, he, hes, -⟩
  obtain ⟨a, ha, rfl⟩ := List.mem_map.mp he
  obtain ⟨k', hk', g, hg, j, hj, h1, h2, rfl⟩ := M.raw_from ha
  simp only [Raw.toAccess, Pair.rawAt] at hes
  rcases step_shape hg hj with ⟨l', κ', m', rfl, hl', hκ', hm', rfl, hst⟩ | ⟨f, ρ, rfl, -, -, hst⟩
  · rw [hst] at hes
    simp only [Main.numRows] at hm
    have : k' = k ∧ m' = m ∧ l' = l ∧ κ' = 2 := by omega
    obtain ⟨rfl, rfl, rfl, rfl⟩ := this
    exact hno ⟨h1, h2⟩
  · rw [hst] at hes
    omega

/-- Reading a register file after a write. -/
theorem vec_get_set (xs : Vector (BitVec 64) 32) {i j : Nat} (_hi : i < 32) (hj : j < 32)
    (v : BitVec 64) : (xs.set! i v)[j]! = if i = j then v else xs[j]! := by
  have e : xs.set! i v = xs.setIfInBounds i v := by
    apply Vector.ext; intro k hk
    simp [Vector.set!, Vector.setIfInBounds, Array.set!]
  rw [e, getElem!_pos (xs.setIfInBounds i v) j hj, getElem!_pos xs j hj]
  split
  · subst_vars; simp [Vector.getElem_setIfInBounds_self hj]
  · rw [Vector.getElem_setIfInBounds_ne hj (by assumption)]

/-- A lane's register pairs, in its columns: `a`'s and `b`'s proving sides, the
store's proving and assuming sides. -/
theorem lane_cols {l : Nat} (hl : l < 4) (x : Main.Ctx F) (t : Main.Trace F) (j : Int) :
    ((PairTag.lane l 0).preArg (F := F)).reps x t j = (t j).a_src_reg l + 0 ∧
    el 1 (tupleOf ⟨(PairTag.lane l 0).preArg (F := F), x, t⟩ j) = (t j).a_offset_imm0 l + 0 ∧
    el 4 (tupleOf ⟨(PairTag.lane l 0).preArg (F := F), x, t⟩ j) = (t j).a l 0 + 0 ∧
    el 5 (tupleOf ⟨(PairTag.lane l 0).preArg (F := F), x, t⟩ j) = (t j).a l 1 + 0 ∧
    ((PairTag.lane l 1).preArg (F := F)).reps x t j = (t j).b_src_reg l + 0 ∧
    el 1 (tupleOf ⟨(PairTag.lane l 1).preArg (F := F), x, t⟩ j) = (t j).b_offset_imm0 l + 0 ∧
    el 4 (tupleOf ⟨(PairTag.lane l 1).preArg (F := F), x, t⟩ j) = (t j).b l 0 + 0 ∧
    el 5 (tupleOf ⟨(PairTag.lane l 1).preArg (F := F), x, t⟩ j) = (t j).b l 1 + 0 ∧
    ((PairTag.lane l 2).preArg (F := F)).reps x t j = (t j).store_reg l + 0 ∧
    el 1 (tupleOf ⟨(PairTag.lane l 2).preArg (F := F), x, t⟩ j) = (t j).store_offset l + 0 ∧
    ((PairTag.lane l 2).curArg (F := F)).reps x t j =
      (t j).store_mem l + (t j).store_ind l + (t j).store_reg l ∧
    el 0 (tupleOf ⟨(PairTag.lane l 2).curArg (F := F), x, t⟩ j) =
      2 * ((t j).store_mem l + (t j).store_ind l) + 3 * (t j).store_reg l ∧
    el 4 (tupleOf ⟨(PairTag.lane l 2).curArg (F := F), x, t⟩ j) =
      (t j).store_pc l * ((t j).pc l + (t j).jmp_offset2 l - (t j).c l 0) + (t j).c l 0 ∧
    el 5 (tupleOf ⟨(PairTag.lane l 2).curArg (F := F), x, t⟩ j) =
      (1 - (t j).store_pc l) * (t j).c l 1 := by
  rcases l with _ | _ | _ | _ | l
  all_goals first
    | exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
    | omega

end Correspondence
