/-
  Binary never proves `pubout`.

  Binary's op is `b_op + 16·mode32`, `mode32` a bit and `b_op` what its first
  byte's lookup into the Binary table carries — the table's op. The table's
  ops are Binary's, all below `0x60`; so if no row of it has op 160 or 144,
  Binary's op is never `pubout`'s, 160. That is a fact about the table's
  fixed data (`BinaryTableNoPub`), which `check_tables.py` checks.
-/
import Proofs.BinaryOps

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- No row the prover `p` serves on the Binary table's bus has op 160 or 144. -/
def BinaryTableNoPub (p : Prover F) : Prop :=
  ∀ j, p.arg.bus p.x p.t j = 125 →
    tupleAt p.arg.tuple p.x p.t j 1 ≠ ((160 : Nat) : F) ∧ tupleAt p.arg.tuple p.x p.t j 1 ≠ ((144 : Nat) : F)

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A byte Binary looks up in the table has no op 160 or 144. -/
theorem noPub_of_lookup {C T : Type} (a : BusArg F C T) (x : C) (t : T) (i : Int)
    (ps : List (Prover F)) (hl : LookupHolds a x t ps) (hbus : ∀ x t i, a.bus x t i = 125)
    (hreps : a.reps x t i ≠ 0) (hd : ∀ p ∈ ps, BinaryTableNoPub p) :
    tupleAt a.tuple x t i 1 ≠ ((160 : Nat) : F) ∧ tupleAt a.tuple x t i 1 ≠ ((144 : Nat) : F) := by
  obtain ⟨-, -, -, -, hb⟩ := hl
  obtain ⟨p, hp, j, -, hpb, htup⟩ := hb i hreps
  rw [htup 1]
  exact hd p hp j (by rw [hpb, hbus])

/-- `b_op + 16·mode32` is not 160 when `b_op` is neither 160 nor 144. -/
theorem binop_ne_pub {bop m : F} (hm : m * (1 - m) = 0)
    (hb : bop ≠ ((160 : Nat) : F) ∧ bop ≠ ((144 : Nat) : F)) : bop + 16 * m ≠ ((160 : Nat) : F) := by
  obtain ⟨b, rfl⟩ := bool_of_idem hm
  intro h
  obtain ⟨h1, h2⟩ := hb
  cases b <;> simp only [bit, Bool.false_eq_true, if_false, if_true] at h
  · apply h1
    generalize ((160 : Nat) : F) = y at h ⊢
    grind
  · apply h2
    have e : ((160 : Nat) : F) = ((144 : Nat) : F) + 16 := by
      rw [show (160 : Nat) = 144 + 16 from rfl, Lean.Grind.Semiring.natCast_add,
        natCast_ofNat' 16]
    rw [e] at h
    clear h1 e
    generalize ((144 : Nat) : F) = y at h ⊢
    grind

/-! ### The seven Binary lanes -/

theorem Binary_bus21_pub {x : Binary.Ctx F} {t : Binary.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, Binary.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p)
    (l0 : LookupHolds Binary.bus13 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 0xA0 pubSem ⟨Binary.bus21, x, t⟩ j := by
  intro j hop
  exfalso
  have t0 := noPub_of_lookup Binary.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, Binary.bus13, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0
  simp only [tupleAt, Binary.bus21, List.getD_cons_zero] at hop
  exact binop_ne_pub (h j).bool_mode32_0 t0 hop

theorem BinaryLarge_bus21_pub {x : BinaryLarge.Ctx F} {t : BinaryLarge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryLarge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p)
    (l0 : LookupHolds BinaryLarge.bus13 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 0xA0 pubSem ⟨BinaryLarge.bus21, x, t⟩ j := by
  intro j hop
  exfalso
  have t0 := noPub_of_lookup BinaryLarge.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryLarge.bus13, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0
  simp only [tupleAt, BinaryLarge.bus21, List.getD_cons_zero] at hop
  exact binop_ne_pub (h j).bool_mode32_0 t0 hop

theorem BinaryLarge_bus32_pub {x : BinaryLarge.Ctx F} {t : BinaryLarge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryLarge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p)
    (l0 : LookupHolds BinaryLarge.bus24 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 0xA0 pubSem ⟨BinaryLarge.bus32, x, t⟩ j := by
  intro j hop
  exfalso
  have t0 := noPub_of_lookup BinaryLarge.bus24 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryLarge.bus24, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0
  simp only [tupleAt, BinaryLarge.bus32, List.getD_cons_zero] at hop
  exact binop_ne_pub (h j).bool_mode32_1 t0 hop

theorem BinaryHuge_bus21_pub {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p)
    (l0 : LookupHolds BinaryHuge.bus13 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 0xA0 pubSem ⟨BinaryHuge.bus21, x, t⟩ j := by
  intro j hop
  exfalso
  have t0 := noPub_of_lookup BinaryHuge.bus13 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus13, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0
  simp only [tupleAt, BinaryHuge.bus21, List.getD_cons_zero] at hop
  exact binop_ne_pub (h j).bool_mode32_0 t0 hop

theorem BinaryHuge_bus32_pub {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p)
    (l0 : LookupHolds BinaryHuge.bus24 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 0xA0 pubSem ⟨BinaryHuge.bus32, x, t⟩ j := by
  intro j hop
  exfalso
  have t0 := noPub_of_lookup BinaryHuge.bus24 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus24, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0
  simp only [tupleAt, BinaryHuge.bus32, List.getD_cons_zero] at hop
  exact binop_ne_pub (h j).bool_mode32_1 t0 hop

theorem BinaryHuge_bus43_pub {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p)
    (l0 : LookupHolds BinaryHuge.bus35 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 0xA0 pubSem ⟨BinaryHuge.bus43, x, t⟩ j := by
  intro j hop
  exfalso
  have t0 := noPub_of_lookup BinaryHuge.bus35 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus35, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0
  simp only [tupleAt, BinaryHuge.bus43, List.getD_cons_zero] at hop
  exact binop_ne_pub (h j).bool_mode32_2 t0 hop

theorem BinaryHuge_bus54_pub {x : BinaryHuge.Ctx F} {t : BinaryHuge.Trace F}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (h : ∀ j, BinaryHuge.everyRow x t j) (hd : ∀ p ∈ binaryTable xv tv, BinaryTableNoPub p)
    (l0 : LookupHolds BinaryHuge.bus46 x t (binaryTable xv tv)) :
    ∀ j, ProvesAt 0xA0 pubSem ⟨BinaryHuge.bus54, x, t⟩ j := by
  intro j hop
  exfalso
  have t0 := noPub_of_lookup BinaryHuge.bus46 x t j _ l0 (fun _ _ _ => rfl) one_ne_zero' hd
  simp only [tupleAt, BinaryHuge.bus46, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at t0
  simp only [tupleAt, BinaryHuge.bus54, List.getD_cons_zero] at hop
  exact binop_ne_pub (h j).bool_mode32_3 t0 hop

end Correspondence
