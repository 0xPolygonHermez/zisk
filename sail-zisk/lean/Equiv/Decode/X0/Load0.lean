/- sail-riscv's decoder: `decode_load0`, on its own so that it checks in parallel (RiscvDecodeX0.lean). -/
import Equiv.RiscvDecodeX0Base

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A load word (any registers, a valid funct3) decodes to LOAD with its fields. -/
theorem decode_load0 (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x03#7)
    (hvalid : valid_load_encdec (width_enc_backwards (w.extractLsb 13 12)).toNat
      (bool_bit_backwards (w.extractLsb 14 14)) = true)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.LOAD (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          bool_bit_backwards (w.extractLsb 14 14), width_enc_backwards (w.extractLsb 13 12))) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_m
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, width_enc_backwards_matches, bool_bit_backwards_matches,
      Functions.base_E_enabled, LeanZiskRV.not, hop]
    generalize BitVec.extractLsb 14 14 w = b; generalize BitVec.extractLsb 13 12 w = c
    revert b c; decide
  simp (config := { decide := true }) only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not,
    Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure', Sail.BitVec.extractLsb]
  rw [if_pos hvalid]
  simp only [interp_pure, run_pure', Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.LOAD.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
