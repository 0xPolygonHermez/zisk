/-
  sail-riscv's decoder on the M extension's MUL, any registers: mul, mulh,
  mulhsu, mulhu, as
  Equiv/RiscvDecode.lean has it for the R-type instructions. Each clause
  checks that M is enabled, which, in ZisK's profile, it is.
-/
import Equiv.RiscvDecodeMBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
theorem decode_mul_mul (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = 0#3) (hf7 : w.extractLsb 31 25 = 1#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.MUL (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          MulSel.mul.op)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel
  m_tail

set_option maxHeartbeats 0 in
theorem decode_mul_mulh (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = 1#3) (hf7 : w.extractLsb 31 25 = 1#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.MUL (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          MulSel.mulh.op)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel
  m_tail

set_option maxHeartbeats 0 in
theorem decode_mul_mulhsu (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = 2#3) (hf7 : w.extractLsb 31 25 = 1#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.MUL (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          MulSel.mulhsu.op)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel
  m_tail

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
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel
  m_tail

theorem decode_mul (s : RVState) (w : BitVec 32) (k : MulSel)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = k.f3) (hf7 : w.extractLsb 31 25 = 1#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (hm : ∃ v, s.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.MUL (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), k.op)) s := by
  cases k
  · exact decode_mul_mul s w hop hf3 hf7 hp sec hsec hm
  · exact decode_mul_mulh s w hop hf3 hf7 hp sec hsec hm
  · exact decode_mul_mulhsu s w hop hf3 hf7 hp sec hsec hm
  · exact decode_mul_mulhu s w hop hf3 hf7 hp sec hsec hm

end Equiv.Riscv
