/- sail-riscv's decoder: `decode_sd`, on its own so that it checks in parallel (RiscvDecodeMem.lean). -/
import Equiv.RiscvDecodeMemBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An `sd` word decodes to STORE with its fields. -/
theorem decode_sd (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x23#7) (hf3 : w.extractLsb 14 12 = 3#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.STORE (w.extractLsb 31 25 ++ w.extractLsb 11 7, .Regidx (w.extractLsb 24 20),
          .Regidx (w.extractLsb 19 15), 8)) s := by
  have h14 : w.extractLsb 14 14 = 0#1 := by bv_decide
  have h13 : w.extractLsb 13 12 = 3#2 := by bv_decide
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_m
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq] at h14 h13 ⊢
    simp [encdec_reg_backwards_matches, width_enc_backwards_matches, Functions.base_E_enabled,
      LeanZiskRV.not, hop, h14, h13]
  simp (config := { decide := true }) only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not,
    Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure', Sail.BitVec.extractLsb,
    h13, width_enc_backwards, Functions.xlen_bytes]
  simp only [Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.STORE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
