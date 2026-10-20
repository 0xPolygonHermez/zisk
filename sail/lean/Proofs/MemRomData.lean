/-
  RomData: the program's read-only data, as Main loads it.

  RomData serves Main's loads from the ROM region. It is a read-only memory of
  two lanes a row, with no ordering of its addresses: each lane either keeps
  the address and value of the lane before (`addr_change` 0), or changes
  address (`addr_change` 1), when it sends op 4 at step 3 — the ROM's
  initialization tuple, which only the Rom's data rows assume, with the
  program's data. So every lane returns the value of some initialization at
  its address (`MemBusWorld.rd_value`): a word of the Rom's data
  (`RomWord`). Along a chain of RomData's segments (`RdWorld`), every lane
  repeats the latest lane before it that changed address (`RdWorld.origin`),
  and that lane's op-4 tuple is assumed by a Rom data row — Main assumes no
  op 4 (`MemBusWorld.main_not4`), and the RAM, InputData and RomData prove.
-/
import Proofs.MemWorld

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-! ### A row's constraints, lane by lane -/

section Rows

variable {x : RomData.Ctx F} {t : RomData.Trace F} {i : Int}

/-- Lane 1 keeps lane 0's address and value unless it changes address. -/
theorem rd_in (h : RomData.everyRow x t i) (h0 : (t i).addr_change 1 = 0) :
    (t i).addr 1 = (t i).addr 0 ∧ (t i).value 1 0 = (t i).value 0 0 ∧
      (t i).value 1 1 = (t i).value 0 1 := by
  have c14 := h.c14; have c15 := h.c15; have c16 := h.c16
  clear h
  simp only [RomData.c14, RomData.c15, RomData.c16, RomData.Zisk_direct_gsum_e_2, h0] at c14 c15 c16
  refine ⟨?_, ?_, ?_⟩ <;> grind

/-- Inside a segment, lane 0 keeps the row before's lane 1 unless it changes
address. -/
theorem rd_prev (h : RomData.everyRow x t i) (hL1 : (t i).__L1__ = 0) (h0 : (t i).addr_change 0 = 0) :
    (t i).addr 0 = (t (i - 1)).addr 1 ∧ (t i).value 0 0 = (t (i - 1)).value 1 0 ∧
      (t i).value 0 1 = (t (i - 1)).value 1 1 := by
  have c11 := h.c11; have c12 := h.c12; have c13 := h.c13
  clear h
  simp only [RomData.c11, RomData.c12, RomData.c13, RomData.Zisk_direct_gsum_e_0,
    RomData.Zisk_direct_gsum_e_1, RomData.RomData_previous_value_0_0, RomData.RomData_previous_value_0_1,
    RomData.RomData_previous_addr_0, hL1, h0] at c11 c12 c13
  refine ⟨?_, ?_, ?_⟩ <;> grind

/-- At a segment's first row, lane 0 keeps what the air values hand on unless
it changes address. -/
theorem rd_first (h : RomData.everyRow x t i) (hL1 : (t i).__L1__ = 1) (h0 : (t i).addr_change 0 = 0) :
    (t i).addr 0 = x.RomData_previous_segment_addr ∧
      (t i).value 0 0 = x.RomData_previous_segment_value 0 ∧
      (t i).value 0 1 = x.RomData_previous_segment_value 1 := by
  have c11 := h.c11; have c12 := h.c12; have c13 := h.c13
  clear h
  simp only [RomData.c11, RomData.c12, RomData.c13, RomData.Zisk_direct_gsum_e_0,
    RomData.Zisk_direct_gsum_e_1, RomData.RomData_previous_value_0_0, RomData.RomData_previous_value_0_1,
    RomData.RomData_previous_addr_0, hL1, h0] at c11 c12 c13
  generalize (t (i - 1)).addr 1 = a at c13
  generalize (t (i - 1)).value 1 0 = b at c11
  generalize (t (i - 1)).value 1 1 = c at c12
  refine ⟨?_, ?_, ?_⟩ <;> grind

