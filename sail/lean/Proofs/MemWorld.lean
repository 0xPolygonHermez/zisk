/-
  The memory bus, around Main: the setting for deriving `MemAnswers`.

  The other AIRs on the memory bus (`MemOthers`) split five ways: the RAM's
  instances, InputData's, the Rom's, RomData's, and the rest. `Quiet` is the
  hypothesis on the rest: none of them has an instance. They are the
  precompiles, the DMA and `MemAlign` AIRs and `JumpDest`, which load and
  store only when Main calls them or accesses memory other than as an aligned
  word; the programs in scope do neither.
-/
import Proofs.MemRamBus
import Proofs.Registers
import Proofs.RegRun
import Proofs.MemMain

namespace Correspondence

open Pil.Zisk

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The RAM's instances' arguments on the memory bus. -/
def MemOthers.ramSenders (S : MemOthers F) : List (Sender F) := S.mem.flatMap MemOnMem.senders

/-- InputData's instances' arguments on the memory bus. -/
def MemOthers.inSenders (S : MemOthers F) : List (Sender F) := S.inputData.flatMap InputDataOnMem.senders

/-- The Rom's instances' arguments on the memory bus. -/
def MemOthers.romSenders (S : MemOthers F) : List (Sender F) := S.rom.flatMap RomOnMem.senders

/-- RomData's instances' arguments on the memory bus. -/
def MemOthers.rdSenders (S : MemOthers F) : List (Sender F) := S.romData.flatMap RomDataOnMem.senders

/-- Every other AIR's arguments on the memory bus. -/
def MemOthers.restSenders (S : MemOthers F) : List (Sender F) :=
  ({ S with mem := [], inputData := [], rom := [], romData := [] } : MemOthers F).senders

theorem MemOthers.mem_split (S : MemOthers F) {s : Sender F} (h : s ∈ S.senders) :
    s ∈ S.ramSenders ∨ s ∈ S.inSenders ∨ s ∈ S.romSenders ∨ s ∈ S.rdSenders ∨ s ∈ S.restSenders := by
  by_cases hr : s ∈ S.ramSenders
  · exact .inl hr
  by_cases hi : s ∈ S.inSenders
  · exact .inr (.inl hi)
  by_cases ho : s ∈ S.romSenders
  · exact .inr (.inr (.inl ho))
  by_cases hd : s ∈ S.rdSenders
  · exact .inr (.inr (.inr (.inl hd)))
  right; right; right; right
  unfold MemOthers.ramSenders at hr
  unfold MemOthers.inSenders at hi
  unfold MemOthers.romSenders at ho
  unfold MemOthers.rdSenders at hd
  simp only [MemOthers.restSenders, MemOthers.senders, List.mem_append, List.flatMap_nil,
    List.not_mem_nil, false_or, or_false] at h ⊢
  simpa [hr, hi, ho, hd] using h

/-- **No other AIR has an instance**: the hypothesis on the AIRs that load and
store on Main's behalf, or on a precompile's. -/
def MemOthers.Quiet (S : MemOthers F) : Prop := S.restSenders = []

/-- A tuple of the memory proper: a load (1), a store (2), or a ROM
initialization (4) — anything but a register tuple. -/
def MemTuple (τ : List F) : Prop := τ.headD 0 = 1 ∨ τ.headD 0 = 2 ∨ τ.headD 0 = 4

theorem headD_ne {τ : List F} {a b : F} (h : τ.headD 0 = a) (hb : a ≠ b) : τ.headD 0 ≠ b := by
  rw [h]; exact hb

