/- sail-riscv's decoder: `decode_rej_opimm32f`, on its own so that it checks in parallel (RiscvDecodeRej.lean). -/
import Equiv.RiscvDecodeRejBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An OP-IMM-32 word of a funct3 with no op decodes to ILLEGAL. -/
theorem decode_rej_opimm32f (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x1B#7 ∧ (w.extractLsb 14 12 = 2#3 ∨ w.extractLsb 14 12 = 3#3 ∨ w.extractLsb 14 12 = 4#3 ∨ w.extractLsb 14 12 = 6#3 ∨ w.extractLsb 14 12 = 7#3))
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  unfold ext_decode
  delta encdec_backwards
  repeat skip_r
  rfl

end Equiv.Riscv
