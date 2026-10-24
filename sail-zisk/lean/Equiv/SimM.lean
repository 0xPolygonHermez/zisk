/-
  The M extension's values: ZisK's arith ops, as the transpiler emits them
  (create_register_op, one op per instruction), compute what sail-riscv's
  MUL, DIV, REM, MULW, DIVW and REMW do, division by zero and the signed
  overflow included: sail-riscv clamps a quotient of 2^63 (MIN / -1) to
  -2^63, which has the same 64 bits.
-/
import Equiv.RiscvM
import Equiv.Sim

set_option linter.unusedSimpArgs false

namespace Equiv.Sim
open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

theorem mul_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_MUL ()) a b).1 = mulVal ⟨.Low, .Signed, .Signed⟩ a b := by
  simp [_root_.Zisk.Functions.zisk_exec, mulVal, mult_to_bits_half, _root_.Zisk.Functions.prod128,
    _root_.Zisk.Functions.to_bits, to_bits_truncate]
theorem mulh_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_MULH ()) a b).1 = mulVal ⟨.High, .Signed, .Signed⟩ a b := by
  simp [_root_.Zisk.Functions.zisk_exec, mulVal, mult_to_bits_half, _root_.Zisk.Functions.prod128,
    _root_.Zisk.Functions.to_bits, to_bits_truncate]
theorem mulhsu_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_MULSUH ()) a b).1 = mulVal ⟨.High, .Signed, .Unsigned⟩ a b := by
  simp [_root_.Zisk.Functions.zisk_exec, mulVal, mult_to_bits_half, _root_.Zisk.Functions.prod128,
    _root_.Zisk.Functions.to_bits, to_bits_truncate]
theorem mulhu_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_MULUH ()) a b).1 = mulVal ⟨.High, .Unsigned, .Unsigned⟩ a b := by
  simp [_root_.Zisk.Functions.zisk_exec, mulVal, mult_to_bits_half, _root_.Zisk.Functions.prod128,
    _root_.Zisk.Functions.to_bits, to_bits_truncate]

theorem gsi_toNatInt (a : BitVec 64) : get_slice_int 64 (BitVec.toNatInt a) 0 = a := by
  apply BitVec.eq_of_toNat_eq
  simp [get_slice_int, BitVec.toNatInt]
  omega

theorem gsi_toInt (a : BitVec 64) : get_slice_int 64 a.toInt 0 = a := by
  apply BitVec.eq_of_toNat_eq
  simp [get_slice_int]
  rw [BitVec.toInt_eq_toNat_cond]
  have := a.isLt
  split <;> omega

theorem gsi_m1 : get_slice_int 64 (-1) 0 = _root_.Zisk.Functions.sail_ones 64 := by decide

theorem toNatInt_eq_zero (b : BitVec 64) : BitVec.toNatInt b = 0 ↔ b = 0 := by
  constructor
  · intro h; apply BitVec.eq_of_toNat_eq; simp [BitVec.toNatInt] at h; simpa using h
  · intro h; subst h; rfl

