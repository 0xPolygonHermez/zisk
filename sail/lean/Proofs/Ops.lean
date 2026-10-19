/-
  External ops, generally.

  `AddPilot.lean` proved one slot of Main executing `add`, which Main sends to
  another AIR over the operation bus. This file states what that took for any
  external op, so the other modeled ones — `ltu`, `eq`, `and` — can follow:

  * `ltuSem`, `eqSem`, `andSem`, `addSem`: what the op does to two 64-bit
    values, as `(c, flag)`; each is proved to be the model's `zisk_exec`.
  * `ProvesAt code sem p j`: prover `p`'s tuple at row `j`, if its op is
    `code`, carries `sem` of its operands — the generalization of `AddsAt`.
  * `OpProves code sem p`: so at every row it proves on the operation bus,
    with a nonzero count. The count matters: Dma's op is a sum of opcodes plus
    `6·sel_extended`, which is `ltu`'s code exactly when no base selector is
    set — and then its count is 0, so it proves nothing.

  And the three ways a prover gets `OpProves`, as for `add`: it computes the op
  (`opProves_of_addsAt`, and the Binary lanes in `BinaryOps.lean`), its op is
  never `code` (`opProves_of_not`), or it is a table whose rows `check_tables.py`
  checks (`opProves_of_frops`).
-/
import Proofs.AddPilot

open Out.Functions

namespace Correspondence

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-! ### What the ops do -/

def addSem (A B : BitVec 64) : BitVec 64 × Bool := (A + B, false)
def ltuSem (A B : BitVec 64) : BitVec 64 × Bool := if A.toNat < B.toNat then (1, true) else (0, false)
def eqSem (A B : BitVec 64) : BitVec 64 × Bool := if A = B then (1, true) else (0, false)
def andSem (A B : BitVec 64) : BitVec 64 × Bool := (A &&& B, false)

theorem exec_add (a b : BitVec 64) : zisk_exec (.ZOP_ADD ()) a b = addSem a b := by
  simp [zisk_exec, addSem]

theorem exec_ltu (a b : BitVec 64) : zisk_exec (.ZOP_LTU ()) a b = ltuSem a b := by
  simp only [zisk_exec, ltuSem, zopz0zI_u]
  by_cases h : a.toNat < b.toNat <;> simp [h] <;> rfl

theorem exec_eq (a b : BitVec 64) : zisk_exec (.ZOP_EQ ()) a b = eqSem a b := by
  simp only [zisk_exec, eqSem]
  by_cases h : a = b
  · subst h; simp; rfl
  · have hb : (a == b) = false := by simpa using h
    simp only [hb, cond_false, h, if_false]; rfl

theorem exec_and (a b : BitVec 64) : zisk_exec (.ZOP_AND ()) a b = andSem a b := by
  simp [zisk_exec, andSem]

/-! ### What a prover must do -/

section Statement

variable {F : Type} [PilField F]

/-- Prover `p`'s tuple at row `j`, if its op is `code`, carries `sem` of its
operands. -/
def ProvesAt (code : Nat) (sem : BitVec 64 → BitVec 64 → BitVec 64 × Bool) (p : Prover F)
    (j : Int) : Prop :=
  tupleAt p.arg.tuple p.x p.t j 0 = ((code : Nat) : F) → ∀ A B : BitVec 64,
    tupleAt p.arg.tuple p.x p.t j 1 = lo A → tupleAt p.arg.tuple p.x p.t j 2 = hi A →
    tupleAt p.arg.tuple p.x p.t j 3 = lo B → tupleAt p.arg.tuple p.x p.t j 4 = hi B →
    tupleAt p.arg.tuple p.x p.t j 5 = lo (sem A B).1 ∧
      tupleAt p.arg.tuple p.x p.t j 6 = hi (sem A B).1 ∧
      tupleAt p.arg.tuple p.x p.t j 7 = bit (sem A B).2

/-- On the operation bus, at every row it proves with a nonzero count, `p`
proves `code` correctly. -/
def OpProves (code : Nat) (sem : BitVec 64 → BitVec 64 → BitVec 64 × Bool) (p : Prover F) :
    Prop :=
  ∀ j, p.arg.bus p.x p.t j = 5000 → p.arg.reps p.x p.t j ≠ 0 → ProvesAt code sem p j

end Statement

