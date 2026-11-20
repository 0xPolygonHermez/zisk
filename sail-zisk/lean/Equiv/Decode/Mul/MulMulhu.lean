/- sail-riscv's decoder: `decode_mul_mulhu`, on its own so that it checks in parallel (RiscvDecodeMul.lean). -/
import Equiv.RiscvDecodeMulBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
theorem decode_mul_mulhu (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = 3#3) (hf7 : w.extractLsb 31 25 = 1#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.MUL (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          MulSel.mulhu.op)) s := by
  unfold ext_decode
  delta encdec_backwards
  repeat skip_p
  try dsimp only
  rw [interp_bind, run_bind']
  try dsimp only
  m_tail

end Equiv.Riscv
