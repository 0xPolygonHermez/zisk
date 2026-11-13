/- sail-riscv's decoder: `decode_addiw`, on its own so that it checks in parallel (RiscvDecodeW.lean). -/
import Equiv.RiscvDecodeWBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An addiw word, any registers, decodes to ADDIW with its fields. -/
theorem decode_addiw (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x1B#7) (hf3 : w.extractLsb 14 12 = 0#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ADDIW (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  repeat skip_p
  try dsimp only
  rw [interp_bind, run_bind']
  try dsimp only
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3]
  simp (config := { decide := true }) only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not,
    Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure', Functions.xlen]
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.ADDIW.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
