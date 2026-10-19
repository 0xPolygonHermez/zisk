/-
  The register file: every read returns the last write.

  `pairs_chain` (`RegChain.lean`) makes good pairs, whose register tuples
  balance, into a `MemCheck` chain; `MainRegs.lean` proves Main's pairs good,
  their gaps range-checked and the flushes' addresses their registers'. This
  file puts the segments together:

  * `MainSegs`: the Main instances the memory bus sees, segment `k` with its
    first step `k·2^24` and its range checks sound (`MainSegs.Ok`);
  * on a sound memory bus over them and every other AIR (`SumHolds`), with
    the global updates `GlobalRegs` describes, each register's accesses —
    lane accesses of `a`, `b` and the store, and the flushes — form a
    `MemCheck` chain (`MainSegs.reg_chain`), whose steps are distinct:
    segments, lanes, slots and flush windows each own their steps;
  * so every access reads the latest write before it, or 0 (`MainSegs.reads`).
-/
import Proofs.MainRegs

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/- Not every lemma needs the characteristic. -/
set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- Main's instances, as the memory bus sees them: segment `k < K`. -/
structure MainSegs (F : Type) [Lean.Grind.Field F] where
  K : Nat
  seg : Nat → Main.Ctx F × Main.Trace F

/-- The segments, in order. -/
def MainSegs.list (M : MainSegs F) : List (Main.Ctx F × Main.Trace F) := (List.range M.K).map M.seg

/-- Every register-access pair of every segment. -/
def MainSegs.pairs (M : MainSegs F) : List (Pair F) :=
  (List.range M.K).flatMap fun k => mainPairs k (M.seg k).1 (M.seg k).2

/-- What the segments need: few enough (`K < 2^20`, so the steps stay far below
the characteristic), each one's first step and row steps, and each one's range
checks of its gaps sound. -/
structure MainSegs.Ok (M : MainSegs F) : Prop where
  small : M.K < 1048576
  seg : ∀ k < M.K, MainSegOk k (M.seg k).1 (M.seg k).2
  ranges : ∀ k < M.K, ∀ g : PairTag, RangeHolds Main.ranges g.rangeArg (M.seg k).1 (M.seg k).2

/-! ### Pairs of all segments -/

theorem MainSegs.mem_pairs {M : MainSegs F} {P : Pair F} (hP : P ∈ M.pairs) :
    ∃ k < M.K, ∃ g ∈ mainTags, P = g.pair k (M.seg k).1 (M.seg k).2 := by
  simp only [MainSegs.pairs, List.mem_flatMap, List.mem_range, mainPairs, List.mem_map] at hP
  obtain ⟨k, hk, g, hg, rfl⟩ := hP
  exact ⟨k, hk, g, hg, rfl⟩

theorem MainSegs.pairs_good {M : MainSegs F} (h : M.Ok) : ∀ P ∈ M.pairs, P.Good := by
  intro P hP
  obtain ⟨k, hk, g, hg, rfl⟩ := M.mem_pairs hP
  exact (mainTags_good (h.seg k hk)).mem g hg

theorem pairSenders_flatMap {α : Type _} (l : List α) (f : α → List (Pair F)) :
    pairSenders (l.flatMap f) = l.flatMap fun a => pairSenders (f a) := by
  induction l with
  | nil => rfl
  | cons a l ih =>
    simp only [List.flatMap_cons, pairSenders, List.flatMap_append] at ih ⊢
    rw [ih]

/-- The segments' senders on the memory bus are their pairs'. -/
theorem MainSegs.pairSenders_pairs (M : MainSegs F) :
    pairSenders M.pairs = M.list.flatMap fun m => mainMemSenders m.1 m.2 := by
  rw [MainSegs.pairs, pairSenders_flatMap, MainSegs.list, List.flatMap_map]
  simp only [pairSenders_mainPairs]

/-! ### Counting the lists -/

theorem provedList_length_le (ss : List (Sender F)) (B : Nat) (h : ∀ s ∈ ss, s.at.length ≤ B) :
    (provedList ss).length ≤ ss.length * B := by
  unfold provedList
  induction ss with
  | nil => simp
  | cons s ss ih =>
    have ih := ih (fun s' hs' => h s' (List.mem_cons_of_mem _ hs'))
    have hs := h s (List.mem_cons_self ..)
    have hsent : s.sent.length ≤ s.at.length := List.length_filterMap_le _ _
    simp only [List.filter_cons]
    split
    · simp only [List.flatMap_cons, List.length_append, List.length_cons]
      rw [Nat.succ_mul]; omega
    · simp only [List.length_cons]; rw [Nat.succ_mul]; omega

