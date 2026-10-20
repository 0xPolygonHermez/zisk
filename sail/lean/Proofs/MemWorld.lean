/-
  The memory bus, around Main: the setting for deriving `MemAnswers`.

  The other AIRs on the memory bus (`MemOthers`) split three ways: the RAM's
  instances, InputData's, and the rest. `NoOtherMem` is what the census is to
  prove: none of the rest sends a load or a store — a tuple with op 1 or 2 —
  with a nonzero count.
-/
import Proofs.MemRamBus
import Proofs.Registers

namespace Correspondence

open Pil.Zisk

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The RAM's instances' arguments on the memory bus. -/
def MemOthers.ramSenders (S : MemOthers F) : List (Sender F) := S.mem.flatMap MemOnMem.senders

/-- InputData's instances' arguments on the memory bus. -/
def MemOthers.inSenders (S : MemOthers F) : List (Sender F) := S.inputData.flatMap InputDataOnMem.senders

/-- Every other AIR's arguments on the memory bus. -/
def MemOthers.restSenders (S : MemOthers F) : List (Sender F) :=
  ({ S with mem := [], inputData := [] } : MemOthers F).senders

theorem MemOthers.mem_split (S : MemOthers F) {s : Sender F} (h : s ∈ S.senders) :
    s ∈ S.ramSenders ∨ s ∈ S.inSenders ∨ s ∈ S.restSenders := by
  by_cases hr : s ∈ S.ramSenders
  · exact .inl hr
  by_cases hi : s ∈ S.inSenders
  · exact .inr (.inl hi)
  right; right
  unfold MemOthers.ramSenders at hr
  unfold MemOthers.inSenders at hi
  simp only [MemOthers.restSenders, MemOthers.senders, List.mem_append, List.flatMap_nil,
    List.not_mem_nil, false_or, or_false] at h ⊢
  simpa [hr, hi] using h

/-- **No other AIR sends a load or a store**, with a nonzero count: what the
census is to prove. -/
def NoOtherMem (S : MemOthers F) : Prop :=
  ∀ s ∈ S.restSenders, ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 →
    (tupleOf s.p j).headD 0 ≠ 1 ∧ (tupleOf s.p j).headD 0 ≠ 2

/-! ### Integer counts -/

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- Sender `s` counts `τ` as the integer `z`: at most its rows, and `2^32` more
for an argument sent once. -/
def IntCount (s : Sender F) (τ : List F) (z : Int) : Prop :=
  s.count τ = ((z : Int) : F) ∧ z.natAbs ≤ s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)

