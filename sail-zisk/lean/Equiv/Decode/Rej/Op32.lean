/- sail-riscv's decoder: `decode_rej_op32`, on its own so that it checks in parallel (RiscvDecodeRej.lean). -/
import Equiv.RiscvDecodeRejBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An OP-32 word of a funct3 and funct7 no op of the profile has decodes to ILLEGAL. -/
theorem decode_rej_op32 (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x3B#7 ∧
      ¬(w.extractLsb 14 12 = 0#3 ∧ w.extractLsb 31 25 = 0#7) ∧
      ¬(w.extractLsb 14 12 = 0#3 ∧ w.extractLsb 31 25 = 32#7) ∧
      ¬(w.extractLsb 14 12 = 1#3 ∧ w.extractLsb 31 25 = 0#7) ∧
      ¬(w.extractLsb 14 12 = 5#3 ∧ w.extractLsb 31 25 = 0#7) ∧
      ¬(w.extractLsb 14 12 = 5#3 ∧ w.extractLsb 31 25 = 32#7) ∧
      ¬(w.extractLsb 14 12 = 1#3 ∧ w.extractLsb 31 25 = 48#7) ∧
      ¬(w.extractLsb 14 12 = 5#3 ∧ w.extractLsb 31 25 = 48#7) ∧
      ¬(w.extractLsb 14 12 = 0#3 ∧ w.extractLsb 31 25 = 4#7) ∧
      ¬(w.extractLsb 14 12 = 2#3 ∧ w.extractLsb 31 25 = 16#7) ∧
      ¬(w.extractLsb 14 12 = 4#3 ∧ w.extractLsb 31 25 = 16#7) ∧
      ¬(w.extractLsb 14 12 = 6#3 ∧ w.extractLsb 31 25 = 16#7) ∧
      ¬(w.extractLsb 14 12 = 4#3 ∧ w.extractLsb 31 25 = 4#7) ∧
      ¬(w.extractLsb 14 12 = 0#3 ∧ w.extractLsb 31 25 = 1#7) ∧
      ¬(w.extractLsb 14 12 = 4#3 ∧ w.extractLsb 31 25 = 1#7) ∧
      ¬(w.extractLsb 14 12 = 5#3 ∧ w.extractLsb 31 25 = 1#7) ∧
      ¬(w.extractLsb 14 12 = 6#3 ∧ w.extractLsb 31 25 = 1#7) ∧
      ¬(w.extractLsb 14 12 = 7#3 ∧ w.extractLsb 31 25 = 1#7))
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