theorem assumedList_length_le (ss : List (Sender F)) (B : Nat) (h : ∀ s ∈ ss, s.at.length ≤ B) :
    (assumedList ss).length ≤ ss.length * B := by
  unfold assumedList
  induction ss with
  | nil => simp
  | cons s ss ih =>
    have ih := ih (fun s' hs' => h s' (List.mem_cons_of_mem _ hs'))
    have hs := h s (List.mem_cons_self ..)
    have hsent : s.sent.length ≤ s.at.length := List.length_filterMap_le _ _
    simp only [List.filter_cons]
    split
    · simp only [List.flatMap_cons, List.length_append, List.length_cons]
      rw [Nat.succ_mul]; omega
    · simp only [List.length_cons]; rw [Nat.succ_mul]; omega

theorem mainTags_length : mainTags.length = 168 := rfl

theorem MainSegs.pairs_length (M : MainSegs F) : M.pairs.length = M.K * 168 := by
  unfold MainSegs.pairs
  induction M.K with
  | zero => rfl
  | succ K ih =>
    rw [List.range_succ, List.flatMap_append, List.length_append, ih]
    simp [mainPairs, mainTags_length, Nat.succ_mul]

theorem pairSenders_length (Ps : List (Pair F)) : (pairSenders Ps).length = 2 * Ps.length := by
  induction Ps with
  | nil => rfl
  | cons P Ps ih => simp only [pairSenders, List.flatMap_cons] at ih ⊢; simp [ih]; omega

theorem MainSegs.at_le (M : MainSegs F) : ∀ s ∈ pairSenders M.pairs, s.at.length ≤ Main.numRows := by
  intro s hs
  obtain ⟨P, hP, hs⟩ := List.mem_flatMap.mp hs
  obtain ⟨k, -, g, -, rfl⟩ := M.mem_pairs hP
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with rfl | rfl <;>
  · simp only [PairTag.pair, Sender.at]
    split
    · simp [Main.numRows]
    · simp

/-! ### What a raw access is -/

theorem Pair.mem_raws {P : Pair F} {r : F} {a : Raw F} (ha : a ∈ P.raws r) :
    ∃ j ∈ P.pre.at, P.pre.p.arg.reps P.pre.p.x P.pre.p.t j = 1 ∧ el 1 (tupleOf P.pre.p j) = r ∧
      a.step = P.step j ∧ a.prevF = el 2 (tupleOf P.pre.p j) := by
  unfold Pair.raws at ha
  obtain ⟨j, hj, he⟩ := List.mem_filterMap.mp ha
  split at he
  · rename_i hc
    obtain rfl := Option.some.inj he
    exact ⟨j, hj, hc.1, hc.2, rfl, rfl⟩
  · exact nomatch he

/-- The tags' indices are in range. -/
def PairTag.Wf : PairTag → Prop
  | .lane l κ => l < 4 ∧ κ < 3
  | .flush f ρ => f < 4 ∧ ρ < 39

instance : DecidablePred PairTag.Wf := fun g => by cases g <;> unfold PairTag.Wf <;> infer_instance

set_option maxRecDepth 100000 in
theorem mainTags_wf : ∀ g ∈ mainTags, g.Wf := by decide

/-! ### Steps -/

/-- The step of a raw access of register `n + 1`, by its pair: a lane access at a
row below `2^22`, or the flush of a window, for that register. -/
def RawStep (k n : Nat) (a : Raw F) : PairTag → Prop
  | .lane l κ => l < 4 ∧ κ < 3 ∧ ∃ m, m < 4194304 ∧ a.step = 1 + 4 * (k * 16777216 + 4 * m + l) + κ
  | .flush f ρ => f < 4 ∧ ρ = n ∧ a.step = 4 * (k * 16777216) + 4 * ((f + 1) * 4194304)

