/-
  The input memory: every read of an address returns the same value.

  `InputData` is the Mem AIR as a free-input memory: read-only, its values the
  program's input, which the prover supplies. It sorts its lanes by address
  alone — one a row — and keeps a lane's value from the lane before when the
  address does not change. The one exception is the region's base address,
  where free reads (`is_free_read`) may each return a fresh value: how ZisK
  serves its free-input calls.

  * `imm_consistent`, in the abstract: along a sequence of lanes, each keeping
    the address and value of the one before or moving to a higher address,
    two lanes at the same address, other than the base, hold the same value.
  * `input_consistent`: InputData's lanes, over a chain of its segments, are
    such a sequence, given their addresses as naturals below `2^40` — which,
    as for the RAM, the memory bus will supply.

  So the input memory is a function of the address: the model's initial
  memory for the region.
-/
import Proofs.MemRam
import Pil.InputData

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

/-! ### In the abstract -/

namespace MemImm

variable {V : Type}

/-- A lane of a read-only memory: its address, its value, and whether it is a
free read. -/
structure ILane (V : Type) where
  addr : Nat
  val : V
  free : Bool

/-- Lane `q` follows lane `p`: the same address, with the same value unless
`q` is a free read, or a higher address. -/
def IStep (p q : ILane V) : Prop :=
  (q.addr = p.addr ∧ (q.free = false → q.val = p.val)) ∨ p.addr < q.addr

variable {g : Nat → ILane V} {n : Nat}

theorem addr_mono (hstep : ∀ k, k + 1 < n → IStep (g k) (g (k + 1))) {i : Nat} :
    ∀ j, i ≤ j → j < n → (g i).addr ≤ (g j).addr
  | 0, hij, _ => by rw [Nat.le_zero.mp hij]; exact Nat.le_refl _
  | j + 1, hij, hj => by
    rcases Nat.eq_or_lt_of_le hij with rfl | hlt
    · exact Nat.le_refl _
    · have := addr_mono hstep (i := i) j (by omega) (by omega)
      rcases hstep j hj with ⟨h, -⟩ | h <;> omega

/-- **Same address, same value.** Two lanes at the same address, other than
`base`, where alone free reads happen, hold the same value. -/
theorem imm_consistent {base : Nat} (hstep : ∀ k, k + 1 < n → IStep (g k) (g (k + 1)))
    (hfree : ∀ k, k < n → (g k).free = true → (g k).addr = base) {i : Nat} :
    ∀ j, i ≤ j → j < n → (g i).addr = (g j).addr → (g j).addr ≠ base → (g i).val = (g j).val
  | 0, hij, _, _, _ => by rw [Nat.le_zero.mp hij]
  | j + 1, hij, hj, ha, hb => by
    rcases Nat.eq_or_lt_of_le hij with rfl | hlt
    · rfl
    · have m1 := addr_mono hstep (i := i) j (by omega) (by omega)
      have m2 := addr_mono hstep (i := j) (j + 1) (by omega) hj
      have hj' : (g j).addr = (g (j + 1)).addr := by omega
      have ih := imm_consistent hstep hfree (i := i) j (by omega) (by omega) (by omega) (by omega)
      rcases hstep j hj with ⟨-, hv⟩ | h
      · have hf : (g (j + 1)).free = false := by
          cases hq : (g (j + 1)).free
          · rfl
          · exact absurd (hfree (j + 1) hj hq) hb
        rw [ih, hv hf]
      · omega

end MemImm


/-! ### InputData -/

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- A row's value: two 32-bit halves, each two 16-bit words. -/
def inVal (r : InputData.Row F) : F × F :=
  (r.value_word 0 0 + 65536 * r.value_word 0 1, r.value_word 0 2 + 65536 * r.value_word 0 3)

