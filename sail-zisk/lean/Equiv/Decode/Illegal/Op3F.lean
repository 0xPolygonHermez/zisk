/- sail-riscv's decoder: `decode_op3F`, on its own so that it checks in parallel (RiscvDecodeIllegal.lean). -/
import Equiv.RiscvDecodeCsrBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A word of major opcode 0x3F decodes to ILLEGAL: no clause's guard holds. -/
theorem decode_op3F (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x3F#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i

end Equiv.Riscv