theorem raw_step {M : MainSegs F} {n : Nat} (hn : n < 39) {k : Nat} {g : PairTag}
    (hg : g ∈ mainTags) {a : Raw F}
    (ha : a ∈ (g.pair k (M.seg k).1 (M.seg k).2).raws ((n + 1 : Nat) : F)) : RawStep k n a g := by
  obtain ⟨j, hj, -, haddr, hstep, -⟩ := Pair.mem_raws ha
  have hwf := mainTags_wf g hg
  cases g with
  | lane l κ =>
    obtain ⟨hl, hκ⟩ := hwf
    have honce : ((PairTag.lane l κ).preArg (F := F)).once = false := (mainTags_once (F := F)).mem _ hg
    obtain ⟨m, hm, rfl⟩ := mem_at_rows
      (s := ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre) honce hj
    refine ⟨hl, hκ, m, by simpa [Main.numRows] using hm, ?_⟩
    rw [hstep]
    simp [PairTag.pair, PairTag.step, laneStep]
  | flush f ρ =>
    obtain ⟨hf, hρ⟩ := hwf
    have hA := (mainTags_addr (F := F)).mem _ hg f ρ rfl (M.seg k).1 (M.seg k).2 j
    have : ((ρ + 1 : Nat) : F) = ((n + 1 : Nat) : F) := hA.symm.trans haddr
    have := natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega) this
    refine ⟨hf, by omega, ?_⟩
    rw [hstep]
    simp only [PairTag.pair, PairTag.step, flushStep]
    have : 1 ≤ (f + 1) * 4194304 := by omega
    omega

/-- **Equal steps, same access kind.** Two raw accesses of register `n + 1` with
the same step come from the same segment and the same pair. -/
theorem step_inj {M : MainSegs F} {n : Nat} (hn : n < 39) {k k' : Nat} (_hk : k < 1048576)
    (hk' : k' < 1048576) {g g' : PairTag} (hg : g ∈ mainTags) (hg' : g' ∈ mainTags) {a b : Raw F}
    (ha : a ∈ (g.pair k (M.seg k).1 (M.seg k).2).raws ((n + 1 : Nat) : F))
    (hb : b ∈ (g'.pair k' (M.seg k').1 (M.seg k').2).raws ((n + 1 : Nat) : F))
    (h : a.step = b.step) : k = k' ∧ g = g' := by
  have sa := raw_step hn hg ha
  have sb := raw_step hn hg' hb
  cases g with
  | lane l κ =>
    obtain ⟨hl, hκ, m, hm, ea⟩ := sa
    cases g' with
    | lane l' κ' =>
      obtain ⟨hl', hκ', m', hm', eb⟩ := sb
      have : k = k' ∧ l = l' ∧ κ = κ' := by omega
      obtain ⟨rfl, rfl, rfl⟩ := this
      exact ⟨rfl, rfl⟩
    | flush f' ρ' =>
      obtain ⟨hf', -, eb⟩ := sb
      exfalso; omega
  | flush f ρ =>
    obtain ⟨hf, hρ, ea⟩ := sa
    cases g' with
    | lane l' κ' =>
      obtain ⟨hl', hκ', m', hm', eb⟩ := sb
      exfalso; omega
    | flush f' ρ' =>
      obtain ⟨hf', hρ', eb⟩ := sb
      have : k = k' ∧ f = f' := by omega
      obtain ⟨rfl, rfl⟩ := this
      subst hρ hρ'
      exact ⟨rfl, rfl⟩

theorem Sender.at_nodup (s : Sender F) : s.at.Nodup := by
  unfold Sender.at
  split
  · exact List.nodup_cons.mpr ⟨by simp, List.nodup_nil⟩
  · exact List.nodup_range.map Int.ofNat (fun a b hab e => hab (Int.ofNat.inj e))

/-- Steps of a filtered row list, injective on the rows, are distinct. -/
theorem steps_nodup {js : List Int} (hjs : js.Nodup) (d : Int → Option (Raw F)) (st : Int → Nat)
    (hd : ∀ j ∈ js, ∀ a, d j = some a → a.step = st j)
    (hinj : ∀ j ∈ js, ∀ j' ∈ js, st j = st j' → j = j') :
    ((js.filterMap d).map (·.step)).Nodup := by
  induction js with
  | nil => exact List.nodup_nil
  | cons j js ih =>
    rw [List.nodup_cons] at hjs
    have ih := ih hjs.2 (fun j' hj' => hd j' (List.mem_cons_of_mem _ hj'))
      (fun a ha b hb => hinj a (List.mem_cons_of_mem _ ha) b (List.mem_cons_of_mem _ hb))
    rw [List.filterMap_cons]
    split
    · exact ih
    · rename_i a hda
      rw [List.map_cons, List.nodup_cons]
      refine ⟨fun hm => ?_, ih⟩
      obtain ⟨b, hb, hbs⟩ := List.mem_map.mp hm
      obtain ⟨j', hj', hdb⟩ := List.mem_filterMap.mp hb
      have e1 := hd j (List.mem_cons_self ..) a hda
      have e2 := hd j' (List.mem_cons_of_mem _ hj') b hdb
      have := hinj j (List.mem_cons_self ..) j' (List.mem_cons_of_mem _ hj') (by rw [← e1, ← e2, hbs])
      subst this
      exact hjs.1 hj'