/-- The constraints of a row of InputData against the lane before it: its
address `pa` and value `pv`. -/
structure InLaneHolds (r : InputData.Row F) (pa : F) (pv : F × F) : Prop where
  ac : r.addr_changes 0 * (1 - r.addr_changes 0) = 0
  free : r.is_free_read 0 * (1 - r.is_free_read 0) = 0
  free_addr : r.is_free_read 0 * (134217728 - r.addr 0) = 0
  addr : (1 - r.addr_changes 0) * (r.addr 0 - pa) = 0
  val0 : (1 - r.addr_changes 0) * (1 - r.is_free_read 0) * ((inVal r).1 - pv.1) = 0
  val1 : (1 - r.addr_changes 0) * (1 - r.is_free_read 0) * ((inVal r).2 - pv.2) = 0

section Rows

variable {x : InputData.Ctx F} {t : InputData.Trace F} {i : Int}

theorem inLane (h : InputData.everyRow x t i) :
    InLaneHolds (t i) (InputData.InputData_previous_addr_0 x t i)
      ((t i).__L1__ * (x.InputData_previous_segment_value 0 - (inVal (t (i - 1))).1) + (inVal (t (i - 1))).1,
       (t i).__L1__ * (x.InputData_previous_segment_value 1 - (inVal (t (i - 1))).2) + (inVal (t (i - 1))).2) := by
  refine ⟨h.c4, h.c5, h.c6, h.c13, ?_, ?_⟩
  · have e := h.c15
    simp only [InputData.c15, InputData.Zisk_direct_gsum_e_3, InputData.InputData_value_0_0,
      InputData.InputData_previous_value_0_0, InputData._'InputData_value_0_0] at e
    exact e
  · have e := h.c16
    simp only [InputData.c16, InputData.Zisk_direct_gsum_e_4, InputData.InputData_value_0_1,
      InputData._'InputData_value_0_1] at e
    exact e

end Rows


/-! ### As naturals -/

/-- Equal in the field and below `2^48`, so equal. -/
theorem natCast_inj48 {x y : Nat} (hx : x < 2 ^ 48) (hy : y < 2 ^ 48) (h : (x : F) = (y : F)) :
    x = y :=
  natCast_inj (by unfold goldilocks; omega) (by unfold goldilocks; omega) h

open MemImm

open Classical in
/-- A row's lane, given its address as a natural. -/
noncomputable def inLaneOf (r : InputData.Row F) (a : Nat) : ILane (F × F) :=
  ⟨a, inVal r, decide (r.is_free_read 0 = 1)⟩

/-- **A row follows the lane before it.** -/
theorem in_step {r : InputData.Row F} {pa : F} {pv : F × F} (hl : InLaneHolds r pa pv)
    (hrng : r.addr_changes 0 ≠ 0 → ∃ n : Nat, n < 2 ^ 24 ∧ r.addr 0 - pa - 1 = ((n : Nat) : F))
    {a PA : Nat} (ha40 : a < 2 ^ 48) (hPA : PA < 2 ^ 48) (ha : r.addr 0 = ((a : Nat) : F))
    (hpa : pa = ((PA : Nat) : F)) (p : ILane (F × F)) (hpA : p.addr = PA) (hpv : p.val = pv) :
    IStep p (inLaneOf r a) := by
  obtain ⟨b, hb⟩ := bool_of_idem hl.ac
  cases b
  · -- The address is kept.
    have e := hl.addr
    rw [hb, ha, hpa] at e
    simp only [bit, Bool.false_eq_true, if_false] at e
    have e' : ((a : Nat) : F) = ((PA : Nat) : F) := by grind
    have := natCast_inj48 (F := F) (by omega) (by omega) e'
    refine .inl ⟨by simp only [inLaneOf]; omega, fun hf => ?_⟩
    simp only [inLaneOf, decide_eq_false_iff_not] at hf ⊢
    have hf0 : r.is_free_read 0 = 0 := by
      obtain ⟨c, hc⟩ := bool_of_idem hl.free
      cases c <;> simp_all [bit]
    have v0 := hl.val0; have v1 := hl.val1
    rw [hb, hf0] at v0 v1
    simp only [bit, Bool.false_eq_true, if_false] at v0 v1
    rw [hpv]
    simp only [inVal] at v0 v1 ⊢
    apply Prod.ext <;> grind
  · -- The address moves up.
    obtain ⟨n, hn, e⟩ := hrng (by rw [hb]; exact one_ne_zero')
    rw [ha, hpa] at e
    have e' : ((a : Nat) : F) = ((PA + n + 1 : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add,
        Lean.Grind.Semiring.natCast_one]; grind
    have := natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega) e'
    exact .inr (by simp only [inLaneOf]; omega)

/-- A free read is at the region's base. -/
theorem in_free {r : InputData.Row F} {pa : F} {pv : F × F} (hl : InLaneHolds r pa pv) {a : Nat}
    (ha40 : a < 2 ^ 48) (ha : r.addr 0 = ((a : Nat) : F)) (hf : (inLaneOf r a).free = true) :
    (inLaneOf r a).addr = 134217728 := by
  simp only [inLaneOf, decide_eq_true_eq] at hf ⊢
  have e := hl.free_addr
  rw [hf, ha, Lean.Grind.Semiring.one_mul] at e
  have e' : ((a : Nat) : F) = ((134217728 : Nat) : F) := by
    rw [← natCast_ofNat' 134217728]; grind
  exact natCast_inj48 (F := F) (by omega) (by omega) e'

/-! ### Segments -/

/-- One segment of InputData: its trace, its constraints at every row,
`__L1__` as the setup fixes it, its address range check sound, and its
addresses as naturals. -/
structure InSeg (F : Type) [Lean.Grind.Field F] where
  x : InputData.Ctx F
  t : InputData.Trace F
  rows : ∀ i, InputData.everyRow x t i
  cyclic : InputData.cyclic t
  first : (t 0).__L1__ = 1
  inner : ∀ i : Int, 0 < i → i < InputData.numRows → (t i).__L1__ = 0
  range : RangeHolds InputData.ranges InputData.bus25 x t
  A : Nat → Nat
  addr : ∀ r : Nat, r < InputData.numRows → A r < 2 ^ 48 ∧ (t r).addr 0 = ((A r : Nat) : F)

/-- Row `i`'s lane. -/
noncomputable def InSeg.lane (G : InSeg F) (r : Nat) : ILane (F × F) := inLaneOf (G.t r) (G.A r)

set_option maxRecDepth 100000 in
omit [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks] in
theorem in_range_max : ∀ d ∈ (InputData.ranges : List RangeDef), d.opid = 102 → d.max = 16777215 := by
  decide

/-- The address range check, at a row. -/
theorem InSeg.rng (G : InSeg F) (i : Int) :
    (G.t i).addr_changes 0 ≠ 0 → ∃ n : Nat, n < 2 ^ 24 ∧
      (G.t i).addr 0 - InputData.InputData_previous_addr_0 G.x G.t i - 1 = ((n : Nat) : F) := by
  intro hac
  obtain ⟨-, -, b, d, hb, hd, hdo, -, -, hall⟩ := G.range
  have hb' : b = 102 := by simpa [InputData.bus25] using hb.symm
  have hmax := in_range_max d hd (by rw [hdo, hb'])
  obtain ⟨n, -, hn, e⟩ := hall i (by simpa [InputData.bus25, Lean.Grind.Semiring.add_zero] using hac)
    _ (List.mem_singleton_self _)
  exact ⟨n, by omega, by simpa [InputData.InputData_delta_addr_0] using e⟩

/-- Inside a segment, each row follows the one before. -/
theorem InSeg.step_inner (G : InSeg F) {r : Nat} (hr : r + 1 < InputData.numRows) :
    IStep (G.lane r) (G.lane (r + 1)) := by
  have hL1 := G.inner ((r + 1 : Nat) : Int) (by omega) (by omega)
  have hl := inLane (G.rows ((r + 1 : Nat) : Int))
  have hi : ((r + 1 : Nat) : Int) - 1 = (r : Int) := by omega
  rw [hL1, hi] at hl
  simp only [Lean.Grind.Semiring.zero_mul, Lean.Grind.Semiring.zero_add] at hl
  have hrng := G.rng ((r + 1 : Nat) : Int)
  simp only [InputData.InputData_previous_addr_0, hL1, hi, Lean.Grind.Semiring.zero_mul,
    Lean.Grind.Semiring.zero_add] at hl hrng
  exact in_step hl hrng (G.addr (r + 1) hr).1 (G.addr r (by omega)).1 (G.addr (r + 1) hr).2
    (G.addr r (by omega)).2 _ rfl rfl

/-- The first row follows the lane its air values name. -/
theorem InSeg.step_first (G : InSeg F) (p : ILane (F × F)) (hpA : p.addr < 2 ^ 48)
    (hpa : ((p.addr : Nat) : F) = G.x.InputData_previous_segment_addr)
    (hpv : p.val = (G.x.InputData_previous_segment_value 0, G.x.InputData_previous_segment_value 1)) :
    IStep p (G.lane 0) := by
  have hl := inLane (G.rows 0)
  have hrng := G.rng 0
  simp only [InputData.InputData_previous_addr_0, G.first, Lean.Grind.Semiring.one_mul] at hl hrng
  have e1 : ∀ a b : F, a - b + b = a := fun a b => by grind
  simp only [e1] at hl hrng
  have h0 := G.addr 0 (by simp [InputData.numRows])
  exact in_step hl hrng h0.1 hpA h0.2 hpa.symm p rfl hpv

/-- The last row is what the exit air values hand on. -/
theorem InSeg.last (G : InSeg F) :
    (((G.lane (InputData.numRows - 1)).addr : Nat) : F) = G.x.InputData_segment_last_addr ∧
      (G.lane (InputData.numRows - 1)).val =
        (G.x.InputData_segment_last_value 0, G.x.InputData_segment_last_value 1) ∧
      (G.lane (InputData.numRows - 1)).addr < 2 ^ 48 := by
  have hwrap : (G.t (((InputData.numRows - 1 : Nat) : Int) + 1)).__L1__ = 1 := by
    have := G.cyclic 0
    rw [Int.zero_add] at this
    have e : ((InputData.numRows - 1 : Nat) : Int) + 1 = (InputData.numRows : Int) := by
      simp [InputData.numRows]
    rw [e, this, G.first]
  have hrow := G.rows ((InputData.numRows - 1 : Nat) : Int)
  have e7 := hrow.c7; have e8 := hrow.c8; have e9 := hrow.c9
  simp only [InputData.c7, InputData.c8, InputData.c9, InputData.InputData_value_0_0,
    InputData.InputData_value_0_1, hwrap, Lean.Grind.Semiring.one_mul] at e7 e8 e9
  have hl := G.addr (InputData.numRows - 1) (by simp [InputData.numRows])
  refine ⟨?_, ?_, hl.1⟩
  · show ((G.A _ : Nat) : F) = _
    rw [← hl.2]; grind
  · show inVal _ = _
    apply Prod.ext <;> simp only [inVal] <;> grind


/-! ### The chain of segments -/

/-- What InputData's continuation bus guarantees: each segment starts from the
address and value the one before hands on. -/
def InChain (seg : Nat → InSeg F) (K : Nat) : Prop :=
  ∀ k, k + 1 < K →
    (seg (k + 1)).x.InputData_previous_segment_addr = (seg k).x.InputData_segment_last_addr ∧
      (seg (k + 1)).x.InputData_previous_segment_value 0 = (seg k).x.InputData_segment_last_value 0 ∧
      (seg (k + 1)).x.InputData_previous_segment_value 1 = (seg k).x.InputData_segment_last_value 1

/-- The `m`-th lane over all segments: row `m mod numRows` of segment
`m / numRows`. -/
noncomputable def inLog (seg : Nat → InSeg F) (m : Nat) : ILane (F × F) :=
  (seg (m / InputData.numRows)).lane (m % InputData.numRows)

/-- **Every read of the input memory returns the same value at an address.**
Over a chain of InputData segments, two lanes at the same address, other than
the region's base, hold the same value. -/
theorem input_consistent {seg : Nat → InSeg F} {K : Nat} (hc : InChain seg K) {i j : Nat}
    (hij : i ≤ j) (hj : j < K * InputData.numRows) (ha : (inLog seg i).addr = (inLog seg j).addr)
    (hb : (inLog seg j).addr ≠ 134217728) : (inLog seg i).val = (inLog seg j).val := by
  have hN : InputData.numRows = 4194304 := rfl
  refine imm_consistent (g := inLog seg) (n := K * InputData.numRows) (base := 134217728)
    ?_ ?_ j hij hj ha hb
  · intro m hm
    by_cases hw : (m + 1) % InputData.numRows = 0
    · -- Row 0 of the next segment, after the last row of this one.
      have hk : (m + 1) / InputData.numRows = m / InputData.numRows + 1 := by
        rw [hN] at hw ⊢; omega
      have hr : m % InputData.numRows = InputData.numRows - 1 := by rw [hN] at hw ⊢; omega
      have hlt : m / InputData.numRows + 1 < K := by
        have := Nat.div_lt_of_lt_mul (m := m + 1) (n := InputData.numRows) (k := K)
          (by rw [Nat.mul_comm]; omega)
        omega
      obtain ⟨ea, ev0, ev1⟩ := hc (m / InputData.numRows) hlt
      obtain ⟨la, lv, lA⟩ := (seg (m / InputData.numRows)).last
      simp only [inLog, hk, hw, hr]
      exact (seg _).step_first _ lA (by rw [ea]; exact la) (by rw [ev0, ev1]; exact lv)
    · have hk : (m + 1) / InputData.numRows = m / InputData.numRows := by rw [hN] at hw ⊢; omega
      have hr : (m + 1) % InputData.numRows = m % InputData.numRows + 1 := by
        rw [hN] at hw ⊢; omega
      simp only [inLog, hk, hr]
      exact (seg _).step_inner (by rw [hN] at hr ⊢; omega)
  · intro m _ hf
    have hm := (seg (m / InputData.numRows)).addr (m % InputData.numRows) (Nat.mod_lt _ (by
      simp [InputData.numRows]))
    exact in_free (inLane ((seg _).rows _)) hm.1 hm.2 hf


/-! ### The addresses, from the AIR -/

set_option maxRecDepth 100000 in
omit [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks] in
theorem in_range_max104 : ∀ d ∈ (InputData.ranges : List RangeDef), d.opid = 104 → d.max = 65535 := by
  decide

/-- A 16-bit air value, by a sound range check. -/
theorem in_chunk {x : InputData.Ctx F} {t : InputData.Trace F}
    {R : BusArg F (InputData.Ctx F) (InputData.Trace F)} (hR : RangeHolds InputData.ranges R x t)
    (hop : R.opids = [104]) (hreps : R.reps x t 0 = 1) {f : InputData.Ctx F → InputData.Trace F → Int → F}
    (hf : R.tuple = [f]) : ∃ n : Nat, n < 2 ^ 16 ∧ f x t 0 = ((n : Nat) : F) := by
  obtain ⟨-, -, b, d, hb, hd, hdo, -, -, hall⟩ := hR
  rw [hop] at hb
  have hb' : b = 104 := by simpa using hb.symm
  have hmax := in_range_max104 d hd (by rw [hdo, hb'])
  obtain ⟨n, -, hn, e⟩ := hall 0 (by rw [hreps]; exact one_ne_zero') f (by rw [hf]; exact List.mem_singleton_self _)
  exact ⟨n, by omega, e⟩

/-- One segment of InputData, as the AIR gives it: its trace, its constraints
at every row, `__L1__` as the setup fixes it, and its range checks sound — the
address step's, and the distances from the region's base to the first
address and from the last address to the region's end. -/
structure InTrace (F : Type) [Lean.Grind.Field F] where
  x : InputData.Ctx F
  t : InputData.Trace F
  rows : ∀ i, InputData.everyRow x t i
  cyclic : InputData.cyclic t
  first : (t 0).__L1__ = 1
  inner : ∀ i : Int, 0 < i → i < InputData.numRows → (t i).__L1__ = 0
  range : RangeHolds InputData.ranges InputData.bus25 x t
  base0 : RangeHolds InputData.ranges InputData.bus17 x t
  base1 : RangeHolds InputData.ranges InputData.bus19 x t
  end0 : RangeHolds InputData.ranges InputData.bus21 x t
  end1 : RangeHolds InputData.ranges InputData.bus23 x t

namespace InTrace

variable (T : InTrace F)

/-- The address range check, at a row. -/
theorem rng (i : Int) :
    (T.t i).addr_changes 0 ≠ 0 → ∃ n : Nat, n < 2 ^ 24 ∧
      (T.t i).addr 0 - InputData.InputData_previous_addr_0 T.x T.t i - 1 = ((n : Nat) : F) := by
  intro hac
  obtain ⟨-, -, b, d, hb, hd, hdo, -, -, hall⟩ := T.range
  have hb' : b = 102 := by simpa [InputData.bus25] using hb.symm
  have hmax := in_range_max d hd (by rw [hdo, hb'])
  obtain ⟨n, -, hn, e⟩ := hall i (by simpa [InputData.bus25, Lean.Grind.Semiring.add_zero] using hac)
    _ (List.mem_singleton_self _)
  exact ⟨n, by omega, by simpa [InputData.InputData_delta_addr_0] using e⟩

/-- The first address, as a natural: the region's base plus its range-checked
distance. -/
theorem base_nat : ∃ P : Nat, 134217728 ≤ P ∧ P < 2 ^ 33 ∧
    T.x.InputData_previous_segment_addr = ((P : Nat) : F) := by
  obtain ⟨d0, h0, e0⟩ := in_chunk T.base0 rfl rfl (f := fun x t i => x.InputData_distance_base 0 + 0) rfl
  obtain ⟨d1, h1, e1⟩ := in_chunk T.base1 rfl rfl (f := fun x t i => x.InputData_distance_base 1 + 0) rfl
  have c := (T.rows 0).c11
  simp only [InputData.c11, InputData.Zisk_direct_gsum_e_0] at c e0 e1
  refine ⟨134217728 + d0 + 65536 * d1, by omega, by omega, ?_⟩
  simp only [Lean.Grind.Semiring.add_zero] at e0 e1
  rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add,
    Lean.Grind.Semiring.natCast_mul, ← natCast_ofNat' 134217728, ← natCast_ofNat' 65536, ← e0, ← e1]
  grind

/-- The first address. -/
noncomputable def P : Nat := Classical.choose T.base_nat

open Classical in
/-- Row `r`'s address step, as a natural: `1 +` its range-checked distance
when the address changes, else 0. -/
noncomputable def inc (r : Nat) : Nat :=
  if (T.t r).addr_changes 0 = 1 then
    (if h : ∃ n : Nat, n < 2 ^ 24 ∧
        (T.t r).addr 0 - InputData.InputData_previous_addr_0 T.x T.t r - 1 = ((n : Nat) : F)
      then Classical.choose h else 0) + 1
  else 0

/-- Row `r`'s address, as a natural. -/
noncomputable def A : Nat → Nat
  | 0 => T.P + T.inc 0
  | r + 1 => A r + T.inc (r + 1)

theorem inc_le (r : Nat) : T.inc r ≤ 2 ^ 24 := by
  unfold inc
  split
  · split
    · rename_i h; have := (Classical.choose_spec h).1; omega
    · omega
  · omega

/-- A row's address is the one before plus its step. -/
theorem addr_step {r : Nat} {Q : Nat} (hQ : InputData.InputData_previous_addr_0 T.x T.t r = ((Q : Nat) : F)) :
    (T.t r).addr 0 = ((Q + T.inc r : Nat) : F) := by
  have hl := inLane (T.rows (r : Int))
  obtain ⟨b, hb⟩ := bool_of_idem hl.ac
  unfold inc
  cases b
  · have h1 : ¬ (T.t (r : Int)).addr_changes 0 = 1 := by
      rw [hb]; simp [bit, Lean.Grind.Field.zero_ne_one]
    rw [if_neg h1, Nat.add_zero]
    have e := hl.addr
    rw [hb, hQ] at e
    simp only [bit, Bool.false_eq_true, if_false] at e
    grind
  · have h1 : (T.t (r : Int)).addr_changes 0 = 1 := by rw [hb]; rfl
    rw [if_pos h1]
    have hex := T.rng r (by rw [h1]; exact one_ne_zero')
    rw [dif_pos hex]
    have e := (Classical.choose_spec hex).2
    generalize Classical.choose hex = n at e ⊢
    rw [hQ] at e
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_one]
    grind

theorem P_spec : 134217728 ≤ T.P ∧ T.P < 2 ^ 33 ∧
    T.x.InputData_previous_segment_addr = ((T.P : Nat) : F) :=
  Classical.choose_spec T.base_nat

/-- **Every address is the natural `A r`.** -/
theorem addr_eq : ∀ r : Nat, r < InputData.numRows → (T.t r).addr 0 = ((T.A r : Nat) : F)
  | 0, _ => by
    apply T.addr_step
    simp only [InputData.InputData_previous_addr_0]
    rw [show ((0 : Nat) : Int) = 0 from rfl, T.first, T.P_spec.2.2]
    grind
  | r + 1, hr => by
    have ih := addr_eq r (by omega)
    apply T.addr_step
    have hL1 := T.inner ((r + 1 : Nat) : Int) (by omega) (by omega)
    have hi : ((r + 1 : Nat) : Int) - 1 = (r : Int) := by omega
    simp only [InputData.InputData_previous_addr_0, hL1, hi, ih]
    grind

theorem A_le : ∀ r, T.A r ≤ T.P + (r + 1) * 2 ^ 24
  | 0 => by have := T.inc_le 0; simp only [A]; omega
  | r + 1 => by have := A_le r; have := T.inc_le (r + 1); simp only [A]; rw [Nat.succ_mul]; omega

theorem A_mono : ∀ {r s : Nat}, r ≤ s → T.A r ≤ T.A s
  | r, 0, h => by rw [Nat.le_zero.mp h]; exact Nat.le_refl _
  | r, s + 1, h => by
    rcases Nat.eq_or_lt_of_le h with rfl | hlt
    · exact Nat.le_refl _
    · have := A_mono (r := r) (s := s) (by omega); simp only [A]; omega

/-- The segment, with its addresses. -/
noncomputable def toSeg : InSeg F where
  x := T.x
  t := T.t
  rows := T.rows
  cyclic := T.cyclic
  first := T.first
  inner := T.inner
  range := T.range
  A := T.A
  addr r hr := ⟨by
    have := T.A_le r; have := T.P_spec.2.1
    simp only [InputData.numRows] at hr; omega, T.addr_eq r hr⟩

/-- **The addresses lie in the region**: internal addresses
`[0x0800_0000, 0x0FFF_FFFF]`, bytes `[0x4000_0000, 0x7FFF_FFFF]`. -/
theorem A_region {r : Nat} (hr : r < InputData.numRows) :
    134217728 ≤ T.A r ∧ T.A r ≤ 268435455 := by
  have hN : InputData.numRows = 4194304 := rfl
  constructor
  · have := T.A_mono (Nat.zero_le r); have := T.P_spec.1; simp only [A] at *; omega
  · -- The last address: the region's end less a range-checked distance.
    obtain ⟨e0, h0, f0⟩ := in_chunk T.end0 rfl rfl (f := fun x t i => x.InputData_distance_end 0 + 0) rfl
    obtain ⟨e1, h1, f1⟩ := in_chunk T.end1 rfl rfl (f := fun x t i => x.InputData_distance_end 1 + 0) rfl
    simp only [Lean.Grind.Semiring.add_zero] at f0 f1
    have c := (T.rows 0).c12
    simp only [InputData.c12, InputData.Zisk_direct_gsum_e_1] at c
    have hlast := (T.toSeg.last).1
    have hla : ((T.A (InputData.numRows - 1) : Nat) : F) = T.x.InputData_segment_last_addr := hlast
    have e : ((T.A (InputData.numRows - 1) + (e0 + 65536 * e1) : Nat) : F) = ((268435455 : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add,
        Lean.Grind.Semiring.natCast_mul, hla, ← natCast_ofNat' 65536, ← f0, ← f1,
        ← natCast_ofNat' 268435455]
      grind
    have hA := (T.toSeg.addr (InputData.numRows - 1) (by omega)).1
    have := natCast_inj (F := F) (by unfold goldilocks; simp only [toSeg] at hA; omega)
      (by unfold goldilocks; omega) e
    have := T.A_mono (r := r) (s := InputData.numRows - 1) (by omega)
    omega

end InTrace

end Correspondence
