/- sail-riscv's decoder: `decode_fence_tso`, on its own so that it checks in parallel (RiscvDecodeFence.lean). -/
import Equiv.RiscvDecodeFenceBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- fence.tso's word decodes to FENCE_TSO. -/
theorem decode_fence_tso (s : RVState)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode 0x8330000F#32)).run s = .ok (.FENCE_TSO ()) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos (by decide)]
  rfl

end Equiv.Riscv
