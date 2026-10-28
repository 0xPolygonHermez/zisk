/- sail-riscv's decoder: `decode_rej_load`, on its own so that it checks in parallel (RiscvDecodeRej.lean). -/
import Equiv.RiscvDecodeRejBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A load of funct3 7 (no width RISC-V has) decodes to ILLEGAL. -/
theorem decode_rej_load (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x03#7 ∧ w.extractLsb 14 12 = 7#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  have hw1 : Sail.BitVec.extractLsb w 13 12 = 3#2 := by simp only [Sail.BitVec.extractLsb]; bv_decide
  have hw2 : Sail.BitVec.extractLsb w 14 14 = 1#1 := by simp only [Sail.BitVec.extractLsb]; bv_decide
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_r

end Equiv.Riscv
