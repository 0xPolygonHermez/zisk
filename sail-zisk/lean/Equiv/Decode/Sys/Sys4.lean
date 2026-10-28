/- sail-riscv's decoder: `decode_sys4`, SYSTEM's funct3 4, on its own so that it checks in parallel. -/
import Equiv.RiscvDecodeSysBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A word of SYSTEM, funct3 4 (H's loads and stores, Zimop, Zicfiss: none
    in the profile), decodes to ILLEGAL. -/
theorem decode_sys4 (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x73#7 ∧ w.extractLsb 14 12 = 4#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  have hf : Sail.BitVec.extractLsb w 14 12 = 4#3 := by simp only [Sail.BitVec.extractLsb]; bv_decide
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_r

end Equiv.Riscv
