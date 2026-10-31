/-
  RiscvDecodeMul: the lemmas of its base (definitions) and of
  Equiv/Decode/Mul/, one per module so that they check in parallel.
-/
import Equiv.RiscvDecodeMBase
import Equiv.RiscvDecodeMulBase
import Equiv.Decode.Mul.MulMul
import Equiv.Decode.Mul.MulMulh
import Equiv.Decode.Mul.MulMulhsu
import Equiv.Decode.Mul.MulMulhu

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

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
