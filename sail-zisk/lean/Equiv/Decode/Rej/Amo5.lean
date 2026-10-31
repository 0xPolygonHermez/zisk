/- sail-riscv's decoder: `decode_rej_amo5`, on its own so that it checks in parallel (RiscvDecodeRej.lean). -/
import Equiv.RiscvDecodeRejBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An AMO of funct3 5 (no width) decodes to ILLEGAL. -/
theorem decode_rej_amo5 (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x2F#7 ∧ w.extractLsb 14 12 = 5#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  have hf : Sail.BitVec.extractLsb w 14 12 = 5#3 := by simp only [Sail.BitVec.extractLsb]; bv_decide
  unfold ext_decode
  delta encdec_backwards
  repeat skip_r
  rfl

end Equiv.Riscv
