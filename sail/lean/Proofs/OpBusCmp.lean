/-
  Who cannot prove `ltu`, `eq` or `and`.

  The census of `OpBus.lean`, for the other modeled external ops: `ltu` (6),
  `eq` (9) and `and` (14). The same 29 proving arguments are ruled out the same
  way — their op is a constant or opcodes weighted by bits, and no combination
  is 6, 9 or 14 — and so, for these three, are the 21 adder lanes, whose op is
  10 or 88 (`sh3add`). Arith and BinaryExtension are ruled out by their tables,
  as for `add`.

  One case needs more than the op: Dma's op is a sum of base opcodes plus
  `6·sel_extended`, which is `ltu`'s code exactly when no base selector is set.
  Dma's count is the sum of the base selectors, so such a row proves nothing;
  `op_ne2` enumerates op and count together over the same bits.

  The theorems below were written out mechanically from the generated bus
  arguments, as in `OpBus.lean`.
-/
import Proofs.Bus
import Proofs.OpBus
import Pil.Add256
import Pil.Arith256X
import Pil.Arith256XLarge
import Pil.ArithBn254
import Pil.ArithBn254Large
import Pil.ArithEq
import Pil.ArithEqLarge
import Pil.ArithEq384
import Pil.ArithEq384Large
import Pil.ArithSecp256K1
import Pil.ArithSecp256K1Large
import Pil.Blake2br
import Pil.Blake2s
import Pil.Blake3f
import Pil.Dma
import Pil.Dma64Aligned
import Pil.Dma64AlignedLarge
import Pil.Dma64AlignedMem
import Pil.Dma64AlignedMemLarge
import Pil.Dma64AlignedMemCpy
import Pil.Dma64AlignedMemSet
import Pil.DmaUnaligned
import Pil.JumpDest
import Pil.Keccakf
import Pil.Poseidon
import Pil.Sha256f
import Pil.TwistedEdwards
import Pil.BinaryAdd
import Pil.BinaryAddLarge
import Pil.BinaryAddHuge
import Pil.BinaryAddHi
import Pil.BinaryAddHiLarge
import Pil.BinaryAddHiHuge
import Pil.Arith
import Pil.BinaryExtension
import Pil.BinaryExtensionLarge
import Pil.VirtualTableZisk1

-- `decide` enumerates every subset of an op's selectors.
set_option maxRecDepth 100000

namespace Correspondence

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

section Lemma

/-- Pairs of subset sums of two coefficient lists over the same bits. -/
def subsetSums2 : List (Int × Int) → List (Int × Int)
  | [] => [(0, 0)]
  | (c, d) :: r => subsetSums2 r ++ (subsetSums2 r).map (fun zw => (c + zw.1, d + zw.2))

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

omit [Lean.Grind.IsCharP F goldilocks] in
theorem linF_bits2 (cds : List (Int × Int)) (ss : List F) (hl : cds.length = ss.length)
    (hb : ∀ s ∈ ss, s * (1 - s) = 0) :
    ∃ zw ∈ subsetSums2 cds, linF (cds.map (fun p => (p.1 : F))) ss = (zw.1 : F) ∧
      linF (cds.map (fun p => (p.2 : F))) ss = (zw.2 : F) := by
  induction cds generalizing ss with
  | nil => exact ⟨(0, 0), by simp [subsetSums2], by simp [linF, Lean.Grind.Ring.intCast_zero],
      by simp [linF, Lean.Grind.Ring.intCast_zero]⟩
  | cons cd cds ih =>
    obtain ⟨c, d⟩ := cd
    cases ss with
    | nil => simp at hl
    | cons s ss =>
      obtain ⟨zw, hz, e1, e2⟩ := ih ss (by simpa using hl) (fun x hx => hb x (by simp [hx]))
      obtain ⟨b, hs⟩ := bool_of_idem (hb s (by simp))
      cases b
      · refine ⟨zw, List.mem_append_left _ hz, ?_, ?_⟩ <;>
          simp only [List.map_cons, linF, e1, e2, hs, bit, Bool.false_eq_true, if_false] <;> grind
      · refine ⟨(c + zw.1, d + zw.2), List.mem_append_right _ (List.mem_map.mpr ⟨zw, hz, rfl⟩), ?_, ?_⟩ <;>
          simp only [List.map_cons, linF, e1, e2, hs, bit, if_true, Lean.Grind.Ring.intCast_add] <;> grind

