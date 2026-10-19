/-
  Who cannot prove an `add`.

  Main sends every external op over the operation bus (5000), and forty AIRs
  prove on it. An `add` slot is correct only if every one of them that could
  prove a tuple with op `0x0a` computes `a + b`, so the first job is to rule out
  the ones that cannot. This file does that for every prover whose op is a
  constant or a sum of opcodes weighted by selectors: from the AIR's own
  constraints, its op is never 10. (The census is in the README, "Who can prove
  an add"; Arith and BinaryExtension, whose op is ruled out by a table, and the
  three kinds that can prove `add`, are not here.)

  One lemma does the work: an op of the form `c₀ + Σ cₖ·sₖ`, every `sₖ` a bit,
  takes one of the finitely many values `c₀ + Σ_{k ∈ S} cₖ`. If none is 10, and
  all are small next to the characteristic, the op is not 10. The values are
  checked by `decide`; that each `sₖ` is a bit is the AIR's booleanity
  constraint, reached by column name (`everyRow.bool_sel_arith256`).

  The 29 theorems below, `<Air>_<bus>_not_add`, one per proving argument, were
  written out mechanically from the generated bus arguments: the op element of
  each tuple, read as a constant plus opcodes times selectors. They name the
  bus argument (`Arith256X.bus46`) and the selectors' columns, so a regenerated
  pilout that renumbers an argument or renames a column breaks the build here
  rather than going unnoticed.
-/
import Proofs.Rom
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

-- `decide` enumerates every subset of an op's selectors: 2048 for ArithEq.
set_option maxRecDepth 100000

namespace Correspondence

section Lemma

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- The values `Σ_{k ∈ S} cₖ` over every subset `S`. -/
def subsetSums : List Int → List Int
  | [] => [0]
  | c :: cs => subsetSums cs ++ (subsetSums cs).map (c + ·)

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- An integer literal, cast into the field, is the field literal. -/
theorem intCast_lit (n : Nat) : (Int.cast (OfNat.ofNat n : Int) : F) = (OfNat.ofNat n : F) := by
  rw [show (OfNat.ofNat n : Int) = ((n : Nat) : Int) from rfl, Lean.Grind.Ring.intCast_natCast,
    Lean.Grind.Semiring.ofNat_eq_natCast]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A negative one likewise. -/
theorem intCast_neg_lit (n : Nat) :
    (Int.cast (-(OfNat.ofNat n : Int)) : F) = -(OfNat.ofNat n : F) := by
  rw [Lean.Grind.Ring.intCast_neg, intCast_lit]

