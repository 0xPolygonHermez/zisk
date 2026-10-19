/-
  Every adder proves `add` correctly.

  The operation bus has three kinds of prover able to prove an `add` (see "Who
  can prove an add" in the README). This file covers the dedicated adders, six
  AIRs from two templates, 21 lanes in all:

  * `BinaryAdd`, `BinaryAddLarge`, `BinaryAddHuge` (1, 2, 4 lanes): a full
    64-bit addition through two 32-bit limbs and a carry bit.
  * `BinaryAddHi`, `BinaryAddHiLarge`, `BinaryAddHiHuge` (2, 4, 8 lanes): an
    addition whose result fits 32 bits, `b`'s high half either 0 or
    `0xFFFF_FFFF` (`sel_b_hi_is_ff`), in which case the low limb must carry and
    the high halves wrap to 0.

  The statement every `add` prover shares is `AddsAt p j`: if prover `p`'s
  tuple at row `j` has op 10, and its `a` and `b` limbs are those of two 64-bit
  values, then its `c` limbs are those of their sum and its flag is 0. It is
  stated over tuple elements, positions 0 to 7 of `(op, a₀, a₁, b₀, b₁, c₀, c₁,
  flag)`, which is what the lookup from Main compares, so the `add` pilot needs
  nothing else from a prover.

  Each template's algebra is one lemma over plain field values (`adder64`,
  `adderHi`); each lane is a short instance naming that lane's constraints and
  range checks. Those instances were written out mechanically from the
  generated AIRs, like the exclusions in `OpBus.lean`.
-/
import Proofs.Rom
import Proofs.Bus
import Pil.BinaryAdd
import Pil.BinaryAddLarge
import Pil.BinaryAddHuge
import Pil.BinaryAddHi
import Pil.BinaryAddHiLarge
import Pil.BinaryAddHiHuge

namespace Correspondence

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

section Statement

variable {F : Type} [PilField F]

/-- Prover `p` proves `add` correctly at row `j`. -/
def AddsAt (p : Prover F) (j : Int) : Prop :=
  tupleAt p.arg.tuple p.x p.t j 0 = 10 → ∀ A B : BitVec 64,
    tupleAt p.arg.tuple p.x p.t j 1 = lo A → tupleAt p.arg.tuple p.x p.t j 2 = hi A →
    tupleAt p.arg.tuple p.x p.t j 3 = lo B → tupleAt p.arg.tuple p.x p.t j 4 = hi B →
    tupleAt p.arg.tuple p.x p.t j 5 = lo (A + B) ∧ tupleAt p.arg.tuple p.x p.t j 6 = hi (A + B) ∧
      tupleAt p.arg.tuple p.x p.t j 7 = 0

end Statement

section Algebra

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

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

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A range-checked value, on an AIR whose range checks all serve 16-bit
ranges, is a number below 2^16. -/
theorem chunk16 {C T : Type} {ranges : List RangeDef} {a : BusArg F C T} {x : C} {t : T}
    (hmax : ∀ d ∈ ranges, d.max = 65535)
    (h : RangeHolds ranges a x t) (ha : a.reps = fun _ _ _ => 1)
    (v : C → T → Int → F) (hv : a.tuple = [v]) (i : Int) :
    ∃ n : Nat, n < 2 ^ 16 ∧ v x t i = (n : F) := by
  obtain ⟨-, -, b, d, -, hd, -, -, -, hall⟩ := h
  obtain ⟨n, -, hn, hv'⟩ := hall i (by rw [ha]; exact fun h => Lean.Grind.Field.zero_ne_one h.symm)
    v (by rw [hv]; exact List.mem_singleton_self v)
  exact ⟨n, by have := hmax d hd; omega, hv'⟩

