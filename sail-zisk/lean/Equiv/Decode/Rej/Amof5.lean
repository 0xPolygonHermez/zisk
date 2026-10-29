/- sail-riscv's decoder: `decode_rej_amof5`, on its own so that it checks in parallel (RiscvDecodeRej.lean). -/
import Equiv.RiscvDecodeRejBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An AMO of .w or .d width and a funct5 RISC-V has no op for decodes to ILLEGAL. -/
theorem decode_rej_amof5 (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x2F#7 ∧ (w.extractLsb 14 12 = 2#3 ∨ w.extractLsb 14 12 = 3#3) ∧ w.extractLsb 31 27 ≠ 2#5 ∧ w.extractLsb 31 27 ≠ 3#5 ∧ w.extractLsb 31 27 ≠ 1#5 ∧ w.extractLsb 31 27 ≠ 0#5 ∧ w.extractLsb 31 27 ≠ 4#5 ∧ w.extractLsb 31 27 ≠ 12#5 ∧ w.extractLsb 31 27 ≠ 8#5 ∧ w.extractLsb 31 27 ≠ 16#5 ∧ w.extractLsb 31 27 ≠ 20#5 ∧ w.extractLsb 31 27 ≠ 24#5 ∧ w.extractLsb 31 27 ≠ 28#5 ∧ w.extractLsb 31 27 ≠ 5#5)
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
