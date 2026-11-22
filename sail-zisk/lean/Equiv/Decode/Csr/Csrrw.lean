/- sail-riscv's decoder: `decode_csrrw`, on its own so that it checks in parallel (RiscvDecodeCsr.lean). -/
import Equiv.RiscvDecodeCsrBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A csrrw word decodes to CSRReg with its fields. -/
theorem decode_csrrw (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x73#7)
    (hf3 : w.extractLsb 14 12 = 1#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.CSRReg (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), .CSRRW)) s := by
  have h12 : w.extractLsb 13 12 = 1#2 := by bv_decide
  have h14 : w.extractLsb 14 14 = 0#1 := by bv_decide
  unfold ext_decode
  delta encdec_backwards
  repeat skip_i
  try dsimp only
  rw [interp_bind, run_bind']
  try dsimp only
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, encdec_csrop_backwards_matches, Functions.base_E_enabled,
      LeanZiskRV.not, hop, h12, h14]
  simp only [encdec_reg_backwards, encdec_csrop_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, h12]
  try simp only [interp_pure, run_pure']
  try dsimp only
  rw [ce_zicsr]; try dsimp only
  try simp only [if_true, interp_pure, run_pure']
  try dsimp only
  try simp only [interp_pure, run_pure']
  simp only [Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.CSRReg.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
