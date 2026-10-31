/- sail-riscv's decoder: `decode_btype`, on its own so that it checks in parallel (RiscvDecodeCtl.lean). -/
import Equiv.RiscvDecodeCtlBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A branch word decodes to BTYPE with its fields: the immediate as the
    decoder assembles it, imm[12] ++ imm[11] ++ imm[10:5] ++ imm[4:1] ++ 0. -/
theorem decode_btype (s : RVState) (w : BitVec 32) (op : bop)
    (hop : w.extractLsb 6 0 = 0x63#7) (hf3 : w.extractLsb 14 12 = btypeF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.BTYPE (w.extractLsb 31 31 ++ w.extractLsb 7 7 ++ w.extractLsb 30 25 ++ w.extractLsb 11 8 ++ 0#1,
          .Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), op)) s := by
  unfold ext_decode
  delta encdec_backwards
  repeat skip_i
  try dsimp only
  rw [interp_bind, run_bind']
  try dsimp only
  have hf3' : Sail.BitVec.extractLsb w 14 12 = btypeF3 op := hf3
  rw [if_pos]; rotate_left
  · simp only [hf3', bop_matches, Bool.and_true]
    simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop]
  simp only [hf3', bop_dec, encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.BTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
