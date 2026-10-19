/-
  BinaryAdd computes `add`.

  An `add` in Main is proved on the operation bus by one of three kinds of AIR
  (see "Who can prove an add" in the README). This file is the first of them,
  the dedicated adder: whenever BinaryAdd proves a tuple whose op is `add`, the
  `c` it proves is `a + b` modulo 2^64, limb by limb.

  BinaryAdd's own constraints carry the addition through two 32-bit limbs with
  a carry bit between them; each `c` limb is two 16-bit chunks. The chunks'
  range is not a constraint but a range-check bus argument, so it enters as
  `RangeHolds`: what a sound range check guarantees, with the range taken from
  the AIR's own generated `range_def` list rather than written down here.

  The operands are the model's: `a` and `b` are the limbs of two 64-bit values.
  In the AIR that is the job of the lookup that brought the tuple from Main,
  whose `a` and `b` columns are the model's sources.
-/
import Proofs.Rom
import Pil.BinaryAdd

open Correspondence

namespace Correspondence

section Range

variable {F C T : Type} [PilField F]

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- What a sound range check guarantees: whenever `a` sends its value, the
value is a number in the range its bus serves. The range must be one of the
AIR's generated `ranges`, on the argument's single bus, with no negated bound. -/
def RangeHolds (ranges : List RangeDef) (a : BusArg F C T) (x : C) (t : T) : Prop :=
  a.piop = .rangeCheck ∧ a.side = .assumes ∧
    ∃ b d, a.opids = [b] ∧ d ∈ ranges ∧ d.opid = b ∧ d.minNeg = false ∧ d.maxNeg = false ∧
      ∀ i, a.reps x t i ≠ 0 → ∀ f ∈ a.tuple, ∃ n : Nat, d.min ≤ n ∧ n ≤ d.max ∧ f x t i = (n : F)

end Range

section Adder

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

open Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A range-checked chunk of BinaryAdd is a number below 2^16. -/
theorem chunk_lt {a : BusArg F (BinaryAdd.Ctx F) (BinaryAdd.Trace F)} {x t}
    (h : RangeHolds BinaryAdd.ranges a x t) (ha : a.reps = fun _ _ _ => 1)
    (v : BinaryAdd.Ctx F → BinaryAdd.Trace F → Int → F) (hv : a.tuple = [v]) (i : Int) :
    ∃ n : Nat, n < 2 ^ 16 ∧ v x t i = (n : F) := by
  obtain ⟨-, -, b, d, -, hd, -, -, -, hall⟩ := h
  have hmax : d.max = 65535 := by
    simp only [BinaryAdd.ranges, List.mem_cons, List.not_mem_nil, or_false] at hd
    rcases hd with rfl | rfl | rfl | rfl <;> rfl
  obtain ⟨n, -, hn, hv'⟩ := hall i (by rw [ha]; exact fun h => Lean.Grind.Field.zero_ne_one h.symm)
    v (by rw [hv]; exact List.mem_singleton_self v)
  exact ⟨n, by omega, hv'⟩

/- Numerals are rewritten to casts of naturals only where `natCast_eq_iff_of_lt`
needs them; see `natCast_two` in `Rom.lean` for why `grind` is kept away from
mixing the two. -/
omit [Lean.Grind.IsCharP F goldilocks] in
theorem natCast_ofNat' (n : Nat) : (OfNat.ofNat (α := F) n) = ((n : Nat) : F) :=
  Lean.Grind.Semiring.ofNat_eq_natCast n

/-- Two naturals below the characteristic that are equal in the field are equal. -/
theorem natCast_inj {x y : Nat} (hx : x < goldilocks) (hy : y < goldilocks)
    (h : (x : F) = (y : F)) : x = y :=
  (Lean.Grind.IsCharP.natCast_eq_iff_of_lt (p := goldilocks) hx hy).mp h

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A bit, as the natural number it casts from. -/
theorem bit_cast (b : Bool) : (bit b : F) = ((if b then 1 else 0 : Nat) : F) := by
  cases b <;> simp [bit, Lean.Grind.Semiring.natCast_zero, Lean.Grind.Semiring.natCast_one]