/-- Turn integer literals cast into the field into field literals. `simp` does
not match a literal against `OfNat.ofNat n`, so this rewrites one at a time. -/
macro "cast_lits" : tactic =>
  `(tactic| (try simp only [List.map_cons, List.map_nil];
             repeat (first | rw [intCast_neg_lit] | rw [intCast_lit])))

/-- `Σ cₖ·sₖ`, with the coefficients as field elements. -/
def linF : List F → List F → F
  | c :: cs, s :: ss => c * s + linF cs ss
  | _, _ => 0

omit [Lean.Grind.IsCharP F goldilocks] in
theorem linF_bits (cs : List Int) (ss : List F) (hl : cs.length = ss.length)
    (hb : ∀ s ∈ ss, s * (1 - s) = 0) :
    ∃ z ∈ subsetSums cs, linF (cs.map (fun c => (c : F))) ss = (z : F) := by
  induction cs generalizing ss with
  | nil => exact ⟨0, by simp [subsetSums], by simp [linF, Lean.Grind.Ring.intCast_zero]⟩
  | cons c cs ih =>
    cases ss with
    | nil => simp at hl
    | cons s ss =>
      obtain ⟨z, hz, e⟩ := ih ss (by simpa using hl) (fun x hx => hb x (by simp [hx]))
      obtain ⟨b, hs⟩ := bool_of_idem (hb s (by simp))
      cases b
      · refine ⟨z, List.mem_append_left _ hz, ?_⟩
        simp only [List.map_cons, linF, e, hs, bit, Bool.false_eq_true, if_false]
        grind
      · refine ⟨c + z, List.mem_append_right _ (List.mem_map.mpr ⟨z, hz, rfl⟩), ?_⟩
        simp only [List.map_cons, linF, e, hs, bit, if_true, Lean.Grind.Ring.intCast_add]
        grind

/-- **An op that is a weighted sum of bits is not `k`**, when no value it can
take is `k` and all of them, and `k`, are far below the characteristic.

The coefficients come twice: as integers `cs`, whose subset sums `decide`
checks, and as the field literals `fs` the op is written with. `hcf` ties them;
keeping the literals as literals is what lets `grind` prove `heq` (see
`natCast_two` in `Rom.lean`). -/
theorem op_ne (c₀ : Int) (cs : List Int) (f₀ : F) (fs ss : List F) (k : Int) (op : F)
    (hc₀ : ((c₀ : Int) : F) = f₀) (hcf : cs.map (fun c => (c : F)) = fs)
    (hl : cs.length = ss.length) (hb : ∀ s ∈ ss, s * (1 - s) = 0)
    (heq : op = f₀ + linF fs ss)
    (hvals : ∀ z ∈ subsetSums cs, c₀ + z ≠ k ∧ -2 ^ 62 < c₀ + z ∧ c₀ + z < 2 ^ 62)
    (hk : -2 ^ 62 < k ∧ k < 2 ^ 62) :
    op ≠ (k : F) := by
  obtain ⟨z, hz, e⟩ := linF_bits cs ss hl hb
  obtain ⟨hne, lo, hi⟩ := hvals z hz
  intro h
  rw [heq, ← hcf, e, ← hc₀, ← Lean.Grind.Ring.intCast_add] at h
  have := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp h
  unfold goldilocks at this
  omega

end Lemma

section Provers

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

open Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The op a bus argument sends: its tuple's first element. -/
def opOf {C T : Type} (a : BusArg F C T) : C → T → Int → F :=
  a.tuple.headD (fun _ _ _ => 0)

theorem Add256_bus59_not_add {x t i} (_h : Add256.everyRow x t i) :
    opOf Add256.bus59 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 240 [] 240 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Add256.bus59, List.headD, linF]
    grind
  · decide
  · decide

theorem Arith256X_bus46_not_add {x t i} (h : Arith256X.everyRow x t i) :
    opOf Arith256X.bus46 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [242, 243] 0 [242, 243] [(t i).sel_arith256, (t i).sel_arith256_mod] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s (rfl | rfl)
    · exact h.bool_sel_arith256
    · exact h.bool_sel_arith256_mod
  · simp only [opOf, Arith256X.bus46, Arith256X.Arith256X_bus_op, List.headD, linF]
    grind
  · decide
  · decide

theorem Arith256XLarge_bus46_not_add {x t i} (h : Arith256XLarge.everyRow x t i) :
    opOf Arith256XLarge.bus46 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [242, 243] 0 [242, 243] [(t i).sel_arith256, (t i).sel_arith256_mod] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s (rfl | rfl)
    · exact h.bool_sel_arith256
    · exact h.bool_sel_arith256_mod
  · simp only [opOf, Arith256XLarge.bus46, Arith256XLarge.Arith256XLarge_bus_op, List.headD, linF]
    grind
  · decide
  · decide

theorem ArithBn254_bus69_not_add {x t i} (h : ArithBn254.everyRow x t i) :
    opOf ArithBn254.bus69 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [250, 251, 252, 253, 254] 0 [250, 251, 252, 253, 254] [(t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul] 10 _
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

theorem ArithBn254Large_bus69_not_add {x t i} (h : ArithBn254Large.everyRow x t i) :
    opOf ArithBn254Large.bus69 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [250, 251, 252, 253, 254] 0 [250, 251, 252, 253, 254] [(t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul] 10 _
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

theorem ArithEq_bus81_not_add {x t i} (h : ArithEq.everyRow x t i) :
    opOf ArithEq.bus81 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] [(t i).sel_arith256, (t i).sel_arith256_mod, (t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl, (t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul, (t i).sel_secp256r1_add, (t i).sel_secp256r1_dbl] 10 _
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

theorem ArithEqLarge_bus81_not_add {x t i} (h : ArithEqLarge.everyRow x t i) :
    opOf ArithEqLarge.bus81 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] 0 [242, 243, 244, 245, 250, 251, 252, 253, 254, 232, 233] [(t i).sel_arith256, (t i).sel_arith256_mod, (t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl, (t i).sel_bn254_curve_add, (t i).sel_bn254_curve_dbl, (t i).sel_bn254_complex_add, (t i).sel_bn254_complex_sub, (t i).sel_bn254_complex_mul, (t i).sel_secp256r1_add, (t i).sel_secp256r1_dbl] 10 _
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

theorem ArithEq384_bus67_not_add {x t i} (h : ArithEq384.everyRow x t i) :
    opOf ArithEq384.bus67 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [226, 227, 228, 229, 230, 231] 0 [226, 227, 228, 229, 230, 231] [(t i).sel_op 0, (t i).sel_op 1, (t i).sel_op 2, (t i).sel_op 3, (t i).sel_op 4, (t i).sel_op 5] 10 _
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

theorem ArithEq384Large_bus67_not_add {x t i} (h : ArithEq384Large.everyRow x t i) :
    opOf ArithEq384Large.bus67 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [226, 227, 228, 229, 230, 231] 0 [226, 227, 228, 229, 230, 231] [(t i).sel_op 0, (t i).sel_op 1, (t i).sel_op 2, (t i).sel_op 3, (t i).sel_op 4, (t i).sel_op 5] 10 _
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

theorem ArithSecp256K1_bus63_not_add {x t i} (h : ArithSecp256K1.everyRow x t i) :
    opOf ArithSecp256K1.bus63 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [244, 245] 0 [244, 245] [(t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s (rfl | rfl)
    · exact h.bool_sel_secp256k1_add
    · exact h.bool_sel_secp256k1_dbl
  · simp only [opOf, ArithSecp256K1.bus63, ArithSecp256K1.ArithSecp256K1_bus_op, List.headD, linF]
    grind
  · decide
  · decide

theorem ArithSecp256K1Large_bus63_not_add {x t i} (h : ArithSecp256K1Large.everyRow x t i) :
    opOf ArithSecp256K1Large.bus63 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [244, 245] 0 [244, 245] [(t i).sel_secp256k1_add, (t i).sel_secp256k1_dbl] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s (rfl | rfl)
    · exact h.bool_sel_secp256k1_add
    · exact h.bool_sel_secp256k1_dbl
  · simp only [opOf, ArithSecp256K1Large.bus63, ArithSecp256K1Large.ArithSecp256K1Large_bus_op, List.headD, linF]
    grind
  · decide
  · decide

theorem Blake2br_bus104_not_add {x t i} (_h : Blake2br.everyRow x t i) :
    opOf Blake2br.bus104 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 234 [] 234 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Blake2br.bus104, List.headD, linF]
    grind
  · decide
  · decide

theorem Blake2s_bus60_not_add {x t i} (_h : Blake2s.everyRow x t i) :
    opOf Blake2s.bus60 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 239 [] 239 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Blake2s.bus60, List.headD, linF]
    grind
  · decide
  · decide

theorem Blake3f_bus62_not_add {x t i} (_h : Blake3f.everyRow x t i) :
    opOf Blake3f.bus62 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 238 [] 238 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Blake3f.bus62, List.headD, linF]
    grind
  · decide
  · decide

theorem Blake3f_bus102_not_add {x t i} (_h : Blake3f.everyRow x t i) :
    opOf Blake3f.bus102 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 238 [] 238 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Blake3f.bus102, List.headD, linF]
    grind
  · decide
  · decide

theorem Dma_bus51_not_add {x t i} (h : Dma.everyRow x t i) :
    opOf Dma.bus51 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [208, 209, 211, 6, 210] 0 [208, 209, 211, 6, 210] [(t i).sel_memcpy, (t i).sel_memcmp, (t i).sel_memset, (t i).sel_extended, (t i).sel_inputcpy] 10 _
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

theorem Dma64Aligned_bus56_not_add {x t i} (h : Dma64Aligned.everyRow x t i) :
    opOf Dma64Aligned.bus56 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [214, 218, 217, -6, 210] 0 [214, 218, 217, -6, 210] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load, (t i).sel_inputcpy] 10 _
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

theorem Dma64AlignedLarge_bus56_not_add {x t i} (h : Dma64AlignedLarge.everyRow x t i) :
    opOf Dma64AlignedLarge.bus56 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [214, 218, 217, -6, 210] 0 [214, 218, 217, -6, 210] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load, (t i).sel_inputcpy] 10 _
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

theorem Dma64AlignedMem_bus34_not_add {x t i} (h : Dma64AlignedMem.everyRow x t i) :
    opOf Dma64AlignedMem.bus34 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [214, 218, 217, -6] 0 [214, 218, 217, -6] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load] 10 _
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

theorem Dma64AlignedMemLarge_bus34_not_add {x t i} (h : Dma64AlignedMemLarge.everyRow x t i) :
    opOf Dma64AlignedMemLarge.bus34 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [214, 218, 217, -6] 0 [214, 218, 217, -6] [(t i).sel_memcpy, (t i).sel_memeq, (t i).sel_memset, (t i).sel_memcpy_count_load] 10 _
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

theorem Dma64AlignedMemCpy_bus34_not_add {x t i} (h : Dma64AlignedMemCpy.everyRow x t i) :
    opOf Dma64AlignedMemCpy.bus34 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [214, -6] 0 [214, -6] [(t i).sel_memcpy, (t i).sel_memcpy_count_load] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s (rfl | rfl)
    · exact h.bool_sel_memcpy
    · exact h.bool_sel_memcpy_count_load
  · simp only [opOf, Dma64AlignedMemCpy.bus34, Dma64AlignedMemCpy.Dma64AlignedMemCpy_op_e635, List.headD, linF]
    grind
  · decide
  · decide

theorem Dma64AlignedMemSet_bus23_not_add {x t i} (h : Dma64AlignedMemSet.everyRow x t i) :
    opOf Dma64AlignedMemSet.bus23 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [217] 0 [217] [(t i).sel_memset] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s rfl
    exact h.bool_sel_memset
  · simp only [opOf, Dma64AlignedMemSet.bus23, Dma64AlignedMemSet.Dma64AlignedMemSet_op_e343, List.headD, linF]
    grind
  · decide
  · decide

theorem DmaUnaligned_bus24_not_add {x t i} (h : DmaUnaligned.everyRow x t i) :
    opOf DmaUnaligned.bus24 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 214 [4] 214 [4] [(t i).is_memeq] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s rfl
    exact h.bool_is_memeq
  · simp only [opOf, DmaUnaligned.bus24, DmaUnaligned.DmaUnaligned_op_e496, List.headD, linF]
    grind
  · decide
  · decide

theorem JumpDest_bus36_not_add {x t i} (_h : JumpDest.everyRow x t i) :
    opOf JumpDest.bus36 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 192 [] 192 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, JumpDest.bus36, List.headD, linF]
    grind
  · decide
  · decide

theorem Keccakf_bus92_not_add {x t i} (_h : Keccakf.everyRow x t i) :
    opOf Keccakf.bus92 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 241 [] 241 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Keccakf.bus92, List.headD, linF]
    grind
  · decide
  · decide

theorem Keccakf_bus93_not_add {x t i} (_h : Keccakf.everyRow x t i) :
    opOf Keccakf.bus93 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 241 [] 241 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Keccakf.bus93, List.headD, linF]
    grind
  · decide
  · decide

theorem Poseidon_bus139_not_add {x t i} (h : Poseidon.everyRow x t i) :
    opOf Poseidon.bus139 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 235 [1] 235 [1] [(t i).sel_poseidon1] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s rfl
    exact h.bool_sel_poseidon1
  · simp only [opOf, Poseidon.bus139, List.headD, linF]
    grind
  · decide
  · decide

theorem Sha256f_bus15_not_add {x t i} (_h : Sha256f.everyRow x t i) :
    opOf Sha256f.bus15 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 249 [] 249 [] [] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp
  · simp only [opOf, Sha256f.bus15, List.headD, linF]
    grind
  · decide
  · decide

theorem TwistedEdwards_bus103_not_add {x t i} (h : TwistedEdwards.everyRow x t i) :
    opOf TwistedEdwards.bus103 x t i ≠ ((10 : Int) : F) := by
  apply op_ne (F := F) 0 [237, 225] 0 [237, 225] [(t i).sel_op 0, (t i).sel_op 1] 10 _
    (by cast_lits) (by cast_lits) rfl
  · simp only [List.mem_cons, List.not_mem_nil, or_false]
    rintro s (rfl | rfl)
    · exact h.bool_sel_op_0
    · exact h.bool_sel_op_1
  · simp only [opOf, TwistedEdwards.bus103, TwistedEdwards.TwistedEdwards_bus_op, List.headD, linF]
    grind
  · decide
  · decide

end Provers

end Correspondence