theorem divu_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_DIVU ()) a b).1 = divVal true a b := by
  simp [_root_.Zisk.Functions.zisk_exec, divVal, _root_.Zisk.Functions.to_bits, to_bits_truncate, gsi_toNatInt]
  by_cases hb : b = 0
  · subst hb; simp [_root_.Zisk.Functions.zeros, BitVec.toNatInt, gsi_m1]
  · have : ¬ BitVec.toNatInt b = 0 := fun h => hb ((toNatInt_eq_zero b).1 h)
    have hb' : ¬ b = 0#64 := hb
    simp [hb', this, _root_.Zisk.Functions.zeros]
theorem remu_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_REMU ()) a b).1 = remVal true a b := by
  simp [_root_.Zisk.Functions.zisk_exec, remVal, _root_.Zisk.Functions.to_bits, to_bits_truncate, gsi_toNatInt]
  by_cases hb : b = 0
  · subst hb; simp [_root_.Zisk.Functions.zeros]; exact (gsi_toNatInt a).symm
  · have : ¬ BitVec.toNatInt b = 0 := fun h => hb ((toNatInt_eq_zero b).1 h)
    have hb' : ¬ b = 0#64 := hb
    simp [hb', this, _root_.Zisk.Functions.zeros]
theorem toInt_eq_zero' (b : BitVec 64) : b.toInt = 0 ↔ b = 0 := by
  constructor
  · intro h; have := BitVec.toInt_inj (x := b) (y := 0); simp_all
  · intro h; subst h; rfl

theorem gsi_min : get_slice_int 64 (-2 ^ 63) 0 = get_slice_int 64 (2 ^ 63) 0 := by decide

theorem div_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_DIV ()) a b).1 = divVal false a b := by
  simp [_root_.Zisk.Functions.zisk_exec, divVal, _root_.Zisk.Functions.to_bits, to_bits_truncate]
  by_cases hb : b = 0
  · subst hb; simp [_root_.Zisk.Functions.zeros]
    rw [if_neg (by decide)]; exact gsi_m1.symm
  · have : ¬ b.toInt = 0 := fun h => hb ((toInt_eq_zero' b).1 h)
    have hb' : ¬ b = 0#64 := hb
    simp [hb', this, _root_.Zisk.Functions.zeros]
    split
    · rename_i hq
      have h1 := Int.natAbs_tdiv_le_natAbs a.toInt b.toInt
      have h2 := BitVec.le_toInt a
      have h3 := BitVec.toInt_le (x := a)
      have : a.toInt.tdiv b.toInt = 2 ^ 63 := by simp at h2 h3; omega
      rw [this]; exact gsi_min.symm
    · rfl

theorem rem_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_REM ()) a b).1 = remVal false a b := by
  simp [_root_.Zisk.Functions.zisk_exec, remVal, _root_.Zisk.Functions.to_bits, to_bits_truncate]
  by_cases hb : b = 0
  · subst hb; simp [_root_.Zisk.Functions.zeros]; exact (gsi_toInt a).symm
  · have : ¬ b.toInt = 0 := fun h => hb ((toInt_eq_zero' b).1 h)
    have hb' : ¬ b = 0#64 := hb
    simp [hb', this, _root_.Zisk.Functions.zeros]

theorem gsi32_toNatInt (a : BitVec 32) : get_slice_int 32 (BitVec.toNatInt a) 0 = a := by
  apply BitVec.eq_of_toNat_eq
  simp [get_slice_int, BitVec.toNatInt]
  omega

theorem gsi32_toInt (a : BitVec 32) : get_slice_int 32 a.toInt 0 = a := by
  apply BitVec.eq_of_toNat_eq
  simp [get_slice_int]
  rw [BitVec.toInt_eq_toNat_cond]
  have := a.isLt
  split <;> omega

theorem toNatInt32_eq_zero (b : BitVec 32) : BitVec.toNatInt b = 0 ↔ b = 0 := by
  constructor
  · intro h; apply BitVec.eq_of_toNat_eq; simp [BitVec.toNatInt] at h; simpa using h
  · intro h; subst h; rfl

theorem toInt32_eq_zero (b : BitVec 32) : b.toInt = 0 ↔ b = 0 := by
  constructor
  · intro h; have := BitVec.toInt_inj (x := b) (y := 0); simp_all
  · intro h; subst h; rfl

theorem gsi32_min : get_slice_int 32 (-2 ^ 31) 0 = get_slice_int 32 (2 ^ 31) 0 := by decide

theorem slice_gsi128 (n : Int) :
    Sail.BitVec.extractLsb (get_slice_int 128 n 0) 31 0 = get_slice_int 32 n 0 := by
  apply BitVec.eq_of_toNat_eq
  simp [get_slice_int, Sail.BitVec.extractLsb]
  omega

theorem ones_ext : Sail.BitVec.signExtend (get_slice_int 32 (-1) 0) 64 = _root_.Zisk.Functions.sail_ones 64 := by
  decide

theorem mulw_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_MUL_W ()) a b).1 = mulwVal a b := by
  simp [_root_.Zisk.Functions.zisk_exec, mulwVal, _root_.Zisk.Functions.prod128, _root_.Zisk.Functions.to_bits,
    to_bits_truncate, _root_.Zisk.Functions.sign_extend, sign_extend]
  congr 1; exact slice_gsi128 _

/-- The divisor's low word, zero or not, as both sides test it. -/
theorem b32_cases (b : BitVec 64) :
    (Sail.BitVec.extractLsb b 31 0 = 0#32 ∧ BitVec.toNatInt (Sail.BitVec.extractLsb b 31 0) = 0 ∧
      (Sail.BitVec.extractLsb b 31 0 : BitVec 32).toInt = 0) ∨
    (¬ Sail.BitVec.extractLsb b 31 0 = 0#32 ∧ ¬ BitVec.toNatInt (Sail.BitVec.extractLsb b 31 0) = 0 ∧
      ¬ (Sail.BitVec.extractLsb b 31 0 : BitVec 32).toInt = 0) := by
  by_cases h : (Sail.BitVec.extractLsb b 31 0 : BitVec 32) = 0
  · left; exact ⟨h, (toNatInt32_eq_zero _).2 h, (toInt32_eq_zero _).2 h⟩
  · right; exact ⟨h, fun e => h ((toNatInt32_eq_zero _).1 e), fun e => h ((toInt32_eq_zero _).1 e)⟩

macro "mw_simp" : tactic => `(tactic|
  simp [_root_.Zisk.Functions.zisk_exec, divwVal, remwVal, _root_.Zisk.Functions.to_bits, to_bits_truncate,
    _root_.Zisk.Functions.sign_extend, sign_extend, _root_.Zisk.Functions.zeros, _root_.Zisk.Functions.sx32])

theorem divuw_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_DIVU_W ()) a b).1 = divwVal true a b := by
  rcases b32_cases b with ⟨h0, h1, -⟩ | ⟨h0, h1, -⟩ <;> mw_simp <;> simp [h0, h1]
  rw [if_pos (by rfl)]; exact ones_ext.symm

theorem remuw_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_REMU_W ()) a b).1 = remwVal true a b := by
  rcases b32_cases b with ⟨h0, h1, -⟩ | ⟨h0, h1, -⟩ <;> mw_simp <;> simp [h0, h1]
  rw [if_pos (by rfl), gsi32_toNatInt]

theorem divw_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_DIV_W ()) a b).1 = divwVal false a b := by
  rcases b32_cases b with ⟨h0, -, h1⟩ | ⟨h0, -, h1⟩ <;> mw_simp <;> simp [h0, h1]
  · rw [if_neg (by decide)]; exact ones_ext.symm
  · split
    · rename_i hq
      have e1 := Int.natAbs_tdiv_le_natAbs (Sail.BitVec.extractLsb a 31 0 : BitVec 32).toInt
        (Sail.BitVec.extractLsb b 31 0 : BitVec 32).toInt
      have e2 := BitVec.le_toInt (Sail.BitVec.extractLsb a 31 0 : BitVec 32)
      have e3 := BitVec.toInt_le (x := (Sail.BitVec.extractLsb a 31 0 : BitVec 32))
      have : (Sail.BitVec.extractLsb a 31 0 : BitVec 32).toInt.tdiv
          (Sail.BitVec.extractLsb b 31 0 : BitVec 32).toInt = 2 ^ 31 := by simp at e2 e3; omega
      rw [this]; exact congrArg _ gsi32_min.symm
    · rfl

theorem remw_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_REM_W ()) a b).1 = remwVal false a b := by
  rcases b32_cases b with ⟨h0, -, h1⟩ | ⟨h0, -, h1⟩ <;> mw_simp <;> simp [h0, h1]
  exact congrArg _ (gsi32_toInt _).symm

end Equiv.Sim