/-- Within one pair, steps are distinct: a lane's rows have distinct steps, a
flush is sent once. -/
theorem pair_steps_nodup {M : MainSegs F} {n : Nat} {k : Nat} {g : PairTag} (hg : g ∈ mainTags) :
    (((g.pair k (M.seg k).1 (M.seg k).2).raws ((n + 1 : Nat) : F)).map (·.step)).Nodup := by
  unfold Pair.raws
  apply steps_nodup (Sender.at_nodup _) _ (g.pair k (M.seg k).1 (M.seg k).2).step
  · intro j _ a ha
    split at ha
    · obtain rfl := Option.some.inj ha; rfl
    · exact nomatch ha
  · have honce := (mainTags_once (F := F)).mem _ hg
    cases g with
    | lane l κ =>
      change ((PairTag.lane l κ).preArg (F := F)).once = false at honce
      intro j hj j' hj' he
      obtain ⟨m, hm, rfl⟩ := mem_at_rows
        (s := ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre) honce hj
      obtain ⟨m', hm', rfl⟩ := mem_at_rows
        (s := ((PairTag.lane l κ).pair k (M.seg k).1 (M.seg k).2).pre) honce hj'
      simp only [PairTag.pair, PairTag.step, laneStep, Int.toNat_natCast] at he
      have : m = m' := by omega
      rw [this]
    | flush f ρ =>
      change ((PairTag.flush f ρ).preArg (F := F)).once = true at honce
      intro j hj j' hj' _
      simp only [PairTag.pair, Sender.at, honce, if_true, List.mem_singleton] at hj hj'
      rw [hj, hj']

/-! ### The chain -/

/-- A list whose pieces are distinct, and whose distinct indices own disjoint
pieces, has no duplicates. -/
theorem nodup_flatMap_of {α β : Type _} {l : List α} {f : α → List β} (hl : l.Nodup)
    (hf : ∀ a ∈ l, (f a).Nodup) (hd : ∀ a ∈ l, ∀ b ∈ l, ∀ x, x ∈ f a → x ∈ f b → a = b) :
    (l.flatMap f).Nodup := by
  induction l with
  | nil => exact List.nodup_nil
  | cons a l ih =>
    rw [List.nodup_cons] at hl
    rw [List.flatMap_cons, List.nodup_append]
    refine ⟨hf a (List.mem_cons_self ..),
      ih hl.2 (fun b hb => hf b (List.mem_cons_of_mem _ hb))
        (fun b hb c hc x => hd b (List.mem_cons_of_mem _ hb) c (List.mem_cons_of_mem _ hc) x), ?_⟩
    intro x hx y hy hxy
    subst hxy
    obtain ⟨b, hb, hxb⟩ := List.mem_flatMap.mp hy
    have := hd a (List.mem_cons_self ..) b (List.mem_cons_of_mem _ hb) x hx hxb
    subst this
    exact hl.1 hb

theorem el_zero_headD (τ : List F) : el 0 τ = τ.headD 0 := by cases τ <;> rfl

theorem mem_steps {l : List (Raw F)} {x : Nat} (h : x ∈ l.map (·.step)) : ∃ a ∈ l, a.step = x :=
  List.mem_map.mp h

/-- **Each register's accesses form a `MemCheck` chain.** On a sound memory bus
over Main's segments and every other AIR, with the register part of the global
updates as `GlobalRegs` writes it, register `x(n+1)`'s accesses, over all
segments, satisfy `MemCheck`'s hypotheses. -/
theorem MainSegs.reg_chain (M : MainSegs F) (h : M.Ok) (S : MemOthers F) {gp ga : List (List F)}
    (hgl : GlobalRegs gp ga)
    (hbus : SumHolds 10 (M.list.flatMap (fun m => mainMemSenders m.1 m.2) ++ S.senders) gp ga)
    {n : Nat} (hn : n < 39) :
    (MemCheck.proved ((M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess)).Perm
        (MemCheck.assumed (0, 0) ((M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess)) ∧
      (∀ e ∈ (M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess, e.prev < e.step) ∧
      (((M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess).map (·.step)).Nodup := by
  have hK := h.small
  apply pairs_chain M.pairs (M.pairs_good h) hn
  · -- Every register tuple balances among Main's pairs.
    intro τ h3
    rw [M.pairSenders_pairs]
    exact reg_chain_balance S M.list hgl hbus (by rw [← el_zero_headD, h3, intCast_lit])
  · have := provedList_length_le _ _ M.at_le
    rw [pairSenders_length, M.pairs_length] at this
    unfold goldilocks; simp only [Main.numRows] at this; omega
  · have := assumedList_length_le _ _ M.at_le
    rw [pairSenders_length, M.pairs_length] at this
    have h39 : (regInits (F := F)).length = 39 := by simp [regInits]
    unfold goldilocks; simp only [Main.numRows] at this; omega
  · -- Steps are positive and far below the characteristic.
    intro a ha
    obtain ⟨P, hP, ha⟩ := List.mem_flatMap.mp ha
    obtain ⟨k, hk, g, hg, rfl⟩ := M.mem_pairs hP
    have := raw_step hn hg ha
    cases g with
    | lane l κ => obtain ⟨hl, hκ, m, hm, e⟩ := this; rw [e]; omega
    | flush f ρ => obtain ⟨hf, -, e⟩ := this; rw [e]; omega
  · -- Gaps are range-checked.
    intro a ha
    obtain ⟨P, hP, ha⟩ := List.mem_flatMap.mp ha
    obtain ⟨k, hk, g, hg, rfl⟩ := M.mem_pairs hP
    obtain ⟨j, hj, hr, -, hs, hp⟩ := Pair.mem_raws ha
    obtain ⟨m, hm, e⟩ := (mainTags_gap (h.seg k hk) (h.ranges k hk)).mem g hg j hj hr
    exact ⟨m, hm, by rw [hs, hp]; exact e⟩
  · -- Steps are distinct: segments, then pairs, own their steps.
    rw [List.map_flatMap, MainSegs.pairs, List.flatMap_assoc]
    simp only [mainPairs, List.flatMap_map]
    apply nodup_flatMap_of List.nodup_range
    · intro k hk
      rw [List.mem_range] at hk
      apply nodup_flatMap_of mainTags_nodup
      · intro g hg; exact pair_steps_nodup hg
      · intro g hg g' hg' x hx hx'
        obtain ⟨a, ha, rfl⟩ := mem_steps hx
        obtain ⟨b, hb, hab⟩ := mem_steps hx'
        exact (step_inj hn (by omega) (by omega) hg hg' ha hb hab.symm).2
    · intro k hk k' hk' x hx hx'
      rw [List.mem_range] at hk hk'
      obtain ⟨g, hg, hx⟩ := List.mem_flatMap.mp hx
      obtain ⟨g', hg', hx'⟩ := List.mem_flatMap.mp hx'
      obtain ⟨a, ha, rfl⟩ := mem_steps hx
      obtain ⟨b, hb, hab⟩ := mem_steps hx'
      exact (step_inj hn (by omega) (by omega) hg hg' ha hb hab.symm).1

/-- **Every register access reads the last write.** On a sound memory bus, every
access to register `x(n+1)` in Main — a lane's `a`, `b` or store, or a flush —
proves the value of the latest access to it before, by memory step (every one
of which wrote a value), or, if it is the first, 0. -/
theorem MainSegs.reads (M : MainSegs F) (h : M.Ok) (S : MemOthers F) {gp ga : List (List F)}
    (hgl : GlobalRegs gp ga)
    (hbus : SumHolds 10 (M.list.flatMap (fun m => mainMemSenders m.1 m.2) ++ S.senders) gp ga)
    {n : Nat} (hn : n < 39) {e : MemCheck.Access (F × F)}
    (he : e ∈ (M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess) :
    (e.pv = (0, 0) ∧ ∀ e' ∈ (M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess,
        ¬ e'.step < e.step) ∨
      ∃ w ∈ (M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess,
        w.cur = some e.pv ∧ w.step < e.step ∧
        ∀ e' ∈ (M.pairs.flatMap (·.raws ((n + 1 : Nat) : F))).map Raw.toAccess,
          e'.step < e.step → e'.step ≤ w.step := by
  obtain ⟨hp, hlt, hnd⟩ := M.reg_chain h S hgl hbus hn
  exact MemCheck.reads_last_write hp hlt hnd he

end Correspondence