/-- The row before a segment's first is its last: what the air values hand
on. -/
theorem rd_last (h : RomData.everyRow x t i) (hL1 : (t (i + 1)).__L1__ = 1) :
    x.RomData_segment_last_addr = (t i).addr 1 ∧ x.RomData_segment_last_value 0 = (t i).value 1 0 ∧
      x.RomData_segment_last_value 1 = (t i).value 1 1 := by
  have c8 := h.c8; have c9 := h.c9; have c10 := h.c10
  clear h
  simp only [RomData.c8, RomData.c9, RomData.c10, hL1] at c8 c9 c10
  refine ⟨?_, ?_, ?_⟩ <;> grind

/-- The first segment's first lane changes address. -/
theorem rd_start (h : RomData.everyRow x t i) (hf : x.RomData_is_first_segment = 1)
    (hL1 : (t i).__L1__ = 1) : (t i).addr_change 0 = 1 := by
  have c3 := h.c3
  clear h
  simp only [RomData.c3, hf, hL1] at c3
  grind

/-- Where a lane changes address, its step is 3. -/
theorem rd_step3 (h : RomData.everyRow x t i) {L : Nat} (hL : L < 2) (h1 : (t i).addr_change L = 1) :
    (t i).step L = 3 := by
  rcases L with _ | _ | L
  · have c5 := h.c5; clear h
    simp only [RomData.c5, h1] at c5
    grind
  · have c7 := h.c7; clear h
    simp only [RomData.c7, h1] at c7
    grind
  · omega

end Rows

/-! ### The chain of segments -/

/-- **RomData's instances, as a chain of segments**: every instance is one of
them and each of them an instance, with its fixed columns as the setup fixes
them, the first marked first, and each later one starting from the address and
value the one before hands on (the continuation bus). -/
structure RdWorld (S : MemOthers F) where
  seg : Nat → RomDataOnMem F
  K : Nat
  mem : ∀ k < K, seg k ∈ S.romData
  all : ∀ I ∈ S.romData, ∃ k < K, seg k = I
  cyclic : ∀ k < K, RomData.cyclic (seg k).t
  first : ∀ k < K, ((seg k).t 0).__L1__ = 1
  inner : ∀ k < K, ∀ i : Int, 0 < i → i < RomData.numRows → ((seg k).t i).__L1__ = 0
  first_seg : 0 < K → (seg 0).x.RomData_is_first_segment = 1
  next : ∀ k, k + 1 < K →
    (seg (k + 1)).x.RomData_previous_segment_addr = (seg k).x.RomData_segment_last_addr ∧
      (seg (k + 1)).x.RomData_previous_segment_value 0 = (seg k).x.RomData_segment_last_value 0 ∧
      (seg (k + 1)).x.RomData_previous_segment_value 1 = (seg k).x.RomData_segment_last_value 1

variable {S : MemOthers F}

/-- Lane `n`'s row: row `(n mod 2N) / 2` of segment `n / 2N`; its lane is
`n mod 2`. -/
def RdWorld.row (R : RdWorld S) (n : Nat) : RomData.Row F :=
  (R.seg (n / 8388608)).t (((n % 8388608) / 2 : Nat) : Int)

def RdWorld.ac (R : RdWorld S) (n : Nat) : F := (R.row n).addr_change (n % 2)
def RdWorld.addr (R : RdWorld S) (n : Nat) : F := (R.row n).addr (n % 2)
def RdWorld.val (R : RdWorld S) (n : Nat) : F × F := ((R.row n).value (n % 2) 0, (R.row n).value (n % 2) 1)

theorem RdWorld.at_lane (R : RdWorld S) {k r L : Nat} (hr : r < 4194304) (hL : L < 2) :
    R.ac (k * 8388608 + 2 * r + L) = ((R.seg k).t r).addr_change L ∧
      R.addr (k * 8388608 + 2 * r + L) = ((R.seg k).t r).addr L ∧
      R.val (k * 8388608 + 2 * r + L) = (((R.seg k).t r).value L 0, ((R.seg k).t r).value L 1) := by
  have h1 : (k * 8388608 + 2 * r + L) / 8388608 = k := by omega
  have h2 : ((k * 8388608 + 2 * r + L) % 8388608) / 2 = r := by omega
  have h3 : (k * 8388608 + 2 * r + L) % 2 = L := by omega
  refine ⟨?_, ?_, ?_⟩ <;> simp only [RdWorld.ac, RdWorld.addr, RdWorld.val, RdWorld.row, h1, h2, h3]

