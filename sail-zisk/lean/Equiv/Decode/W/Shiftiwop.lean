/- sail-riscv's decoder: `decode_shiftiwop`, on its own so that it checks in parallel (RiscvDecodeW.lean). -/
import Equiv.RiscvDecodeWBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A SHIFTIWOP word, any registers, decodes to SHIFTIWOP with its fields. -/
theorem decode_shiftiwop (s : RVState) (w : BitVec 32) (op : sopw)
    (hop : w.extractLsb 6 0 = 0x1B#7) (hf3 : w.extractLsb 14 12 = shiftwF3 op)
    (hf7 : w.extractLsb 31 25 = shiftwF7 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.SHIFTIWOP (w.extractLsb 24 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [shiftwF3, shiftwF7] at hf3 hf7 <;>
  · unfold ext_decode
    delta encdec_backwards
    repeat skip_p
    try dsimp only
    rw [interp_bind, run_bind']
    try dsimp only
    rw [if_pos]; rotate_left
    · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, hf7]
    simp (config := { decide := true }) only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not,
      Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure', Functions.xlen]
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.SHIFTIWOP.injEq, Prod.mk.injEq, regidx.Regidx.injEq,
      and_true]
    refine ⟨?_, ?_, ?_⟩ <;>
    first | trivial | (apply BitVec.eq_of_getLsbD_eq; intro i hi; simp)

end Equiv.Riscv