section Routes

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- An adder or Binary lane's `AddsAt`. -/
theorem provesAt_of_addsAt {p : Prover F} {j : Int} (h : AddsAt p j) : ProvesAt 10 addSem p j := by
  intro hop A B h1 h2 h3 h4
  obtain ⟨r5, r6, r7⟩ := h (by rw [hop, ← natCast_ofNat' 10]) A B h1 h2 h3 h4
  exact ⟨r5, r6, by rw [r7]; simp [addSem, bit]⟩

omit [Lean.Grind.IsCharP F goldilocks] in
theorem opProves_of_addsAt {p : Prover F} (h : ∀ j, AddsAt p j) : OpProves 10 addSem p :=
  fun j _ _ => provesAt_of_addsAt (h j)

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A prover that never sends `code` at a row it proves. -/
theorem opProves_of_not {code : Nat} {sem} {p : Prover F}
    (h : ∀ j, p.arg.reps p.x p.t j ≠ 0 → opOf p.arg p.x p.t j ≠ ((code : Int) : F)) :
    OpProves code sem p := by
  intro j _ hr hop
  exfalso
  apply h j hr
  rw [opOf_tupleAt, hop, Lean.Grind.Ring.intCast_natCast]

/-- What `check_tables.py` checks of a FROPS table for one op: every row on the
operation bus whose op is `code` carries 32-bit limbs and the op's result,
`semN` of the operands as naturals, `(c, flag)`. -/
def FropsProves (code : Nat) (semN : Nat → Nat → Nat × Nat) (p : Prover F) : Prop :=
  ∀ j, p.arg.bus p.x p.t j = 5000 → tupleAt p.arg.tuple p.x p.t j 0 = ((code : Nat) : F) →
    ∃ a0 a1 b0 b1 c0 c1 f : Nat, a0 < 2 ^ 32 ∧ a1 < 2 ^ 32 ∧ b0 < 2 ^ 32 ∧ b1 < 2 ^ 32 ∧
      c0 < 2 ^ 32 ∧ c1 < 2 ^ 32 ∧
      tupleAt p.arg.tuple p.x p.t j 1 = (a0 : F) ∧ tupleAt p.arg.tuple p.x p.t j 2 = (a1 : F) ∧
      tupleAt p.arg.tuple p.x p.t j 3 = (b0 : F) ∧ tupleAt p.arg.tuple p.x p.t j 4 = (b1 : F) ∧
      tupleAt p.arg.tuple p.x p.t j 5 = (c0 : F) ∧ tupleAt p.arg.tuple p.x p.t j 6 = (c1 : F) ∧
      tupleAt p.arg.tuple p.x p.t j 7 = (f : F) ∧
      (c0 + 4294967296 * c1, f) = semN (a0 + 4294967296 * a1) (b0 + 4294967296 * b1)

/-- A FROPS table proves `code` correctly, when `semN` is the op on naturals. -/
theorem opProves_of_frops {code : Nat} {sem} {semN : Nat → Nat → Nat × Nat} {p : Prover F}
    (hsem : ∀ A B : BitVec 64,
      semN A.toNat B.toNat = ((sem A B).1.toNat, if (sem A B).2 then 1 else 0))
    (h : FropsProves code semN p) : OpProves code sem p := by
  intro j hbus _ hop A B ha0 ha1 hb0 hb1
  obtain ⟨a0, a1, b0, b1, c0, c1, f, la0, la1, lb0, lb1, lc0, lc1, e1, e2, e3, e4, e5, e6, e7, hs⟩ :=
    h j hbus hop
  have hA := A.isLt
  have hB := B.isLt
  have nA0 : a0 = A.toNat % 2 ^ 32 := nat_eq (by omega) (by omega) (e1.symm.trans ha0)
  have nA1 : a1 = A.toNat / 2 ^ 32 := nat_eq (by omega) (by omega) (e2.symm.trans ha1)
  have nB0 : b0 = B.toNat % 2 ^ 32 := nat_eq (by omega) (by omega) (e3.symm.trans hb0)
  have nB1 : b1 = B.toNat / 2 ^ 32 := nat_eq (by omega) (by omega) (e4.symm.trans hb1)
  have eA : a0 + 4294967296 * a1 = A.toNat := by omega
  have eB : b0 + 4294967296 * b1 = B.toNat := by omega
  rw [eA, eB, hsem] at hs
  simp only [Prod.mk.injEq] at hs
  obtain ⟨hc, hf⟩ := hs
  have hC := (sem A B).1.isLt
  have m0 : c0 = (sem A B).1.toNat % 2 ^ 32 := by omega
  have m1 : c1 = (sem A B).1.toNat / 2 ^ 32 := by omega
  refine ⟨by rw [e5, lo, m0], by rw [e6, hi, m1], ?_⟩
  rw [e7, hf]
  cases (sem A B).2 <;> simp [bit, Lean.Grind.Semiring.natCast_zero, Lean.Grind.Semiring.natCast_one]

/-- The ops on naturals, as `check_tables.py` computes them. -/
def addN (a b : Nat) : Nat × Nat := ((a + b) % 2 ^ 64, 0)
def ltuN (a b : Nat) : Nat × Nat := if a < b then (1, 1) else (0, 0)
def eqN (a b : Nat) : Nat × Nat := if a = b then (1, 1) else (0, 0)
def andN (a b : Nat) : Nat × Nat := (a &&& b, 0)

theorem addN_sem (A B : BitVec 64) :
    addN A.toNat B.toNat = ((addSem A B).1.toNat, if (addSem A B).2 then 1 else 0) := by
  simp [addN, addSem, BitVec.toNat_add]

theorem ltuN_sem (A B : BitVec 64) :
    ltuN A.toNat B.toNat = ((ltuSem A B).1.toNat, if (ltuSem A B).2 then 1 else 0) := by
  unfold ltuN ltuSem; split <;> simp

theorem eqN_sem (A B : BitVec 64) :
    eqN A.toNat B.toNat = ((eqSem A B).1.toNat, if (eqSem A B).2 then 1 else 0) := by
  unfold eqN eqSem
  by_cases h : A = B
  · subst h; simp
  · have : A.toNat ≠ B.toNat := fun e => h (BitVec.eq_of_toNat_eq e)
    simp [h, this]

theorem andN_sem (A B : BitVec 64) :
    andN A.toNat B.toNat = ((andSem A B).1.toNat, if (andSem A B).2 then 1 else 0) := by
  simp [andN, andSem]

end Routes

end Correspondence
