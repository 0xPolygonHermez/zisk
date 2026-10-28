/- sail-riscv's decoder: `decode_rej_opimm5`, on its own so that it checks in parallel (RiscvDecodeRej.lean). -/
import Equiv.RiscvDecodeRejBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An OP-IMM word of funct3 5 neither srli, srai nor a Zbb/Zbs/Zbkb op decodes to ILLEGAL. -/
theorem decode_rej_opimm5 (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x13#7 ∧ w.extractLsb 14 12 = 5#3 ∧ w.extractLsb 31 26 ≠ 0#6 ∧ w.extractLsb 31 26 ≠ 16#6 ∧ w.extractLsb 31 26 ≠ 18#6 ∧ w.extractLsb 31 26 ≠ 24#6 ∧ w.extractLsb 31 20 ≠ 0x6B8#12 ∧ w.extractLsb 31 20 ≠ 0x687#12 ∧ w.extractLsb 31 20 ≠ 0x287#12)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_r

end Equiv.Riscv