/-- 10 + 78·s = 10 makes `s` zero: 78 is not zero in the field. -/
theorem sel_zero {s : F} (h : 10 + s * 78 = 10) : s = 0 := by
  have h78 : (78 : F) ≠ 0 := by
    rw [natCast_ofNat' 78, ← Lean.Grind.Semiring.natCast_zero]
    intro h; have := natCast_inj (F := F) (by decide) (by decide) h; omega
  have : s * 78 = 0 := by grind
  have := congrArg (fun y => y * (78 : F)⁻¹) this
  simp only at this
  grind

/-- The algebra of a `BinaryAdd` lane: two limbs, a carry bit between them, each
result limb two 16-bit chunks. -/
theorem adder64 (a0 a1 b0 b1 s k0 k1 h0 h1 h2 h3 : F) (A B : BitVec 64)
    (hop : 10 + s * 78 = 10) (hk0 : k0 * (1 - k0) = 0) (hk1 : k1 * (1 - k1) = 0)
    (e1 : a0 + b0 + s * a0 * 7 - (k0 * 4294967296 + (h1 * 65536 + h0)) = 0)
    (e2 : a1 + b1 + k0 - (k1 * 4294967296 + (h3 * 65536 + h2)) = 0)
    (r0 : ∃ n : Nat, n < 2 ^ 16 ∧ h0 + 0 = (n : F)) (r1 : ∃ n : Nat, n < 2 ^ 16 ∧ h1 + 0 = (n : F))
    (r2 : ∃ n : Nat, n < 2 ^ 16 ∧ h2 + 0 = (n : F)) (r3 : ∃ n : Nat, n < 2 ^ 16 ∧ h3 + 0 = (n : F))
    (ha0 : a0 = lo A) (ha1 : a1 = hi A) (hb0 : b0 = lo B) (hb1 : b1 = hi B) :
    h1 * 65536 + h0 = lo (A + B) ∧ h3 * 65536 + h2 = hi (A + B) := by
  have hs := sel_zero hop
  obtain ⟨n0, l0, e0⟩ := r0
  obtain ⟨n1, l1, e1'⟩ := r1
  obtain ⟨n2, l2, e2'⟩ := r2
  obtain ⟨n3, l3, e3⟩ := r3
  have hc0 : h0 = (n0 : F) := by grind
  have hc1 : h1 = (n1 : F) := by grind
  have hc2 : h2 = (n2 : F) := by grind
  have hc3 : h3 = (n3 : F) := by grind
  obtain ⟨q0, eq0⟩ := bool_of_idem hk0
  obtain ⟨q1, eq1⟩ := bool_of_idem hk1
  rw [bit_cast] at eq0 eq1
  generalize hq0 : (if q0 then 1 else 0 : Nat) = c0 at eq0
  generalize hq1 : (if q1 then 1 else 0 : Nat) = c1 at eq1
  have b0' : c0 ≤ 1 := by rw [← hq0]; split <;> omega
  have b1' : c1 ≤ 1 := by rw [← hq1]; split <;> omega
  subst hs ha0 ha1 hb0 hb1 eq0 eq1 hc0 hc1 hc2 hc3
  have hA := A.isLt
  have hB := B.isLt
  have N1 : A.toNat % 2 ^ 32 + B.toNat % 2 ^ 32 = c0 * 4294967296 + (n1 * 65536 + n0) := by
    apply natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega)
    simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul]
    rw [← natCast_ofNat' 4294967296, ← natCast_ofNat' 65536]
    simp only [lo] at e1
    grind
  have N2 : A.toNat / 2 ^ 32 + B.toNat / 2 ^ 32 + c0 = c1 * 4294967296 + (n3 * 65536 + n2) := by
    apply natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega)
    simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul]
    rw [← natCast_ofNat' 4294967296, ← natCast_ofNat' 65536]
    simp only [hi] at e2
    grind
  have hsum : (A + B).toNat = (A.toNat + B.toNat) % 2 ^ 64 := BitVec.toNat_add A B
  have m0 : n1 * 65536 + n0 = (A + B).toNat % 2 ^ 32 := by omega
  have m1 : n3 * 65536 + n2 = (A + B).toNat / 2 ^ 32 := by omega
  constructor
  · rw [lo, ← m0, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
      ← natCast_ofNat' 65536]
  · rw [hi, ← m1, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
      ← natCast_ofNat' 65536]

/-- The algebra of a `BinaryAddHi` lane: `a` has a zero high half, `b`'s high
half is `f · 0xFFFF_FFFF` with `f` a bit, and the 32-bit result carries exactly
when `f` is set. -/
theorem adderHi (a b s f h0 h1 : F) (A B : BitVec 64)
    (hop : 10 + s * 78 = 10) (hf : f * (1 - f) = 0)
    (e0 : (1 - f) * (a + s * a * 7 + b - h1 * 65536 - h0) = 0)
    (e1 : f * (a + s * a * 7 + b - h1 * 65536 - h0 - 4294967296) = 0)
    (r0 : ∃ n : Nat, n < 2 ^ 16 ∧ h0 + 0 = (n : F)) (r1 : ∃ n : Nat, n < 2 ^ 16 ∧ h1 + 0 = (n : F))
    (ha0 : a = lo A) (ha1 : (0 : F) = hi A) (hb0 : b = lo B) (hb1 : f * 4294967295 = hi B) :
    h1 * 65536 + h0 = lo (A + B) ∧ (0 : F) = hi (A + B) := by
  have hs := sel_zero hop
  obtain ⟨n0, l0, e0'⟩ := r0
  obtain ⟨n1, l1, e1'⟩ := r1
  have hc0 : h0 = (n0 : F) := by grind
  have hc1 : h1 = (n1 : F) := by grind
  obtain ⟨q, eq⟩ := bool_of_idem hf
  have hA := A.isLt
  have hB := B.isLt
  have hsum : (A + B).toNat = (A.toNat + B.toNat) % 2 ^ 64 := BitVec.toNat_add A B
  -- `a` has no high half.
  have hA1 : A.toNat / 2 ^ 32 = 0 := by
    apply natCast_inj (F := F) (by unfold goldilocks; omega) (by decide)
    rw [Lean.Grind.Semiring.natCast_zero, ← hi, ← ha1]
  subst hs ha0 hb0 hc0 hc1
  cases q
  · -- No carry: `b` has no high half either, and `a + b` fits 32 bits.
    simp only [bit, Bool.false_eq_true, if_false] at eq
    subst eq
    have hB1 : B.toNat / 2 ^ 32 = 0 := by
      apply natCast_inj (F := F) (by unfold goldilocks; omega) (by decide)
      rw [Lean.Grind.Semiring.natCast_zero, ← hi, ← hb1]; grind
    have N : A.toNat % 2 ^ 32 + B.toNat % 2 ^ 32 = n1 * 65536 + n0 := by
      apply natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega)
      simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul]
      rw [← natCast_ofNat' 65536]
      simp only [lo] at e0
      grind
    have m0 : n1 * 65536 + n0 = (A + B).toNat % 2 ^ 32 := by omega
    have m1 : (A + B).toNat / 2 ^ 32 = 0 := by omega
    constructor
    · rw [lo, ← m0, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
        ← natCast_ofNat' 65536]
    · rw [hi, m1, Lean.Grind.Semiring.natCast_zero]
  · -- A carry: `b`'s high half is all ones, and the high halves wrap to 0.
    simp only [bit, if_true] at eq
    subst eq
    have hB1 : B.toNat / 2 ^ 32 = 4294967295 := by
      apply natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega)
      rw [← hi, ← hb1, ← natCast_ofNat' 4294967295]; grind
    have N : A.toNat % 2 ^ 32 + B.toNat % 2 ^ 32 = n1 * 65536 + n0 + 4294967296 := by
      apply natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega)
      simp only [Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul]
      rw [← natCast_ofNat' 65536, ← natCast_ofNat' 4294967296]
      simp only [lo] at e1
      grind
    have m0 : n1 * 65536 + n0 = (A + B).toNat % 2 ^ 32 := by omega
    have m1 : (A + B).toNat / 2 ^ 32 = 0 := by omega
    constructor
    · rw [lo, ← m0, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
        ← natCast_ofNat' 65536]
    · rw [hi, m1, Lean.Grind.Semiring.natCast_zero]

end Algebra

section Lanes

/-! ### One instance per lane

Each names its lane's op-bus argument, limb constraints, intermediates and
range checks. The range checks are the hypotheses: one `RangeHolds` per
16-bit chunk the lane's `c` is built from. -/

open Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

theorem BinaryAdd_bus13_adds {x : BinaryAdd.Ctx F} {t : BinaryAdd.Trace F}
    (h : ∀ j, BinaryAdd.everyRow x t j)
    (r0 : RangeHolds BinaryAdd.ranges BinaryAdd.bus6 x t) (r1 : RangeHolds BinaryAdd.ranges BinaryAdd.bus8 x t)
    (r2 : RangeHolds BinaryAdd.ranges BinaryAdd.bus10 x t) (r3 : RangeHolds BinaryAdd.ranges BinaryAdd.bus12 x t) :
    ∀ j, AddsAt ⟨BinaryAdd.bus13, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAdd.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAdd.bus13, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e1 := (h j).c1
  have e2 := (h j).c4
  simp only [BinaryAdd.c1, BinaryAdd.c4, BinaryAdd.BinaryAdd_c_0_0, BinaryAdd.BinaryAdd_c_0_1] at e1 e2
  obtain ⟨c0, c1⟩ := adder64 ((t j).a 0 0) ((t j).a 0 1) ((t j).b 0 0) ((t j).b 0 1)
    ((t j).sel_sh3add 0) ((t j).cout 0 0) ((t j).cout 0 1) ((t j).c_chunks 0 0)
    ((t j).c_chunks 0 1) ((t j).c_chunks 0 2) ((t j).c_chunks 0 3) A B hop
    (h j).bool_cout_0_0 (h j).bool_cout_0_1 e1 e2
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j)
    (chunk16 hmax r2 rfl _ rfl j) (chunk16 hmax r3 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddLarge_bus13_adds {x : BinaryAddLarge.Ctx F} {t : BinaryAddLarge.Trace F}
    (h : ∀ j, BinaryAddLarge.everyRow x t j)
    (r0 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus6 x t) (r1 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus8 x t)
    (r2 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus10 x t) (r3 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus12 x t) :
    ∀ j, AddsAt ⟨BinaryAddLarge.bus13, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddLarge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddLarge.bus13, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e1 := (h j).c1
  have e2 := (h j).c4
  simp only [BinaryAddLarge.c1, BinaryAddLarge.c4, BinaryAddLarge.BinaryAddLarge_c_0_0, BinaryAddLarge.BinaryAddLarge_c_0_1] at e1 e2
  obtain ⟨c0, c1⟩ := adder64 ((t j).a 0 0) ((t j).a 0 1) ((t j).b 0 0) ((t j).b 0 1)
    ((t j).sel_sh3add 0) ((t j).cout 0 0) ((t j).cout 0 1) ((t j).c_chunks 0 0)
    ((t j).c_chunks 0 1) ((t j).c_chunks 0 2) ((t j).c_chunks 0 3) A B hop
    (h j).bool_cout_0_0 (h j).bool_cout_0_1 e1 e2
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j)
    (chunk16 hmax r2 rfl _ rfl j) (chunk16 hmax r3 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddLarge_bus22_adds {x : BinaryAddLarge.Ctx F} {t : BinaryAddLarge.Trace F}
    (h : ∀ j, BinaryAddLarge.everyRow x t j)
    (r0 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus15 x t) (r1 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus17 x t)
    (r2 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus19 x t) (r3 : RangeHolds BinaryAddLarge.ranges BinaryAddLarge.bus21 x t) :
    ∀ j, AddsAt ⟨BinaryAddLarge.bus22, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddLarge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddLarge.bus22, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e1 := (h j).c7
  have e2 := (h j).c10
  simp only [BinaryAddLarge.c7, BinaryAddLarge.c10, BinaryAddLarge.BinaryAddLarge_c_1_0, BinaryAddLarge.BinaryAddLarge_c_1_1, BinaryAddLarge.Zisk_direct_gsum_e_0, BinaryAddLarge.Zisk_direct_gsum_e_1, BinaryAddLarge.Zisk_direct_gsum_e_2] at e1 e2
  obtain ⟨c0, c1⟩ := adder64 ((t j).a 1 0) ((t j).a 1 1) ((t j).b 1 0) ((t j).b 1 1)
    ((t j).sel_sh3add 1) ((t j).cout 1 0) ((t j).cout 1 1) ((t j).c_chunks 1 0)
    ((t j).c_chunks 1 1) ((t j).c_chunks 1 2) ((t j).c_chunks 1 3) A B hop
    (h j).bool_cout_1_0 (h j).bool_cout_1_1 e1 e2
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j)
    (chunk16 hmax r2 rfl _ rfl j) (chunk16 hmax r3 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHuge_bus13_adds {x : BinaryAddHuge.Ctx F} {t : BinaryAddHuge.Trace F}
    (h : ∀ j, BinaryAddHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus6 x t) (r1 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus8 x t)
    (r2 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus10 x t) (r3 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus12 x t) :
    ∀ j, AddsAt ⟨BinaryAddHuge.bus13, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHuge.bus13, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e1 := (h j).c1
  have e2 := (h j).c4
  simp only [BinaryAddHuge.c1, BinaryAddHuge.c4, BinaryAddHuge.BinaryAddHuge_c_0_0, BinaryAddHuge.BinaryAddHuge_c_0_1] at e1 e2
  obtain ⟨c0, c1⟩ := adder64 ((t j).a 0 0) ((t j).a 0 1) ((t j).b 0 0) ((t j).b 0 1)
    ((t j).sel_sh3add 0) ((t j).cout 0 0) ((t j).cout 0 1) ((t j).c_chunks 0 0)
    ((t j).c_chunks 0 1) ((t j).c_chunks 0 2) ((t j).c_chunks 0 3) A B hop
    (h j).bool_cout_0_0 (h j).bool_cout_0_1 e1 e2
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j)
    (chunk16 hmax r2 rfl _ rfl j) (chunk16 hmax r3 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHuge_bus22_adds {x : BinaryAddHuge.Ctx F} {t : BinaryAddHuge.Trace F}
    (h : ∀ j, BinaryAddHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus15 x t) (r1 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus17 x t)
    (r2 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus19 x t) (r3 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus21 x t) :
    ∀ j, AddsAt ⟨BinaryAddHuge.bus22, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHuge.bus22, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e1 := (h j).c7
  have e2 := (h j).c10
  simp only [BinaryAddHuge.c7, BinaryAddHuge.c10, BinaryAddHuge.BinaryAddHuge_c_1_0, BinaryAddHuge.BinaryAddHuge_c_1_1, BinaryAddHuge.Zisk_direct_gsum_e_0, BinaryAddHuge.Zisk_direct_gsum_e_1, BinaryAddHuge.Zisk_direct_gsum_e_2] at e1 e2
  obtain ⟨c0, c1⟩ := adder64 ((t j).a 1 0) ((t j).a 1 1) ((t j).b 1 0) ((t j).b 1 1)
    ((t j).sel_sh3add 1) ((t j).cout 1 0) ((t j).cout 1 1) ((t j).c_chunks 1 0)
    ((t j).c_chunks 1 1) ((t j).c_chunks 1 2) ((t j).c_chunks 1 3) A B hop
    (h j).bool_cout_1_0 (h j).bool_cout_1_1 e1 e2
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j)
    (chunk16 hmax r2 rfl _ rfl j) (chunk16 hmax r3 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHuge_bus31_adds {x : BinaryAddHuge.Ctx F} {t : BinaryAddHuge.Trace F}
    (h : ∀ j, BinaryAddHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus24 x t) (r1 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus26 x t)
    (r2 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus28 x t) (r3 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus30 x t) :
    ∀ j, AddsAt ⟨BinaryAddHuge.bus31, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHuge.bus31, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e1 := (h j).c13
  have e2 := (h j).c16
  simp only [BinaryAddHuge.c13, BinaryAddHuge.c16, BinaryAddHuge.BinaryAddHuge_c_2_0, BinaryAddHuge.BinaryAddHuge_c_2_1, BinaryAddHuge.Zisk_direct_gsum_e_3, BinaryAddHuge.Zisk_direct_gsum_e_4] at e1 e2
  obtain ⟨c0, c1⟩ := adder64 ((t j).a 2 0) ((t j).a 2 1) ((t j).b 2 0) ((t j).b 2 1)
    ((t j).sel_sh3add 2) ((t j).cout 2 0) ((t j).cout 2 1) ((t j).c_chunks 2 0)
    ((t j).c_chunks 2 1) ((t j).c_chunks 2 2) ((t j).c_chunks 2 3) A B hop
    (h j).bool_cout_2_0 (h j).bool_cout_2_1 e1 e2
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j)
    (chunk16 hmax r2 rfl _ rfl j) (chunk16 hmax r3 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHuge_bus40_adds {x : BinaryAddHuge.Ctx F} {t : BinaryAddHuge.Trace F}
    (h : ∀ j, BinaryAddHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus33 x t) (r1 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus35 x t)
    (r2 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus37 x t) (r3 : RangeHolds BinaryAddHuge.ranges BinaryAddHuge.bus39 x t) :
    ∀ j, AddsAt ⟨BinaryAddHuge.bus40, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHuge.bus40, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e1 := (h j).c19
  have e2 := (h j).c22
  simp only [BinaryAddHuge.c19, BinaryAddHuge.c22, BinaryAddHuge.BinaryAddHuge_c_3_0, BinaryAddHuge.Zisk_direct_gsum_e_6, BinaryAddHuge.Zisk_direct_gsum_e_7] at e1 e2
  obtain ⟨c0, c1⟩ := adder64 ((t j).a 3 0) ((t j).a 3 1) ((t j).b 3 0) ((t j).b 3 1)
    ((t j).sel_sh3add 3) ((t j).cout 3 0) ((t j).cout 3 1) ((t j).c_chunks 3 0)
    ((t j).c_chunks 3 1) ((t j).c_chunks 3 2) ((t j).c_chunks 3 3) A B hop
    (h j).bool_cout_3_0 (h j).bool_cout_3_1 e1 e2
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j)
    (chunk16 hmax r2 rfl _ rfl j) (chunk16 hmax r3 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHi_bus9_adds {x : BinaryAddHi.Ctx F} {t : BinaryAddHi.Trace F}
    (h : ∀ j, BinaryAddHi.everyRow x t j)
    (r0 : RangeHolds BinaryAddHi.ranges BinaryAddHi.bus6 x t) (r1 : RangeHolds BinaryAddHi.ranges BinaryAddHi.bus8 x t) :
    ∀ j, AddsAt ⟨BinaryAddHi.bus9, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHi.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHi.bus9, BinaryAddHi.BinaryAddHi_c_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c0
  have e1 := (h j).c1
  simp only [BinaryAddHi.c0, BinaryAddHi.c1] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 0) ((t j).b 0) ((t j).sel_sh3add 0)
    ((t j).sel_b_hi_is_ff 0) ((t j).c_chunks 0 0) ((t j).c_chunks 0 1) A B hop
    (h j).bool_sel_b_hi_is_ff_0 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHi_bus14_adds {x : BinaryAddHi.Ctx F} {t : BinaryAddHi.Trace F}
    (h : ∀ j, BinaryAddHi.everyRow x t j)
    (r0 : RangeHolds BinaryAddHi.ranges BinaryAddHi.bus11 x t) (r1 : RangeHolds BinaryAddHi.ranges BinaryAddHi.bus13 x t) :
    ∀ j, AddsAt ⟨BinaryAddHi.bus14, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHi.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHi.bus14, BinaryAddHi.BinaryAddHi_c_1, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c4
  have e1 := (h j).c5
  simp only [BinaryAddHi.c4, BinaryAddHi.c5, BinaryAddHi.Zisk_direct_gsum_e_0, BinaryAddHi.Zisk_direct_gsum_e_1] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 1) ((t j).b 1) ((t j).sel_sh3add 1)
    ((t j).sel_b_hi_is_ff 1) ((t j).c_chunks 1 0) ((t j).c_chunks 1 1) A B hop
    (h j).bool_sel_b_hi_is_ff_1 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiLarge_bus9_adds {x : BinaryAddHiLarge.Ctx F} {t : BinaryAddHiLarge.Trace F}
    (h : ∀ j, BinaryAddHiLarge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus6 x t) (r1 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus8 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiLarge.bus9, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiLarge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiLarge.bus9, BinaryAddHiLarge.BinaryAddHiLarge_c_0, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c0
  have e1 := (h j).c1
  simp only [BinaryAddHiLarge.c0, BinaryAddHiLarge.c1] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 0) ((t j).b 0) ((t j).sel_sh3add 0)
    ((t j).sel_b_hi_is_ff 0) ((t j).c_chunks 0 0) ((t j).c_chunks 0 1) A B hop
    (h j).bool_sel_b_hi_is_ff_0 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiLarge_bus14_adds {x : BinaryAddHiLarge.Ctx F} {t : BinaryAddHiLarge.Trace F}
    (h : ∀ j, BinaryAddHiLarge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus11 x t) (r1 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus13 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiLarge.bus14, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiLarge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiLarge.bus14, BinaryAddHiLarge.BinaryAddHiLarge_c_1, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c4
  have e1 := (h j).c5
  simp only [BinaryAddHiLarge.c4, BinaryAddHiLarge.c5, BinaryAddHiLarge.Zisk_direct_gsum_e_0, BinaryAddHiLarge.Zisk_direct_gsum_e_1] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 1) ((t j).b 1) ((t j).sel_sh3add 1)
    ((t j).sel_b_hi_is_ff 1) ((t j).c_chunks 1 0) ((t j).c_chunks 1 1) A B hop
    (h j).bool_sel_b_hi_is_ff_1 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiLarge_bus19_adds {x : BinaryAddHiLarge.Ctx F} {t : BinaryAddHiLarge.Trace F}
    (h : ∀ j, BinaryAddHiLarge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus16 x t) (r1 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus18 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiLarge.bus19, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiLarge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiLarge.bus19, BinaryAddHiLarge.BinaryAddHiLarge_c_2, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c8
  have e1 := (h j).c9
  simp only [BinaryAddHiLarge.c8, BinaryAddHiLarge.c9, BinaryAddHiLarge.Zisk_direct_gsum_e_3, BinaryAddHiLarge.Zisk_direct_gsum_e_4] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 2) ((t j).b 2) ((t j).sel_sh3add 2)
    ((t j).sel_b_hi_is_ff 2) ((t j).c_chunks 2 0) ((t j).c_chunks 2 1) A B hop
    (h j).bool_sel_b_hi_is_ff_2 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiLarge_bus24_adds {x : BinaryAddHiLarge.Ctx F} {t : BinaryAddHiLarge.Trace F}
    (h : ∀ j, BinaryAddHiLarge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus21 x t) (r1 : RangeHolds BinaryAddHiLarge.ranges BinaryAddHiLarge.bus23 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiLarge.bus24, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiLarge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiLarge.bus24, BinaryAddHiLarge.BinaryAddHiLarge_c_3, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c12
  have e1 := (h j).c13
  simp only [BinaryAddHiLarge.c12, BinaryAddHiLarge.c13, BinaryAddHiLarge.Zisk_direct_gsum_e_5, BinaryAddHiLarge.Zisk_direct_gsum_e_6] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 3) ((t j).b 3) ((t j).sel_sh3add 3)
    ((t j).sel_b_hi_is_ff 3) ((t j).c_chunks 3 0) ((t j).c_chunks 3 1) A B hop
    (h j).bool_sel_b_hi_is_ff_3 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus9_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus6 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus8 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus9, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus9, BinaryAddHiHuge.Zisk_direct_gsum_e_17, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c0
  have e1 := (h j).c1
  simp only [BinaryAddHiHuge.c0, BinaryAddHiHuge.c1] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 0) ((t j).b 0) ((t j).sel_sh3add 0)
    ((t j).sel_b_hi_is_ff 0) ((t j).c_chunks 0 0) ((t j).c_chunks 0 1) A B hop
    (h j).bool_sel_b_hi_is_ff_0 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus14_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus11 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus13 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus14, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus14, BinaryAddHiHuge.BinaryAddHiHuge_c_1, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c4
  have e1 := (h j).c5
  simp only [BinaryAddHiHuge.c4, BinaryAddHiHuge.c5, BinaryAddHiHuge.Zisk_direct_gsum_e_0, BinaryAddHiHuge.Zisk_direct_gsum_e_1] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 1) ((t j).b 1) ((t j).sel_sh3add 1)
    ((t j).sel_b_hi_is_ff 1) ((t j).c_chunks 1 0) ((t j).c_chunks 1 1) A B hop
    (h j).bool_sel_b_hi_is_ff_1 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus19_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus16 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus18 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus19, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus19, BinaryAddHiHuge.Zisk_direct_gsum_e_27, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c8
  have e1 := (h j).c9
  simp only [BinaryAddHiHuge.c8, BinaryAddHiHuge.c9, BinaryAddHiHuge.Zisk_direct_gsum_e_3, BinaryAddHiHuge.Zisk_direct_gsum_e_4] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 2) ((t j).b 2) ((t j).sel_sh3add 2)
    ((t j).sel_b_hi_is_ff 2) ((t j).c_chunks 2 0) ((t j).c_chunks 2 1) A B hop
    (h j).bool_sel_b_hi_is_ff_2 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus24_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus21 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus23 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus24, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus24, BinaryAddHiHuge.BinaryAddHiHuge_c_3, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c12
  have e1 := (h j).c13
  simp only [BinaryAddHiHuge.c12, BinaryAddHiHuge.c13, BinaryAddHiHuge.Zisk_direct_gsum_e_5, BinaryAddHiHuge.Zisk_direct_gsum_e_6] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 3) ((t j).b 3) ((t j).sel_sh3add 3)
    ((t j).sel_b_hi_is_ff 3) ((t j).c_chunks 3 0) ((t j).c_chunks 3 1) A B hop
    (h j).bool_sel_b_hi_is_ff_3 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus29_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus26 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus28 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus29, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus29, BinaryAddHiHuge.Zisk_direct_gsum_e_37, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c16
  have e1 := (h j).c17
  simp only [BinaryAddHiHuge.c16, BinaryAddHiHuge.c17, BinaryAddHiHuge.Zisk_direct_gsum_e_8, BinaryAddHiHuge.Zisk_direct_gsum_e_9] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 4) ((t j).b 4) ((t j).sel_sh3add 4)
    ((t j).sel_b_hi_is_ff 4) ((t j).c_chunks 4 0) ((t j).c_chunks 4 1) A B hop
    (h j).bool_sel_b_hi_is_ff_4 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus34_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus31 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus33 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus34, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus34, BinaryAddHiHuge.BinaryAddHiHuge_c_5, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c20
  have e1 := (h j).c21
  simp only [BinaryAddHiHuge.c20, BinaryAddHiHuge.c21, BinaryAddHiHuge.Zisk_direct_gsum_e_10, BinaryAddHiHuge.Zisk_direct_gsum_e_11] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 5) ((t j).b 5) ((t j).sel_sh3add 5)
    ((t j).sel_b_hi_is_ff 5) ((t j).c_chunks 5 0) ((t j).c_chunks 5 1) A B hop
    (h j).bool_sel_b_hi_is_ff_5 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus39_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus36 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus38 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus39, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus39, BinaryAddHiHuge.BinaryAddHiHuge_c_6, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c24
  have e1 := (h j).c25
  simp only [BinaryAddHiHuge.c24, BinaryAddHiHuge.c25, BinaryAddHiHuge.Zisk_direct_gsum_e_12, BinaryAddHiHuge.Zisk_direct_gsum_e_13] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 6) ((t j).b 6) ((t j).sel_sh3add 6)
    ((t j).sel_b_hi_is_ff 6) ((t j).c_chunks 6 0) ((t j).c_chunks 6 1) A B hop
    (h j).bool_sel_b_hi_is_ff_6 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