/-- **An op that is `k` only where the count is 0.** As `op_ne`, with the count
a second weighted sum of the same bits: no combination with a nonzero count
has the op `k`. -/
theorem op_ne2 (c₀ d₀ : Int) (cds : List (Int × Int)) (f₀ e₀ : F) (fs es ss : List F) (k : Int)
    (op reps : F) (hc₀ : ((c₀ : Int) : F) = f₀) (hd₀ : ((d₀ : Int) : F) = e₀)
    (hcf : cds.map (fun p => (p.1 : F)) = fs) (hdf : cds.map (fun p => (p.2 : F)) = es)
    (hl : cds.length = ss.length) (hb : ∀ s ∈ ss, s * (1 - s) = 0)
    (heq : op = f₀ + linF fs ss) (hreq : reps = e₀ + linF es ss)
    (hvals : ∀ zw ∈ subsetSums2 cds, (d₀ + zw.2 = 0 ∨ c₀ + zw.1 ≠ k) ∧
      -2 ^ 62 < c₀ + zw.1 ∧ c₀ + zw.1 < 2 ^ 62 ∧ -2 ^ 62 < d₀ + zw.2 ∧ d₀ + zw.2 < 2 ^ 62)
    (hk : -2 ^ 62 < k ∧ k < 2 ^ 62) (hr : reps ≠ 0) :
    op ≠ (k : F) := by
  obtain ⟨zw, hz, e1, e2⟩ := linF_bits2 cds ss hl hb
  obtain ⟨hor, l1, h1, l2, h2⟩ := hvals zw hz
  rcases hor with h0 | hne
  · exfalso; apply hr
    rw [hreq, ← hdf, e2, ← hd₀, ← Lean.Grind.Ring.intCast_add, h0, Lean.Grind.Ring.intCast_zero]
  · intro h
    rw [heq, ← hcf, e1, ← hc₀, ← Lean.Grind.Ring.intCast_add] at h
    have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp h
    unfold goldilocks at this
    omega

end Lemma

section Provers

open Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

theorem Add256_bus59_not_cmp {x t i} (_h : Add256.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Add256.bus59 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 240 [] 240 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Add256.bus59, List.headD, linF]
      grind
    · decide
    · decide

