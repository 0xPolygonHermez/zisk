/- sail-riscv's decoder: `decode_rtype0`, on its own so that it checks in parallel (RiscvDecodeX0.lean). -/
import Equiv.RiscvDecodeX0Base

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An R-type word, any registers, decodes to RTYPE with its fields. (`add x0,
    x0, rs2` matches Zihintntl's clause, which, Zihintntl off, yields nothing.) -/
theorem decode_rtype0 (s : RVState) (w : BitVec 32) (op : rop)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = rtypeF3 op)
    (hf7 : w.extractLsb 31 25 = rtypeF7 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [rtypeF3, rtypeF7] at hf3 hf7
  case ADD =>
    unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel
    -- Zihintntl's ntl.*: add x0, x0, rs2 for rs2 in x2..x5; its clause,
    -- Zihintntl off, yields nothing.
    by_cases hg : (encdec_ntl_backwards_matches (Sail.BitVec.extractLsb w 24 20) &&
        (Sail.BitVec.extractLsb w 31 25 == 0#7 && Sail.BitVec.extractLsb w 19 0 == 51#20)) = true
    all_goals first
      | (rw [if_pos hg]
         have hm : encdec_ntl_backwards_matches (Sail.BitVec.extractLsb w 24 20) = true := by
           simp only [Bool.and_eq_true] at hg; exact hg.1
         obtain ⟨v, hv⟩ := ntl_run s _ hm
         rw [interp_bind, run_bind', hv]; dsimp only
         repeat (first | peel | peel_f))
      | (rw [if_neg hg]
         repeat (first | peel | peel_f))
    all_goals
      rw [if_pos]; rotate_left
      · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
        simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, hf7]
      simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false, Bool.true_or,
        if_true, interp_bind, run_bind', interp_pure, run_pure']
      simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
      simp only [EStateM.Result.ok.injEq, instruction.RTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true]
      refine ⟨?_, ?_, ?_⟩ <;>
      · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp
  all_goals
    unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel
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
