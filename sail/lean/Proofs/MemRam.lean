/-
  The RAM's constraints as a sorted log.

  `MemSort.lean` proves what a sorted log buys. Here the RAM — the Mem AIR as
  ZisK instantiates it, mutable, with dual reads and four lanes a row — is
  shown to be one, lane by lane: each lane, against the lane before it,
  `Follows` in `MemSort`'s sense.

  The AIR fixes only differences, in the field: the increment
  `l_increment + 2^22·h_increment + 1`, range-checked to `[1, 2^38]`, is the
  address's step up when the address changes, and the step's (plus one for a
  read) when it does not. Taken with Nat values for the lanes' addresses and
  steps below `2^40` — which the memory bus will supply, every lane's tuple
  being one some AIR assumes — the field relations are relations between
  naturals (`follows_cols`).
-/
import Proofs.MemSort
import Proofs.Rom
import Proofs.Adders
import Pil.Mem

namespace Correspondence

open MemSort

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- Equal in the field and below `2^41`, so equal. -/
theorem natCast_inj41 {x y : Nat} (hx : x < 2 ^ 41) (hy : y < 2 ^ 41) (h : (x : F) = (y : F)) :
    x = y :=
  natCast_inj (by unfold goldilocks; omega) (by unfold goldilocks; omega) h

/-- **One lane follows the one before it.** The field relations of a lane of
the RAM — the increment, the address kept when it does not change, a read's
value — against the previous lane's address `pA`, last step `pL` and value
`pv`, with every address and step a natural below `2^40`. -/
theorem follows_cols {pA pL a s inc : Nat} {pv v : F × F} {ac w : Bool} (dual : Option Nat)
    (hpA : pA < 2 ^ 40) (hpL : pL < 2 ^ 40) (ha : a < 2 ^ 40) (hs : s < 2 ^ 40)
    (hincl : inc < 2 ^ 38)
    (hinc : (inc : F) + 1 = bit ac * (((a : F) - (pA : F)) - ((s : F) - (pL : F) + (1 - bit w))) +
      ((s : F) - (pL : F) + (1 - bit w)))
    (haddr : (1 - bit ac) * ((a : F) - (pA : F)) = 0)
    (hv0 : (1 - bit ac) * (1 - bit w) * (v.1 - pv.1) = 0)
    (hv1 : (1 - bit ac) * (1 - bit w) * (v.2 - pv.2) = 0)
    (hz0 : bit ac * (1 - bit w) * v.1 = 0) (hz1 : bit ac * (1 - bit w) * v.2 = 0)
    (p : Lane (F × F)) (hpa : p.addr = pA) (hpl : p.last = pL) (hpv : p.val = pv) :
    Follows (0, 0) p ⟨a, s, w, v, dual⟩ := by
  cases ac <;> simp only [bit, Bool.false_eq_true, if_false, if_true] at hinc haddr hv0 hv1 hz0 hz1
  · -- The address is kept.
    have e : (a : F) = (pA : F) := by grind
    have ea := natCast_inj41 (F := F) (by omega) (by omega) e
    refine .inl ⟨by simp only; omega, ?_, ?_⟩
    · cases w <;> simp only [bit, Bool.false_eq_true, if_false, if_true] at hinc
      · have e' : (s : F) = ((pL + inc : Nat) : F) := by
          rw [Lean.Grind.Semiring.natCast_add]; grind
        have := natCast_inj41 (F := F) (by omega) (by omega) e'
        simp only [Bool.false_eq_true, if_false]; omega
      · have e' : (s : F) = ((pL + inc + 1 : Nat) : F) := by
          rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add,
            Lean.Grind.Semiring.natCast_one]; grind
        have := natCast_inj41 (F := F) (by omega) (by omega) e'
        simp only [if_true]; omega
    · intro hw
      simp only at hw
      subst hw
      simp only [bit, Bool.false_eq_true, if_false] at hv0 hv1
      rw [hpv]
      apply Prod.ext <;> simp only <;> grind
  · -- The address moves up.
    have e' : (a : F) = ((pA + inc + 1 : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add,
        Lean.Grind.Semiring.natCast_one]; grind
    have := natCast_inj41 (F := F) (by omega) (by omega) e'
    refine .inr ⟨by simp only; omega, ?_⟩
    intro hw
    simp only at hw
    subst hw
    simp only [bit, Bool.false_eq_true, if_false] at hz0 hz1
    apply Prod.ext <;> simp only <;> grind

/-! ### A lane's constraints -/

open Pil.Zisk

/-- The constraints of lane `L` of a RAM row against the lane before it: its
address `pa`, the step it hands on `ps` (the `previous_step` column) and its
value `(v0, v1)`. -/
structure RamLaneHolds (r : Mem.Row F) (L : Nat) (pa v0 v1 : F) : Prop where
  ac : r.addr_changes L * (1 - r.addr_changes L) = 0
  wr : r.wr L * (1 - r.wr L) = 0
  sel : r.sel_dual L * (1 - r.sel_dual L) = 0
  inc : r.l_increment L + 4194304 * r.h_increment L + 1 =
    r.addr_changes L * ((r.addr L - pa) - (r.step L - r.previous_step L + (1 - r.wr L))) +
      (r.step L - r.previous_step L + (1 - r.wr L))
  addr : (1 - r.addr_changes L) * (r.addr L - pa) = 0
  val0 : (1 - r.addr_changes L) * (1 - r.wr L) * (r.value L 0 - v0) = 0
  val1 : (1 - r.addr_changes L) * (1 - r.wr L) * (r.value L 1 - v1) = 0
  z0 : r.addr_changes L * (1 - r.wr L) * r.value L 0 = 0
  z1 : r.addr_changes L * (1 - r.wr L) * r.value L 1 = 0