theorem intCount_bits (s : Sender F)
    (hb : ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    (τ : List F) : ∃ z, IntCount s τ z ∧ 0 ≤ z := by
  obtain ⟨e, hle⟩ := s.count_of_bits hb τ
  exact ⟨_, ⟨e, by split <;> omega⟩, by omega⟩

theorem intCount_never (s : Sender F) {τ : List F} {k : F} (hk : τ.headD 0 = k)
    (h : ∀ j, s.p.arg.reps s.p.x s.p.t j ≠ 0 → (tupleOf s.p j).headD 0 ≠ k) :
    IntCount s τ 0 :=
  ⟨s.count_of_never hk h, by simp⟩

/-- The tuple the RAM's padding cancellation sends. -/
def padTuple (x : Mem.Ctx F) : List F :=
  [1, 3221225464, x.Mem_segment_last_step, 8, x.Mem_segment_last_value 0, x.Mem_segment_last_value 1]

set_option maxRecDepth 100000 in
omit [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks] in
theorem mem_range_max104 : ∀ d ∈ (Mem.ranges : List RangeDef), d.opid = 104 → d.max = 65535 := by
  decide

/-- **The padding's count**: `−padding_size`, below `2^32` by its range
checks on the last segment and 0 on the others, on its own tuple, and 0 on
any other. -/
theorem intCount_pad {x : Mem.Ctx F} {t : Mem.Trace F} (hrows : ∀ i, Mem.everyRow x t i)
    (r58 : RangeHolds Mem.ranges Mem.bus58 x t) (r60 : RangeHolds Mem.ranges Mem.bus60 x t)
    (τ : List F) :
    ∃ z, IntCount ⟨⟨Mem.bus65, x, t⟩, Mem.numRows⟩ τ z ∧ z ≤ 0 ∧ (z ≠ 0 → τ = padTuple x) := by
  -- The padding size, a natural below 2^32.
  obtain ⟨P, hP, eP⟩ : ∃ P : Nat, P < 2 ^ 32 ∧ Mem.Mem_padding_size x t 0 = ((P : Nat) : F) := by
    obtain ⟨b, hb⟩ := bool_of_idem (hrows 0).c1
    have e36 := (hrows 0).c36
    have e37 := (hrows 0).c37
    simp only [Mem.c36, Mem.c37, Mem.Zisk_direct_gsum_e_16, hb] at e36 e37
    cases b
    · refine ⟨0, by omega, ?_⟩
      simp only [bit, Bool.false_eq_true, if_false] at e36 e37
      have h0 : x.Mem_padding_size_chunks 0 = 0 := by grind
      have h1 : x.Mem_padding_size_chunks 1 = 0 := by grind
      simp only [Mem.Mem_padding_size, h0, h1, Lean.Grind.Semiring.natCast_zero]; grind
    · have hl : x.Mem_is_last_segment = 1 := by rw [hb]; rfl
      have chunk : ∀ (R : BusArg F (Mem.Ctx F) (Mem.Trace F)), RangeHolds Mem.ranges R x t →
          R.opids = [104] → R.reps x t 0 = x.Mem_is_last_segment + 0 →
          ∀ f, R.tuple = [f] → ∃ n : Nat, n < 2 ^ 16 ∧ f x t 0 = ((n : Nat) : F) := by
        intro R hR hop hreps f hf
        obtain ⟨-, -, b', d, hb', hd, hdo, -, -, hall⟩ := hR
        rw [hop] at hb'
        have hb'' : b' = 104 := by simpa using hb'.symm
        have hmax := mem_range_max104 d hd (by rw [hdo, hb''])
        obtain ⟨n, -, hn, e⟩ := hall 0 (by rw [hreps, hl, Lean.Grind.Semiring.add_zero]; exact one_ne_zero')
          f (by rw [hf]; exact List.mem_singleton_self _)
        exact ⟨n, by omega, e⟩
      obtain ⟨c0, h0, e0⟩ := chunk _ r58 rfl rfl _ rfl
      obtain ⟨c1, h1, e1⟩ := chunk _ r60 rfl rfl _ rfl
      simp only [Lean.Grind.Semiring.add_zero] at e0 e1
      refine ⟨c0 + 65536 * c1, by omega, ?_⟩
      simp only [Mem.Mem_padding_size, e0, e1]
      rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul, natCast_ofNat' 65536]
  -- Its count.
  have hat : (⟨⟨Mem.bus65, x, t⟩, Mem.numRows⟩ : Sender F).at = [0] := rfl
  by_cases ht : tupleOf (⟨Mem.bus65, x, t⟩ : Prover F) 0 = τ
  · refine ⟨-(P : Int), ⟨?_, ?_⟩, by omega, fun _ => ?_⟩
    · simp only [Sender.count, hat, List.map_cons, List.map_nil, List.foldr_cons, List.foldr_nil,
        if_pos ht, Lean.Grind.Semiring.add_zero]
      show -Mem.Mem_padding_size x t 0 = _
      rw [eP, Lean.Grind.Ring.intCast_neg, Lean.Grind.Ring.intCast_natCast]
    · simp only [hat, List.length_singleton, Mem.bus65, if_true]; omega
    · rw [← ht]; simp [tupleOf, Mem.bus65, padTuple, Lean.Grind.Semiring.add_zero]
  · refine ⟨0, ⟨?_, by simp⟩, Int.le_refl _, fun h => absurd rfl h⟩
    simp only [Sender.count, hat, List.map_cons, List.map_nil, List.foldr_cons, List.foldr_nil,
      if_neg ht, Lean.Grind.Semiring.add_zero, Lean.Grind.Ring.intCast_zero]

/-- A count that is a bit at every row. -/
theorem bits_of_bit {s : Sender F} (h : ∀ j, s.p.arg.reps s.p.x s.p.t j * (1 - s.p.arg.reps s.p.x s.p.t j) = 0) :
    ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1 :=
  fun j _ => bit_or (h j)

theorem bits_goal (s : Sender F) (τ : List F) (P : Int → Prop)
    (hb : ∀ j, s.p.arg.reps s.p.x s.p.t j * (1 - s.p.arg.reps s.p.x s.p.t j) = 0) :
    ∃ z, IntCount s τ z ∧ (0 ≤ z ∨ P z) := by
  obtain ⟨z, hz, h0⟩ := intCount_bits s (bits_of_bit hb) τ
  exact ⟨z, hz, .inl h0⟩

/-- **The RAM's counts.** Every one of an instance's nine arguments proves,
and counts a tuple as a non-negative integer — but the padding's, which counts
its own tuple as a non-positive one. -/
theorem ram_counts (I : MemOnMem F) (r58 : RangeHolds Mem.ranges Mem.bus58 I.x I.t)
    (r60 : RangeHolds Mem.ranges Mem.bus60 I.x I.t) (τ : List F) :
    ∀ s ∈ I.senders, s.proving = true ∧
      ∃ z, IntCount s τ z ∧ (0 ≤ z ∨ (z ≤ 0 ∧ (z ≠ 0 → τ = padTuple I.x))) := by
  intro s hs
  simp only [MemOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
  · obtain ⟨z, hz, h0, hp⟩ := intCount_pad I.rows r58 r60 τ
    exact ⟨rfl, z, hz, .inr ⟨h0, hp⟩⟩
  all_goals refine ⟨rfl, bits_goal _ τ _ fun j => ?_⟩
  all_goals first
    | (show (1 : F) * (1 - 1) = 0; grind)
    | (have e := (I.rows j).c3; simp only [Mem.c3] at e
       simpa [Mem.bus67, Lean.Grind.Semiring.add_zero] using e)
    | (have e := (I.rows j).c4; simp only [Mem.c4] at e
       simpa [Mem.bus69, Lean.Grind.Semiring.add_zero] using e)
    | (have e := (I.rows j).c5; simp only [Mem.c5] at e
       simpa [Mem.bus71, Lean.Grind.Semiring.add_zero] using e)
    | (have e := (I.rows j).c6; simp only [Mem.c6] at e
       simpa [Mem.bus73, Lean.Grind.Semiring.add_zero] using e)

/-- **InputData's count**: a bit, `sel`, at every row. -/
theorem input_counts (I : InputDataOnMem F) (τ : List F) :
    ∀ s ∈ I.senders, s.proving = true ∧ ∃ z, IntCount s τ z ∧ 0 ≤ z := by
  intro s hs
  simp only [InputDataOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs
  subst hs
  refine ⟨rfl, intCount_bits _ (bits_of_bit (fun j => by
    have e := (I.rows j).c3
    simp only [InputData.c3] at e
    simp only [InputData.bus26, Lean.Grind.Semiring.add_zero]
    exact e)) τ⟩

/-! ### The memory bus around Main -/

/-- The senders on the memory bus: Main's, then every other AIR's. -/
def busSenders (M : MainSegs F) (S : MemOthers F) : List (Sender F) :=
  M.list.flatMap (fun m => mainMemSenders m.1 m.2) ++ S.senders

/-- What deriving `MemAnswers` takes of the memory bus: Main's segments as the
register chain needs them, a sound bus over them and every other AIR, global
updates that balance every load and store among themselves, the RAM's padding
and last-step range checks sound, no other AIR sending a load or a store (`NoOtherMem`),
and few enough rows that every count stays far below the characteristic. -/
structure MemBusWorld (M : MainSegs F) (S : MemOthers F) (gp ga : List (List F)) : Prop where
  ok : M.Ok
  bus : SumHolds 10 (busSenders M S) gp ga
  globals : ∀ τ : List F, (τ.headD 0 = 1 ∨ τ.headD 0 = 2) → times gp τ = times ga τ
  pad : ∀ I ∈ S.mem, RangeHolds Mem.ranges Mem.bus58 I.x I.t ∧ RangeHolds Mem.ranges Mem.bus60 I.x I.t
  last : ∀ I ∈ S.mem, RangeHolds Mem.ranges Mem.bus54 I.x I.t ∧ RangeHolds Mem.ranges Mem.bus56 I.x I.t
  none : NoOtherMem S
  size : ((busSenders M S).map fun s => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)).foldr
    (· + ·) 0 < 2 ^ 62

/-- **Every sender's count on a load or a store is an integer**: non-negative
on the assuming side, and negative only for a RAM padding, on its own tuple. -/
theorem MemBusWorld.counts {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) :
    ∀ s ∈ busSenders M S, ∃ z, IntCount s τ z ∧ (s.proving = false → 0 ≤ z) ∧
      (z < 0 → s.proving = true ∧ ∃ I ∈ S.mem, τ = padTuple I.x) := by
  intro s hs
  rcases List.mem_append.mp hs with hm | ho
  · -- Main's: bits.
    rw [← M.pairSenders_pairs] at hm
    obtain ⟨z, hz, h0⟩ := intCount_bits s (pairSenders_bits M.pairs (M.pairs_good W.ok) s hm) τ
    exact ⟨z, hz, fun _ => h0, fun h => absurd h0 (by omega)⟩
  · rcases S.mem_split ho with hr | hi | hn
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hr
      obtain ⟨hp, z, hz, hz'⟩ := ram_counts I (W.pad I hI).1 (W.pad I hI).2 τ s hs'
      refine ⟨z, hz, fun h => (by rw [hp] at h; exact nomatch h), fun hneg => ⟨hp, I, hI, ?_⟩⟩
      rcases hz' with h | ⟨-, h⟩
      · omega
      · exact h (by omega)
    · obtain ⟨I, -, hs'⟩ := List.mem_flatMap.mp hi
      obtain ⟨hp, z, hz, h0⟩ := input_counts I τ s hs'
      exact ⟨z, hz, fun _ => h0, fun h => absurd h0 (by omega)⟩
    · refine ⟨0, ?_, fun _ => Int.le_refl _, fun h => absurd h (by omega)⟩
      rcases hop with h1 | h2
      · exact intCount_never s h1 fun j hr => (W.none s hn j hr).1
      · exact intCount_never s h2 fun j hr => (W.none s hn j hr).2

open Classical in
/-- The integer count `MemBusWorld.counts` gives each sender. -/
noncomputable def MemBusWorld.z {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) (s : Sender F) : Int :=
  if h : s ∈ busSenders M S then Classical.choose (W.counts hop s h) else 0

theorem MemBusWorld.z_spec {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) {s : Sender F}
    (hs : s ∈ busSenders M S) :
    IntCount s τ (W.z hop s) ∧ (s.proving = false → 0 ≤ W.z hop s) ∧
      (W.z hop s < 0 → s.proving = true ∧ ∃ I ∈ S.mem, τ = padTuple I.x) := by
  unfold MemBusWorld.z
  rw [dif_pos hs]
  exact Classical.choose_spec (W.counts hop s hs)

theorem MemBusWorld.bound {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) :
    (((busSenders M S).map fun s => (W.z hop s).natAbs).foldr (· + ·) 0) < 2 ^ 62 := by
  have hle : ((busSenders M S).map fun s => (W.z hop s).natAbs).foldr (· + ·) 0 ≤
      ((busSenders M S).map fun s => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)).foldr
        (· + ·) 0 := by
    have key : ∀ l : List (Sender F), (∀ s ∈ l, s ∈ busSenders M S) →
        (l.map fun s => (W.z hop s).natAbs).foldr (· + ·) 0 ≤
          (l.map fun s => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)).foldr (· + ·) 0 := by
      intro l hl
      induction l with
      | nil => simp
      | cons s l ih =>
        simp only [List.map_cons, List.foldr_cons]
        have := (W.z_spec hop (hl s (List.mem_cons_self ..))).1.2
        have := ih (fun s' hs' => hl s' (List.mem_cons_of_mem _ hs'))
        omega
    exact key _ (fun s hs => hs)
  exact Nat.lt_of_le_of_lt hle W.size

