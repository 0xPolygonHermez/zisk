/- sail-riscv's decoder: `decode_op1F`, on its own so that it checks in parallel (RiscvDecodeIllegal.lean). -/
import Equiv.RiscvDecodeCsrBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A word of major opcode 0x1F decodes to ILLEGAL: no clause's guard holds. -/
theorem decode_op1F (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x1F#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  unfold ext_decode
  delta encdec_backwards
  repeat skip_i
  rfl

end Equiv.Riscv