/-- **BinaryAdd adds.** At a row where lane 0 proves an `add` (its op element is
10, so `sel_sh3add` is clear) on operands that are the limbs of `A` and `B`, the
`c` limbs it proves are those of `A + B`. -/
theorem binaryAdd_add (x : BinaryAdd.Ctx F) (t : BinaryAdd.Trace F) (i : Int)
    (h : BinaryAdd.everyRow x t i)
    (r6 : RangeHolds BinaryAdd.ranges BinaryAdd.bus6 x t)
    (r8 : RangeHolds BinaryAdd.ranges BinaryAdd.bus8 x t)
    (r10 : RangeHolds BinaryAdd.ranges BinaryAdd.bus10 x t)
    (r12 : RangeHolds BinaryAdd.ranges BinaryAdd.bus12 x t)
    (hop : 10 + (t i).sel_sh3add 0 * 78 = (10 : F))
    (A B : BitVec 64)
    (ha : (t i).a 0 0 = lo A ∧ (t i).a 0 1 = hi A)
    (hb : (t i).b 0 0 = lo B ∧ (t i).b 0 1 = hi B) :
    (t i).c_chunks 0 1 * 65536 + (t i).c_chunks 0 0 = lo (A + B) ∧
      (t i).c_chunks 0 3 * 65536 + (t i).c_chunks 0 2 = hi (A + B) := by
  -- `add` is op 10, so the SH3ADD selector is clear: 78 is not zero in the field.
  have hs : (t i).sel_sh3add 0 = 0 := by
    have h78 : (78 : F) ≠ 0 := by
      rw [natCast_ofNat' 78, ← Lean.Grind.Semiring.natCast_zero]
      intro h; have := natCast_inj (F := F) (by decide) (by decide) h; omega
    have : (t i).sel_sh3add 0 * 78 = 0 := by grind
    have := congrArg (fun y => y * (78 : F)⁻¹) this
    simp only at this
    grind
  -- Each chunk is a number below 2^16, by the range checks.
  obtain ⟨n0, l0, e0⟩ := chunk_lt r6 rfl _ rfl i
  obtain ⟨n1, l1, e1⟩ := chunk_lt r8 rfl _ rfl i
  obtain ⟨n2, l2, e2⟩ := chunk_lt r10 rfl _ rfl i
  obtain ⟨n3, l3, e3⟩ := chunk_lt r12 rfl _ rfl i
  -- Each carry is a bit.
  obtain ⟨k0, ek0⟩ := bool_of_idem (by have := h.c2; simpa [BinaryAdd.c2] using this)
  obtain ⟨k1, ek1⟩ := bool_of_idem (by have := h.c5; simpa [BinaryAdd.c5] using this)
  rw [bit_cast] at ek0 ek1
  generalize hk0 : (if k0 then 1 else 0 : Nat) = c0 at ek0
  generalize hk1 : (if k1 then 1 else 0 : Nat) = c1 at ek1
  have b0 : c0 ≤ 1 := by rw [← hk0]; split <;> omega
  have b1 : c1 ≤ 1 := by rw [← hk1]; split <;> omega
  -- The two limb equations, as equalities of naturals.
  have q1 := h.c1
  have q4 := h.c4
  simp only [BinaryAdd.c1, BinaryAdd.c4, BinaryAdd.BinaryAdd_c_0_0, BinaryAdd.BinaryAdd_c_0_1]
    at q1 q4
  obtain ⟨ha0, ha1⟩ := ha
  obtain ⟨hb0, hb1⟩ := hb
  have hc0 : (t i).c_chunks 0 0 = (n0 : F) := by grind
  have hc1 : (t i).c_chunks 0 1 = (n1 : F) := by grind
  have hc2 : (t i).c_chunks 0 2 = (n2 : F) := by grind
  have hc3 : (t i).c_chunks 0 3 = (n3 : F) := by grind
  simp only [hs, ha0, ha1, hb0, hb1, ek0, ek1, hc0, hc1, hc2, hc3] at q1 q4
  have hA := A.isLt
  have hB := B.isLt
  have N1 : A.toNat % 2 ^ 32 + B.toNat % 2 ^ 32 = c0 * 4294967296 + (n1 * 65536 + n0) := by
    apply natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega)
    simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul]
    rw [← natCast_ofNat' 4294967296, ← natCast_ofNat' 65536]
    simp only [lo] at q1
    grind
  have N2 : A.toNat / 2 ^ 32 + B.toNat / 2 ^ 32 + c0 = c1 * 4294967296 + (n3 * 65536 + n2) := by
    apply natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega)
    simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul]
    rw [← natCast_ofNat' 4294967296, ← natCast_ofNat' 65536]
    simp only [hi] at q4
    grind
  -- What remains is arithmetic on naturals.
  have hsum : (A + B).toNat = (A.toNat + B.toNat) % 2 ^ 64 := BitVec.toNat_add A B
  have m0 : n1 * 65536 + n0 = (A + B).toNat % 2 ^ 32 := by omega
  have m1 : n3 * 65536 + n2 = (A + B).toNat / 2 ^ 32 := by omega
  constructor
  · rw [hc0, hc1, lo, ← m0, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
      ← natCast_ofNat' 65536]
  · rw [hc2, hc3, hi, ← m1, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
      ← natCast_ofNat' 65536]

end Adder

end Correspondence
