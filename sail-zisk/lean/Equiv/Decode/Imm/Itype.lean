/- sail-riscv's decoder: `decode_itype`, on its own so that it checks in parallel (RiscvDecodeImm.lean). -/
import Equiv.RiscvDecodeImmBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

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
  repeat skip_i
  try dsimp only
  rw [interp_bind, run_bind']
  try dsimp only
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

end Equiv.Riscv