/-- A lane's position: its segment, row and lane. -/
theorem lane_split (n : Nat) : n = (n / 8388608) * 8388608 + 2 * ((n % 8388608) / 2) + n % 2 ∧
    (n % 8388608) / 2 < 4194304 ∧ n % 2 < 2 := by omega

/-- **One lane on.** A lane that does not change address keeps the address
and value of the lane before. -/
theorem RdWorld.step (R : RdWorld S) {n : Nat} (hn : n + 1 < R.K * 8388608) (h0 : R.ac (n + 1) = 0) :
    R.addr (n + 1) = R.addr n ∧ R.val (n + 1) = R.val n := by
  obtain ⟨e, hr, hL⟩ := lane_split (n + 1)
  generalize hk : (n + 1) / 8388608 = k at e
  generalize hr' : ((n + 1) % 8388608) / 2 = r at e hr
  generalize hL' : (n + 1) % 2 = L at e hL
  have hkK : k < R.K := by
    have : k * 8388608 < R.K * 8388608 := by omega
    exact Nat.lt_of_mul_lt_mul_right this
  have hN : RomData.numRows = 4194304 := rfl
  obtain ⟨a1, d1, v1⟩ := R.at_lane (k := k) hr hL
  rw [← e] at a1 d1 v1
  rw [a1] at h0
  rw [d1, v1]
  rcases L with _ | _ | L
  · rcases r with _ | r
    · -- Lane 0 of a segment's first row: the segment before's last lane.
      obtain ⟨k, rfl⟩ : ∃ k', k = k' + 1 := ⟨k - 1, by omega⟩
      have hn' : n = k * 8388608 + 2 * 4194303 + 1 := by omega
      obtain ⟨-, d0, v0⟩ := R.at_lane (k := k) (r := 4194303) (L := 1) (by omega) (by omega)
      rw [← hn'] at d0 v0
      rw [d0, v0]
      have hf := rd_first ((R.seg (k + 1)).rows ((0 : Nat) : Int)) (by simpa using R.first _ hkK) h0
      obtain ⟨ea, ev0, ev1⟩ := R.next k hkK
      have hwrap : ((R.seg k).t ((4194303 : Nat) + 1 : Int)).__L1__ = 1 := by
        have := R.cyclic k (by omega) 0
        rw [Int.zero_add] at this
        rw [show ((4194303 : Nat) + 1 : Int) = (RomData.numRows : Int) by rw [hN]; rfl, this]
        exact R.first k (by omega)
      have hl := rd_last ((R.seg k).rows ((4194303 : Nat) : Int)) hwrap
      refine ⟨?_, ?_⟩
      · rw [hf.1, ea, hl.1]
      · rw [hf.2.1, hf.2.2, ev0, ev1, hl.2.1, hl.2.2]
    · -- Lane 0 of a later row: the row before's lane 1.
      have hn' : n = k * 8388608 + 2 * r + 1 := by omega
      obtain ⟨-, d0, v0⟩ := R.at_lane (k := k) (r := r) (L := 1) (by omega) (by omega)
      rw [← hn'] at d0 v0
      rw [d0, v0]
      have hp := rd_prev ((R.seg k).rows ((r + 1 : Nat) : Int))
        (R.inner k hkK _ (by omega) (by rw [hN]; omega)) h0
      have hi : ((r + 1 : Nat) : Int) - 1 = (r : Int) := by omega
      rw [hi] at hp
      refine ⟨hp.1, ?_⟩
      rw [hp.2.1, hp.2.2]
  · -- Lane 1: lane 0 of its row.
    have hn' : n = k * 8388608 + 2 * r + 0 := by omega
    obtain ⟨-, d0, v0⟩ := R.at_lane (k := k) (r := r) (L := 0) hr (by omega)
    rw [← hn'] at d0 v0
    rw [d0, v0]
    have hp := rd_in ((R.seg k).rows (r : Int)) h0
    refine ⟨hp.1, ?_⟩
    rw [hp.2.1, hp.2.2]
  · omega

theorem rd_ac_bit (R : RdWorld S) (n : Nat) : R.ac n = 0 ∨ R.ac n = 1 := by
  simp only [RdWorld.ac, RdWorld.row]
  have := (R.seg (n / 8388608)).rows (((n % 8388608) / 2 : Nat) : Int)
  rcases Nat.mod_two_eq_zero_or_one n with h | h <;> rw [h]
  · exact bit_or this.c4
  · exact bit_or this.c6

/-- **Every lane repeats one that changed address**: the latest at or before
it. -/
theorem RdWorld.origin (R : RdWorld S) :
    ∀ n, n < R.K * 8388608 → ∃ n' ≤ n, R.ac n' = 1 ∧ R.addr n' = R.addr n ∧ R.val n' = R.val n
  | 0, h0 => by
    refine ⟨0, Nat.le_refl _, ?_, rfl, rfl⟩
    have hK : 0 < R.K := by omega
    obtain ⟨a, -, -⟩ := R.at_lane (k := 0) (r := 0) (L := 0) (by omega) (by omega)
    simp only [Nat.zero_mul, Nat.mul_zero, Nat.add_zero] at a
    rw [a]
    exact rd_start ((R.seg 0).rows 0) (R.first_seg hK) (R.first 0 hK)
  | n + 1, h => by
    rcases rd_ac_bit R (n + 1) with h0 | h1
    · obtain ⟨n', hn', ha, hd, hv⟩ := R.origin n (by omega)
      obtain ⟨sd, sv⟩ := R.step h h0
      exact ⟨n', by omega, ha, hd.trans sd.symm, hv.trans sv.symm⟩
    · exact ⟨n + 1, Nat.le_refl _, h1, rfl, rfl⟩

/-! ### Main assumes no op 4 -/

theorem three_ne_four : (3 : F) ≠ 4 := fun h => four_ne_three h.symm

/-- **Main assumes no op 4**: a lane access's op is 1, 2 or 3, a flush's 3. -/
theorem MemBusWorld.main_not4 {M : MainSegs F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) {s : Sender F}
    (hs : s ∈ M.list.flatMap (fun m => mainMemSenders m.1 m.2)) (hside : s.proving = false)
    {j : Int} (hj : j ∈ s.at) (hr : s.p.arg.reps s.p.x s.p.t j ≠ 0) : (tupleOf s.p j).headD 0 ≠ 4 := by
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
  rw [← el_zero_headD]
  have hopk := (mainTags_op (F := F)).mem g hgm
  have hcur := (mainTags_cur (F := F)).mem g hgm
  cases g with
  | flush f ρ =>
    have e := hcur (M.seg k).1 (M.seg k).2 j
    intro h4
    have e' : el 0 (tupleOf ⟨(PairTag.flush f ρ).curArg, (M.seg k).1, (M.seg k).2⟩ j) = 4 := h4
    rw [e] at e'
    exact three_ne_four e'
  | lane l κ =>
    have e := hopk (M.seg k).1 (M.seg k).2 j
    change ((PairTag.lane l κ).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 1 at hr1
    change ((PairTag.lane l κ).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 0 ∨
      ((PairTag.lane l κ).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 j = 1 at hrp
    intro h4
    have e' : el 0 (tupleOf ⟨(PairTag.lane l κ).curArg, (M.seg k).1, (M.seg k).2⟩ j) = 4 := h4
    rw [e, hr1] at e'
    by_cases hκ : κ = 2
    · subst hκ
      simp only [if_true] at e'
      rcases hrp with h | h <;> rw [h] at e'
      · exact two_ne_four (by rw [← e']; grind)
      · exact three_ne_four (by rw [← e']; grind)
    · simp only [hκ, if_false] at e'
      rcases hrp with h | h <;> rw [h] at e'
      · exact one_ne_four (by rw [← e']; grind)
      · exact three_ne_four (by rw [← e']; grind)

/-! ### RomData's values are the Rom's data -/

/-- **A word of the Rom's data**: a Rom data row sends the ROM's
initialization tuple `(4, X, 3, 8, v)`. -/
def RomWord (S : MemOthers F) (X : F) (v : F × F) : Prop :=
  ∃ I ∈ S.rom, ∃ s ∈ I.senders, ∃ j : Int, s.p.arg.reps s.p.x s.p.t j ≠ 0 ∧
    tupleOf s.p j = [4, X, 3, 8, v.1, v.2]

/-- RomData's lane `L`'s argument. -/
def rdArg : Nat → BusArg F (RomData.Ctx F) (RomData.Trace F)
  | 0 => RomData.bus4 | _ => RomData.bus5

theorem rd_lane_tuple {L : Nat} (hL : L < 2) (x : RomData.Ctx F) (t : RomData.Trace F) (j : Int) :
    (rdArg L).reps x t j = 1 ∧
      tupleOf ⟨rdArg L, x, t⟩ j = [(t j).addr_change L * 3 + 1, (t j).addr L * 8, (t j).step L, 8,
        (t j).value L 0, (t j).value L 1] := by
  rcases L with _ | _ | L
  all_goals first
    | omega
    | exact ⟨rfl, by simp [tupleOf, rdArg, RomData.bus4, RomData.bus5, Lean.Grind.Semiring.add_zero]⟩

theorem rd_sender_mem (I : RomDataOnMem F) {L : Nat} (hL : L < 2) :
    (⟨⟨rdArg L, I.x, I.t⟩, RomData.numRows⟩ : Sender F) ∈ I.senders := by
  rcases L with _ | _ | L
  all_goals first | omega | simp [RomDataOnMem.senders, rdArg]

/-- **A lane that changes address sends a word of the Rom's data**: its
op-4 tuple, which only a Rom data row assumes. -/
theorem MemBusWorld.rd_word {M : MainSegs F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (R : RdWorld S) {k r L : Nat} (hk : k < R.K) (hr : r < 4194304)
    (hL : L < 2) (hac : ((R.seg k).t r).addr_change L = 1) :
    RomWord S (((R.seg k).t r).addr L * 8) (((R.seg k).t r).value L 0, ((R.seg k).t r).value L 1) := by
  have hN : RomData.numRows = 4194304 := rfl
  have hI := R.mem k hk
  have hs : (⟨⟨rdArg L, (R.seg k).x, (R.seg k).t⟩, RomData.numRows⟩ : Sender F) ∈ busSenders M S :=
    List.mem_append_right _ (S.rd_sub (List.mem_flatMap.mpr ⟨_, hI, rd_sender_mem _ hL⟩))
  obtain ⟨hr1, ht⟩ := rd_lane_tuple hL (R.seg k).x (R.seg k).t r
  have h3 := rd_step3 ((R.seg k).rows r) hL hac
  have e4 : ((R.seg k).t r).addr_change L * 3 + 1 = 4 := by rw [hac]; grind
  rw [e4, h3] at ht
  have hop : MemTuple ([4, ((R.seg k).t r).addr L * 8, 3, 8, ((R.seg k).t r).value L 0,
      ((R.seg k).t r).value L 1] : List F) := .inr (.inr rfl)
  have hnp : ¬ IsPad S [4, ((R.seg k).t r).addr L * 8, 3, 8, ((R.seg k).t r).value L 0,
      ((R.seg k).t r).value L 1] := by
    rintro (⟨I', -, h⟩ | ⟨I', -, h⟩)
    · simp only [padTuple, List.cons.injEq] at h; exact one_ne_four h.1.symm
    · simp only [rdPadTuple, List.cons.injEq] at h; exact one_ne_four h.1.symm
  have honce : (rdArg (F := F) L).once = false := by
    rcases L with _ | _ | L <;> first | omega | rfl
  have hat : (r : Int) ∈ (⟨⟨rdArg L, (R.seg k).x, (R.seg k).t⟩, RomData.numRows⟩ : Sender F).at := by
    simp only [Sender.at, honce, Bool.false_eq_true, if_false, List.mem_map, List.mem_range]
    exact ⟨r, by rw [hN]; exact hr, rfl⟩
  have hp0 : (⟨⟨rdArg L, (R.seg k).x, (R.seg k).t⟩, RomData.numRows⟩ : Sender F).proving = true := by
    rcases L with _ | _ | L <;> first | omega | rfl
  obtain ⟨s, hs', hp, j', hj', ht', hr'⟩ := W.assumed hop hnp hs hp0
    (fun j _ => .inr (rd_lane_tuple hL _ _ j).1) hat ht hr1
  rcases List.mem_append.mp hs' with hm | ho
  · exfalso
    have := W.main_not4 hm hp hj' hr'
    rw [ht'] at this
    exact this rfl
  · rcases S.mem_split ho with hra | hi | hro | hrd | hn
    · obtain ⟨I', hI', hs''⟩ := List.mem_flatMap.mp hra
      have := (ram_counts I' (W.pad I' hI').1 (W.pad I' hI').2 [] s hs'').1
      rw [hp] at this; exact nomatch this
    · obtain ⟨I', -, hs''⟩ := List.mem_flatMap.mp hi
      have := (input_counts I' [] s hs'').1
      rw [hp] at this; exact nomatch this
    · obtain ⟨I', hI', hs''⟩ := List.mem_flatMap.mp hro
      exact ⟨I', hI', s, hs'', j', hr', ht'⟩
    · obtain ⟨I', hI', hs''⟩ := List.mem_flatMap.mp hrd
      have := (rd_counts I' (W.rdPad I' hI') [] s hs'').1
      rw [hp] at this; exact nomatch this
    · exfalso
      have := W.quiet
      unfold MemOthers.Quiet at this
      rw [this] at hn
      exact nomatch hn

/-- **A RomData read returns a word of the Rom's data**, at its bus address. -/
theorem MemBusWorld.rd_value {M : MainSegs F} {gp ga : List (List F)}
    (W : MemBusWorld M S gp ga) (R : RdWorld S) {I : RomDataOnMem F} (hI : I ∈ S.romData)
    {m : Nat} (hm : m < RomData.numRows) {X st v0 v1 : F}
    (h : tupleOf (⟨RomData.bus4, I.x, I.t⟩ : Prover F) m = [1, X, st, 8, v0, v1] ∨
      tupleOf (⟨RomData.bus5, I.x, I.t⟩ : Prover F) m = [1, X, st, 8, v0, v1]) :
    RomWord S X (v0, v1) := by
  have hN : RomData.numRows = 4194304 := rfl
  obtain ⟨k, hk, rfl⟩ := R.all I hI
  obtain ⟨L, hL, ht⟩ : ∃ L < 2, tupleOf (⟨rdArg L, (R.seg k).x, (R.seg k).t⟩ : Prover F) m =
      [1, X, st, 8, v0, v1] := by
    rcases h with h | h
    · exact ⟨0, by omega, h⟩
    · exact ⟨1, by omega, h⟩
  rw [(rd_lane_tuple hL _ _ m).2] at ht
  simp only [List.cons.injEq] at ht
  obtain ⟨-, hX, -, -, hv0, hv1, -⟩ := ht
  -- The lane's origin.
  have hn : k * 8388608 + 2 * m + L < R.K * 8388608 := by
    have : (k + 1) * 8388608 ≤ R.K * 8388608 := Nat.mul_le_mul_right _ hk
    rw [hN] at hm; rw [Nat.add_mul, Nat.one_mul] at this; omega
  obtain ⟨n', hle, hac, had, hav⟩ := R.origin _ hn
  obtain ⟨-, d1, v1'⟩ := R.at_lane (k := k) (r := m) (L := L) (by rw [hN] at hm; exact hm) hL
  obtain ⟨e, hr', hL'⟩ := lane_split n'
  generalize hk' : n' / 8388608 = k' at e
  generalize hr'' : (n' % 8388608) / 2 = r' at e hr'
  generalize hL'' : n' % 2 = L' at e hL'
  have hkK : k' < R.K := by
    have : k' * 8388608 ≤ n' := by omega
    have : n' < R.K * 8388608 := by omega
    have : k' * 8388608 < R.K * 8388608 := by omega
    exact Nat.lt_of_mul_lt_mul_right this
  obtain ⟨a0, d0, v0'⟩ := R.at_lane (k := k') (r := r') (L := L') hr' hL'
  rw [← e] at a0 d0 v0'
  rw [a0] at hac
  have hw := W.rd_word R hkK hr' hL' hac
  have eA : ((R.seg k').t r').addr L' = ((R.seg k).t m).addr L := by rw [← d0, had, d1]
  have eV : (((R.seg k').t r').value L' 0, ((R.seg k').t r').value L' 1) =
      (((R.seg k).t m).value L 0, ((R.seg k).t m).value L 1) := by rw [← v0', hav, v1']
  rw [eA, eV] at hw
  rw [← hX, ← hv0, ← hv1]
  exact hw

end Correspondence