section Lanes

variable {x : Mem.Ctx F} {t : Mem.Trace F} {i : Int}

theorem ramLane_0 (h : Mem.everyRow x t i) :
    RamLaneHolds (t i) 0 (Mem.Mem_previous_addr_0 x t i) (Mem.Mem_previous_value_0_0 x t i)
      (Mem.Mem_previous_value_0_1 x t i) := by
  have rs := h.c26; simp only [Mem.c26] at rs
  refine ⟨h.c7, h.c11, h.c3, ?_, ?_, ?_, ?_, h.c44, h.c46⟩
  · have e := h.c25; simp only [Mem.c25, Mem.Zisk_direct_gsum_e_7, Mem.Mem_delta_step_0] at e; grind
  · have e := h.c39; simp only [Mem.c39, Mem.Zisk_direct_gsum_e_17] at e; exact e
  · have e := h.c43; simp only [Mem.c43] at e; grind
  · have e := h.c45; simp only [Mem.c45] at e; grind

theorem ramLane_1 (h : Mem.everyRow x t i) :
    RamLaneHolds (t i) 1 ((t i).addr 0) ((t i).value 0 0) ((t i).value 0 1) := by
  have rs := h.c28; simp only [Mem.c28] at rs
  refine ⟨h.c8, h.c12, h.c4, ?_, h.c40, ?_, ?_, h.c48, h.c50⟩
  · have e := h.c27; simp only [Mem.c27, Mem.Zisk_direct_gsum_e_9, Mem.Mem_delta_step_1] at e; grind
  · have e := h.c47; simp only [Mem.c47, Mem.Zisk_direct_gsum_e_20] at e; grind
  · have e := h.c49; simp only [Mem.c49] at e; grind

theorem ramLane_2 (h : Mem.everyRow x t i) :
    RamLaneHolds (t i) 2 ((t i).addr 1) ((t i).value 1 0) ((t i).value 1 1) := by
  have rs := h.c30; simp only [Mem.c30] at rs
  refine ⟨h.c9, ?_, h.c5, ?_, h.c41, ?_, ?_, h.c52, ?_⟩
  · have e := h.c13; simp only [Mem.c13, Mem.Zisk_direct_gsum_e_0] at e; exact e
  · have e := h.c29; simp only [Mem.c29, Mem.Zisk_direct_gsum_e_11, Mem.Mem_delta_step_2] at e; grind
  · have e := h.c51; simp only [Mem.c51, Mem.Zisk_direct_gsum_e_21] at e; grind
  · have e := h.c53; simp only [Mem.c53] at e; grind
  · have e := h.c54; simp only [Mem.c54, Mem.Zisk_direct_gsum_e_22] at e; exact e

theorem ramLane_3 (h : Mem.everyRow x t i) :
    RamLaneHolds (t i) 3 ((t i).addr 2) ((t i).value 2 0) ((t i).value 2 1) := by
  have rs := h.c32; simp only [Mem.c32] at rs
  refine ⟨h.c10, h.c14, h.c6, ?_, ?_, ?_, ?_, h.c56, ?_⟩
  · have e := h.c31; simp only [Mem.c31, Mem.Zisk_direct_gsum_e_13, Mem.Mem_delta_step_3] at e; grind
  · have e := h.c42; simp only [Mem.c42, Mem.Zisk_direct_gsum_e_18] at e; exact e
  · have e := h.c55; simp only [Mem.c55] at e; grind
  · have e := h.c57; simp only [Mem.c57] at e; grind
  · have e := h.c58; simp only [Mem.c58, Mem.Zisk_direct_gsum_e_23] at e; exact e

/-- `previous_step` of lanes 1–3: the last step of the lane before. -/
theorem ramPrevStep (h : Mem.everyRow x t i) :
    (t i).previous_step 1 = (t i).sel_dual 0 * ((t i).step_dual 0 - (t i).step 0) + (t i).step 0 ∧
    (t i).previous_step 2 = (t i).sel_dual 1 * ((t i).step_dual 1 - (t i).step 1) + (t i).step 1 ∧
    (t i).previous_step 3 = (t i).sel_dual 2 * ((t i).step_dual 2 - (t i).step 2) + (t i).step 2 := by
  have e1 := h.c22; have e2 := h.c23; have e3 := h.c24
  simp only [Mem.c22, Mem.c23, Mem.c24, Mem.Zisk_direct_gsum_e_5, Mem.Zisk_direct_gsum_e_6] at e1 e2 e3
  exact ⟨by grind, by grind, by grind⟩

