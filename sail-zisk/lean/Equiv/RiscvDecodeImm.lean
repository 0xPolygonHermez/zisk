/-
  sail-riscv's decoder on the immediate instructions: an I-type word
  (addi, slti, sltiu, xori, ori, andi) decodes to ITYPE, an immediate shift
  (slli, srli, srai) to SHIFTIOP, with their fields; as Equiv/RiscvDecode.lean
  does for the R-type, by peeling the decoder's clauses.
-/
import Equiv.RiscvDecode
import Equiv.RiscvImm

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A clause's guard is false for the word: as `guard_false`, also through
    the I-type op mapping. -/
macro "guard_false_i" : tactic => `(tactic| (
  intro h
  simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq] at h
  first
  | bv_decide
  | (simp_all (config := { decide := true }) [encdec_uop_backwards_matches, encdec_iop_backwards_matches]; done)))

/-- One step through the decoder's chain of clauses (`peel`, with `guard_false_i`). -/
macro "peel_i" : tactic => `(tactic| first
  | (rw [if_neg]; rotate_left; focus guard_false_i)
  | (rw [interp_pure, run_pure']; try dsimp only)
  | (rw [ce_run]; rotate_left; (focus (simp [currentlyEnabled, hartSupports]; done)); try dsimp only)
  | (rw [ce_zicfilp _ (by assumption) _ (by assumption)]; try dsimp only)
  | (rw [interp_bind, run_bind']; try dsimp only))

/-- funct3 of each I-type op. -/
def itypeF3 : iop → BitVec 3
  | .ADDI => 0#3 | .SLTI => 2#3 | .SLTIU => 3#3 | .XORI => 4#3 | .ORI => 6#3 | .ANDI => 7#3

theorem iop_dec (op : iop) : encdec_iop_backwards (itypeF3 op) = pure op := by cases op <;> rfl

theorem iop_matches (op : iop) : encdec_iop_backwards_matches (itypeF3 op) = true := by cases op <;> rfl

set_option maxHeartbeats 0 in
/-- An I-type word, rd ≠ x0, decodes to ITYPE with its fields. -/
theorem decode_itype (s : RVState) (w : BitVec 32) (op : iop)
    (hop : w.extractLsb 6 0 = 0x13#7) (hf3 : w.extractLsb 14 12 = itypeF3 op) (hrd : w.extractLsb 11 7 ≠ 0#5)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ITYPE (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  have hf3' : Sail.BitVec.extractLsb w 14 12 = itypeF3 op := hf3
  rw [if_pos]; rotate_left
  · simp only [hf3', iop_matches, Bool.and_true, Bool.true_and]
    simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop]
  simp only [hf3', iop_dec, encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.ITYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

/-- funct3 and funct6 of each immediate shift. -/
def shiftF3 : sop → BitVec 3
  | .SLLI => 1 | .SRLI => 5 | .SRAI => 5

def shiftF6 : sop → BitVec 6
  | .SRAI => 0x10 | _ => 0

set_option maxHeartbeats 0 in
/-- An immediate-shift word, rd ≠ x0, decodes to SHIFTIOP with its fields. -/
theorem decode_shiftiop (s : RVState) (w : BitVec 32) (op : sop)
    (hop : w.extractLsb 6 0 = 0x13#7) (hf3 : w.extractLsb 14 12 = shiftF3 op)
    (hf6 : w.extractLsb 31 26 = shiftF6 op) (hrd : w.extractLsb 11 7 ≠ 0#5)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.SHIFTIOP (w.extractLsb 25 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [shiftF3, shiftF6] at hf3 hf6 <;>
  · unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel_i
    rw [if_pos]; rotate_left
    · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, hf6]
    simp (config := { decide := true }) only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not,
      Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure', Functions.xlen,
      beq_self_eq_true]
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.SHIFTIOP.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
      true_and]
    refine ⟨?_, ?_⟩ <;>
    · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
