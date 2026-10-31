/- sail-riscv's decoder: `decode_rtype`, on its own so that it checks in parallel (RiscvDecode.lean). -/
import Equiv.RiscvDecodeBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An R-type word, rd ≠ x0, decodes to RTYPE with its fields. -/
theorem decode_rtype (s : RVState) (w : BitVec 32) (op : rop)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = rtypeF3 op)
    (hf7 : w.extractLsb 31 25 = rtypeF7 op) (hrd : w.extractLsb 11 7 ≠ 0#5)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [rtypeF3, rtypeF7] at hf3 hf7 <;>
  · unfold ext_decode
    delta encdec_backwards
    repeat skip_p
    try dsimp only
    rw [interp_bind, run_bind']
    try dsimp only
    rw [if_pos]; rotate_left
    · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, hf7]
    simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false, Bool.true_or,
      if_true, interp_bind, run_bind', interp_pure, run_pure']
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.RTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true]
    refine ⟨?_, ?_, ?_⟩ <;>
    · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
