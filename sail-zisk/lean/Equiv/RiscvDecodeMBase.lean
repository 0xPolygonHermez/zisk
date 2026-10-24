/-
  What the decode lemmas of the M extension share (Equiv/RiscvDecodeM.lean,
  Equiv/RiscvDecodeMul.lean): M enabled, which each clause checks (misa's M
  bit, set in ZisK's profile), and the closing step of each lemma.
-/
import Equiv.RiscvDecode
import Equiv.RiscvM

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- M enabled: misa's M bit set. -/
theorem m_enabled (s : RVState) (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (currentlyEnabled .Ext_M)).run s = .ok true s := by
  obtain ⟨v, hv, hM⟩ := hm
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, hM]

/-- Zmmul (multiplication alone) enabled, as M is: MUL checks either. -/
theorem zmmul_enabled (s : RVState) (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    ∃ b, (interpretSailEffects (currentlyEnabled .Ext_Zmmul)).run s = .ok b s := by
  obtain ⟨v, hv, hM⟩ := hm
  exact ⟨_, by simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, hM]; rfl⟩

/-- The tail of an M decode lemma: the clause's guard holds, and its body,
    M enabled, gives the instruction. -/
macro "m_tail" : tactic => `(tactic| (
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    try simp [encdec_reg_backwards_matches, encdec_mul_op_backwards_matches, bool_bit_backwards_matches,
      Functions.base_E_enabled, LeanZiskRV.not, *]
    try bv_decide
  try simp (config := { decide := true }) only [encdec_reg_backwards, encdec_mul_op_backwards, bool_bit_backwards,
    Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind',
    interp_pure, run_pure', Sail.BitVec.extractLsb, Functions.xlen, *]
  try (rw [m_enabled _ (by assumption)]; dsimp only)
  try (obtain ⟨zb, hzb⟩ := zmmul_enabled _ (by assumption); rw [hzb]; dsimp only)
  try simp (config := { decide := true }) only [Bool.true_or, if_true, interp_pure, run_pure',
    Functions.regidx_bit_width]
  try simp only [EStateM.Result.ok.injEq, instruction.MUL.injEq, instruction.DIV.injEq, instruction.REM.injEq,
    instruction.MULW.injEq, instruction.DIVW.injEq, instruction.REMW.injEq, Prod.mk.injEq,
    regidx.Regidx.injEq, and_true, true_and]
  and_intros
  all_goals first
    | trivial
    | (apply BitVec.eq_of_getLsbD_eq; intro i hi; simp)
    | simp_all))

/-- Which of mul, mulh, mulhsu, mulhu: its funct3 and mul_op. -/
inductive MulSel | mul | mulh | mulhsu | mulhu

def MulSel.f3 : MulSel → BitVec 3
  | .mul => 0 | .mulh => 1 | .mulhsu => 2 | .mulhu => 3

def MulSel.op : MulSel → mul_op
  | .mul => ⟨.Low, .Signed, .Signed⟩ | .mulh => ⟨.High, .Signed, .Signed⟩
  | .mulhsu => ⟨.High, .Signed, .Unsigned⟩ | .mulhu => ⟨.High, .Unsigned, .Unsigned⟩

end Equiv.Riscv
