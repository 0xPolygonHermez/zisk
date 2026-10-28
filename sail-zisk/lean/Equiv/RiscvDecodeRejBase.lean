/-
  sail-riscv's decoder on words RISC-V rejects within an opcode the profile
  has: its clauses for that opcode match bits the word may have, and find
  it illegal only inside — an extension off, a field combination invalid.
  `peel_r` steps through the chain as `peel_i`, and through such a clause
  by its body: when the guard holds, the body decodes nothing
  (`ite_none_run`), through binds whose values it does not need
  (`bind_none_run`), extension checks, and branches the word's bits decide.
-/
import Equiv.RiscvDecodeCsrBase
import Equiv.RiscvDecodeAmoBase
import Equiv.RiscvDecodeZbaBase
import Equiv.RiscvDecodeZbsBase
import Equiv.RiscvDecodeZbbBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A clause whose body, when its guard holds, decodes nothing: nothing. -/
theorem ite_none_run {α : Type} (c : Bool) (A : SailM (Option α)) (s : RVState)
    (hA : c = true → (interpretSailEffects A).run s = .ok none s) :
    (interpretSailEffects (if c = true then A else pure none)).run s = .ok none s := by
  split
  · next h => exact hA h
  · rfl

/-- A step whose value the rest does not need, the state as it was. -/
theorem bind_none_run {α β : Type} (m : SailM α) (f : α → SailM (Option β)) (s : RVState)
    (hm : ∃ a, (interpretSailEffects m).run s = .ok a s)
    (hf : ∀ a, (interpretSailEffects (f a)).run s = .ok none s) :
    (interpretSailEffects (m >>= f)).run s = .ok none s := by
  obtain ⟨a, ha⟩ := hm
  rw [interp_bind, run_bind', ha]; exact hf a

theorem ce_zabha (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zabha)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_zalrsc (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) (ha : _get_Misa_A v = 1#1) :
    (interpretSailEffects (currentlyEnabled .Ext_Zalrsc)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, ha]

theorem ce_zicfiss (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zicfiss)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_zacas (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zacas)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ite_pure_run (s : RVState) (c : Prop) [Decidable c] (a b : Bool) :
    (interpretSailEffects (if c then pure a else pure b : SailM Bool)).run s = .ok (if c then a else b) s := by
  split <;> rfl

/-- An AMO of 1, 2 or 16 bytes is no valid encoding here: Zabha and Zacas
    are off, and 16 bytes is past xlen for anything but amocas. -/
theorem amo_valid_false (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v)
    (n : Nat) (hn : n = 1 ∨ n = 2 ∨ n = 16) (op : amoop) (rs2 rd : regidx) :
    (interpretSailEffects (amo_encoding_valid n op rs2 rd)).run s = .ok false s := by
  obtain ⟨rs2⟩ := rs2
  obtain ⟨rd⟩ := rd
  by_cases hodd : BitVec.access rs2 0 = 1#1 ∨ BitVec.access rd 0 = 1#1
  · rcases hn with rfl | rfl | rfl <;> cases op <;>
      simp (config := { decide := true }) [amo_encoding_valid, currentlyEnabled, hartSupports,
        readReg_run _ _ _ hv, Functions.xlen_bytes, amocas_odd_register_reserved_behavior, ite_pure_run, beq_iff_eq, hodd]
  · rcases hn with rfl | rfl | rfl <;> cases op <;>
      simp (config := { decide := true }) [amo_encoding_valid, currentlyEnabled, hartSupports,
        readReg_run _ _ _ hv, Functions.xlen_bytes, amocas_odd_register_reserved_behavior, ite_pure_run, beq_iff_eq, hodd]

/-- An AMO compare-and-swap is no valid encoding here, Zacas off, whatever its width. -/
theorem amocas_valid_false (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v)
    (n : Nat) (rs2 rd : regidx) :
    (interpretSailEffects (amo_encoding_valid n .AMOCAS rs2 rd)).run s = .ok false s := by
  obtain ⟨rs2⟩ := rs2
  obtain ⟨rd⟩ := rd
  by_cases hodd : BitVec.access rs2 0 = 1#1 ∨ BitVec.access rd 0 = 1#1 <;>
  by_cases hn : n < 4 <;>
    simp (config := { decide := true }) [amo_encoding_valid, currentlyEnabled, hartSupports,
      readReg_run _ _ _ hv, Functions.xlen_bytes, amocas_odd_register_reserved_behavior, ite_pure_run, beq_iff_eq, hn]

/-- The branch functs RISC-V has, as bits. -/
theorem bop_matches_eq (b : BitVec 3) : encdec_bop_backwards_matches b = !(b == 2#3 || b == 3#3) := by
  revert b; decide

/-- The AMO functs RISC-V has, as bits. -/
theorem amoop_matches_eq (b : BitVec 5) : encdec_amoop_backwards_matches b =
    (b == 1#5 || b == 0#5 || b == 4#5 || b == 12#5 || b == 8#5 || b == 16#5 || b == 20#5 || b == 24#5 ||
      b == 28#5 || b == 5#5) := by
  revert b; decide

/-- bv_decide's view of an AMO op, made here once: each class module would
    make its own, and two of them could not be imported together. -/
theorem amoop_bv (a : amoop) (h : a = .AMOCAS) : a = .AMOCAS := by bv_decide

theorem rej_xlen32 : (Functions.xlen == 32) = false := by decide
theorem rej_xlen64 : (Functions.xlen == 64) = true := by decide

/-- A value the code returns, the state as it was. -/
macro "returns" : tactic => `(tactic| first
  | exact ⟨_, rfl⟩
  | exact ⟨_, readReg_run _ _ _ (by assumption)⟩
  | (simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false, Bool.true_or,
      if_true]; exact ⟨_, rfl⟩)
  | (unfold encdec_cbop_backwards; split <;> first | exact ⟨_, rfl⟩ | (exfalso; simp_all [encdec_cbop_backwards_matches]))
  | (unfold encdec_biop_backwards; split <;> first | exact ⟨_, rfl⟩ | (exfalso; simp_all [encdec_biop_backwards_matches]))
  | (unfold encdec_amoop_backwards; split <;> first | exact ⟨_, rfl⟩ | (exfalso; simp_all [encdec_amoop_backwards_matches]))
  | (unfold encdec_zicondop_backwards; split <;>
      first | exact ⟨_, rfl⟩ | (exfalso; simp_all [encdec_zicondop_backwards_matches]))
  | (unfold encdec_zilx_mode_backwards; split <;>
      first | exact ⟨_, rfl⟩ | (exfalso; simp_all [encdec_zilx_mode_backwards_matches])))

/-- A step whose value the body needs: an extension check, a value given. -/
macro "known" : tactic => `(tactic| first
  | (rw [ce_run]; rotate_left; (focus (simp [currentlyEnabled, hartSupports]; done)))
  | rw [ce_zba _ _ (by assumption) (by assumption)]
  | rw [ce_zbs _ _ (by assumption) (by assumption)]
  | rw [ce_zbb _ _ (by assumption) (by assumption)]
  | rw [ce_zaamo _ _ (by assumption) (by assumption)]
  | rw [ce_zabha _ _ (by assumption)]
  | rw [ce_zacas _ _ (by assumption)]
  | rw [ce_zalrsc _ _ (by assumption) (by assumption)]
  | rw [ce_zicfiss _ _ (by assumption)]
  | rw [ce_zbkb]
  | rw [‹(interpretSailEffects (encdec_amoop_backwards _)).run _ = _›]
  | rw [amocas_valid_false _ _ (by assumption)]
  | rw [‹(interpretSailEffects (width_enc_wide_backwards _)).run _ = _›]
  | (rw [amo_valid_false _ _ (by assumption)]; rotate_left; (focus decide)))

/-- A branch's condition is false, given the clause's guard and the word's bits. -/
macro "cond_false" : tactic => `(tactic| (
  intro h
  simp only [rej_xlen32, rej_xlen64, Bool.false_and, Bool.and_false, Bool.true_and, Bool.and_true, Bool.or_true,
    Bool.true_or, Bool.false_eq_true, Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq, bne_iff_ne, ne_eq] at *
  all_goals bv_decide))

/-- A clause's guard is false: a funct RISC-V has none of, given the word's bits. -/
macro "funct_false" : tactic => `(tactic| (
  intro h
  simp only [bop_matches_eq, amoop_matches_eq, Sail.BitVec.extractLsb, Bool.and_eq_true, Bool.or_eq_true,
    Bool.not_eq_true', beq_iff_eq] at *
  bv_decide))

/-- Through a clause's body, its guard holding: nothing decoded. -/
macro "body_none" : tactic => `(tactic| repeat' (first
  | rfl
  | (rw [interp_pure, run_pure']; try dsimp only)
  | (rw [interp_bind, run_bind']; known; try dsimp only)
  | (simp only [Bool.false_eq_true, if_false, Bool.false_and, Bool.and_false, ite_false, Bool.and_true,
      Bool.true_and]; try dsimp only)
  | (rw [if_neg]; rotate_left; focus guard_false_i)
  | (rw [if_neg]; rotate_left; focus cond_false)
  | (rw [if_neg]; rotate_left; (focus (intro h; simp (config := { decide := true }) [*, valid_load_encdec,
      width_enc_backwards, bool_bit_backwards, Functions.xlen_bytes, lrsc_width_valid] at h; done)))
  | (apply bind_none_run; · returns)
  | intro))

/-- One step through the decoder's chain: `peel_i`, or a clause by its body. -/
macro "peel_r" : tactic => `(tactic| first
  | peel_i
  | (rw [if_neg]; rotate_left; focus funct_false)
  | (rw [ite_none_run]; rotate_left; focus (intro; body_none; done); try dsimp only)
  | (dsimp only))

end Equiv.Riscv
