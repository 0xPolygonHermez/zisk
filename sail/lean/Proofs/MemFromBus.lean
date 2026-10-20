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
open Out.Functions

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

/-! ### The RAM's writes are Main's stores -/

open Classical in
/-- Every emission of the RAM's log has an address and a step below `2^48`. -/
theorem MemBusWorld.log_bounds {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {e : Emit (F × F)}
    (he : e ∈ MemSort.reads (ramLog (W.segs hram)) ∨ e ∈ MemSort.writes (ramLog (W.segs hram))) :
    e.addr < 2 ^ 48 ∧ e.step < 2 ^ 48 := by
  obtain ⟨lane, hlane, hel⟩ : ∃ lane ∈ ramLog (W.segs hram), e ∈ lane.reads ∨ e ∈ lane.writes := by
    rcases he with h | h
    · obtain ⟨lane, hl, h'⟩ := List.mem_flatMap.mp h; exact ⟨lane, hl, .inl h'⟩
    · obtain ⟨lane, hl, h'⟩ := List.mem_flatMap.mp h; exact ⟨lane, hl, .inr h'⟩
  obtain ⟨seg, hseg, hls⟩ := List.mem_flatMap.mp hlane
  obtain ⟨⟨I, hI⟩, -, rfl⟩ := List.mem_map.mp hseg
  obtain ⟨n, hn, rfl⟩ := List.mem_map.mp hls
  rw [List.mem_range] at hn
  have hm : n / 4 < Mem.numRows := by
    have : Mem.numRows = 4194304 := rfl
    rw [this] at hn ⊢; omega
  have hL : n % 4 < 4 := Nat.mod_lt _ (by omega)
  let N := (W.seg hI (hram I hI)).nat
  have ha := (N.addr ((n / 4 : Nat) : Int) (n % 4) (inRows_nat hm) hL).1
  have hs := (N.step ((n / 4 : Nat) : Int) (n % 4) (inRows_nat hm) hL).1
  rcases hel with h | h
  · have h' : e ∈ (N.lane ((n / 4 : Nat) : Int) (n % 4)).reads := h
    simp only [RamNat.lane, Lane.reads, List.mem_append] at h'
    rcases h' with h' | h'
    · split at h'
      · exact nomatch h'
      · rw [List.mem_singleton.mp h']; exact ⟨ha, hs⟩
    · split at h'
      · rename_i hd
        split at hd
        · rename_i h1
          cases hd
          rw [List.mem_singleton.mp h']
          exact ⟨ha, (N.dual ((n / 4 : Nat) : Int) (n % 4) (inRows_nat hm) hL (by
            show ((W.seg hI (hram I hI)).t _).sel_dual (n % 4) ≠ 0
            rw [h1]; exact one_ne_zero')).1⟩
        · exact nomatch hd
      · exact nomatch h'
  · have h' : e ∈ (N.lane ((n / 4 : Nat) : Int) (n % 4)).writes := h
    simp only [RamNat.lane, Lane.writes] at h'
    split at h'
    · rw [List.mem_singleton.mp h']; exact ⟨ha, hs⟩
    · exact nomatch h'

/-- **A store Main assumes has width 8**: its prover, a RAM lane, sends 8. -/
theorem MemBusWorld.store_width {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {X st w v0 v1 : F}
    {s₀ : Sender F} (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = false)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = [2, X, st, w, v0, v1])
    (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) : w = 8 := by
  have hop : ([2, X, st, w, v0, v1] : List F).headD 0 = 1 ∨ ([2, X, st, w, v0, v1] : List F).headD 0 = 2 :=
    .inr rfl
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

theorem natCast_ofNat_lt {n : Nat} (hn : n < goldilocks) : natOf ((n : Nat) : F) = n := natOf_cast hn

/-- A write at a byte address: the log's word address, times 8. -/
def toByte (w : Emit (F × F)) : Emit (F × F) := ⟨8 * w.addr, w.step, w.val⟩

theorem laneStep_store {k l m : Nat} : laneStep k l 2 (m : Int) = memBase k m l + 2 := by
  simp only [laneStep, memBase, Int.toNat_natCast]

/-- A store's step element, in Main's columns: its lane's store step. -/
theorem MemBusWorld.store_step {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {k m l : Nat} (hk : k < M.K) (hm : m < Main.numRows) (hl : l < 4) :
    el 2 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)) =
      ((memBase k m l + 2 : Nat) : F) := by
  have hcur := (mainTags_cur (F := F)).mem (PairTag.lane l 2) (lane_mem hl (by omega))
  rw [hcur k (M.seg k).1 (M.seg k).2 (W.ok.seg k hk) m hm, laneStep_store]

theorem memBase_lt {M : MainSegs F} {k m l : Nat} (hk : k < M.K) (hK : M.K < 1048576)
    (hm : m < Main.numRows) (hl : l < 4) : memBase k m l + 2 < 2 ^ 48 := by
  simp only [memBase, Main.numRows] at hm ⊢; omega

/-- **Every write of the RAM's log is one of Main's stores**, at its byte
address. -/
theorem MemBusWorld.write_in_stores {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {w : Emit (F × F)}
    (hw : w ∈ MemSort.writes (ramLog (W.segs hram))) : toByte w ∈ mainStores M := by
  obtain ⟨k, hk, m, hm, l, hl, hst, ht⟩ := W.write_is_store hram hw
  have hb := W.log_bounds hram (.inr hw)
  have hmem := mainStores_mem hk hm hl hst
  have e1 : el 1 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)) =
      ((8 * w.addr : Nat) : F) := by rw [ht]; rfl
  have e2 := W.store_step hk hm hl
  rw [ht] at e2
  have e2' : ((w.step : Nat) : F) = ((memBase k m l + 2 : Nat) : F) := e2
  have hstep := natCast_inj49 (F := F) (by omega) (by have := memBase_lt hk W.ok.small hm hl; omega) e2'
  rw [e1, natCast_ofNat_lt (F := F) (by unfold goldilocks; omega), ← hstep, ht] at hmem
  exact hmem

/-- A lane's store access is among the memory bus's senders, assuming, a
sender of bits. -/
theorem MemBusWorld.store_sender {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {k l : Nat} (hk : k < M.K) (hl : l < 4) :
    (⟨⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F) ∈ busSenders M S ∧
      (⟨⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F).proving = false ∧
      (∀ j ∈ (⟨⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F).at,
        ((PairTag.lane l 2).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 0 ∨
          ((PairTag.lane l 2).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 1) ∧
      (∀ j, ((PairTag.lane l 2).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 0 ∨
          ((PairTag.lane l 2).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 1) ∧
      (∀ m : Nat, m < Main.numRows →
        (m : Int) ∈ (⟨⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F).at) := by
  have hg : PairTag.lane l 2 ∈ mainTags := lane_mem hl (by omega)
  have hP : (PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2 ∈ M.pairs := by
    simp only [MainSegs.pairs, List.mem_flatMap, List.mem_range, mainPairs, List.mem_map]
    exact ⟨k, hk, PairTag.lane l 2, hg, rfl⟩
  have hgood := M.pairs_good W.ok _ hP
  have honce := (mainTags_once (F := F)).mem _ hg
  refine ⟨?_, hgood.side_cur, fun j hj => hgood.bits_cur j (by rw [← hgood.same]; exact hj), ?_,
    fun m hm => by
      have := lane_at (M := M) (k := k) hm honce
      rw [← hgood.same] at this
      exact this⟩
  · apply List.mem_append_left
    rw [← M.pairSenders_pairs]
    exact List.mem_flatMap.mpr ⟨_, hP, by simp [PairTag.pair]⟩
  · intro j
    obtain ⟨-, -, -, -, -, -, -, -, c9, -⟩ := lane_cols (F := F) hl (M.seg k).1 (M.seg k).2 j
    rw [c9, Lean.Grind.Semiring.add_zero]
    have := (W.ok.seg k hk).rows j
    rcases l with _ | _ | _ | _ | l
    · exact bit_or this.bool_store_reg_0
    · exact bit_or this.bool_store_reg_1
    · exact bit_or this.bool_store_reg_2
    · exact bit_or this.bool_store_reg_3
    · omega

/-- A count of bits `x + y`, with `x` 1 and `y` a bit: `y` is 0. -/
theorem reg_zero {x y : F} (hx : x = 1) (hb : x + y = 0 ∨ x + y = 1) (hy : y = 0 ∨ y = 1) : y = 0 := by
  rcases hy with h | h
  · exact h
  · exfalso
    rw [hx, h] at hb
    rcases hb with h' | h'
    · have h0 : (2 : F) = 0 := by rw [← h']; grind
      have := natCast_inj49 (F := F) (x := 2) (y := 0) (by omega) (by omega)
        (by rw [← natCast_ofNat' 2, Lean.Grind.Semiring.natCast_zero]; exact h0)
      omega
    · have : (1 : F) = 0 := by
        have e : (1 : F) + 1 - 1 = 1 - 1 := by rw [h']
        grind
      exact Lean.Grind.Field.zero_ne_one this.symm

/-- **A lane that stores to memory writes the RAM's log**, at its byte
address, at its store step. -/
theorem MemBusWorld.store_at {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {k m l : Nat} (hk : k < M.K)
    (hm : m < Main.numRows) (hl : l < 4) (hst : StoresAt M k m l) :
    ∃ e ∈ MemSort.writes (ramLog (W.segs hram)),
      e.val = (el 4 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)),
        el 5 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int))) ∧
      el 1 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)) =
        ((8 * e.addr : Nat) : F) ∧ e.step = memBase k m l + 2 := by
  obtain ⟨hs, hside, hbits, hpre, hat⟩ := W.store_sender hk hl
  obtain ⟨-, -, -, -, -, -, -, -, c9, -, c11, c12, -, -⟩ := lane_cols (F := F) hl (M.seg k).1 (M.seg k).2 m
  have hlen := (access_cols (F := F) hl (M.seg k).1 (M.seg k).2 m).2.2.2.2.2.2.2.2.2.2.2.2.2.2.2.2
  unfold StoresAt at hst
  -- The register bit is 0, the count 1, the op 2.
  have hb := hbits (m : Int) (hat m hm)
  have hp := hpre (m : Int)
  rw [c11] at hb
  rw [c9, Lean.Grind.Semiring.add_zero] at hp
  have hreg := reg_zero hst hb hp
  have hr1 : ((PairTag.lane l 2).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 (m : Int) = 1 := by
    rw [c11, hst, hreg]; grind
  have hop : el 0 (tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)) = 2 := by
    rw [c12, hst, hreg]; grind
  have ht := tuple6 hlen
  rw [hop] at ht
  have hw8 := W.store_width hs hside hbits (hat m hm) ht hr1
  rw [hw8] at ht
  obtain ⟨e, he, hv, hX, hst'⟩ := W.store_write hram hs hside hbits (hat m hm) ht hr1
  refine ⟨e, he, hv, hX, ?_⟩
  have hb' := W.log_bounds hram (.inr he)
  have e2 := W.store_step hk hm hl
  rw [hst'] at e2
  exact natCast_inj49 (F := F) (by omega) (by have := memBase_lt hk W.ok.small hm hl; omega) e2

/-- **Every store of Main's is a write of the RAM's log**, at its byte
address. -/
theorem MemBusWorld.store_in_writes {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {w' : Emit (F × F)}
    (hw' : w' ∈ mainStores M) : ∃ w ∈ MemSort.writes (ramLog (W.segs hram)), toByte w = w' := by
  obtain ⟨k, hk, m, hm, l, hl, hst, -, rfl⟩ := mem_mainStores hw'
  obtain ⟨e, he, hv, hX, hstep⟩ := W.store_at hram hk hm hl hst
  refine ⟨e, he, ?_⟩
  have hb' := W.log_bounds hram (.inr he)
  unfold toByte
  rw [hX, natCast_ofNat_lt (F := F) (by unfold goldilocks; omega), hstep, hv]

/-! ### Reads, as `memVal` of Main's stores -/

theorem mainStores_step3 {M : MainSegs F} {w : Emit (F × F)} (hw : w ∈ mainStores M) : w.step % 4 = 3 := by
  obtain ⟨k, -, m, -, l, -, -, hst, -⟩ := mem_mainStores hw
  rw [hst]; simp only [memBase]; omega

/-- **A read of the RAM's log returns `memVal` of Main's stores**, at its byte
address, when its step is not a store's and the address's initial value is
0. -/
theorem MemBusWorld.read_memVal {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) (hc : RamChain (W.segs hram))
    {init : Nat → F × F} {e : Emit (F × F)} (he : e ∈ MemSort.reads (ramLog (W.segs hram)))
    (hs : e.step % 4 ≠ 3) (hinit : init (8 * e.addr) = (0, 0)) :
    e.val = memVal init (mainStores M) (8 * e.addr) e.step := by
  have hnd := mainStores_nodup M
  rcases ram_reads_last_write hc e he with ⟨w, hw, ha, hle, hv, hmax⟩ | ⟨hv, hnone⟩
  · -- The latest write, a store of Main's.
    have hw' := W.write_in_stores hram hw
    have hlt : w.step < e.step := by
      have := mainStores_step3 hw'
      simp only [toByte] at this
      omega
    rw [memVal_of_latest hnd (w := toByte w)]
    · exact hv.symm
    · refine ⟨hw', by simp [toByte, ha], hlt, fun w'' hw'' ha'' hs'' => ?_⟩
      obtain ⟨w₀, hw₀, rfl⟩ := W.store_in_writes hram hw''
      simp only [toByte] at ha'' hs'' ⊢
      exact hmax w₀ hw₀ (by omega) (by omega)
  · -- No write before it: the initial value.
    rw [memVal_of_none, hinit]
    · exact hv
    · intro w'' hw'' ha'' hs''
      obtain ⟨w₀, hw₀, rfl⟩ := W.store_in_writes hram hw''
      simp only [toByte] at ha'' hs''
      exact hnone w₀ hw₀ (by omega) (by omega)

/-! ### Main's loads, from the bus -/

theorem Lane.emit_addr {V : Type} {q : Lane V} {e : Emit V} (h : e ∈ q.reads ∨ e ∈ q.writes) :
    e.addr = q.addr := by
  rcases h with h | h
  · simp only [Lane.reads, List.mem_append] at h
    rcases h with h | h
    · split at h
      · exact nomatch h
      · rw [List.mem_singleton.mp h]
    · split at h
      · rw [List.mem_singleton.mp h]
      · exact nomatch h
  · simp only [Lane.writes] at h
    split at h
    · rw [List.mem_singleton.mp h]
    · exact nomatch h

/-- **Every emission of the RAM's log is in the RAM's region.** -/
theorem MemBusWorld.log_region {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {e : Emit (F × F)}
    (he : e ∈ MemSort.reads (ramLog (W.segs hram)) ∨ e ∈ MemSort.writes (ramLog (W.segs hram))) :
    335544320 ≤ e.addr ∧ e.addr ≤ 402653183 := by
  obtain ⟨lane, hlane, hel⟩ : ∃ lane ∈ ramLog (W.segs hram), e ∈ lane.reads ∨ e ∈ lane.writes := by
    rcases he with h | h
    · obtain ⟨lane, hl, h'⟩ := List.mem_flatMap.mp h; exact ⟨lane, hl, .inl h'⟩
    · obtain ⟨lane, hl, h'⟩ := List.mem_flatMap.mp h; exact ⟨lane, hl, .inr h'⟩
  obtain ⟨seg, hseg, hls⟩ := List.mem_flatMap.mp hlane
  obtain ⟨⟨I, hI⟩, -, rfl⟩ := List.mem_map.mp hseg
  obtain ⟨n, hn, rfl⟩ := List.mem_map.mp hls
  rw [List.mem_range] at hn
  rw [Lane.emit_addr hel]
  show 335544320 ≤ (hram I hI).trace.AA (4 * ((n / 4 : Nat) : Int).toNat + n % 4) ∧
    (hram I hI).trace.AA (4 * ((n / 4 : Nat) : Int).toNat + n % 4) ≤ 402653183
  rw [Int.toNat_natCast]
  exact (hram I hI).trace.AA_region (by omega)

/-- A lane's access `κ` is among the memory bus's senders, assuming, a sender
of bits, at every row. -/
theorem MemBusWorld.access_sender {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {k l κ : Nat} (hk : k < M.K) (hl : l < 4) (hκ : κ < 3) :
    (⟨⟨(PairTag.lane l κ).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F) ∈ busSenders M S ∧
      (⟨⟨(PairTag.lane l κ).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F).proving = false ∧
      (∀ j ∈ (⟨⟨(PairTag.lane l κ).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F).at,
        ((PairTag.lane l κ).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 0 ∨
          ((PairTag.lane l κ).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 1) ∧
      (∀ m : Nat, m < Main.numRows →
        (m : Int) ∈ (⟨⟨(PairTag.lane l κ).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ : Sender F).at) := by
  have hg : PairTag.lane l κ ∈ mainTags := lane_mem hl hκ
  have hP : (PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2 ∈ M.pairs := by
    simp only [MainSegs.pairs, List.mem_flatMap, List.mem_range, mainPairs, List.mem_map]
    exact ⟨k, hk, PairTag.lane l κ, hg, rfl⟩
  have hgood := M.pairs_good W.ok _ hP
  have honce := (mainTags_once (F := F)).mem _ hg
  refine ⟨?_, hgood.side_cur, fun j hj => hgood.bits_cur j (by rw [← hgood.same]; exact hj),
    fun m hm => by
      have := lane_at (M := M) (k := k) hm honce
      rw [← hgood.same] at this
      exact this⟩
  apply List.mem_append_left
  rw [← M.pairSenders_pairs]
  exact List.mem_flatMap.mpr ⟨_, hP, by simp [PairTag.pair]⟩

/-- A lane's register bits are bits. -/
theorem MemBusWorld.pre_bits {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {k l : Nat} (hk : k < M.K) (hl : l < 4) (j : Int) :
    (((M.seg k).2 j).a_src_reg l = 0 ∨ ((M.seg k).2 j).a_src_reg l = 1) ∧
      (((M.seg k).2 j).b_src_reg l = 0 ∨ ((M.seg k).2 j).b_src_reg l = 1) := by
  have := (W.ok.seg k hk).rows j
  rcases l with _ | _ | _ | _ | l
  · exact ⟨bit_or this.bool_a_src_reg_0, bit_or this.bool_b_src_reg_0⟩
  · exact ⟨bit_or this.bool_a_src_reg_1, bit_or this.bool_b_src_reg_1⟩
  · exact ⟨bit_or this.bool_a_src_reg_2, bit_or this.bool_b_src_reg_2⟩
  · exact ⟨bit_or this.bool_a_src_reg_3, bit_or this.bool_b_src_reg_3⟩
  · omega

/-- **A load a lane assumes is a read of the RAM's log, or an InputData
row**, at the access's step. -/
theorem MemBusWorld.access_load {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {k m l κ : Nat} (hk : k < M.K)
    (hm : m < Main.numRows) (hl : l < 4) (hκ : κ < 3) {τ : List F}
    (hτ : tupleOf ⟨(PairTag.lane l κ).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int) = τ)
    (hr1 : ((PairTag.lane l κ).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 (m : Int) = 1)
    (hop : el 0 τ = 1) (hlen : τ.length = 6) :
    (∃ e ∈ MemSort.reads (ramLog (W.segs hram)), (el 4 τ, el 5 τ) = e.val ∧
      el 1 τ = ((8 * e.addr : Nat) : F) ∧ e.step = memBase k m l + κ) ∨
    (∃ I ∈ S.inputData, ∃ r : Nat, r < InputData.numRows ∧ (I.t r).sel 0 ≠ 0 ∧
      tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) r =
        [1, el 1 τ, ((memBase k m l + κ : Nat) : F), 8, el 4 τ, el 5 τ]) := by
  obtain ⟨hs, hside, hbits, hat⟩ := W.access_sender hk hl hκ
  have hstep : el 2 τ = ((memBase k m l + κ : Nat) : F) := by
    have hcur := (mainTags_cur (F := F)).mem (PairTag.lane l κ) (lane_mem hl hκ)
    rw [← hτ, hcur k (M.seg k).1 (M.seg k).2 (W.ok.seg k hk) m hm]
    simp only [laneStep, memBase, Int.toNat_natCast]
  have ht := tuple6 hlen
  rw [hop, hstep] at ht
  have htup := hτ.trans ht
  have hw8 := W.load_width hs hside hbits (hat m hm) htup hr1
  rw [hw8] at htup
  rcases W.load_read hram hs hside hbits (hat m hm) htup hr1 with ⟨e, he, hv, hX, hst⟩ | hin
  · left
    have hb := W.log_bounds hram (.inl he)
    have := memBase_lt hk W.ok.small hm hl
    exact ⟨e, he, hv.symm, hX, natCast_inj49 (F := F) (by omega) (by omega) hst.symm⟩
  · right; exact hin

/-- **A lane's `a` load**, from the bus: a read of the RAM's log at its
address, or an InputData row. -/
theorem MemBusWorld.a_access {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {k m l : Nat} (hk : k < M.K)
    (hm : m < Main.numRows) (hl : l < 4) (ha : ((M.seg k).2 m).a_src_mem l = 1) :
    (∃ e ∈ MemSort.reads (ramLog (W.segs hram)),
      (((M.seg k).2 m).a l 0, ((M.seg k).2 m).a l 1) = e.val ∧
      ((M.seg k).2 m).a_offset_imm0 l = ((8 * e.addr : Nat) : F) ∧ e.step = memBase k m l) ∨
    (∃ I ∈ S.inputData, ∃ r : Nat, r < InputData.numRows ∧ (I.t r).sel 0 ≠ 0 ∧
      tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) r =
        [1, ((M.seg k).2 m).a_offset_imm0 l, ((memBase k m l : Nat) : F), 8,
          ((M.seg k).2 m).a l 0, ((M.seg k).2 m).a l 1]) := by
  obtain ⟨-, -, hbits, hat⟩ := W.access_sender hk hl (κ := 0) (by omega)
  obtain ⟨c0, c1, c2, -, c4, c5, c6, -⟩ := access_cols (F := F) hl (M.seg k).1 (M.seg k).2 m
  have hb := hbits m (hat m hm)
  rw [c0] at hb
  have hreg := reg_zero ha hb (W.pre_bits hk hl m).1
  have hr1 : ((PairTag.lane l 0).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 (m : Int) = 1 := by
    rw [c0, ha, hreg, Lean.Grind.Semiring.add_zero]
  have hop : el 0 (tupleOf ⟨(PairTag.lane l 0).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)) = 1 := by
    rw [c1, ha, hreg, Lean.Grind.Semiring.mul_zero, Lean.Grind.Semiring.add_zero]
  rcases W.access_load hram hk hm hl (by omega) rfl hr1 hop c6 with ⟨e, he, hv, hX, hst⟩ | ⟨I, hI, r, hr, hs, ht⟩
  · left
    rw [c4, c5, Lean.Grind.Semiring.add_zero, Lean.Grind.Semiring.add_zero] at hv
    rw [c2, Lean.Grind.Semiring.add_zero] at hX
    exact ⟨e, he, hv, hX, by rw [hst, Nat.add_zero]⟩
  · right
    refine ⟨I, hI, r, hr, hs, ht.trans ?_⟩
    rw [c2, c4, c5, Nat.add_zero, Lean.Grind.Semiring.add_zero, Lean.Grind.Semiring.add_zero,
      Lean.Grind.Semiring.add_zero]

/-- **A lane's `b` load**, from the bus: a read of the RAM's log at its
address, or an InputData row. -/
theorem MemBusWorld.b_access {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) {k m l : Nat} (hk : k < M.K)
    (hm : m < Main.numRows) (hl : l < 4)
    (hbm : ((M.seg k).2 m).b_src_mem l + ((M.seg k).2 m).b_src_ind l = 1) :
    (∃ e ∈ MemSort.reads (ramLog (W.segs hram)),
      (((M.seg k).2 m).b l 0, ((M.seg k).2 m).b l 1) = e.val ∧
      ((M.seg k).2 m).addr1 l = ((8 * e.addr : Nat) : F) ∧ e.step = memBase k m l + 1) ∨
    (∃ I ∈ S.inputData, ∃ r : Nat, r < InputData.numRows ∧ (I.t r).sel 0 ≠ 0 ∧
      tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) r =
        [1, ((M.seg k).2 m).addr1 l, ((memBase k m l + 1 : Nat) : F), 8,
          ((M.seg k).2 m).b l 0, ((M.seg k).2 m).b l 1]) := by
  obtain ⟨-, -, hbits, hat⟩ := W.access_sender hk hl (κ := 1) (by omega)
  obtain ⟨-, -, -, -, -, -, -, c0, c1, c2, -, c4, c5, c6, -⟩ :=
    access_cols (F := F) hl (M.seg k).1 (M.seg k).2 m
  have hb := hbits m (hat m hm)
  rw [c0] at hb
  have hreg := reg_zero hbm hb (W.pre_bits hk hl m).2
  have hr1 : ((PairTag.lane l 1).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 (m : Int) = 1 := by
    rw [c0, hbm, hreg, Lean.Grind.Semiring.add_zero]
  have hop : el 0 (tupleOf ⟨(PairTag.lane l 1).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int)) = 1 := by
    rw [c1, hbm, hreg, Lean.Grind.Semiring.mul_zero, Lean.Grind.Semiring.add_zero]
  rcases W.access_load hram hk hm hl (by omega) rfl hr1 hop c6 with ⟨e, he, hv, hX, hst⟩ | ⟨I, hI, r, hr, hs, ht⟩
  · left
    rw [c4, c5, Lean.Grind.Semiring.add_zero, Lean.Grind.Semiring.add_zero] at hv
    rw [c2, Lean.Grind.Semiring.add_zero] at hX
    exact ⟨e, he, hv, hX, hst⟩
  · right
    rw [c2, c4, c5] at ht
    simp only [Lean.Grind.Semiring.add_zero] at ht
    exact ⟨I, hI, r, hr, hs, ht⟩

/-! ### The input memory, as the initial memory -/

/-- InputData's instances on the memory bus, as a chain of its segments. -/
structure InWorld (S : MemOthers F) where
  seg : Nat → InTrace F
  K : Nat
  chain : InChain (fun k => (seg k).toSeg) K
  inst : ∀ I ∈ S.inputData, ∃ k < K, I.x = (seg k).x ∧ I.t = (seg k).t

/-- The input memory's lanes, over all its segments. -/
noncomputable def InWorld.log {S : MemOthers F} (IW : InWorld S) (n : Nat) : MemImm.ILane (F × F) :=
  inLog (fun k => (IW.seg k).toSeg) n

open Classical in
/-- **The initial memory**: at a byte address the input memory holds, its value
there; elsewhere 0. -/
noncomputable def InWorld.init {S : MemOthers F} (IW : InWorld S) (A : Nat) : F × F :=
  if h : ∃ n, n < IW.K * InputData.numRows ∧ 8 * (IW.log n).addr = A then
    (IW.log (Classical.choose h)).val else (0, 0)

theorem InWorld.log_region {S : MemOthers F} (IW : InWorld S) (n : Nat) :
    134217728 ≤ (IW.log n).addr ∧ (IW.log n).addr ≤ 268435455 :=
  (IW.seg (n / InputData.numRows)).A_region (Nat.mod_lt _ (by simp [InputData.numRows]))

/-- Above the input region, the initial memory is 0. -/
theorem InWorld.init_ram {S : MemOthers F} (IW : InWorld S) {a : Nat} (ha : 268435455 < a) :
    IW.init (8 * a) = (0, 0) := by
  unfold InWorld.init
  rw [dif_neg]
  rintro ⟨n, -, hn⟩
  have := (IW.log_region n).2
  omega

/-- **An InputData row reads the initial memory**: its byte address is `8a`,
`a` in the input region, and, away from the region's base, the initial memory
there is its value. -/
theorem InWorld.row {S : MemOthers F} (IW : InWorld S) {I : InputDataOnMem F} (hI : I ∈ S.inputData)
    {r : Nat} (hr : r < InputData.numRows) {X st v0 v1 : F}
    (ht : tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) r = [1, X, st, 8, v0, v1]) :
    ∃ a, 134217728 ≤ a ∧ a ≤ 268435455 ∧ X = ((8 * a : Nat) : F) ∧
      (a ≠ 134217728 → IW.init (8 * a) = (v0, v1)) := by
  obtain ⟨k, hk, -, hxt⟩ := IW.inst I hI
  simp only [tupleOf, InputData.bus26, List.map_cons, List.map_nil, List.cons.injEq] at ht
  have hX := ht.2.1
  have hv0 := ht.2.2.2.2.1
  have hv1 := ht.2.2.2.2.2.1
  rw [hxt] at hX hv0 hv1
  have hN : InputData.numRows = 4194304 := rfl
  obtain ⟨n, hn_def⟩ : ∃ n, n = k * InputData.numRows + r := ⟨_, rfl⟩
  have hnk : n / InputData.numRows = k := by rw [hn_def]; rw [hN] at hr ⊢; omega
  have hnr : n % InputData.numRows = r := by rw [hn_def]; rw [hN] at hr ⊢; omega
  have hlog : IW.log n = inLaneOf ((IW.seg k).t r) ((IW.seg k).A r) := by
    simp only [InWorld.log, inLog, hnk, hnr]; rfl
  have hreg := (IW.seg k).A_region hr
  refine ⟨(IW.seg k).A r, hreg.1, hreg.2, ?_, fun hb => ?_⟩
  · rw [← hX, (IW.seg k).addr_eq r hr, Lean.Grind.Semiring.natCast_mul, natCast_ofNat' 8]
    exact Lean.Grind.CommSemiring.mul_comm _ _
  · have hn : n < IW.K * InputData.numRows := by
      have : (k + 1) * InputData.numRows ≤ IW.K * InputData.numRows := Nat.mul_le_mul_right _ hk
      rw [Nat.add_mul, Nat.one_mul] at this; omega
    have hadn : (IW.log n).addr = (IW.seg k).A r := by rw [hlog]; rfl
    have hex : ∃ n, n < IW.K * InputData.numRows ∧ 8 * (IW.log n).addr = 8 * (IW.seg k).A r :=
      ⟨n, hn, by rw [hadn]⟩
    unfold InWorld.init
    rw [dif_pos hex]
    obtain ⟨hc, hca⟩ := Classical.choose_spec hex
    have hval : (IW.log n).val = (v0, v1) := by
      rw [hlog]
      show inVal _ = _
      simp only [inVal]
      rw [hv0, hv1]
    have hsame : (IW.log (Classical.choose hex)).addr = (IW.log n).addr := by omega
    have hbn : (IW.log n).addr ≠ 134217728 := by omega
    have hbc : (IW.log (Classical.choose hex)).addr ≠ 134217728 := by omega
    rw [← hval]
    rcases Nat.le_total (Classical.choose hex) n with h | h
    · exact input_consistent IW.chain h hn hsame hbn
    · exact (input_consistent IW.chain h hc hsame.symm hbc).symm

/-! ### `MemAnswers` -/

/-- An address Main's memory loads and stores may use: off the register file,
and not the input region's base, where free reads return fresh values. -/
def Avoids (A : Nat) : Prop := hits_registers (BitVec.ofNat 64 A) 8 = false ∧ A ≠ 1073741824

/-- **Main's memory accesses avoid the register file and the free-input
base**, a hypothesis: the model asserts the first of each access, and reads
the second as ordinary memory. -/
def MainAvoids (M : MainSegs F) : Prop :=
  ∀ k m l, k < M.K → m < Main.numRows → l < 4 → ∀ A : Nat, A < 2 ^ 32 →
    (((M.seg k).2 m).a_src_mem l = 1 → ((M.seg k).2 m).a_offset_imm0 l = ((A : Nat) : F) → Avoids A) ∧
    (((M.seg k).2 m).b_src_mem l + ((M.seg k).2 m).b_src_ind l = 1 →
      ((M.seg k).2 m).addr1 l = ((A : Nat) : F) → Avoids A) ∧
    (StoresAt M k m l →
      ((M.seg k).2 m).store_offset l + ((M.seg k).2 m).store_ind l * ((M.seg k).2 m).a l 0 =
        ((A : Nat) : F) → Avoids A)

/-- The addresses the bus answers for: words, 32-bit, avoided. -/
def busD (A : Nat) : Prop := A % 8 = 0 ∧ A < 2 ^ 32 ∧ Avoids A

theorem busD_domain : MemDomain busD := ⟨fun _ h => h.1, fun _ h => h.2.1, fun _ h => h.2.2.1⟩

/-- **A load, answered**: whether the RAM or InputData proves it, its value is
`memVal` of Main's stores over the input memory, at its address and step. -/
theorem MemBusWorld.load_answer {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) (hc : RamChain (W.segs hram))
    (IW : InWorld S) {X v0 v1 : F} {s : Nat} (hs4 : s % 4 ≠ 3)
    (havoid : ∀ A : Nat, A < 2 ^ 32 → X = ((A : Nat) : F) → Avoids A)
    (h : (∃ e ∈ MemSort.reads (ramLog (W.segs hram)), (v0, v1) = e.val ∧
        X = ((8 * e.addr : Nat) : F) ∧ e.step = s) ∨
      (∃ I ∈ S.inputData, ∃ r : Nat, r < InputData.numRows ∧ (I.t r).sel 0 ≠ 0 ∧
        tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) r = [1, X, ((s : Nat) : F), 8, v0, v1])) :
    ∃ A, busD A ∧ X = ((A : Nat) : F) ∧ (v0, v1) = memVal IW.init (mainStores M) A s := by
  rcases h with ⟨e, he, hv, hX, hst⟩ | ⟨I, hI, r, hr, -, ht⟩
  · have hreg := W.log_region hram (.inl he)
    have hav := havoid (8 * e.addr) (by omega) hX
    refine ⟨8 * e.addr, ⟨by omega, by omega, hav⟩, hX, ?_⟩
    rw [hv, ← hst]
    exact W.read_memVal hram hc he (by omega) (IW.init_ram (by omega))
  · obtain ⟨a, ha1, ha2, hX, hinit⟩ := IW.row hI hr ht
    have hav := havoid (8 * a) (by omega) hX
    refine ⟨8 * a, ⟨by omega, by omega, hav⟩, hX, ?_⟩
    rw [memVal_of_none, hinit (by have := hav.2; omega)]
    intro w hw ha _
    obtain ⟨w₀, hw₀, rfl⟩ := W.store_in_writes hram hw
    have := W.log_region hram (.inr hw₀)
    simp only [toByte] at ha
    omega

/-- **`MemAnswers`, from the memory bus.** Over a sound memory bus whose
other AIRs send no loads or stores (`MemBusWorld`), with the RAM's instances
a chain of segments, InputData's a chain of its own, and Main's accesses
avoiding the register file and the free-input base, Main's loads return
`memVal` of Main's stores over the input memory. -/
theorem MemBusWorld.answers {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : ∀ I ∈ S.mem, RamInst I) (hc : RamChain (W.segs hram))
    (IW : InWorld S) (hav : MainAvoids M) :
    MemAnswers M busD IW.init (mainStores M) where
  nodup := mainStores_nodup M
  a_load k m l hk hm hl ha :=
    W.load_answer hram hc IW (s := memBase k m l) (by simp only [memBase]; omega)
      (fun A hA hX => (hav k m l hk hm hl A hA).1 ha hX) (W.a_access hram hk hm hl ha)
  b_load k m l hk hm hl hb :=
    W.load_answer hram hc IW (s := memBase k m l + 1) (by simp only [memBase]; omega)
      (fun A hA hX => (hav k m l hk hm hl A hA).2.1 hb hX) (W.b_access hram hk hm hl hb)
  store k m l hk hm hl hst := by
    obtain ⟨e, he, hv, hX, hstep⟩ := W.store_at hram hk hm hl hst
    obtain ⟨-, -, -, -, -, -, -, -, -, -, -, -, c13, c14⟩ := lane_cols (F := F) hl (M.seg k).1 (M.seg k).2 m
    have c1 := (access_cols (F := F) hl (M.seg k).1 (M.seg k).2 m).2.2.2.2.2.2.2.2.2.2.2.2.2.2.1
    have hX' := hX
    rw [c1, Lean.Grind.Semiring.add_zero] at hX'
    have hreg := W.log_region hram (.inr he)
    refine ⟨8 * e.addr, ⟨by omega, by omega, (hav k m l hk hm hl _ (by omega)).2.2 hst hX'⟩, hX', ?_⟩
    have hmem := mainStores_mem hk hm hl hst
    rw [hX, natCast_ofNat_lt (F := F) (by unfold goldilocks; omega), c13, c14] at hmem
    exact hmem
  only w hw := by
    refine ⟨mainStores_step3 hw, fun k m l hk hm hl hs => ?_⟩
    obtain ⟨k', -, m', hm', l', hl', hst, hs', -⟩ := mem_mainStores hw
    have := memBase_index (k := k) hm hl
    have := memBase_index (k := k') hm' hl'
    have hN : Main.numRows = 4194304 := rfl
    rw [hN] at hm hm'
    obtain ⟨rfl, rfl, rfl⟩ : k = k' ∧ m = m' ∧ l = l' := by omega
    exact hst

end Correspondence