theorem one_ne_three : (1 : F) ≠ 3 := by
  intro h; have := natCast_inj49 (F := F) (x := 1) (y := 3) (by omega) (by omega)
    (by rw [Lean.Grind.Semiring.natCast_one, ← natCast_ofNat' 3]; exact h)
  omega

theorem natCast_ne12 : (1 : F) ≠ 2 := by
  intro h; have := natCast_inj49 (F := F) (x := 1) (y := 2) (by omega) (by omega)
    (by rw [Lean.Grind.Semiring.natCast_one, ← natCast_ofNat' 2]; exact h)
  omega

theorem two_ne_three : (2 : F) ≠ 3 := by
  intro h; have := natCast_inj49 (F := F) (x := 2) (y := 3) (by omega) (by omega)
    (by rw [← natCast_ofNat' 2, ← natCast_ofNat' 3]; exact h)
  omega

theorem four_ne_three : (4 : F) ≠ 3 := by
  intro h; have := natCast_inj49 (F := F) (x := 4) (y := 3) (by omega) (by omega)
    (by rw [← natCast_ofNat' 4, ← natCast_ofNat' 3]; exact h)
  omega

theorem one_ne_four : (1 : F) ≠ 4 := by
  intro h; have := natCast_inj49 (F := F) (x := 1) (y := 4) (by omega) (by omega)
    (by rw [Lean.Grind.Semiring.natCast_one, ← natCast_ofNat' 4]; exact h)
  omega

theorem two_ne_four : (2 : F) ≠ 4 := by
  intro h; have := natCast_inj49 (F := F) (x := 2) (y := 4) (by omega) (by omega)
    (by rw [← natCast_ofNat' 2, ← natCast_ofNat' 4]; exact h)
  omega

/-- A tuple of the memory proper is no register tuple. -/
theorem MemTuple.ne3 {τ : List F} (h : MemTuple τ) : τ.headD 0 ≠ 3 := by
  rcases h with h | h | h
  · exact headD_ne h one_ne_three
  · exact headD_ne h two_ne_three
  · exact headD_ne h four_ne_three

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

/-- **The Rom's counts**: it assumes, and counts a tuple as how many of its
data rows send it. -/
theorem rom_counts (I : RomOnMem F) (τ : List F) :
    ∀ s ∈ I.senders, s.proving = false ∧ ∃ z, IntCount s τ z ∧ 0 ≤ z := by
  intro s hs
  simp only [RomOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs
  have hb : ∀ j, (I.t j).is_data * (1 - (I.t j).is_data) = 0 := fun j => by
    have e := (I.rows j).c1
    simp only [Rom.c1] at e
    generalize (I.t j).is_data = d at e ⊢
    grind
  rcases hs with rfl | rfl | rfl | rfl
  all_goals exact ⟨rfl, intCount_bits _ (bits_of_bit fun j => by
    simp only [Rom.bus1, Rom.bus2, Rom.bus3, Rom.bus4, Lean.Grind.Semiring.add_zero]; exact hb j) τ⟩

/-- The tuple RomData's padding cancellation sends. -/
def rdPadTuple (x : RomData.Ctx F) : List F :=
  [1, x.RomData_segment_last_addr * 8, 3, 8, x.RomData_segment_last_value 0,
    x.RomData_segment_last_value 1]

set_option maxRecDepth 100000 in
omit [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks] in
theorem rd_range_max : ∀ d ∈ (RomData.ranges : List RangeDef), d.opid = 102 → d.max = 16777215 := by
  decide

/-- **RomData's padding's count**: `−padding_size`, below `2^24` by its range
check on the last segment and 0 on the others, on its own tuple, and 0 on any
other. -/
theorem intCount_rdPad {x : RomData.Ctx F} {t : RomData.Trace F} (hrows : ∀ i, RomData.everyRow x t i)
    (r10 : RangeHolds RomData.ranges RomData.bus10 x t) (τ : List F) :
    ∃ z, IntCount ⟨⟨RomData.bus8, x, t⟩, RomData.numRows⟩ τ z ∧ z ≤ 0 ∧ (z ≠ 0 → τ = rdPadTuple x) := by
  -- The padding size, a natural below 2^24.
  obtain ⟨P, hP, eP⟩ : ∃ P : Nat, P < 2 ^ 32 ∧ x.RomData_padding_size = ((P : Nat) : F) := by
    obtain ⟨b, hb⟩ := bool_of_idem (hrows 0).c1
    have e17 := (hrows 0).c17
    simp only [RomData.c17, RomData.Zisk_direct_gsum_e_3, hb] at e17
    cases b
    · refine ⟨0, by omega, ?_⟩
      simp only [bit, Bool.false_eq_true, if_false] at e17
      rw [Lean.Grind.Semiring.natCast_zero]
      generalize x.RomData_padding_size = q at e17 ⊢
      grind
    · obtain ⟨-, -, b', d, hb', hd, hdo, -, -, hall⟩ := r10
      have hb'' : b' = 102 := by simpa [RomData.bus10] using hb'.symm
      have hmax := rd_range_max d hd (by rw [hdo, hb''])
      obtain ⟨n, -, hn, e⟩ := hall 0 (by
          simp only [RomData.bus10, hb, bit, if_true, Lean.Grind.Semiring.add_zero]
          exact one_ne_zero') _ (List.mem_singleton_self _)
      exact ⟨n, by omega, by simpa [Lean.Grind.Semiring.add_zero] using e⟩
  -- Its count.
  have hat : (⟨⟨RomData.bus8, x, t⟩, RomData.numRows⟩ : Sender F).at = [0] := rfl
  by_cases ht : tupleOf (⟨RomData.bus8, x, t⟩ : Prover F) 0 = τ
  · refine ⟨-(P : Int), ⟨?_, ?_⟩, by omega, fun _ => ?_⟩
    · simp only [Sender.count, hat, List.map_cons, List.map_nil, List.foldr_cons, List.foldr_nil,
        if_pos ht, Lean.Grind.Semiring.add_zero]
      show -x.RomData_padding_size = _
      rw [eP, Lean.Grind.Ring.intCast_neg, Lean.Grind.Ring.intCast_natCast]
    · simp only [hat, List.length_singleton, RomData.bus8, if_true]; omega
    · rw [← ht]; simp [tupleOf, RomData.bus8, rdPadTuple, Lean.Grind.Semiring.add_zero]
  · refine ⟨0, ⟨?_, by simp⟩, Int.le_refl _, fun h => absurd rfl h⟩
    simp only [Sender.count, hat, List.map_cons, List.map_nil, List.foldr_cons, List.foldr_nil,
      if_neg ht, Lean.Grind.Semiring.add_zero, Lean.Grind.Ring.intCast_zero]

/-- **RomData's counts.** Its two lanes prove, each counting a tuple as how
many of its rows send it; its padding proves, counting its own tuple as a
non-positive integer. -/
theorem rd_counts (I : RomDataOnMem F) (r10 : RangeHolds RomData.ranges RomData.bus10 I.x I.t)
    (τ : List F) :
    ∀ s ∈ I.senders, s.proving = true ∧
      ∃ z, IntCount s τ z ∧ (0 ≤ z ∨ (z ≤ 0 ∧ (z ≠ 0 → τ = rdPadTuple I.x))) := by
  intro s hs
  simp only [RomDataOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with rfl | rfl | rfl
  · exact ⟨rfl, bits_goal _ τ _ fun j => by show (1 : F) * (1 - 1) = 0; grind⟩
  · exact ⟨rfl, bits_goal _ τ _ fun j => by show (1 : F) * (1 - 1) = 0; grind⟩
  · obtain ⟨z, hz, h0, hp⟩ := intCount_rdPad I.rows r10 τ
    exact ⟨rfl, z, hz, .inr ⟨h0, hp⟩⟩

/-- A padding cancellation's tuple: the RAM's or RomData's, the only senders
that count negatively. -/
def IsPad (S : MemOthers F) (τ : List F) : Prop :=
  (∃ I ∈ S.mem, τ = padTuple I.x) ∨ (∃ I ∈ S.romData, τ = rdPadTuple I.x)

/-! ### The memory bus around Main -/

/-- The senders on the memory bus: Main's, then every other AIR's. -/
def busSenders (M : MainSegs F) (S : MemOthers F) : List (Sender F) :=
  M.list.flatMap (fun m => mainMemSenders m.1 m.2) ++ S.senders

/-- What deriving `MemAnswers` takes of the memory bus: Main's segments as the
register chain needs them, a sound bus over them and every other AIR, global
updates that balance every tuple of the memory proper among themselves, the
RAM's and RomData's padding range checks and the RAM's last-step ones sound,
no instance of the AIRs `Quiet` rules out, and few enough rows that every
count stays far below the characteristic. -/
structure MemBusWorld (M : MainSegs F) (S : MemOthers F) (gp ga : List (List F)) : Prop where
  ok : M.Ok
  bus : SumHolds 10 (busSenders M S) gp ga
  globals : ∀ τ : List F, MemTuple τ → times gp τ = times ga τ
  pad : ∀ I ∈ S.mem, RangeHolds Mem.ranges Mem.bus58 I.x I.t ∧ RangeHolds Mem.ranges Mem.bus60 I.x I.t
  last : ∀ I ∈ S.mem, RangeHolds Mem.ranges Mem.bus54 I.x I.t ∧ RangeHolds Mem.ranges Mem.bus56 I.x I.t
  rdPad : ∀ I ∈ S.romData, RangeHolds RomData.ranges RomData.bus10 I.x I.t
  quiet : S.Quiet
  size : ((busSenders M S).map fun s => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)).foldr
    (· + ·) 0 < 2 ^ 62

/-- **Every sender's count is an integer**: non-negative on the assuming side,
and negative only for a padding cancellation, on its own tuple. -/
theorem MemBusWorld.counts {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (τ : List F) :
    ∀ s ∈ busSenders M S, ∃ z, IntCount s τ z ∧ (s.proving = false → 0 ≤ z) ∧
      (z < 0 → s.proving = true ∧ IsPad S τ) := by
  intro s hs
  rcases List.mem_append.mp hs with hm | ho
  · -- Main's: bits.
    rw [← M.pairSenders_pairs] at hm
    obtain ⟨z, hz, h0⟩ := intCount_bits s (pairSenders_bits M.pairs (M.pairs_good W.ok) s hm) τ
    exact ⟨z, hz, fun _ => h0, fun h => absurd h0 (by omega)⟩
  · rcases S.mem_split ho with hr | hi | hro | hrd | hn
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hr
      obtain ⟨hp, z, hz, hz'⟩ := ram_counts I (W.pad I hI).1 (W.pad I hI).2 τ s hs'
      refine ⟨z, hz, fun h => (by rw [hp] at h; exact nomatch h), fun hneg => ⟨hp, .inl ⟨I, hI, ?_⟩⟩⟩
      rcases hz' with h | ⟨-, h⟩
      · omega
      · exact h (by omega)
    · obtain ⟨I, -, hs'⟩ := List.mem_flatMap.mp hi
      obtain ⟨hp, z, hz, h0⟩ := input_counts I τ s hs'
      exact ⟨z, hz, fun _ => h0, fun h => absurd h0 (by omega)⟩
    · obtain ⟨I, -, hs'⟩ := List.mem_flatMap.mp hro
      obtain ⟨hp, z, hz, h0⟩ := rom_counts I τ s hs'
      exact ⟨z, hz, fun _ => h0, fun h => absurd h0 (by omega)⟩
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hrd
      obtain ⟨hp, z, hz, hz'⟩ := rd_counts I (W.rdPad I hI) τ s hs'
      refine ⟨z, hz, fun h => (by rw [hp] at h; exact nomatch h), fun hneg => ⟨hp, .inr ⟨I, hI, ?_⟩⟩⟩
      rcases hz' with h | ⟨-, h⟩
      · omega
      · exact h (by omega)
    · exfalso
      have := W.quiet
      unfold MemOthers.Quiet at this
      rw [this] at hn
      exact nomatch hn

open Classical in
/-- The integer count `MemBusWorld.counts` gives each sender, on `τ`. -/
noncomputable def MemBusWorld.z {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (τ : List F) (s : Sender F) : Int :=
  if h : s ∈ busSenders M S then Classical.choose (W.counts τ s h) else 0

theorem MemBusWorld.z_spec {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (τ : List F) {s : Sender F} (hs : s ∈ busSenders M S) :
    IntCount s τ (W.z τ s) ∧ (s.proving = false → 0 ≤ W.z τ s) ∧
      (W.z τ s < 0 → s.proving = true ∧ IsPad S τ) := by
  unfold MemBusWorld.z
  rw [dif_pos hs]
  exact Classical.choose_spec (W.counts τ s hs)

theorem MemBusWorld.bound {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (τ : List F) :
    (((busSenders M S).map fun s => (W.z τ s).natAbs).foldr (· + ·) 0) < 2 ^ 62 := by
  have hle : ((busSenders M S).map fun s => (W.z τ s).natAbs).foldr (· + ·) 0 ≤
      ((busSenders M S).map fun s => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)).foldr
        (· + ·) 0 := by
    have key : ∀ l : List (Sender F), (∀ s ∈ l, s ∈ busSenders M S) →
        (l.map fun s => (W.z τ s).natAbs).foldr (· + ·) 0 ≤
          (l.map fun s => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)).foldr (· + ·) 0 := by
      intro l hl
      induction l with
      | nil => simp
      | cons s l ih =>
        simp only [List.map_cons, List.foldr_cons]
        have := (W.z_spec τ (hl s (List.mem_cons_self ..))).1.2
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
    (W : MemBusWorld M S gp ga) {τ : List F} {s : Sender F}
    (hs : s ∈ busSenders M S)
    (hb : ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    {j : Int} (hj : j ∈ s.at) (ht : tupleOf s.p j = τ) (hr : s.p.arg.reps s.p.x s.p.t j = 1) :
    1 ≤ W.z τ s := by
  obtain ⟨e, hle⟩ := s.count_of_bits hb τ
  have hz := (W.z_spec τ hs).1
  have hsent : τ ∈ s.sent := by
    unfold Sender.sent
    exact List.mem_filterMap.mpr ⟨j, hj, by rw [if_pos hr, ht]⟩
  have hpos := List.count_pos_iff.mpr hsent
  rw [hz.1] at e
  have hza := Nat.lt_of_le_of_lt (le_foldr_of_mem (fun s => (W.z τ s).natAbs) hs) (W.bound τ)
  have hla := Nat.lt_of_le_of_lt
    (le_foldr_of_mem (fun s : Sender F => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)) hs) W.size
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp e
  unfold goldilocks at this
  simp only at hza hla
  split at hla <;> omega

/-- **Assumed tuples are proved.** A tuple of the memory proper some assuming
sender of bits sends with count 1 is sent by some proving sender, at one of its
rows, with a nonzero count. -/
theorem MemBusWorld.proved {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : MemTuple τ) {s₀ : Sender F}
    (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = false)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = τ) (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    ∃ s ∈ busSenders M S, s.proving = true ∧ 1 ≤ W.z τ s ∧
      ∃ j ∈ s.at, tupleOf s.p j = τ ∧ s.p.arg.reps s.p.x s.p.t j ≠ 0 := by
  obtain ⟨s, hs, hp, h1⟩ := W.bus.proved_of_assumed (W.globals τ hop) (W.z τ)
    (fun s hs => (W.z_spec τ hs).1.1) (W.bound τ) (fun s hs => (W.z_spec τ hs).2.1)
    hs₀ hside (W.z_pos hs₀ hb hj ht hr)
  have hza := Nat.lt_of_le_of_lt (le_foldr_of_mem (fun s => (W.z τ s).natAbs) hs) (W.bound τ)
  exact ⟨s, hs, hp, h1, s.sends_of_count (W.z_spec τ hs).1.1 h1 (by simp only at hza; omega)⟩

/-- **Proved tuples are assumed.** A tuple of the memory proper, not a padding
tuple, that some proving sender of bits sends with count 1 is sent by some
assuming sender, at one of its rows, with a nonzero count. -/
theorem MemBusWorld.assumed {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : MemTuple τ) (hnp : ¬ IsPad S τ) {s₀ : Sender F}
    (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = true)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = τ) (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    ∃ s ∈ busSenders M S, s.proving = false ∧
      ∃ j ∈ s.at, tupleOf s.p j = τ ∧ s.p.arg.reps s.p.x s.p.t j ≠ 0 := by
  have hpos : ∀ s ∈ busSenders M S, s.proving = true → 0 ≤ W.z τ s := by
    intro s hs _
    apply Classical.byContradiction
    intro hneg
    exact hnp ((W.z_spec τ hs).2.2 (by omega)).2
  obtain ⟨s, hs, hp, h1⟩ := W.bus.assumed_of_proved (W.globals τ hop) (W.z τ)
    (fun s hs => (W.z_spec τ hs).1.1) (W.bound τ) hpos hs₀ hside (W.z_pos hs₀ hb hj ht hr)
  have hza := Nat.lt_of_le_of_lt (le_foldr_of_mem (fun s => (W.z τ s).natAbs) hs) (W.bound τ)
  exact ⟨s, hs, hp, s.sends_of_count (W.z_spec τ hs).1.1 h1 (by simp only at hza; omega)⟩

/-! ### The steps the RAM's tuples carry -/

/-- A load or store Main assumes carries a natural step below `2^48`: a lane
access's memory step. -/
theorem MemBusWorld.main_step {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {s : Sender F}
    (hs : s ∈ M.list.flatMap (fun m => mainMemSenders m.1 m.2)) (hside : s.proving = false)
    {j : Int} (hj : j ∈ s.at) {τ : List F} (ht : tupleOf s.p j = τ) (hop : MemTuple τ) :
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
    exact absurd e hop.ne3

theorem MemOthers.ram_sub (S : MemOthers F) {s : Sender F} (h : s ∈ S.ramSenders) : s ∈ S.senders := by
  unfold MemOthers.ramSenders at h
  simp only [MemOthers.senders, List.mem_append, h, true_or, or_true]

theorem MemOthers.in_sub (S : MemOthers F) {s : Sender F} (h : s ∈ S.inSenders) : s ∈ S.senders := by
  unfold MemOthers.inSenders at h
  simp only [MemOthers.senders, List.mem_append, h, true_or, or_true]

theorem MemOthers.rom_sub (S : MemOthers F) {s : Sender F} (h : s ∈ S.romSenders) : s ∈ S.senders := by
  unfold MemOthers.romSenders at h
  simp only [MemOthers.senders, List.mem_append, h, true_or, or_true]

theorem MemOthers.rd_sub (S : MemOthers F) {s : Sender F} (h : s ∈ S.rdSenders) : s ∈ S.senders := by
  unfold MemOthers.rdSenders at h
  simp only [MemOthers.senders, List.mem_append, h, true_or, or_true]

/-- A Rom data row's tuple: its `flags` as the op, at step 3, width 8; its
count, `is_data`. -/
theorem rom_tuple (I : RomOnMem F) {s : Sender F} (hs : s ∈ I.senders) (j : Int) :
    s.p.arg.reps s.p.x s.p.t j = (I.t j).is_data ∧
      el 0 (tupleOf s.p j) = (I.t j).flags ∧ el 2 (tupleOf s.p j) = 3 := by
  simp only [RomOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with rfl | rfl | rfl | rfl
  all_goals exact ⟨by simp [Rom.bus1, Rom.bus2, Rom.bus3, Rom.bus4, Lean.Grind.Semiring.add_zero],
    by simp [tupleOf, el, Rom.bus1, Rom.bus2, Rom.bus3, Rom.bus4, Lean.Grind.Semiring.add_zero],
    by simp [tupleOf, el, Rom.bus1, Rom.bus2, Rom.bus3, Rom.bus4]⟩

/-- **A word the Rom's data writes to the RAM**: a Rom data row sends the
RAM's initialization tuple `(2, X, 3, 8, v)`. -/
def RamWord (S : MemOthers F) (X : F) (v : F × F) : Prop :=
  ∃ I ∈ S.rom, ∃ s ∈ I.senders, ∃ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j ≠ 0 ∧
    tupleOf s.p j = [2, X, 3, 8, v.1, v.2]

theorem lane_sender_mem (I : MemOnMem F) {L : Nat} (hL : L < 4) :
    (⟨⟨ramLaneArg L, I.x, I.t⟩, Mem.numRows⟩ : Sender F) ∈ I.senders ∧
      (⟨⟨ramDualArg L, I.x, I.t⟩, Mem.numRows⟩ : Sender F) ∈ I.senders := by
  rcases L with _ | _ | _ | _ | L
  all_goals first | omega | simp [MemOnMem.senders, ramLaneArg, ramDualArg]

theorem MemBusWorld.ram_mem {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (_W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) {s : Sender F}
    (hs : s ∈ I.senders) : s ∈ busSenders M S :=
  List.mem_append_right _ (S.ram_sub (List.mem_flatMap.mpr ⟨I, hI, hs⟩))

/-- A tuple of the memory proper, sent by a sender of bits that proves, with
count 1, and not a padding tuple, carries a natural step below `2^48`: some
Main access assumes it, or a Rom data row, at step 3. -/
theorem MemBusWorld.proved_step {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : MemTuple τ) (hnp : ¬ IsPad S τ) {s₀ : Sender F}
    (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = true)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = τ) (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    ∃ n : Nat, n < 2 ^ 48 ∧ el 2 τ = ((n : Nat) : F) := by
  obtain ⟨s, hs, hp, j', hj', ht', hr'⟩ := W.assumed hop hnp hs₀ hside hb hj ht hr
  rcases List.mem_append.mp hs with hm | ho
  · exact W.main_step hm hp hj' ht' hop
  · rcases S.mem_split ho with hr | hi | hro | hrd | hn
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hr
      have := (ram_counts I (W.pad I hI).1 (W.pad I hI).2 τ s hs').1
      rw [hp] at this; exact nomatch this
    · obtain ⟨I, -, hs'⟩ := List.mem_flatMap.mp hi
      have := (input_counts I τ s hs').1
      rw [hp] at this; exact nomatch this
    · obtain ⟨I, -, hs'⟩ := List.mem_flatMap.mp hro
      refine ⟨3, by omega, ?_⟩
      rw [← ht', (rom_tuple I hs' j').2.2]
      exact natCast_ofNat' (F := F) 3
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hrd
      have := (rd_counts I (W.rdPad I hI) τ s hs').1
      rw [hp] at this; exact nomatch this
    · exfalso
      have := W.quiet
      unfold MemOthers.Quiet at this
      rw [this] at hn
      exact nomatch hn

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
dual read's: some Main access or Rom data row assumes its tuple, or it is a
padding tuple, whose step is a RAM segment's last step, or 3. -/
theorem MemBusWorld.step_of_sent {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {I : MemOnMem F} (hI : I ∈ S.mem) {s : Sender F} (hs : s ∈ I.senders)
    (hb : ∀ j ∈ s.at, s.p.arg.reps s.p.x s.p.t j = 0 ∨ s.p.arg.reps s.p.x s.p.t j = 1)
    {j : Int} (hj : j ∈ s.at) (hr : s.p.arg.reps s.p.x s.p.t j = 1) (hop : MemTuple (tupleOf s.p j)) :
    ∃ n : Nat, n < 2 ^ 48 ∧ el 2 (tupleOf s.p j) = ((n : Nat) : F) := by
  by_cases hpad : IsPad S (tupleOf s.p j)
  · rcases hpad with ⟨I', hI', he⟩ | ⟨I', -, he⟩
    · obtain ⟨n, hn, e⟩ := W.last_step hI'
      refine ⟨n, by omega, ?_⟩
      rw [he]; exact e
    · refine ⟨3, by omega, ?_⟩
      rw [he]; exact natCast_ofNat' (F := F) 3
  · exact W.proved_step hop hpad (W.ram_mem hI hs) (ram_counts I (W.pad I hI).1 (W.pad I hI).2 [] s hs).1
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
    have hop : MemTuple (tupleOf (⟨ramLaneArg L, I.x, I.t⟩ : Prover F) j) := by
      unfold MemTuple
      rw [ht1]
      obtain ⟨b, hb'⟩ := bool_of_idem (ram_wr_bit (I.rows j) hL)
      cases b <;> simp only [List.headD_cons, hb', bit, Bool.false_eq_true, if_false, if_true]
      · left; grind
      · right; left; grind
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
    (W : MemBusWorld M S gp ga) {τ : List F} {s : Sender F}
    (hs : s ∈ busSenders M S) {z : Int} (hz : IntCount s τ z) : W.z τ s = z := by
  have h1 := (W.z_spec τ hs).1
  have hla := Nat.lt_of_le_of_lt
    (le_foldr_of_mem (fun s : Sender F => s.at.length + (if s.p.arg.once then 2 ^ 32 else 0)) hs) W.size
  have e : ((W.z τ s : Int) : F) = ((z : Int) : F) := h1.1.symm.trans hz.1
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp e
  have b1 := h1.2
  have b2 := hz.2
  unfold goldilocks at this
  simp only at hla
  split at b1 <;> split at b2 <;> omega

/-! ### Who proves a load, who a store -/

/-- **Who proves a tuple of the memory proper.** A proving sender with a
positive count that sends one at one of its rows is a RAM lane's access or dual
read, an InputData row, or a RomData lane. -/
theorem MemBusWorld.who {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {τ : List F} (hop : MemTuple τ)
    {s : Sender F} (hs : s ∈ busSenders M S) (hp : s.proving = true) (h1 : 1 ≤ W.z τ s)
    {j : Int} (hj : j ∈ s.at) (ht : tupleOf s.p j = τ) (hr : s.p.arg.reps s.p.x s.p.t j ≠ 0) :
    (∃ I ∈ S.mem, ∃ m : Nat, m < Mem.numRows ∧ ∃ L : Nat, L < 4 ∧
      (tupleOf (⟨ramLaneArg L, I.x, I.t⟩ : Prover F) m = τ ∨
        ((I.t m).sel_dual L ≠ 0 ∧ tupleOf (⟨ramDualArg L, I.x, I.t⟩ : Prover F) m = τ))) ∨
    (∃ I ∈ S.inputData, ∃ m : Nat, m < InputData.numRows ∧ (I.t m).sel 0 ≠ 0 ∧
      tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) m = τ) ∨
    (∃ I ∈ S.romData, ∃ m : Nat, m < RomData.numRows ∧
      (tupleOf (⟨RomData.bus4, I.x, I.t⟩ : Prover F) m = τ ∨
        tupleOf (⟨RomData.bus5, I.x, I.t⟩ : Prover F) m = τ)) := by
  have hne3 := hop.ne3
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
  · rcases S.mem_split ho with hr' | hi | hro | hrd | hn
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hr'
      left
      refine ⟨I, hI, ?_⟩
      simp only [MemOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs'
      rcases hs' with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl
      · -- The padding counts its tuple non-positively.
        exfalso
        obtain ⟨z, hz, h0, -⟩ := intCount_pad I.rows (W.pad I hI).1 (W.pad I hI).2 τ
        rw [W.z_eq hs hz] at h1
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
      right; left
      simp only [InputDataOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs'
      subst hs'
      obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨InputData.bus26, I.x, I.t⟩, InputData.numRows⟩) rfl hj
      exact ⟨I, hI, m, hm, by simpa [InputData.bus26, Lean.Grind.Semiring.add_zero] using hr, ht⟩
    · -- The Rom assumes.
      exfalso
      obtain ⟨I, -, hs'⟩ := List.mem_flatMap.mp hro
      rw [(rom_counts I τ s hs').1] at hp
      exact nomatch hp
    · obtain ⟨I, hI, hs'⟩ := List.mem_flatMap.mp hrd
      right; right
      refine ⟨I, hI, ?_⟩
      simp only [RomDataOnMem.senders, List.mem_cons, List.not_mem_nil, or_false] at hs'
      rcases hs' with rfl | rfl | rfl
      · obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨RomData.bus4, I.x, I.t⟩, RomData.numRows⟩) rfl hj
        exact ⟨m, hm, .inl ht⟩
      · obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨RomData.bus5, I.x, I.t⟩, RomData.numRows⟩) rfl hj
        exact ⟨m, hm, .inr ht⟩
      · -- The padding counts its tuple non-positively.
        exfalso
        obtain ⟨z, hz, h0, -⟩ := intCount_rdPad I.rows (W.rdPad I hI) τ
        rw [W.z_eq hs hz] at h1
        omega
    · exfalso
      have := W.quiet
      unfold MemOthers.Quiet at this
      rw [this] at hn
      exact nomatch hn

/-! ### Main's loads as reads of the log -/

/-- **The RAM's instances**, with their fixed columns and range checks
(`RamInst`), in an order that lists every one: the order the continuation bus
chains them in. -/
structure RamSegs (S : MemOthers F) where
  inst : ∀ I ∈ S.mem, RamInst I
  order : List {I // I ∈ S.mem}
  all : ∀ I, I ∈ order

/-- The RAM's segments, in the order `hram` lists them. -/
noncomputable def MemBusWorld.segs {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : RamSegs S) : List (RamSeg F) :=
  hram.order.map fun I => W.seg I.2 (hram.inst I.1 I.2)

theorem lanes_mem {x : Mem.Ctx F} {t : Mem.Trace F} (N : RamNat x t) {m : Nat} (hm : m < Mem.numRows)
    {L : Nat} (hL : L < 4) : N.lane (m : Int) L ∈ N.lanes := by
  unfold RamNat.lanes
  refine List.mem_map.mpr ⟨4 * m + L, List.mem_range.mpr (by omega), ?_⟩
  rw [show (4 * m + L) / 4 = m by omega, show (4 * m + L) % 4 = L by omega]

/-- A lane's reads and writes are the log's. -/
theorem MemBusWorld.emit_mem {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : RamSegs S) {I : MemOnMem F} (hI : I ∈ S.mem)
    {m : Nat} (hm : m < Mem.numRows) {L : Nat} (hL : L < 4) :
    (∀ e ∈ ((W.seg hI (hram.inst I hI)).nat.lane (m : Int) L).reads, e ∈ MemSort.reads (ramLog (W.segs hram))) ∧
      (∀ e ∈ ((W.seg hI (hram.inst I hI)).nat.lane (m : Int) L).writes,
        e ∈ MemSort.writes (ramLog (W.segs hram))) := by
  have hseg : W.seg hI (hram.inst I hI) ∈ W.segs hram :=
    List.mem_map.mpr ⟨⟨I, hI⟩, hram.all _, rfl⟩
  have hlane := lanes_mem (W.seg hI (hram.inst I hI)).nat hm hL
  have hlog : (W.seg hI (hram.inst I hI)).nat.lane (m : Int) L ∈ ramLog (W.segs hram) :=
    List.mem_flatMap.mpr ⟨_, hseg, hlane⟩
  exact ⟨fun e he => List.mem_flatMap.mpr ⟨_, hlog, he⟩, fun e he => List.mem_flatMap.mpr ⟨_, hlog, he⟩⟩

/-- RomData's op: 1, a read, or 4, where its address changes. -/
theorem rd_op (I : RomDataOnMem F) (m : Int) :
    ((tupleOf (⟨RomData.bus4, I.x, I.t⟩ : Prover F) m).headD 0 = 1 ∨
      (tupleOf (⟨RomData.bus4, I.x, I.t⟩ : Prover F) m).headD 0 = 4) ∧
    ((tupleOf (⟨RomData.bus5, I.x, I.t⟩ : Prover F) m).headD 0 = 1 ∨
      (tupleOf (⟨RomData.bus5, I.x, I.t⟩ : Prover F) m).headD 0 = 4) := by
  obtain ⟨b0, h0⟩ := bool_of_idem (I.rows m).c4
  obtain ⟨b1, h1⟩ := bool_of_idem (I.rows m).c6
  simp only [tupleOf, RomData.bus4, RomData.bus5, List.map_cons, List.headD_cons, h0, h1]
  refine ⟨?_, ?_⟩
  · cases b0 <;> simp only [bit, Bool.false_eq_true, if_false, if_true]
    · left; grind
    · right; grind
  · cases b1 <;> simp only [bit, Bool.false_eq_true, if_false, if_true]
    · left; grind
    · right; grind

/-- **A load Main assumes is a read of the RAM's log, an InputData row, or a
RomData lane.** -/
theorem MemBusWorld.load_read {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : RamSegs S) {X st v0 v1 : F}
    {s₀ : Sender F} (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = false)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = [1, X, st, 8, v0, v1])
    (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    (∃ e ∈ MemSort.reads (ramLog (W.segs hram)), e.val = (v0, v1) ∧
      X = ((8 * e.addr : Nat) : F) ∧ st = ((e.step : Nat) : F)) ∨
    (∃ I ∈ S.inputData, ∃ m : Nat, m < InputData.numRows ∧ (I.t m).sel 0 ≠ 0 ∧
      tupleOf (⟨InputData.bus26, I.x, I.t⟩ : Prover F) m = [1, X, st, 8, v0, v1]) ∨
    (∃ I ∈ S.romData, ∃ m : Nat, m < RomData.numRows ∧
      (tupleOf (⟨RomData.bus4, I.x, I.t⟩ : Prover F) m = [1, X, st, 8, v0, v1] ∨
        tupleOf (⟨RomData.bus5, I.x, I.t⟩ : Prover F) m = [1, X, st, 8, v0, v1])) := by
  have hop : MemTuple ([1, X, st, 8, v0, v1] : List F) := .inl rfl
  obtain ⟨s, hs, hp, h1, j', hj', ht', hr'⟩ := W.proved hop hs₀ hside hb hj ht hr
  rcases W.who hop hs hp h1 hj' ht' hr' with ⟨I, hI, m, hm, L, hL, h | ⟨hsel, h⟩⟩ | hin | hrd
  · left
    let N := (W.seg hI (hram.inst I hI)).nat
    obtain ⟨he, hX, hst⟩ := N.read_emit hm hL (ram_wr_bit (I.rows m) hL) (by exact h)
    exact ⟨_, (W.emit_mem hram hI hm hL).1 _ he, rfl, hX, hst⟩
  · left
    let N := (W.seg hI (hram.inst I hI)).nat
    have hbit : (I.t m).sel_dual L * (1 - (I.t m).sel_dual L) = 0 := by
      rcases L with _ | _ | _ | _ | L
      · exact (I.rows m).c3
      · exact (I.rows m).c4
      · exact (I.rows m).c5
      · exact (I.rows m).c6
      · omega
    obtain ⟨he, hX, hst⟩ := N.dual_emit hm hL hbit hsel (by exact h)
    exact ⟨_, (W.emit_mem hram hI hm hL).1 _ he, rfl, hX, hst⟩
  · right; left; exact hin
  · right; right; exact hrd

/-- **A store Main assumes is a write of the RAM's log.** -/
theorem MemBusWorld.store_write {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : RamSegs S) {X st v0 v1 : F}
    {s₀ : Sender F} (hs₀ : s₀ ∈ busSenders M S) (hside : s₀.proving = false)
    (hb : ∀ j ∈ s₀.at, s₀.p.arg.reps s₀.p.x s₀.p.t j = 0 ∨ s₀.p.arg.reps s₀.p.x s₀.p.t j = 1)
    {j : Int} (hj : j ∈ s₀.at) (ht : tupleOf s₀.p j = [2, X, st, 8, v0, v1])
    (hr : s₀.p.arg.reps s₀.p.x s₀.p.t j = 1) :
    ∃ e ∈ MemSort.writes (ramLog (W.segs hram)), e.val = (v0, v1) ∧
      X = ((8 * e.addr : Nat) : F) ∧ st = ((e.step : Nat) : F) := by
  have hop : MemTuple ([2, X, st, 8, v0, v1] : List F) := .inr (.inl rfl)
  have h12 : (1 : F) ≠ 2 := natCast_ne12
  obtain ⟨s, hs, hp, h1, j', hj', ht', hr'⟩ := W.proved hop hs₀ hside hb hj ht hr
  rcases W.who hop hs hp h1 hj' ht' hr' with ⟨I, hI, m, hm, L, hL, h | ⟨-, h⟩⟩ | ⟨I, -, m, -, -, h⟩ |
      ⟨I, -, m, -, h | h⟩
  · let N := (W.seg hI (hram.inst I hI)).nat
    obtain ⟨he, hX, hst⟩ := N.write_emit hm hL (ram_wr_bit (I.rows m) hL) (by exact h)
    exact ⟨_, (W.emit_mem hram hI hm hL).2 _ he, rfl, hX, hst⟩
  · -- A dual read sends op 1.
    rw [(ram_dual_tuple hL I.x I.t m).2] at h
    simp only [List.cons.injEq] at h
    exact absurd h.1 h12
  · -- InputData sends op 1.
    simp only [tupleOf, InputData.bus26, List.map_cons, List.map_nil, List.cons.injEq] at h
    exact absurd h.1 h12
  · -- RomData sends op 1 or 4.
    have e := congrArg (·.headD 0) h
    simp only [List.headD_cons] at e
    rcases (rd_op I m).1 with h' | h' <;> rw [h'] at e
    · exact absurd e h12
    · exact absurd e.symm two_ne_four
  · have e := congrArg (·.headD 0) h
    simp only [List.headD_cons] at e
    rcases (rd_op I m).2 with h' | h' <;> rw [h'] at e
    · exact absurd e h12
    · exact absurd e.symm two_ne_four

/-! ### Every RAM write is a Main store or the Rom's data -/

/-- **A Main access assuming a store is a store lane.** A Main sender that
assumes a tuple with op 2, at a row with a nonzero count, is the store access
of lane `l` of row `m` of segment `k`, which stores to memory. -/
theorem MemBusWorld.main_store {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {s : Sender F}
    (hs : s ∈ M.list.flatMap (fun m => mainMemSenders m.1 m.2)) (hside : s.proving = false)
    {j : Int} (hj : j ∈ s.at) (hr : s.p.arg.reps s.p.x s.p.t j ≠ 0) (hop : (tupleOf s.p j).headD 0 = 2) :
    ∃ k < M.K, ∃ m < Main.numRows, ∃ l < 4,
      s = ⟨⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩, Main.numRows⟩ ∧ j = (m : Int) ∧
      StoresAt M k m l := by
  rw [← M.pairSenders_pairs] at hs
  obtain ⟨P, hP, hs⟩ := List.mem_flatMap.mp hs
  have hg := M.pairs_good W.ok P hP
  obtain ⟨k, hk, g, hgm, rfl⟩ := M.mem_pairs hP
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hs
  rcases hs with rfl | rfl
  · rw [hg.side_pre] at hside; exact nomatch hside
  have hj' : j ∈ (g.pair k (M.seg k).1 (M.seg k).2).pre.at := by rw [← hg.same]; exact hj
  have hrc := hg.bits_cur j hj'
  have hrp := hg.bits_pre j hj'
  have hr1 : (g.pair k (M.seg k).1 (M.seg k).2).cur.p.arg.reps (M.seg k).1 (M.seg k).2 j = 1 := by
    rcases hrc with h | h
    · exact absurd h hr
    · exact h
  rw [← el_zero_headD] at hop
  have hopk := (mainTags_op (F := F)).mem g hgm
  have hcur := (mainTags_cur (F := F)).mem g hgm
  have honce := (mainTags_once (F := F)).mem g hgm
  have hwf := mainTags_wf _ hgm
  cases g with
  | flush f ρ =>
    exfalso
    have e := hcur (M.seg k).1 (M.seg k).2 j
    have hop' : el 0 (tupleOf ⟨(PairTag.flush f ρ).curArg, (M.seg k).1, (M.seg k).2⟩ j) = 2 := hop
    rw [e] at hop'
    exact two_ne_three hop'.symm
  | lane l κ =>
    simp only [PairTag.Wf] at hwf
    obtain ⟨m, hm, rfl⟩ := mem_at_rows (s := ⟨⟨(PairTag.lane l κ).preArg, (M.seg k).1, (M.seg k).2⟩,
      Main.numRows⟩) honce hj'
    have e := hopk (M.seg k).1 (M.seg k).2 m
    have hop' : el 0 (tupleOf ⟨(PairTag.lane l κ).curArg, (M.seg k).1, (M.seg k).2⟩ m) = 2 := hop
    change ((PairTag.lane l κ).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 1 at hr1
    change ((PairTag.lane l κ).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 0 ∨
      ((PairTag.lane l κ).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 1 at hrp
    rw [e, hr1] at hop'
    by_cases hκ : κ = 2
    · subst hκ
      simp only [if_true] at hop'
      have hreg : ((PairTag.lane l 2).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 0 := by
        rcases hrp with h | h
        · exact h
        · exfalso; rw [h] at hop'
          have : (1 : F) = 0 := by
            generalize ((PairTag.lane l 2).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = r at *
            grind
          exact Lean.Grind.Field.zero_ne_one this.symm
      refine ⟨k, hk, m, hm, l, by omega, rfl, rfl, ?_⟩
      obtain ⟨-, -, -, -, -, -, -, -, c9, -, c11, -⟩ := lane_cols (F := F) (l := l) (by omega) (M.seg k).1 (M.seg k).2 m
      rw [c9, Lean.Grind.Semiring.add_zero] at hreg
      rw [c11, hreg, Lean.Grind.Semiring.add_zero] at hr1
      exact hr1
    · exfalso
      simp only [hκ, if_false] at hop'
      rcases hrp with h | h <;> rw [h] at hop'
      · have : (1 : F) = 2 := by generalize (2 : F) = r at *; grind
        exact (natCast_ne12 this)
      · have : (3 : F) = 2 := by grind
        exact two_ne_three this.symm

open Classical in
/-- **Every write of the RAM's log is a Main store, or the Rom's data**: some
sender assumes its tuple, a Main store access or a Rom data row, whose step
is 3. -/
theorem MemBusWorld.write_is_store {M : MainSegs F} {S : MemOthers F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (hram : RamSegs S) {w : MemSort.Emit (F × F)}
    (hw : w ∈ MemSort.writes (ramLog (W.segs hram))) :
    (∃ k < M.K, ∃ m < Main.numRows, ∃ l < 4, StoresAt M k m l ∧
      tupleOf ⟨(PairTag.lane l 2).curArg, (M.seg k).1, (M.seg k).2⟩ (m : Int) =
        [2, ((8 * w.addr : Nat) : F), ((w.step : Nat) : F), 8, w.val.1, w.val.2]) ∨
    (w.step = 3 ∧ RamWord S ((8 * w.addr : Nat) : F) w.val) := by
  -- The lane the write is.
  obtain ⟨lane, hlane, hwl⟩ := List.mem_flatMap.mp hw
  obtain ⟨seg, hseg, hls⟩ := List.mem_flatMap.mp hlane
  obtain ⟨⟨I, hI⟩, -, rfl⟩ := List.mem_map.mp hseg
  obtain ⟨n, hn, rfl⟩ := List.mem_map.mp hls
  rw [List.mem_range] at hn
  have hN : Mem.numRows = 4194304 := rfl
  have hm : n / 4 < Mem.numRows := by rw [hN] at hn ⊢; omega
  have hL : n % 4 < 4 := Nat.mod_lt _ (by omega)
  let N := (W.seg hI (hram.inst I hI)).nat
  have hwl' : w ∈ (N.lane ((n / 4 : Nat) : Int) (n % 4)).writes := hwl
  unfold MemSort.Lane.writes at hwl'
  split at hwl'
  rotate_left
  · exact absurd hwl' (List.not_mem_nil)
  rename_i hwrb
  have hwr : (I.t ((n / 4 : Nat) : Int)).wr (n % 4) = 1 := by
    simpa [RamNat.lane] using hwrb
  have hwe : w = ⟨N.A ((n / 4 : Nat) : Int) (n % 4), N.S ((n / 4 : Nat) : Int) (n % 4),
      ((I.t ((n / 4 : Nat) : Int)).value (n % 4) 0, (I.t ((n / 4 : Nat) : Int)).value (n % 4) 1)⟩ :=
    List.mem_singleton.mp hwl'
  -- Its tuple.
  have ha : (I.t ((n / 4 : Nat) : Int)).addr (n % 4) = ((N.A ((n / 4 : Nat) : Int) (n % 4) : Nat) : F) :=
    (N.addr ((n / 4 : Nat) : Int) (n % 4) (inRows_nat hm) hL).2
  have hs : (I.t ((n / 4 : Nat) : Int)).step (n % 4) = ((N.S ((n / 4 : Nat) : Int) (n % 4) : Nat) : F) :=
    (N.step ((n / 4 : Nat) : Int) (n % 4) (inRows_nat hm) hL).2
  obtain ⟨hr1, ht1⟩ := ram_lane_tuple hL I.x I.t ((n / 4 : Nat) : Int)
  have hτ : tupleOf (⟨ramLaneArg (n % 4), I.x, I.t⟩ : Prover F) ((n / 4 : Nat) : Int) =
      [2, ((8 * w.addr : Nat) : F), ((w.step : Nat) : F), 8, w.val.1, w.val.2] := by
    have e8 : ((N.A ((n / 4 : Nat) : Int) (n % 4) : Nat) : F) * 8 =
        ((8 * N.A ((n / 4 : Nat) : Int) (n % 4) : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_mul, natCast_ofNat' 8]
      exact Lean.Grind.CommSemiring.mul_comm _ _
    have e2 : (1 : F) + 1 = 2 := by grind
    rw [ht1, hwe, hwr, ha, hs, e8, e2]
  -- Someone assumes it: a Main store.
  have honce : (ramLaneArg (F := F) (n % 4)).once = false := by
    generalize n % 4 = L at hL ⊢
    rcases L with _ | _ | _ | _ | L <;> first | omega | rfl
  have hop : MemTuple ([2, ((8 * w.addr : Nat) : F), ((w.step : Nat) : F), 8, w.val.1, w.val.2] :
      List F) := .inr (.inl rfl)
  have hnp : ¬ IsPad S [2, ((8 * w.addr : Nat) : F), ((w.step : Nat) : F), 8, w.val.1, w.val.2] := by
    rintro (⟨I', -, h⟩ | ⟨I', -, h⟩)
    · simp only [padTuple, List.cons.injEq] at h
      exact natCast_ne12 h.1.symm
    · simp only [rdPadTuple, List.cons.injEq] at h
      exact natCast_ne12 h.1.symm
  obtain ⟨s, hs', hp, j', hj', ht', hr'⟩ := W.assumed hop hnp (W.ram_mem hI (lane_sender_mem I hL).1)
    (ram_counts I (W.pad I hI).1 (W.pad I hI).2 [] _ (lane_sender_mem I hL).1).1
    (fun j _ => .inr (ram_lane_tuple hL I.x I.t j).1) (lane_at_mem honce hm) hτ hr1
  rcases List.mem_append.mp hs' with hmain | ho
  · obtain ⟨k, hk, m, hm', l, hl, rfl, rfl, hst⟩ := W.main_store hmain hp hj' hr' (by rw [ht']; rfl)
    exact .inl ⟨k, hk, m, hm', l, hl, hst, ht'⟩
  · rcases S.mem_split ho with hr | hi | hro | hrd | hn
    · obtain ⟨I', hI', hs''⟩ := List.mem_flatMap.mp hr
      have := (ram_counts I' (W.pad I' hI').1 (W.pad I' hI').2 [] s hs'').1
      rw [hp] at this; exact nomatch this
    · obtain ⟨I', -, hs''⟩ := List.mem_flatMap.mp hi
      have := (input_counts I' [] s hs'').1
      rw [hp] at this; exact nomatch this
    · -- A Rom data row, at step 3.
      right
      obtain ⟨I', hI', hs''⟩ := List.mem_flatMap.mp hro
      have h2 := (rom_tuple I' hs'' j').2.2
      rw [ht'] at h2
      have e3 : ((w.step : Nat) : F) = ((3 : Nat) : F) := by
        rw [← natCast_ofNat' (F := F) 3]; exact h2
      have hb := (N.step ((n / 4 : Nat) : Int) (n % 4) (inRows_nat hm) hL).1
      have hws : w.step = N.S ((n / 4 : Nat) : Int) (n % 4) := by rw [hwe]
      have h3 : w.step = 3 := natCast_inj49 (F := F) (by omega) (by omega) e3
      refine ⟨h3, I', hI', s, hs'', j', hj', hr', ?_⟩
      rw [ht', h3, ← natCast_ofNat' (F := F) 3]
    · obtain ⟨I', hI', hs''⟩ := List.mem_flatMap.mp hrd
      have := (rd_counts I' (W.rdPad I' hI') [] s hs'').1
      rw [hp] at this; exact nomatch this
    · exfalso
      have := W.quiet
      unfold MemOthers.Quiet at this
      rw [this] at hn
      exact nomatch hn

end Correspondence
