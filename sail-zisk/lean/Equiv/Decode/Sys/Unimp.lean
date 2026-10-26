/- sail-riscv's decoder: `decode_unimp`, on its own so that it checks in parallel (RiscvDecodeCsr.lean). -/
import Equiv.RiscvDecodeCsrBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- `unimp`, `csrrw x0, cycle, x0`, decodes to CSRReg. -/
theorem decode_unimp (s : RVState) (w : BitVec 32) (hw : w = 0xC0001073#32)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.CSRReg (0xC00#12, .Regidx 0#5, .Regidx 0#5, .CSRRW)) s := by
  subst hw
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp [Sail.BitVec.extractLsb, encdec_reg_backwards_matches, encdec_csrop_backwards_matches,
      Functions.base_E_enabled, LeanZiskRV.not]
  simp only [encdec_reg_backwards, encdec_csrop_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  have h12 : Sail.BitVec.extractLsb (0xC0001073#32) 13 12 = 1#2 := by decide
  simp only [h12]
  try simp only [interp_pure, run_pure']
  try dsimp only
  rw [ce_zicsr]; try dsimp only
  try simp only [if_true, interp_pure, run_pure']
  try dsimp only
  try simp only [interp_pure, run_pure']
  rfl

end Equiv.Riscv