theorem le_foldr_of_mem {α : Type _} (f : α → Nat) {l : List α} {a : α} (h : a ∈ l) :
    f a ≤ (l.map f).foldr (· + ·) 0 := by
  induction l with
  | nil => exact nomatch h
  | cons b l ih =>
    simp only [List.map_cons, List.foldr_cons]
    rcases List.mem_cons.mp h with rfl | h
    · omega
    · have := ih h; omega

open Classical in
/-- A sender of `busSenders` that sends `τ` with count 1, a bit sender's row,
counts it positively. -/
theorem MemBusWorld.z_pos {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) {s : Sender F}
    (hs : s ∈ busSenders M S)
    (hb : ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    {j : Int} (hj : j ∈ s.at) (ht : tupleOf s.p j = τ) (hr : s.p.arg.reps s.p.x s.p.t j = 1) :
    1 ≤ W.z hop s := by
  obtain ⟨e, hle⟩ := s.count_of_bits hb τ
  have hz := (W.z_spec hop hs).1
  have hsent : τ ∈ s.sent := by
    unfold Sender.sent
    exact List.mem_filterMap.mpr ⟨j, hj, by rw [if_pos hr, ht]⟩
  have hpos := List.count_pos_iff.mpr hsent
  rw [hz.1] at e
  have hza := Nat.lt_of_le_of_lt (le_foldr_of_mem (fun s => (W.z hop s).natAbs) hs) (W.bound hop)
  have hla := Nat.lt_of_le_of_lt
    (le_foldr_of_mem (fun s : Sender F => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)) hs) W.size
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp e
  unfold goldilocks at this
  simp only at hza hla
  split at hla <;> omega