theorem Arith256X_bus46_not_cmp {x t i} (h : Arith256X.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Arith256X.bus46 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [242, 243] 0 [242, 243] [(t i).sel_arith256, (t i).sel_arith256_mod] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl)
      · exact h.bool_sel_arith256
      · exact h.bool_sel_arith256_mod
    · simp only [opOf, Arith256X.bus46, Arith256X.Arith256X_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem Arith256XLarge_bus46_not_cmp {x t i} (h : Arith256XLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Arith256XLarge.bus46 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [242, 243] 0 [242, 243] [(t i).sel_arith256, (t i).sel_arith256_mod] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl)
      · exact h.bool_sel_arith256
      · exact h.bool_sel_arith256_mod
    · simp only [opOf, Arith256XLarge.bus46, Arith256XLarge.Arith256XLarge_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithBn254_bus69_not_cmp {x t i} (h : ArithBn254.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithBn254.bus69 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [250, 251, 252, 253, 254] 0 [250, 251, 252, 253, 254] [(t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_bn254_curve_add
      · exact h.bool_sel_bn254_curve_dbl
      · exact h.bool_sel_bn254_complex_add
      · exact h.bool_sel_bn254_complex_sub
      · exact h.bool_sel_bn254_complex_mul
    · simp only [opOf, ArithBn254.bus69, ArithBn254.ArithBn254_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithBn254Large_bus69_not_cmp {x t i} (h : ArithBn254Large.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithBn254Large.bus69 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [250, 251, 252, 253, 254] 0 [250, 251, 252, 253, 254] [(t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_bn254_curve_add
      · exact h.bool_sel_bn254_curve_dbl
      · exact h.bool_sel_bn254_complex_add
      · exact h.bool_sel_bn254_complex_sub
      · exact h.bool_sel_bn254_complex_mul
    · simp only [opOf, ArithBn254Large.bus69, ArithBn254Large.ArithBn254Large_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithEq_bus81_not_cmp {x t i} (h : ArithEq.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithEq.bus81 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] [(t i).sel_arith256, (t i).sel_arith256_mod, (t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl, (t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul, (t i).sel_secp256r1_add, (t i).sel_secp256r1_dbl] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_arith256
      · exact h.bool_sel_arith256_mod
      · exact h.bool_sel_secp256k1_add
      · exact h.bool_sel_secp256k1_dbl
      · exact h.bool_sel_bn254_curve_add
      · exact h.bool_sel_bn254_curve_dbl
      · exact h.bool_sel_bn254_complex_add
      · exact h.bool_sel_bn254_complex_sub
      · exact h.bool_sel_bn254_complex_mul
      · exact h.bool_sel_secp256r1_add
      · exact h.bool_sel_secp256r1_dbl
    · simp only [opOf, ArithEq.bus81, ArithEq.ArithEq_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithEqLarge_bus81_not_cmp {x t i} (h : ArithEqLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithEqLarge.bus81 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] [(t i).sel_arith256, (t i).sel_arith256_mod, (t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl, (t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul, (t i).sel_secp256r1_add, (t i).sel_secp256r1_dbl] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_arith256
      · exact h.bool_sel_arith256_mod
      · exact h.bool_sel_secp256k1_add
      · exact h.bool_sel_secp256k1_dbl
      · exact h.bool_sel_bn254_curve_add
      · exact h.bool_sel_bn254_curve_dbl
      · exact h.bool_sel_bn254_complex_add
      · exact h.bool_sel_bn254_complex_sub
      · exact h.bool_sel_bn254_complex_mul
      · exact h.bool_sel_secp256r1_add
      · exact h.bool_sel_secp256r1_dbl
    · simp only [opOf, ArithEqLarge.bus81, ArithEqLarge.ArithEqLarge_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithEq384_bus67_not_cmp {x t i} (h : ArithEq384.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithEq384.bus67 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [226, 227, 228, 229, 230, 231] 0 [226, 227, 228, 229, 230, 231] [(t i).sel_op 0, (t i).sel_op 1, (t i).sel_op 2, (t i).sel_op 3, (t i).sel_op 4, (t i).sel_op 5] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_op_0
      · exact h.bool_sel_op_1
      · exact h.bool_sel_op_2
      · exact h.bool_sel_op_3
      · exact h.bool_sel_op_4
      · exact h.bool_sel_op_5
    · simp only [opOf, ArithEq384.bus67, ArithEq384.ArithEq384_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithEq384Large_bus67_not_cmp {x t i} (h : ArithEq384Large.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithEq384Large.bus67 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [226, 227, 228, 229, 230, 231] 0 [226, 227, 228, 229, 230, 231] [(t i).sel_op 0, (t i).sel_op 1, (t i).sel_op 2, (t i).sel_op 3, (t i).sel_op 4, (t i).sel_op 5] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_op_0
      · exact h.bool_sel_op_1
      · exact h.bool_sel_op_2
      · exact h.bool_sel_op_3
      · exact h.bool_sel_op_4
      · exact h.bool_sel_op_5
    · simp only [opOf, ArithEq384Large.bus67, ArithEq384Large.ArithEq384Large_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithSecp256K1_bus63_not_cmp {x t i} (h : ArithSecp256K1.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithSecp256K1.bus63 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [244, 245] 0 [244, 245] [(t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl)
      · exact h.bool_sel_secp256k1_add
      · exact h.bool_sel_secp256k1_dbl
    · simp only [opOf, ArithSecp256K1.bus63, ArithSecp256K1.ArithSecp256K1_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem ArithSecp256K1Large_bus63_not_cmp {x t i} (h : ArithSecp256K1Large.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf ArithSecp256K1Large.bus63 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [244, 245] 0 [244, 245] [(t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl)
      · exact h.bool_sel_secp256k1_add
      · exact h.bool_sel_secp256k1_dbl
    · simp only [opOf, ArithSecp256K1Large.bus63, ArithSecp256K1Large.ArithSecp256K1Large_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem Blake2br_bus104_not_cmp {x t i} (_h : Blake2br.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Blake2br.bus104 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 234 [] 234 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Blake2br.bus104, List.headD, linF]
      grind
    · decide
    · decide

theorem Blake2s_bus60_not_cmp {x t i} (_h : Blake2s.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Blake2s.bus60 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 239 [] 239 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Blake2s.bus60, List.headD, linF]
      grind
    · decide
    · decide

theorem Blake3f_bus62_not_cmp {x t i} (_h : Blake3f.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Blake3f.bus62 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 238 [] 238 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Blake3f.bus62, List.headD, linF]
      grind
    · decide
    · decide

theorem Blake3f_bus102_not_cmp {x t i} (_h : Blake3f.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Blake3f.bus102 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 238 [] 238 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Blake3f.bus102, List.headD, linF]
      grind
    · decide
    · decide

theorem Dma_bus51_not_cmp {x t i} (h : Dma.everyRow x t i) :
    ∀ k ∈ ([9, 14] : List Int), opOf Dma.bus51 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [208, 209, 211, 6, 210] 0 [208, 209, 211, 6, 210] [(t i).sel_memcpy, (t i).sel_memcmp, (t i).sel_memset, (t i).sel_extended, (t i).sel_inputcpy] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_memcpy
      · exact h.bool_sel_memcmp
      · exact h.bool_sel_memset
      · exact h.bool_sel_extended
      · exact h.bool_sel_inputcpy
    · simp only [opOf, Dma.bus51, Dma.Dma_bus_op, Dma.Zisk_direct_gsum_e_46, List.headD, linF]
      grind
    · decide
    · decide

theorem Dma64Aligned_bus56_not_cmp {x t i} (h : Dma64Aligned.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Dma64Aligned.bus56 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [214, 218, 217, -6, 210] 0 [214, 218, 217, -6, 210] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load, (t i).sel_inputcpy] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_memcpy
      · exact h.bool_sel_memeq
      · exact h.bool_sel_memset
      · exact h.bool_sel_memcpy_count_load
      · exact h.bool_sel_inputcpy
    · simp only [opOf, Dma64Aligned.bus56, Dma64Aligned.Dma64Aligned_op_e697, List.headD, linF]
      grind
    · decide
    · decide

theorem Dma64AlignedLarge_bus56_not_cmp {x t i} (h : Dma64AlignedLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Dma64AlignedLarge.bus56 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [214, 218, 217, -6, 210] 0 [214, 218, 217, -6, 210] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load, (t i).sel_inputcpy] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl | rfl)
      · exact h.bool_sel_memcpy
      · exact h.bool_sel_memeq
      · exact h.bool_sel_memset
      · exact h.bool_sel_memcpy_count_load
      · exact h.bool_sel_inputcpy
    · simp only [opOf, Dma64AlignedLarge.bus56, Dma64AlignedLarge.Dma64AlignedLarge_op_e697, List.headD, linF]
      grind
    · decide
    · decide

theorem Dma64AlignedMem_bus34_not_cmp {x t i} (h : Dma64AlignedMem.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Dma64AlignedMem.bus34 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [214, 218, 217, -6] 0 [214, 218, 217, -6] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl)
      · exact h.bool_sel_memcpy
      · exact h.bool_sel_memeq
      · exact h.bool_sel_memset
      · exact h.bool_sel_memcpy_count_load
    · simp only [opOf, Dma64AlignedMem.bus34, Dma64AlignedMem.Dma64AlignedMem_op_e471, List.headD, linF]
      grind
    · decide
    · decide

theorem Dma64AlignedMemLarge_bus34_not_cmp {x t i} (h : Dma64AlignedMemLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Dma64AlignedMemLarge.bus34 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [214, 218, 217, -6] 0 [214, 218, 217, -6] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl | rfl | rfl)
      · exact h.bool_sel_memcpy
      · exact h.bool_sel_memeq
      · exact h.bool_sel_memset
      · exact h.bool_sel_memcpy_count_load
    · simp only [opOf, Dma64AlignedMemLarge.bus34, Dma64AlignedMemLarge.Dma64AlignedMemLarge_op_e471, List.headD, linF]
      grind
    · decide
    · decide

theorem Dma64AlignedMemCpy_bus34_not_cmp {x t i} (h : Dma64AlignedMemCpy.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Dma64AlignedMemCpy.bus34 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [214, -6] 0 [214, -6] [(t i).sel_memcpy, (t i).sel_memcpy_count_load] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl)
      · exact h.bool_sel_memcpy
      · exact h.bool_sel_memcpy_count_load
    · simp only [opOf, Dma64AlignedMemCpy.bus34, Dma64AlignedMemCpy.Dma64AlignedMemCpy_op_e635, List.headD, linF]
      grind
    · decide
    · decide

theorem Dma64AlignedMemSet_bus23_not_cmp {x t i} (h : Dma64AlignedMemSet.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Dma64AlignedMemSet.bus23 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [217] 0 [217] [(t i).sel_memset] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_memset
    · simp only [opOf, Dma64AlignedMemSet.bus23, Dma64AlignedMemSet.Dma64AlignedMemSet_op_e343, List.headD, linF]
      grind
    · decide
    · decide

theorem DmaUnaligned_bus24_not_cmp {x t i} (h : DmaUnaligned.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf DmaUnaligned.bus24 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 214 [4] 214 [4] [(t i).is_memeq] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_is_memeq
    · simp only [opOf, DmaUnaligned.bus24, DmaUnaligned.DmaUnaligned_op_e496, List.headD, linF]
      grind
    · decide
    · decide

theorem JumpDest_bus36_not_cmp {x t i} (_h : JumpDest.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf JumpDest.bus36 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 192 [] 192 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, JumpDest.bus36, List.headD, linF]
      grind
    · decide
    · decide

theorem Keccakf_bus92_not_cmp {x t i} (_h : Keccakf.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Keccakf.bus92 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 241 [] 241 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Keccakf.bus92, List.headD, linF]
      grind
    · decide
    · decide

theorem Keccakf_bus93_not_cmp {x t i} (_h : Keccakf.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Keccakf.bus93 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 241 [] 241 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Keccakf.bus93, List.headD, linF]
      grind
    · decide
    · decide

theorem Poseidon_bus139_not_cmp {x t i} (h : Poseidon.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Poseidon.bus139 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 235 [1] 235 [1] [(t i).sel_poseidon1] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_poseidon1
    · simp only [opOf, Poseidon.bus139, List.headD, linF]
      grind
    · decide
    · decide

theorem Sha256f_bus15_not_cmp {x t i} (_h : Sha256f.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Sha256f.bus15 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 249 [] 249 [] [] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp
    · simp only [opOf, Sha256f.bus15, List.headD, linF]
      grind
    · decide
    · decide

theorem TwistedEdwards_bus103_not_cmp {x t i} (h : TwistedEdwards.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf TwistedEdwards.bus103 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 0 [237, 225] 0 [237, 225] [(t i).sel_op 0, (t i).sel_op 1] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s (rfl | rfl)
      · exact h.bool_sel_op_0
      · exact h.bool_sel_op_1
    · simp only [opOf, TwistedEdwards.bus103, TwistedEdwards.TwistedEdwards_bus_op, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAdd_bus13_not_cmp {x t i} (h : BinaryAdd.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAdd.bus13 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 0] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_0
    · simp only [opOf, BinaryAdd.bus13, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddLarge_bus13_not_cmp {x t i} (h : BinaryAddLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddLarge.bus13 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 0] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_0
    · simp only [opOf, BinaryAddLarge.bus13, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddLarge_bus22_not_cmp {x t i} (h : BinaryAddLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddLarge.bus22 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 1] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_1
    · simp only [opOf, BinaryAddLarge.bus22, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHuge_bus13_not_cmp {x t i} (h : BinaryAddHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHuge.bus13 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 0] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_0
    · simp only [opOf, BinaryAddHuge.bus13, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHuge_bus22_not_cmp {x t i} (h : BinaryAddHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHuge.bus22 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 1] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_1
    · simp only [opOf, BinaryAddHuge.bus22, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHuge_bus31_not_cmp {x t i} (h : BinaryAddHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHuge.bus31 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 2] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_2
    · simp only [opOf, BinaryAddHuge.bus31, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHuge_bus40_not_cmp {x t i} (h : BinaryAddHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHuge.bus40 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 3] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_3
    · simp only [opOf, BinaryAddHuge.bus40, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHi_bus9_not_cmp {x t i} (h : BinaryAddHi.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHi.bus9 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 0] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_0
    · simp only [opOf, BinaryAddHi.bus9, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHi_bus14_not_cmp {x t i} (h : BinaryAddHi.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHi.bus14 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 1] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_1
    · simp only [opOf, BinaryAddHi.bus14, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiLarge_bus9_not_cmp {x t i} (h : BinaryAddHiLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiLarge.bus9 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 0] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_0
    · simp only [opOf, BinaryAddHiLarge.bus9, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiLarge_bus14_not_cmp {x t i} (h : BinaryAddHiLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiLarge.bus14 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 1] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_1
    · simp only [opOf, BinaryAddHiLarge.bus14, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiLarge_bus19_not_cmp {x t i} (h : BinaryAddHiLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiLarge.bus19 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 2] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_2
    · simp only [opOf, BinaryAddHiLarge.bus19, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiLarge_bus24_not_cmp {x t i} (h : BinaryAddHiLarge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiLarge.bus24 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 3] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_3
    · simp only [opOf, BinaryAddHiLarge.bus24, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus9_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus9 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 0] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_0
    · simp only [opOf, BinaryAddHiHuge.bus9, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus14_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus14 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 1] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_1
    · simp only [opOf, BinaryAddHiHuge.bus14, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus19_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus19 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 2] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_2
    · simp only [opOf, BinaryAddHiHuge.bus19, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus24_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus24 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 3] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_3
    · simp only [opOf, BinaryAddHiHuge.bus24, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus29_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus29 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 4] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_4
    · simp only [opOf, BinaryAddHiHuge.bus29, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus34_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus34 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 5] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_5
    · simp only [opOf, BinaryAddHiHuge.bus34, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus39_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus39 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 6] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_6
    · simp only [opOf, BinaryAddHiHuge.bus39, List.headD, linF]
      grind
    · decide
    · decide

theorem BinaryAddHiHuge_bus44_not_cmp {x t i} (h : BinaryAddHiHuge.everyRow x t i) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryAddHiHuge.bus44 x t i ≠ ((k : Int) : F) := by
  intro k hk
  simp only [List.mem_cons, List.not_mem_nil, or_false] at hk
  rcases hk with rfl | rfl | rfl
  all_goals
    apply op_ne (F := F) 10 [78] 10 [78] [(t i).sel_sh3add 7] _ _
      (by cast_lits) (by cast_lits) rfl
    · simp only [List.mem_cons, List.not_mem_nil, or_false]
      rintro s rfl
      exact h.bool_sel_sh3add_7
    · simp only [opOf, BinaryAddHiHuge.bus44, List.headD, linF]
      grind
    · decide
    · decide

/-- Dma never proves an `ltu`: its op is 6 only when no base selector is set,
and then its count is 0. -/
theorem Dma_bus51_not_ltu {x t i} (h : Dma.everyRow x t i) (hr : Dma.bus51.reps x t i ≠ 0) :
    opOf Dma.bus51 x t i ≠ ((6 : Int) : F) := by
  apply op_ne2 (F := F) 0 0 [(208, 1), (209, 1), (211, 1), (6, 0), (210, 1)] 0 0
    [208, 209, 211, 6, 210] [1, 1, 1, 0, 1]
    [(t i).sel_memcpy, (t i).sel_memcmp, (t i).sel_memset, (t i).sel_extended, (t i).sel_inputcpy]
    6 _ _ (by cast_lits) (by cast_lits) (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s (rfl | rfl | rfl | rfl | rfl)
    · exact h.bool_sel_memcpy
    · exact h.bool_sel_memcmp
    · exact h.bool_sel_memset
    · exact h.bool_sel_extended
    · exact h.bool_sel_inputcpy
  · simp only [opOf, Dma.bus51, Dma.Dma_bus_op, Dma.Zisk_direct_gsum_e_46, List.headD, linF]
    grind
  · show Dma.Dma_sel x t i + 0 = _
    simp only [Dma.Dma_sel, linF]
    grind
  · decide
  · decide
  · exact hr

omit [Lean.Grind.IsCharP F goldilocks] in
theorem Arith_bus30_not_cmp {x : Arith.Ctx F} {t : Arith.Trace F} {i : Int}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (hl : LookupHolds Arith.bus41 x t [⟨VirtualTableZisk1.bus16, xv, tv⟩])
    (hd : ∀ k ∈ ([6, 9, 14] : List Int), TableNever ⟨VirtualTableZisk1.bus16, xv, tv⟩ 331 ((k : Int) : F)) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf Arith.bus30 x t i ≠ ((k : Int) : F) := by
  intro k hk
  have := not_of_table Arith.bus41 x t i _ 331 _ hl (fun _ _ _ => rfl) one_ne_zero'
    (by simpa using hd k hk)
  simpa [opOf, tupleAt, Arith.bus30, Arith.bus41] using this

omit [Lean.Grind.IsCharP F goldilocks] in
theorem BinaryExtension_bus21_not_cmp {x : BinaryExtension.Ctx F} {t : BinaryExtension.Trace F}
    {i : Int} {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (hl : LookupHolds BinaryExtension.bus11 x t
      [⟨VirtualTableZisk1.bus17, xv, tv⟩, ⟨VirtualTableZisk1.bus18, xv, tv⟩])
    (hd17 : ∀ k ∈ ([6, 9, 14] : List Int), TableNever ⟨VirtualTableZisk1.bus17, xv, tv⟩ 124 ((k : Int) : F))
    (hd18 : ∀ k ∈ ([6, 9, 14] : List Int), TableNever ⟨VirtualTableZisk1.bus18, xv, tv⟩ 124 ((k : Int) : F)) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryExtension.bus21 x t i ≠ ((k : Int) : F) := by
  intro k hk
  have := not_of_table BinaryExtension.bus11 x t i _ 124 _ hl (fun _ _ _ => rfl) one_ne_zero'
    (by simp only [List.mem_cons, List.not_mem_nil, or_false]
        rintro p (rfl | rfl)
        · exact hd17 k hk
        · exact hd18 k hk)
  simpa [opOf, tupleAt, BinaryExtension.bus21, BinaryExtension.bus11] using this

omit [Lean.Grind.IsCharP F goldilocks] in
theorem BinaryExtensionLarge_bus21_not_cmp {x : BinaryExtensionLarge.Ctx F}
    {t : BinaryExtensionLarge.Trace F} {i : Int}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (hl : LookupHolds BinaryExtensionLarge.bus11 x t
      [⟨VirtualTableZisk1.bus17, xv, tv⟩, ⟨VirtualTableZisk1.bus18, xv, tv⟩])
    (hd17 : ∀ k ∈ ([6, 9, 14] : List Int), TableNever ⟨VirtualTableZisk1.bus17, xv, tv⟩ 124 ((k : Int) : F))
    (hd18 : ∀ k ∈ ([6, 9, 14] : List Int), TableNever ⟨VirtualTableZisk1.bus18, xv, tv⟩ 124 ((k : Int) : F)) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryExtensionLarge.bus21 x t i ≠ ((k : Int) : F) := by
  intro k hk
  have := not_of_table BinaryExtensionLarge.bus11 x t i _ 124 _ hl (fun _ _ _ => rfl) one_ne_zero'
    (by simp only [List.mem_cons, List.not_mem_nil, or_false]
        rintro p (rfl | rfl)
        · exact hd17 k hk
        · exact hd18 k hk)
  simpa [opOf, tupleAt, BinaryExtensionLarge.bus21, BinaryExtensionLarge.bus11] using this

omit [Lean.Grind.IsCharP F goldilocks] in
theorem BinaryExtensionLarge_bus32_not_cmp {x : BinaryExtensionLarge.Ctx F}
    {t : BinaryExtensionLarge.Trace F} {i : Int}
    {xv : VirtualTableZisk1.Ctx F} {tv : VirtualTableZisk1.Trace F}
    (hl : LookupHolds BinaryExtensionLarge.bus22 x t
      [⟨VirtualTableZisk1.bus17, xv, tv⟩, ⟨VirtualTableZisk1.bus18, xv, tv⟩])
    (hd17 : ∀ k ∈ ([6, 9, 14] : List Int), TableNever ⟨VirtualTableZisk1.bus17, xv, tv⟩ 124 ((k : Int) : F))
    (hd18 : ∀ k ∈ ([6, 9, 14] : List Int), TableNever ⟨VirtualTableZisk1.bus18, xv, tv⟩ 124 ((k : Int) : F)) :
    ∀ k ∈ ([6, 9, 14] : List Int), opOf BinaryExtensionLarge.bus32 x t i ≠ ((k : Int) : F) := by
  intro k hk
  have := not_of_table BinaryExtensionLarge.bus22 x t i _ 124 _ hl (fun _ _ _ => rfl) one_ne_zero'
    (by simp only [List.mem_cons, List.not_mem_nil, or_false]
        rintro p (rfl | rfl)
        · exact hd17 k hk
        · exact hd18 k hk)
  simpa [opOf, tupleAt, BinaryExtensionLarge.bus32, BinaryExtensionLarge.bus22] using this

end Provers

end Correspondence
