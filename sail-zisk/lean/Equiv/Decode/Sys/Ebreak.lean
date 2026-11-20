/- sail-riscv's decoder: `decode_ebreak`, on its own so that it checks in parallel (RiscvDecodeSys.lean). -/
import Equiv.RiscvDecodeCsrBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- `ebreak` decodes to EBREAK. -/
theorem decode_ebreak (s : RVState) (w : BitVec 32) (hw : w = 0x00100073#32)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.EBREAK ()) s := by
  subst hw
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  simp only [beq_self_eq_true, if_true, interp_pure, run_pure']

end Equiv.Riscv