/-- `previous_step` of lane 0: the previous segment's last step on the first
row, else the last step of lane 3 of the row before. -/
theorem ramPrevStep0 (h : Mem.everyRow x t i) :
    (t i).previous_step 0 = (t i).__L1__ * (x.Mem_previous_segment_step -
      ((t (i - 1)).sel_dual 3 * ((t (i - 1)).step_dual 3 - (t (i - 1)).step 3) + (t (i - 1)).step 3)) +
      ((t (i - 1)).sel_dual 3 * ((t (i - 1)).step_dual 3 - (t (i - 1)).step 3) + (t (i - 1)).step 3) := by
  have e := h.c21; simp only [Mem.c21, Mem.Zisk_direct_gsum_e_3] at e; grind

/-- A lane's `wr` is a bit. -/
theorem ram_wr_bit (h : Mem.everyRow x t i) {L : Nat} (hL : L < 4) :
    (t i).wr L * (1 - (t i).wr L) = 0 := by
  rcases L with _ | _ | _ | _ | L
  · exact h.c11
  · exact h.c12
  · have e := h.c13; simp only [Mem.c13, Mem.Zisk_direct_gsum_e_0] at e; exact e
  · exact h.c14
  · omega

end Lanes

/-! ### Range checks -/

set_option maxRecDepth 100000 in
omit [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks] in
theorem mem_range_max : ∀ d ∈ (Mem.ranges : List RangeDef),
    (d.opid = 105 → d.max = 4194303) ∧ (d.opid = 104 → d.max = 65535) := by
  decide

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A value a sound range check of the RAM sends, on bus 104 or 105. -/
theorem mem_range {x : Mem.Ctx F} {t : Mem.Trace F} {R : BusArg F (Mem.Ctx F) (Mem.Trace F)}
    (hR : RangeHolds Mem.ranges R x t) {j : Int} (hr : R.reps x t j ≠ 0)
    {f : Mem.Ctx F → Mem.Trace F → Int → F} (hf : f ∈ R.tuple) :
    (R.opids = [105] → ∃ n : Nat, n < 2 ^ 22 ∧ f x t j = ((n : Nat) : F)) ∧
      (R.opids = [104] → ∃ n : Nat, n < 2 ^ 16 ∧ f x t j = ((n : Nat) : F)) := by
  obtain ⟨-, -, b, d, hb, hd, hdo, -, -, hall⟩ := hR
  obtain ⟨n, -, hn, e⟩ := hall j hr f hf
  refine ⟨fun hop => ?_, fun hop => ?_⟩
  · rw [hop] at hb
    have hb' : b = 105 := by simpa using hb.symm
    have := (mem_range_max d hd).1 (by rw [hdo, hb'])
    exact ⟨n, by omega, e⟩
  · rw [hop] at hb
    have hb' : b = 104 := by simpa using hb.symm
    have := (mem_range_max d hd).2 (by rw [hdo, hb'])
    exact ⟨n, by omega, e⟩

/-- The range checks of lane `L`: its increment's two chunks, and its dual
read's distance. -/
def lIncArg : Nat → BusArg F (Mem.Ctx F) (Mem.Trace F)
  | 0 => Mem.bus26 | 1 => Mem.bus33 | 2 => Mem.bus40 | _ => Mem.bus47
def hIncArg : Nat → BusArg F (Mem.Ctx F) (Mem.Trace F)
  | 0 => Mem.bus28 | 1 => Mem.bus35 | 2 => Mem.bus42 | _ => Mem.bus49
def dualArg : Nat → BusArg F (Mem.Ctx F) (Mem.Trace F)
  | 0 => Mem.bus31 | 1 => Mem.bus38 | 2 => Mem.bus45 | _ => Mem.bus52

/-- The RAM's per-lane range checks are sound. -/
def RamRanges (x : Mem.Ctx F) (t : Mem.Trace F) : Prop :=
  ∀ L, L < 4 → RangeHolds Mem.ranges (lIncArg L) x t ∧ RangeHolds Mem.ranges (hIncArg L) x t ∧
    RangeHolds Mem.ranges (dualArg L) x t

omit [Lean.Grind.IsCharP F goldilocks] in
/-- What the range checks give at a lane. -/
theorem ram_lane_ranges {x : Mem.Ctx F} {t : Mem.Trace F} (hR : RamRanges x t) {L : Nat}
    (hL : L < 4) (i : Int) :
    (∃ n : Nat, n < 2 ^ 22 ∧ (t i).l_increment L = ((n : Nat) : F)) ∧
    (∃ n : Nat, n < 2 ^ 16 ∧ (t i).h_increment L = ((n : Nat) : F)) ∧
    ((t i).sel_dual L ≠ 0 → ∃ n : Nat, n < 2 ^ 22 ∧
      (t i).step_dual L - (t i).step L - (t i).wr L = ((n : Nat) : F)) := by
  obtain ⟨r1, r2, r3⟩ := hR L hL
  have one : (1 : F) ≠ 0 := one_ne_zero'
  rcases L with _ | _ | _ | _ | L
  all_goals first
    | omega
    | refine ⟨?_, ?_, fun hs => ?_⟩
      · obtain ⟨n, hn, e⟩ := (mem_range r1 (j := i) one (List.mem_singleton_self _)).1 rfl
        exact ⟨n, hn, by simpa [Lean.Grind.Semiring.add_zero] using e⟩
      · obtain ⟨n, hn, e⟩ := (mem_range r2 (j := i) one (List.mem_singleton_self _)).2 rfl
        exact ⟨n, hn, by simpa [Lean.Grind.Semiring.add_zero] using e⟩
      · obtain ⟨n, hn, e⟩ := (mem_range r3 (j := i) (by simpa [dualArg, Mem.bus31, Mem.bus38,
          Mem.bus45, Mem.bus52, Lean.Grind.Semiring.add_zero] using hs) (List.mem_singleton_self _)).1 rfl
        exact ⟨n, hn, by simpa [Lean.Grind.Semiring.add_zero] using e⟩


/-! ### Lanes as naturals -/

omit [Lean.Grind.IsCharP F goldilocks] in
open Classical in
/-- A bit column, as the `bit` of whether it is 1. -/
theorem bit_decide {s : F} (h : s * (1 - s) = 0) : s = bit (decide (s = 1)) := by
  obtain ⟨b, rfl⟩ := bool_of_idem h
  cases b <;> simp [bit, Lean.Grind.Field.zero_ne_one]

/-- The RAM's addresses, steps and dual steps, as naturals below `2^40`: what
the memory bus will supply. -/
structure RamNat (x : Mem.Ctx F) (t : Mem.Trace F) where
  A : Int → Nat → Nat
  S : Int → Nat → Nat
  D : Int → Nat → Nat
  addr : ∀ i L, L < 4 → A i L < 2 ^ 40 ∧ (t i).addr L = ((A i L : Nat) : F)
  step : ∀ i L, L < 4 → S i L < 2 ^ 40 ∧ (t i).step L = ((S i L : Nat) : F)
  dual : ∀ i L, L < 4 → (t i).sel_dual L ≠ 0 → D i L < 2 ^ 40 ∧ (t i).step_dual L = ((D i L : Nat) : F)

open Classical in
/-- Lane `L` of row `i`, as a lane of the sorted log. -/
noncomputable def RamNat.lane {x : Mem.Ctx F} {t : Mem.Trace F} (N : RamNat x t) (i : Int) (L : Nat) :
    Lane (F × F) :=
  ⟨N.A i L, N.S i L, decide ((t i).wr L = 1), ((t i).value L 0, (t i).value L 1),
    if (t i).sel_dual L = 1 then some (N.D i L) else none⟩

section Nat

variable {x : Mem.Ctx F} {t : Mem.Trace F} (N : RamNat x t)

/-- A lane's last step, in the field: what the next lane's `previous_step`
reads. -/
theorem RamNat.last_cast {i : Int} {L : Nat} (hL : L < 4)
    (hsel : (t i).sel_dual L * (1 - (t i).sel_dual L) = 0) :
    (t i).sel_dual L * ((t i).step_dual L - (t i).step L) + (t i).step L =
      (((N.lane i L).last : Nat) : F) ∧ (N.lane i L).last < 2 ^ 40 := by
  have hs := N.step i L hL
  by_cases h1 : (t i).sel_dual L = 1
  · have hd := N.dual i L hL (by rw [h1]; exact one_ne_zero')
    simp only [RamNat.lane, Lane.last, h1, if_true, Option.getD_some]
    rw [hd.2]
    exact ⟨by grind, hd.1⟩
  · have h0 : (t i).sel_dual L = 0 := by
      obtain ⟨b, hb⟩ := bool_of_idem hsel
      cases b <;> simp_all [bit]
    simp only [RamNat.lane, Lane.last, h1, if_false, Option.getD_none]
    rw [h0, hs.2]
    exact ⟨by grind, hs.1⟩

open Classical in
/-- A lane's dual read comes no earlier than its access, strictly later after
a write: its range check. -/
theorem RamNat.lane_ok (hR : RamRanges x t) {i : Int} {L : Nat} (hL : L < 4)
    (hwr : (t i).wr L * (1 - (t i).wr L) = 0) : (N.lane i L).Ok := by
  intro sd hsd
  simp only [RamNat.lane] at hsd
  split at hsd
  · rename_i h1
    cases hsd
    have hd := N.dual i L hL (by rw [h1]; exact one_ne_zero')
    have hs := N.step i L hL
    obtain ⟨-, -, hdr⟩ := ram_lane_ranges hR hL i
    obtain ⟨n, hn, e⟩ := hdr (by rw [h1]; exact one_ne_zero')
    have hw := bit_decide hwr
    generalize hb : decide ((t i).wr L = 1) = b at hw
    simp only [RamNat.lane, hb]
    rw [hd.2, hs.2, hw] at e
    have e' : ((N.D i L : Nat) : F) = ((N.S i L + (if b then 1 else 0) + n : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_add, ← bit_cast]; grind
    have := natCast_inj41 (F := F) (by omega) (by split <;> omega) e'
    omega
  · exact nomatch hsd

open Classical in
/-- **A lane follows the lane before it**, as naturals: from its constraints,
its range checks, and the previous lane's address `PA` and last step `PL`. -/
theorem RamNat.lane_follows (hR : RamRanges x t) {i : Int} {L : Nat} (hL : L < 4)
    {pa v0 v1 : F} (hl : RamLaneHolds (t i) L pa v0 v1) {PA PL : Nat} (hPA : PA < 2 ^ 40)
    (hPL : PL < 2 ^ 40) (hpa : pa = ((PA : Nat) : F))
    (hps : (t i).previous_step L = ((PL : Nat) : F))
    (p : Lane (F × F)) (hpA : p.addr = PA) (hpL : p.last = PL) (hpv : p.val = (v0, v1)) :
    Follows (0, 0) p (N.lane i L) := by
  obtain ⟨⟨l, hl22, el⟩, ⟨h, hh16, eh⟩, -⟩ := ram_lane_ranges hR hL i
  have ha := N.addr i L hL
  have hs := N.step i L hL
  have hac := bit_decide hl.ac
  have hw := bit_decide hl.wr
  have hinc := hl.inc
  have haddr := hl.addr
  have hv0 := hl.val0
  have hv1 := hl.val1
  have hz0 := hl.z0
  have hz1 := hl.z1
  generalize decide ((t i).addr_changes L = 1) = ac at hac
  rw [el, eh, ha.2, hs.2, hps, hpa, hac, hw] at hinc
  rw [ha.2, hpa, hac] at haddr
  rw [hac, hw] at hv0 hv1 hz0 hz1
  have einc : ((l : Nat) : F) + 4194304 * ((h : Nat) : F) = ((l + 4194304 * h : Nat) : F) := by
    rw [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul, natCast_ofNat' 4194304]
  rw [einc] at hinc
  exact follows_cols (N.lane i L).dual hPA hPL ha.1 hs.1 (by omega) hinc haddr hv0 hv1 hz0 hz1
    p hpA hpL hpv

end Nat


/-! ### A segment -/

/-- A segment's lanes, in order: lane `n mod 4` of row `n / 4`. -/
noncomputable def RamNat.lanes {x : Mem.Ctx F} {t : Mem.Trace F} (N : RamNat x t) : List (Lane (F × F)) :=
  (List.range (4 * Mem.numRows)).map fun n => N.lane ((n / 4 : Nat) : Int) (n % 4)

section Segment

variable {x : Mem.Ctx F} {t : Mem.Trace F} (N : RamNat x t)

/-- **Inside a segment, each lane follows the one before.** -/
theorem RamNat.follows_next (hrows : ∀ i, Mem.everyRow x t i) (hR : RamRanges x t)
    (hinner : ∀ i : Int, 0 < i → i < Mem.numRows → (t i).__L1__ = 0)
    {n : Nat} (hn : n + 1 < 4 * Mem.numRows) :
    Follows (0, 0) (N.lane ((n / 4 : Nat) : Int) (n % 4)) (N.lane (((n + 1) / 4 : Nat) : Int) ((n + 1) % 4)) := by
  have h4 : (n + 1) % 4 = 0 ∨ ((n + 1) / 4 = n / 4 ∧ (n + 1) % 4 = n % 4 + 1) := by omega
  rcases h4 with h0 | ⟨hr, hl⟩
  · -- Lane 0 of the next row, after lane 3 of this one.
    have hl3 : n % 4 = 3 := by omega
    rw [hl3, h0]
    have hi : (((n + 1) / 4 : Nat) : Int) - 1 = ((n / 4 : Nat) : Int) := by omega
    have hL1 := hinner (((n + 1) / 4 : Nat) : Int) (by omega) (by simp only [Mem.numRows] at hn ⊢; omega)
    have hrow := hrows (((n + 1) / 4 : Nat) : Int)
    have hl := ramLane_0 hrow
    have hps := ramPrevStep0 hrow
    rw [hL1, hi] at hps
    have hlast := N.last_cast (i := ((n / 4 : Nat) : Int)) (L := 3) (by omega)
      (hrows ((n / 4 : Nat) : Int)).c6
    simp only [Mem.Mem_previous_addr_0, Mem.Mem_previous_value_0_0, Mem.Mem_previous_value_0_1,
      Mem.Zisk_direct_gsum_e_19, hL1, hi, Lean.Grind.Semiring.zero_mul,
      Lean.Grind.Semiring.zero_add] at hl
    simp only [Lean.Grind.Semiring.zero_mul, Lean.Grind.Semiring.zero_add] at hps
    have ha := N.addr ((n / 4 : Nat) : Int) 3 (by omega)
    exact N.lane_follows hR (L := 0) (by omega) hl ha.1 hlast.2 ha.2 (by rw [hps, hlast.1]) _
      rfl rfl rfl
  · -- The next lane of the same row.
    rw [hr, hl]
    have hrow := hrows ((n / 4 : Nat) : Int)
    have hps := ramPrevStep hrow
    have hn4 : n % 4 < 3 := by omega
    generalize n % 4 = L at hn4 ⊢
    generalize ((n / 4 : Nat) : Int) = i at hrow hps ⊢
    rcases L with _ | _ | _ | L
    · have hlast := N.last_cast (i := i) (L := 0) (by omega) hrow.c3
      exact N.lane_follows hR (by omega) (ramLane_1 hrow) (N.addr i 0 (by omega)).1 hlast.2
        (N.addr i 0 (by omega)).2 (by rw [hps.1, hlast.1]) _ rfl rfl rfl
    · have hlast := N.last_cast (i := i) (L := 1) (by omega) hrow.c4
      exact N.lane_follows hR (by omega) (ramLane_2 hrow) (N.addr i 1 (by omega)).1 hlast.2
        (N.addr i 1 (by omega)).2 (by rw [hps.2.1, hlast.1]) _ rfl rfl rfl
    · have hlast := N.last_cast (i := i) (L := 2) (by omega) hrow.c5
      exact N.lane_follows hR (by omega) (ramLane_3 hrow) (N.addr i 2 (by omega)).1 hlast.2
        (N.addr i 2 (by omega)).2 (by rw [hps.2.2, hlast.1]) _ rfl rfl rfl
    · omega

/-- The first lane's constraints, on the first row: against the previous
segment's air values. -/
theorem ramLane_first (hrows : ∀ i, Mem.everyRow x t i) (hfirst : (t 0).__L1__ = 1) :
    RamLaneHolds (t 0) 0 (x.Mem_previous_segment_addr - x.Mem_is_first_segment)
      (x.Mem_previous_segment_value 0) (x.Mem_previous_segment_value 1) ∧
      (t 0).previous_step 0 = x.Mem_previous_segment_step := by
  have hl := ramLane_0 (hrows 0)
  have hps := ramPrevStep0 (hrows 0)
  simp only [Mem.Mem_previous_addr_0, Mem.Mem_previous_value_0_0, Mem.Mem_previous_value_0_1,
    Mem.Zisk_direct_gsum_e_19, hfirst, Lean.Grind.Semiring.one_mul] at hl
  rw [hfirst, Lean.Grind.Semiring.one_mul] at hps
  have e1 : ∀ a b : F, a - b + b = a := fun a b => by grind
  simp only [e1] at hl hps
  exact ⟨hl, hps⟩

/-- A run of `m` lanes after a first one, each following the one before:
the log `[f 0, …, f m]`, stated for any `m` so that nothing unfolds the
segment's 2^24 lanes. -/
theorem sorted_run {f : Nat → Lane (F × F)} {m : Nat} (hok : ∀ k, k ≤ m → (f k).Ok)
    (hnext : ∀ k, k < m → Follows (0, 0) (f k) (f (k + 1))) {L : List (Lane (F × F))}
    (h : Sorted (0, 0) (L ++ [f 0])) :
    Sorted (0, 0) (L ++ (List.range (m + 1)).map f) ∧
      (L ++ (List.range (m + 1)).map f).getLast? = some (f m) := by
  obtain ⟨hs, hlast⟩ := Sorted.extend (fun k => f (k + 1)) m h (by simp)
    (fun k hk => hok (k + 1) (by omega)) (fun hm => hnext 0 hm) (fun k hk => hnext (k + 1) (by omega))
  have e : L ++ (List.range (m + 1)).map f = L ++ [f 0] ++ (List.range m).map (fun k => f (k + 1)) := by
    rw [List.range_succ_eq_map]
    simp [List.map_map, Function.comp_def]
  rw [e]
  refine ⟨hs, ?_⟩
  rw [hlast]
  cases m with
  | zero => rfl
  | succ m => simp

/-- **The first segment's lanes are a sorted log.** -/
theorem RamNat.sorted_first (hrows : ∀ i, Mem.everyRow x t i) (hR : RamRanges x t)
    (hfirst : (t 0).__L1__ = 1) (hinner : ∀ i : Int, 0 < i → i < Mem.numRows → (t i).__L1__ = 0)
    (hf1 : x.Mem_is_first_segment = 1) :
    Sorted (0, 0) N.lanes ∧
      N.lanes.getLast? = some (N.lane ((Mem.numRows - 1 : Nat) : Int) 3) := by
  have hN : 0 < Mem.numRows := by simp [Mem.numRows]
  let f : Nat → Lane (F × F) := fun n => N.lane ((n / 4 : Nat) : Int) (n % 4)
  -- The first lane changes address, so a read there returns 0.
  obtain ⟨hl, -⟩ := ramLane_first hrows hfirst
  have hac : (t 0).addr_changes 0 = 1 := by
    have e := (hrows 0).c33; simp only [Mem.c33, hf1, hfirst] at e; grind
  have hone : Sorted (0, 0) ([] ++ [f 0]) := by
    refine Sorted.one _ (N.lane_ok hR (by omega) (hrows 0).c11) fun hw => ?_
    simp only [f, RamNat.lane, Nat.zero_div, Nat.zero_mod, Int.natCast_zero,
      decide_eq_false_iff_not] at hw ⊢
    have hw0 : (t 0).wr 0 = 0 := by
      obtain ⟨b, hb⟩ := bool_of_idem (hrows 0).c11
      cases b <;> simp_all [bit]
    have z0 := hl.z0; have z1 := hl.z1
    rw [hac, hw0] at z0 z1
    apply Prod.ext <;> simp only <;> grind
  have hm : 4 * Mem.numRows = (4 * Mem.numRows - 1) + 1 := by omega
  have hidx : (4 * Mem.numRows - 1) / 4 = Mem.numRows - 1 ∧ (4 * Mem.numRows - 1) % 4 = 3 := by omega
  obtain ⟨hs, hlast⟩ := sorted_run (f := f) (m := 4 * Mem.numRows - 1)
    (fun k _ => N.lane_ok hR (Nat.mod_lt _ (by omega)) (ram_wr_bit (hrows _) (Nat.mod_lt _ (by omega))))
    (fun k hk => N.follows_next hrows hR hinner (n := k) (by omega)) hone
  rw [List.nil_append, ← hm] at hs hlast
  refine ⟨hs, ?_⟩
  change (List.map f (List.range (4 * Mem.numRows))).getLast? = _
  rw [hlast]
  simp only [f, hidx]

/-- **A later segment's lanes extend the log**, from the lane its air values
name as the one before. -/
theorem RamNat.sorted_next (hrows : ∀ i, Mem.everyRow x t i) (hR : RamRanges x t)
    (hfirst : (t 0).__L1__ = 1) (hinner : ∀ i : Int, 0 < i → i < Mem.numRows → (t i).__L1__ = 0)
    (hf0 : x.Mem_is_first_segment = 0) {L : List (Lane (F × F))} {p : Lane (F × F)}
    (h : Sorted (0, 0) L) (hl : L.getLast? = some p) (hpA : p.addr < 2 ^ 40) (hpL : p.last < 2 ^ 40)
    (hpa : ((p.addr : Nat) : F) = x.Mem_previous_segment_addr)
    (hpl : ((p.last : Nat) : F) = x.Mem_previous_segment_step)
    (hpv : p.val = (x.Mem_previous_segment_value 0, x.Mem_previous_segment_value 1)) :
    Sorted (0, 0) (L ++ N.lanes) ∧
      (L ++ N.lanes).getLast? = some (N.lane ((Mem.numRows - 1 : Nat) : Int) 3) := by
  have hN : 0 < Mem.numRows := by simp [Mem.numRows]
  let f : Nat → Lane (F × F) := fun n => N.lane ((n / 4 : Nat) : Int) (n % 4)
  obtain ⟨hl0, hps⟩ := ramLane_first hrows hfirst
  have hpa' : x.Mem_previous_segment_addr - x.Mem_is_first_segment = ((p.addr : Nat) : F) := by
    rw [hf0, hpa]; grind
  have hf : Follows (0, 0) p (f 0) :=
    N.lane_follows hR (i := 0) (L := 0) (by omega) hl0 hpA hpL hpa' (by rw [hps, hpl]) p rfl rfl hpv
  have hone : Sorted (0, 0) (L ++ [f 0]) := h.snoc' hl (N.lane_ok hR (by omega) (hrows 0).c11) hf
  have hm : 4 * Mem.numRows = (4 * Mem.numRows - 1) + 1 := by omega
  have hidx : (4 * Mem.numRows - 1) / 4 = Mem.numRows - 1 ∧ (4 * Mem.numRows - 1) % 4 = 3 := by omega
  obtain ⟨hs, hlast⟩ := sorted_run (f := f) (m := 4 * Mem.numRows - 1)
    (fun k _ => N.lane_ok hR (Nat.mod_lt _ (by omega)) (ram_wr_bit (hrows _) (Nat.mod_lt _ (by omega))))
    (fun k hk => N.follows_next hrows hR hinner (n := k) (by omega)) hone
  rw [← hm] at hs hlast
  refine ⟨hs, ?_⟩
  change (L ++ List.map f (List.range (4 * Mem.numRows))).getLast? = _
  rw [hlast]
  simp only [f, hidx]

/-- The last lane is what the exit air values hand on. -/
theorem RamNat.last_lane (hrows : ∀ i, Mem.everyRow x t i) (hcyc : Mem.cyclic t)
    (hfirst : (t 0).__L1__ = 1) :
    (((N.lane ((Mem.numRows - 1 : Nat) : Int) 3).addr : Nat) : F) = x.Mem_segment_last_addr ∧
      (((N.lane ((Mem.numRows - 1 : Nat) : Int) 3).last : Nat) : F) = x.Mem_segment_last_step ∧
      (N.lane ((Mem.numRows - 1 : Nat) : Int) 3).val =
        (x.Mem_segment_last_value 0, x.Mem_segment_last_value 1) ∧
      (N.lane ((Mem.numRows - 1 : Nat) : Int) 3).addr < 2 ^ 40 ∧
      (N.lane ((Mem.numRows - 1 : Nat) : Int) 3).last < 2 ^ 40 := by
  have hwrap : (t (((Mem.numRows - 1 : Nat) : Int) + 1)).__L1__ = 1 := by
    have := hcyc 0
    rw [Int.zero_add] at this
    have e : ((Mem.numRows - 1 : Nat) : Int) + 1 = (Mem.numRows : Int) := by simp [Mem.numRows]
    rw [e, this, hfirst]
  have hrow := hrows ((Mem.numRows - 1 : Nat) : Int)
  have e15 := hrow.c15; have e16 := hrow.c16; have e17 := hrow.c17; have e18 := hrow.c18
  simp only [Mem.c15, Mem.c16, Mem.c17, Mem.c18, Mem.Zisk_direct_gsum_e_1, hwrap,
    Lean.Grind.Semiring.one_mul] at e15 e16 e17 e18
  have hlast := N.last_cast (i := ((Mem.numRows - 1 : Nat) : Int)) (L := 3) (by omega) hrow.c6
  have ha := N.addr ((Mem.numRows - 1 : Nat) : Int) 3 (by omega)
  refine ⟨?_, ?_, ?_, ha.1, hlast.2⟩
  · show ((N.A _ 3 : Nat) : F) = _
    rw [← ha.2]; grind
  · rw [← hlast.1]; grind
  · show ((t _).value 3 0, (t _).value 3 1) = _
    apply Prod.ext <;> simp only <;> grind

end Segment


/-! ### The chain of segments -/

/-- One segment of the RAM: its trace, its constraints at every row, `__L1__`
as the setup fixes it, its range checks sound, and its addresses and steps as
naturals. -/
structure RamSeg (F : Type) [Lean.Grind.Field F] where
  x : Mem.Ctx F
  t : Mem.Trace F
  rows : ∀ i, Mem.everyRow x t i
  cyclic : Mem.cyclic t
  first : (t 0).__L1__ = 1
  inner : ∀ i : Int, 0 < i → i < Mem.numRows → (t i).__L1__ = 0
  ranges : RamRanges x t
  nat : RamNat x t

/-- What the RAM's continuation bus guarantees: the first segment is marked
first, and each later one is not, and starts from the air values the one
before hands on. -/
structure RamChain (segs : List (RamSeg F)) : Prop where
  first : ∀ h : 0 < segs.length, segs[0].x.Mem_is_first_segment = 1
  next : ∀ k (h : k + 1 < segs.length),
    segs[k + 1].x.Mem_is_first_segment = 0 ∧
      segs[k + 1].x.Mem_previous_segment_addr = segs[k].x.Mem_segment_last_addr ∧
      segs[k + 1].x.Mem_previous_segment_step = segs[k].x.Mem_segment_last_step ∧
      segs[k + 1].x.Mem_previous_segment_value 0 = segs[k].x.Mem_segment_last_value 0 ∧
      segs[k + 1].x.Mem_previous_segment_value 1 = segs[k].x.Mem_segment_last_value 1

/-- The RAM's log: every segment's lanes, in order. -/
noncomputable def ramLog (segs : List (RamSeg F)) : List (Lane (F × F)) :=
  segs.flatMap fun G => G.nat.lanes

/-- The last lane of a segment. -/
noncomputable def RamSeg.lastLane (G : RamSeg F) : Lane (F × F) :=
  G.nat.lane ((Mem.numRows - 1 : Nat) : Int) 3

/-- **The RAM's segments make one sorted log.** -/
theorem ramLog_sorted {segs : List (RamSeg F)} (hc : RamChain segs) :
    ∀ k (hk : k < segs.length), Sorted (0, 0) (ramLog (segs.take (k + 1))) ∧
      (ramLog (segs.take (k + 1))).getLast? = some segs[k].lastLane
  | 0, hk => by
    have e : ramLog (segs.take 1) = segs[0].nat.lanes := by
      cases segs with
      | nil => exact absurd hk (by simp)
      | cons G gs => simp [ramLog]
    rw [e]
    exact segs[0].nat.sorted_first segs[0].rows segs[0].ranges segs[0].first segs[0].inner
      (hc.first hk)
  | k + 1, hk => by
    obtain ⟨ih, ihl⟩ := ramLog_sorted hc k (by omega)
    obtain ⟨hf0, ha, hs, hv0, hv1⟩ := hc.next k hk
    obtain ⟨la, ll, lv, lA, lL⟩ := segs[k].nat.last_lane segs[k].rows segs[k].cyclic segs[k].first
    have e : ramLog (segs.take (k + 2)) = ramLog (segs.take (k + 1)) ++ segs[k + 1].nat.lanes := by
      rw [List.take_succ, ramLog, List.flatMap_append, List.getElem?_eq_getElem hk]
      simp [ramLog]
    rw [e]
    exact segs[k + 1].nat.sorted_next segs[k + 1].rows segs[k + 1].ranges segs[k + 1].first
      segs[k + 1].inner hf0 ih ihl lA lL (by rw [ha]; exact la) (by rw [hs]; exact ll)
      (by rw [hv0, hv1]; exact lv)

/-- **Every read of the RAM returns the last write.** Along a chain of RAM
segments, every read the log emits returns the value of the latest write to
its address at or before its step, or, if there is none, 0. -/
theorem ram_reads_last_write {segs : List (RamSeg F)} (hc : RamChain segs) :
    ∀ e ∈ reads (ramLog segs), ReadsLatest (0, 0) (writes (ramLog segs)) e := by
  cases hl : segs.length with
  | zero =>
    have : segs = [] := List.eq_nil_of_length_eq_zero hl
    subst this
    intro e he
    simp [ramLog, reads] at he
  | succ k =>
    have := (ramLog_sorted hc k (by omega)).1
    rw [List.take_of_length_le (by omega)] at this
    exact reads_last_write this

end Correspondence
