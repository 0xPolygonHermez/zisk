/- sail-riscv's decoder: `decode_shiftiop0`, on its own so that it checks in parallel (RiscvDecodeX0.lean). -/
import Equiv.RiscvDecodeX0Base

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An immediate-shift word, any registers, decodes to SHIFTIOP with its fields. -/
theorem decode_shiftiop0 (s : RVState) (w : BitVec 32) (op : sop)
    (hop : w.extractLsb 6 0 = 0x13#7) (hf3 : w.extractLsb 14 12 = shiftF3 op)
    (hf6 : w.extractLsb 31 26 = shiftF6 op)
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