/-- **Main's loads and stores are proved.** A load or store tuple some
assuming sender of bits sends with count 1 is sent by some proving sender, at
one of its rows, with a nonzero count. -/
theorem MemBusWorld.proved {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) {s₀ : Sender F}
    (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = false)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = τ) (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    ∃ s ∈ busSenders M S, s.proving = true ∧ 1 ≤ W.z hop s ∧
      ∃ j ∈ s.at, tupleOf s.p j = τ ∧ s.p.arg.reps s.p.x s.p.t j ≠ 0 := by
  obtain ⟨s, hs, hp, h1⟩ := W.bus.proved_of_assumed (W.globals τ hop) (W.z hop)
    (fun s hs => (W.z_spec hop hs).1.1) (W.bound hop) (fun s hs => (W.z_spec hop hs).2.1)
    hs₀ hside (W.z_pos hop hs₀ hb hj ht hr)
  have hza := Nat.lt_of_le_of_lt (le_foldr_of_mem (fun s => (W.z hop s).natAbs) hs) (W.bound hop)
  exact ⟨s, hs, hp, h1, s.sends_of_count (W.z_spec hop hs).1.1 h1 (by simp only at hza; omega)⟩

/-- **The provers' loads and stores are assumed.** A load or store tuple, not a
RAM padding's, that some proving sender of bits sends with count 1 is sent by
some assuming sender, at one of its rows, with a nonzero count. -/
theorem MemBusWorld.assumed {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2)
    (hnp : ∀ I ∈ S.mem, τ ≠ padTuple I.x) {s₀ : Sender F}
    (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = true)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = τ) (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    ∃ s ∈ busSenders M S, s.proving = false ∧
      ∃ j ∈ s.at, tupleOf s.p j = τ ∧ s.p.arg.reps s.p.x s.p.t j ≠ 0 := by
  have hpos : ∀ s ∈ busSenders M S, s.proving = true → 0 ≤ W.z hop s := by
    intro s hs _
    apply Classical.byContradiction
    intro hneg
    obtain ⟨-, I, hI, hτ⟩ := (W.z_spec hop hs).2.2 (by omega)
    exact hnp I hI hτ
  obtain ⟨s, hs, hp, h1⟩ := W.bus.assumed_of_proved (W.globals τ hop) (W.z hop)
    (fun s hs => (W.z_spec hop hs).1.1) (W.bound hop) hpos hs₀ hside (W.z_pos hop hs₀ hb hj ht hr)
  have hza := Nat.lt_of_le_of_lt (le_foldr_of_mem (fun s => (W.z hop s).natAbs) hs) (W.bound hop)
  exact ⟨s, hs, hp, s.sends_of_count (W.z_spec hop hs).1.1 h1 (by simp only at hza; omega)⟩

/-! ### The steps the RAM's tuples carry -/

/-- A load or store Main assumes carries a natural step below `2^48`: a lane
access's memory step. -/
theorem MemBusWorld.main_step {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {s : Sender F}
    (hs : s ∈ M.list.flatMap (fun m => mainMemSenders m.1 m.2)) (hside : s.proving = false)
    {j : Int} (hj : j ∈ s.at) {τ : List F} (ht : tupleOf s.p j = τ)
    (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) :
    ∃ n : Nat, n < 2 ^ 48 ∧ el 2 τ = ((n : Nat) : F) := by
  rw [← M.pairSenders_pairs] at hs
  obtain ⟨P, hP, hs⟩ := List.mem_flatMap.mp hs
  have hg := M.pairs_good W.ok P hP
  obtain ⟨k, hk, g, hgm, rfl⟩ := M.mem_pairs hP
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with rfl | rfl
  · rw [hg.side_pre] at hside; exact nomatch hside
  have hcur := (mainTags_cur (F := F)).mem g hgm
  have honce := (mainTags_once (F := F)).mem g hgm
  have hK := W.ok.small
  rw [hg.same] at hj
  cases g with
  | lane l κ =>
    obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨(PairTag.lane l κ).preArg, (M.seg k).1, (M.seg k).2⟩,
      Main.numRows⟩) honce hj
    have e := hcur k (M.seg k).1 (M.seg k).2 (W.ok.seg k hk) m hm
    have hl := mainTags_wf _ hgm
    simp only [PairTag.Wf] at hl
    refine ⟨laneStep k l κ m, ?_, ?_⟩
    · simp only [laneStep, Main.numRows] at hm ⊢; omega
    · rw [← ht]; exact e
  | flush f ρ =>
    have e := hcur (M.seg k).1 (M.seg k).2 j
    have ht' : tupleOf ⟨(PairTag.flush f ρ).curArg, (M.seg k).1, (M.seg k).2⟩ j = τ := ht
    rw [ht', el_zero_headD] at e
    rcases hop with h | h <;> rw [h] at e
    · exact absurd e (by
        intro h13; have := natCast_inj49 (F := F) (x := 1) (y := 3) (by omega) (by omega)
          (by rw [Lean.Grind.Semiring.natCast_one, ← natCast_ofNat' 3]; exact h13)
        omega)
    · exact absurd e (by
        intro h23; have := natCast_inj49 (F := F) (x := 2) (y := 3) (by omega) (by omega)
          (by rw [← natCast_ofNat' 2, ← natCast_ofNat' 3]; exact h23)
        omega)

theorem MemOthers.ram_sub (S : MemOthers F) {s : Sender F} (h : s ∈ S.ramSenders) : s ∈ S.senders := by
  unfold MemOthers.ramSenders at h
  simp only [MemOthers.senders, List.mem_append, h, true_or, or_true]

theorem MemOthers.in_sub (S : MemOthers F) {s : Sender F} (h : s ∈ S.inSenders) : s ∈ S.senders := by
  unfold MemOthers.inSenders at h
  simp only [MemOthers.senders, List.mem_append, h, true_or, or_true]

theorem lane_sender_mem (I : MemOnMem F) {L : Nat} (hL : L < 4) :
    (⟨⟨ramLaneArg L, I.x, I.t⟩, Mem.numRows⟩ : Sender F) ∈ I.senders ∧
      (⟨⟨ramDualArg L, I.x, I.t⟩, Mem.numRows⟩ : Sender F) ∈ I.senders := by
  rcases L with _ | _ | _ | _ | L
  all_goals first | omega | simp [MemOnMem.senders, ramLaneArg, ramDualArg]

theorem MemBusWorld.ram_mem {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (_W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) {s : Sender F}
    (hs : s ∈ I.senders) : s ∈ busSenders M S :=
  List.mem_append_right _ (S.ram_sub (List.mem_flatMap.mpr ⟨I, hI, hs⟩))

/-- A tuple with op 1 or 2, sent by a sender of bits that proves, with count 1,
and not a padding tuple, carries a natural step below `2^48`: some Main access
assumes it. -/
theorem MemBusWorld.proved_step {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2)
    (hnp : ∀ I ∈ S.mem, τ ≠ padTuple I.x) {s₀ : Sender F}
    (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = true)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = τ) (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    ∃ n : Nat, n < 2 ^ 48 ∧ el 2 τ = ((n : Nat) : F) := by
  obtain ⟨s, hs, hp, j', hj', ht', hr'⟩ := W.assumed hop hnp hs₀ hside hb hj ht hr
  rcases List.mem_append.mp hs with hm | ho
  · exact W.main_step hm hp hj' ht' hop
  · rcases S.mem_split ho with hr | hi | hn
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hr
      have := (ram_counts I (W.pad I hI).1 (W.pad I hI).2 τ s hs').1
      rw [hp] at this; exact nomatch this
    · obtain ⟨I, -, hs'⟩ := List.mem_flatMap.mp hi
      have := (input_counts I τ s hs').1
      rw [hp] at this; exact nomatch this
    · have := W.none s hn j' hr'
      rw [ht'] at this
      rcases hop with h | h
      · exact absurd h this.1
      · exact absurd h this.2

/-- A RAM instance's last step, a natural below `2^38`: its range-checked
chunks. -/
theorem MemBusWorld.last_step {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) :
    ∃ n : Nat, n < 2 ^ 38 ∧ I.x.Mem_segment_last_step = ((n : Nat) : F) := by
  obtain ⟨r54, r56⟩ := W.last I hI
  obtain ⟨n0, h0, e0⟩ := (mem_range r54 (j := 0) one_ne_zero' (List.mem_singleton_self _)).1 rfl
  obtain ⟨n1, h1, e1⟩ := (mem_range r56 (j := 0) one_ne_zero' (List.mem_singleton_self _)).2 rfl
  have c := (I.rows 0).c34
  simp only [Mem.c34, Mem.Zisk_direct_gsum_e_15] at c
  simp only [Mem.bus54, Mem.bus56, Lean.Grind.Semiring.add_zero] at e0 e1
  refine ⟨n0 + 4194304 * n1, by omega, ?_⟩
  rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul, ← e0, ← e1]
  generalize I.x.Mem_segment_last_step = a at c ⊢
  clear e0 e1
  rw [← natCast_ofNat' 4194304]
  grind

/-- **A step a RAM lane sends is a natural below `2^48`**, its access's or its
dual read's: some Main access assumes its tuple, or it is a padding tuple,
whose step is the last step. -/
theorem MemBusWorld.step_of_sent {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) {s : Sender F} (hs : s ∈ I.senders)
    (hb : ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    {j : Int} (hj : j ∈ s.at) (hr : s.p.arg.reps s.p.x s.p.t j = 1)
    (hop : (tupleOf s.p j).headD 0 = 1 ∨ (tupleOf s.p j).headD 0 = 2) :
    ∃ n : Nat, n < 2 ^ 48 ∧ el 2 (tupleOf s.p j) = ((n : Nat) : F) := by
  by_cases hpad : ∃ I' ∈ S.mem, tupleOf s.p j = padTuple I'.x
  · obtain ⟨I', hI', he⟩ := hpad
    obtain ⟨n, hn, e⟩ := W.last_step hI'
    refine ⟨n, by omega, ?_⟩
    rw [he]; exact e
  · have hnp : ∀ I' ∈ S.mem, tupleOf s.p j ≠ padTuple I'.x := fun I' hI' h => hpad ⟨I', hI', h⟩
    exact W.proved_step hop hnp (W.ram_mem hI hs) (ram_counts I (W.pad I hI).1 (W.pad I hI).2 [] s hs).1
      hb hj rfl hr

theorem lane_at_mem {C T : Type} {a : BusArg F C T} {x : C} {t : T} (ho : a.once = false) {j : Nat}
    (hj : j < Mem.numRows) : ((j : Nat) : Int) ∈ (⟨⟨a, x, t⟩, Mem.numRows⟩ : Sender F).at := by
  simp only [Sender.at, ho, Bool.false_eq_true, if_false, List.mem_map, List.mem_range]
  exact ⟨j, hj, rfl⟩

/-- **The RAM's steps are naturals below `2^48`**: each lane's, and each dual
read's where it is set. -/
theorem MemBusWorld.ram_steps {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) {j : Nat} (hj : j < Mem.numRows)
    {L : Nat} (hL : L < 4) :
    (∃ n : Nat, n < 2 ^ 48 ∧ (I.t j).step L = ((n : Nat) : F)) ∧
      ((I.t j).sel_dual L ≠ 0 → ∃ n : Nat, n < 2 ^ 48 ∧ (I.t j).step_dual L = ((n : Nat) : F)) := by
  obtain ⟨hml, hmd⟩ := lane_sender_mem I hL
  obtain ⟨hr1, ht1⟩ := ram_lane_tuple hL I.x I.t j
  obtain ⟨hr2, ht2⟩ := ram_dual_tuple hL I.x I.t j
  have honce1 : (ramLaneArg (F := F) L).once = false := by
    rcases L with _ | _ | _ | _ | L <;> first | omega | rfl
  have honce2 : (ramDualArg (F := F) L).once = false := by
    rcases L with _ | _ | _ | _ | L <;> first | omega | rfl
  constructor
  · have hb : ∀ j' ∈ (⟨⟨ramLaneArg L, I.x, I.t⟩, Mem.numRows⟩ : Sender F).at,
        (ramLaneArg (F := F) L).reps I.x I.t j' = 0 ∨ (ramLaneArg (F := F) L).reps I.x I.t j' = 1 :=
      fun j' _ => .inr (ram_lane_tuple hL I.x I.t j').1
    have hop : (tupleOf (⟨ramLaneArg L, I.x, I.t⟩ : Prover F) j).headD 0 = 1 ∨
        (tupleOf (⟨ramLaneArg L, I.x, I.t⟩ : Prover F) j).headD 0 = 2 := by
      rw [ht1]
      obtain ⟨b, hb'⟩ := bool_of_idem (ram_wr_bit (I.rows j) hL)
      cases b <;> simp only [List.headD_cons, hb', bit, Bool.false_eq_true, if_false, if_true]
      · left; grind
      · right; grind
    obtain ⟨n, hn, e⟩ := W.step_of_sent hI hml hb (lane_at_mem honce1 hj) hr1 hop
    rw [ht1] at e
    exact ⟨n, hn, e⟩
  · intro hsel
    have h1 : (I.t j).sel_dual L = 1 := by
      have : (I.t j).sel_dual L * (1 - (I.t j).sel_dual L) = 0 := by
        rcases L with _ | _ | _ | _ | L
        · exact (I.rows j).c3
        · exact (I.rows j).c4
        · exact (I.rows j).c5
        · exact (I.rows j).c6
        · omega
      obtain ⟨b, hb'⟩ := bool_of_idem this
      cases b <;> simp_all [bit]
    have hb : ∀ j' ∈ (⟨⟨ramDualArg L, I.x, I.t⟩, Mem.numRows⟩ : Sender F).at,
        (ramDualArg (F := F) L).reps I.x I.t j' = 0 ∨ (ramDualArg (F := F) L).reps I.x I.t j' = 1 := by
      intro j' _
      rw [(ram_dual_tuple hL I.x I.t j').1]
      have : (I.t j').sel_dual L * (1 - (I.t j').sel_dual L) = 0 := by
        rcases L with _ | _ | _ | _ | L
        · exact (I.rows j').c3
        · exact (I.rows j').c4
        · exact (I.rows j').c5
        · exact (I.rows j').c6
        · omega
      exact bit_or this
    obtain ⟨n, hn, e⟩ := W.step_of_sent hI hmd hb (lane_at_mem honce2 hj) (by rw [hr2, h1])
      (by rw [ht2]; left; rfl)
    rw [ht2] at e
    exact ⟨n, hn, e⟩

/-! ### Assembling the RAM's segments -/

/-- What a RAM instance needs beyond its rows: the trace's period, `__L1__` as
the setup fixes it, and its range checks sound. -/
structure RamInst (I : MemOnMem F) : Prop where
  cyclic : Mem.cyclic I.t
  first : (I.t 0).__L1__ = 1
  inner : ∀ i : Int, 0 < i → i < Mem.numRows → (I.t i).__L1__ = 0
  ranges : RamRanges I.x I.t
  base0 : RangeHolds Mem.ranges Mem.bus10 I.x I.t
  base1 : RangeHolds Mem.ranges Mem.bus12 I.x I.t
  end0 : RangeHolds Mem.ranges Mem.bus14 I.x I.t
  end1 : RangeHolds Mem.ranges Mem.bus16 I.x I.t

/-- The instance as `MemRam.lean` takes it. -/
def RamInst.trace {I : MemOnMem F} (h : RamInst I) : RamTrace F :=
  ⟨I.x, I.t, I.rows, h.cyclic, h.first, h.inner, h.ranges, h.base0, h.base1, h.end0, h.end1⟩

open Classical in
/-- **The instance's steps, from the memory bus.** -/
noncomputable def MemBusWorld.steps {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) : RamSteps I.x I.t where
  S i L := if h : InRows i ∧ L < 4 then
      Classical.choose (W.ram_steps hI (j := i.toNat) (by have := h.1.2; have := h.1.1; omega) h.2).1
    else 0
  D i L := if h : InRows i ∧ L < 4 ∧ (I.t i).sel_dual L ≠ 0 then
      Classical.choose ((W.ram_steps hI (j := i.toNat) (by have := h.1.2; have := h.1.1; omega) h.2.1).2
        (by rw [show ((i.toNat : Nat) : Int) = i by have := h.1.1; omega]; exact h.2.2))
    else 0
  step i L hi hL := by
    rw [dif_pos ⟨hi, hL⟩]
    have hs := Classical.choose_spec (W.ram_steps hI (j := i.toNat)
      (by have := hi.2; have := hi.1; omega) hL).1
    have hii : I.t ((i.toNat : Nat) : Int) = I.t i := by
      rw [show ((i.toNat : Nat) : Int) = i by have := hi.1; omega]
    exact ⟨hs.1, (congrArg (fun r => r.step L) hii).symm.trans hs.2⟩
  dual i L hi hL hd := by
    rw [dif_pos ⟨hi, hL, hd⟩]
    have hs := Classical.choose_spec ((W.ram_steps hI (j := i.toNat)
      (by have := hi.2; have := hi.1; omega) hL).2
        (by rw [show ((i.toNat : Nat) : Int) = i by have := hi.1; omega]; exact hd))
    have hii : I.t ((i.toNat : Nat) : Int) = I.t i := by
      rw [show ((i.toNat : Nat) : Int) = i by have := hi.1; omega]
    exact ⟨hs.1, (congrArg (fun r => r.step_dual L) hii).symm.trans hs.2⟩

/-- **The RAM segment an instance is**, its addresses from its range checks and
its steps from the memory bus. -/
noncomputable def MemBusWorld.seg {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) (h : RamInst I) : RamSeg F :=
  h.trace.toSeg (W.steps hI)

/-- A sender's integer count, among `busSenders`, is unique. -/
theorem MemBusWorld.z_eq {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2) {s : Sender F}
    (hs : s ∈ busSenders M S) {z : Int} (hz : IntCount s τ z) : W.z hop s = z := by
  have h1 := (W.z_spec hop hs).1
  have hla := Nat.lt_of_le_of_lt
    (le_foldr_of_mem (fun s : Sender F => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)) hs) W.size
  have e : ((W.z hop s : Int) : F) = ((z : Int) : F) := h1.1.symm.trans hz.1
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp e
  have b1 := h1.2
  have b2 := hz.2
  unfold goldilocks at this
  simp only at hla
  split at b1 <;> split at b2 <;> omega

/-! ### Who proves a load, who a store -/

theorem headD_ne {τ : List F} {a b : F} (h : τ.headD 0 = a) (hb : a ≠ b) : τ.headD 0 ≠ b := by
  rw [h]; exact hb

theorem one_ne_three : (1 : F) ≠ 3 := by
  intro h; have := natCast_inj49 (F := F) (x := 1) (y := 3) (by omega) (by omega)
    (by rw [Lean.Grind.Semiring.natCast_one, ← natCast_ofNat' 3]; exact h)
  omega

theorem two_ne_three : (2 : F) ≠ 3 := by
  intro h; have := natCast_inj49 (F := F) (x := 2) (y := 3) (by omega) (by omega)
    (by rw [← natCast_ofNat' 2, ← natCast_ofNat' 3]; exact h)
  omega

/-- **Who proves a load or a store.** A proving sender with a positive count
that sends a load or store tuple at one of its rows is a RAM lane's access or
dual read, or an InputData row. -/
theorem MemBusWorld.who {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : τ.headD 0 = 1 ∨ τ.headD 0 = 2)
    {s : Sender F} (hs : s ∈ busSenders M S) (hp : s.proving = true) (h1 : 1 ≤ W.z hop s)
    {j : Int} (hj : j ∈ s.at) (ht : tupleOf s.p j = τ) (hr : s.p.arg.reps s.p.x s.p.t j ≠ 0) :
    (∃ I ∈ S.mem, ∃ m : Nat, m < Mem.numRows ∧ ∃ L : Nat, L < 4 ∧
      (tupleOf (⟨ramLaneArg L, I.x, I.t⟩ : Prover F) m = τ ∨
        ((I.t m).sel_dual L ≠ 0 ∧ tupleOf (⟨ramDualArg L, I.x, I.t⟩ : Prover F) m = τ))) ∨
    (∃ I ∈ S.inputData, ∃ m : Nat, m < InputData.numRows ∧ (I.t m).sel 0 ≠ 0 ∧
      tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) m = τ) := by
  have hne3 : τ.headD 0 ≠ 3 := by
    rcases hop with h | h
    · exact headD_ne h one_ne_three
    · exact headD_ne h two_ne_three
  rcases List.mem_append.mp hs with hm | ho
  · -- Main proves only register tuples.
    exfalso
    rw [← M.pairSenders_pairs] at hm
    obtain ⟨P, hP, hs'⟩ := List.mem_flatMap.mp hm
    have hg := M.pairs_good W.ok P hP
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hs'
    rcases hs' with rfl | rfl
    · have hj' : j ∈ P.pre.at := hj
      rw [hg.shape j hj'] at ht
      rw [← ht] at hne3
      exact hne3 rfl
    · rw [hg.side_cur] at hp; exact nomatch hp
  · rcases S.mem_split ho with hr' | hi | hn
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hr'
      left
      refine ⟨I, hI, ?_⟩
      simp only [MemOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs'
      rcases hs' with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      · -- The padding counts its tuple non-positively.
        exfalso
        obtain ⟨z, hz, h0, -⟩ := intCount_pad I.rows (W.pad I hI).1 (W.pad I hI).2 τ
        rw [W.z_eq hop hs hz] at h1
        omega
      all_goals
        obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨_, I.x, I.t⟩, Mem.numRows⟩) rfl hj
        refine ⟨m, hm, ?_⟩
      · exact ⟨0, by omega, .inl ht⟩
      · exact ⟨0, by omega, .inr ⟨by simpa [Mem.bus67, Lean.Grind.Semiring.add_zero] using hr, ht⟩⟩
      · exact ⟨1, by omega, .inl ht⟩
      · exact ⟨1, by omega, .inr ⟨by simpa [Mem.bus69, Lean.Grind.Semiring.add_zero] using hr, ht⟩⟩
      · exact ⟨2, by omega, .inl ht⟩
      · exact ⟨2, by omega, .inr ⟨by simpa [Mem.bus71, Lean.Grind.Semiring.add_zero] using hr, ht⟩⟩
      · exact ⟨3, by omega, .inl ht⟩
      · exact ⟨3, by omega, .inr ⟨by simpa [Mem.bus73, Lean.Grind.Semiring.add_zero] using hr, ht⟩⟩
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hi
      right
      simp only [InputDataOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs'
      subst hs'
      obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨InputData.bus26, I.x, I.t⟩, InputData.numRows⟩) rfl hj
      exact ⟨I, hI, m, hm, by simpa [InputData.bus26, Lean.Grind.Semiring.add_zero] using hr, ht⟩
    · exfalso
      have := W.none s hn j hr
      rw [ht] at this
      rcases hop with h | h
      · exact this.1 h
      · exact this.2 h

end Correspondence
