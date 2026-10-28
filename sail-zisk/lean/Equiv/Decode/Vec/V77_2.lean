/- sail-riscv's decoder: `decode_vec77_2`, on its own so that it checks in parallel (RiscvDecodeVec.lean). -/
import Equiv.RiscvDecodeVecBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A word of opcode 0x77, funct3 2, decodes to ILLEGAL. -/
theorem decode_vec77_2 (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x77#7 ∧ w.extractLsb 14 12 = 2#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (vt : RegisterType vtype) (hvt : s.regs.get? vtype = some vt) (hok : VtypeOk vt)
    (l : RegisterType vl) (hl : s.regs.get? vl = some l) (st : RegisterType vstart) (hst : s.regs.get? vstart = some st) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  have hf : Sail.BitVec.extractLsb w 14 12 = 2#3 := by simp only [Sail.BitVec.extractLsb]; bv_decide
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_r

end Equiv.Riscv
