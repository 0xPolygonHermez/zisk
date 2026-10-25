/- sail-riscv's decoder: `decode_jal`, on its own so that it checks in parallel (RiscvDecodeCtl.lean). -/
import Equiv.RiscvDecodeCtlBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A jal word decodes to JAL with its fields: the offset as the decoder
    assembles it, imm[20] ++ imm[19:12] ++ imm[11] ++ imm[10:1] ++ 0. -/
theorem decode_jal (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x6f#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.JAL (w.extractLsb 31 31 ++ w.extractLsb 19 12 ++ w.extractLsb 20 20 ++ w.extractLsb 30 21 ++ 0#1,
          .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.JAL.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