theorem BinaryAddHiHuge_bus44_adds {x : BinaryAddHiHuge.Ctx F} {t : BinaryAddHiHuge.Trace F}
    (h : ∀ j, BinaryAddHiHuge.everyRow x t j)
    (r0 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus41 x t) (r1 : RangeHolds BinaryAddHiHuge.ranges BinaryAddHiHuge.bus43 x t) :
    ∀ j, AddsAt ⟨BinaryAddHiHuge.bus44, x, t⟩ j := by
  intro j hop A B ha0 ha1 hb0 hb1
  have hmax : ∀ d ∈ BinaryAddHiHuge.ranges, d.max = 65535 := by decide
  simp only [tupleAt, BinaryAddHiHuge.bus44, BinaryAddHiHuge.BinaryAddHiHuge_c_7, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at hop ha0 ha1 hb0 hb1 ⊢
  have e0 := (h j).c28
  have e1 := (h j).c29
  simp only [BinaryAddHiHuge.c28, BinaryAddHiHuge.c29, BinaryAddHiHuge.Zisk_direct_gsum_e_15, BinaryAddHiHuge.Zisk_direct_gsum_e_16] at e0 e1
  obtain ⟨c0, c1⟩ := adderHi ((t j).a 7) ((t j).b 7) ((t j).sel_sh3add 7)
    ((t j).sel_b_hi_is_ff 7) ((t j).c_chunks 7 0) ((t j).c_chunks 7 1) A B hop
    (h j).bool_sel_b_hi_is_ff_7 (by grind) (by grind)
    (chunk16 hmax r0 rfl _ rfl j) (chunk16 hmax r1 rfl _ rfl j) ha0 ha1 hb0 hb1
  exact ⟨c0, c1, by trivial⟩

end Lanes

end Correspondence
