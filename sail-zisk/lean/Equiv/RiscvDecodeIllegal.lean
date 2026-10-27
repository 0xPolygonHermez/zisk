/-
  sail-riscv's decoder on the words RISC-V rejects for their major opcode,
  in ZisK's profile — the custom opcodes (0x0B, 0x2B, 0x5B, 0x7B), the
  reserved one (0x6B) and the longer
  encodings' (0x1F, 0x3F, 0x5F, 0x7F): ILLEGAL, no clause's guard holding.
  A lemma per opcode in Equiv/Decode/Illegal/, one per module so that they
  check in parallel.
-/
import Equiv.Decode.Illegal.Op0B
import Equiv.Decode.Illegal.Op1F
import Equiv.Decode.Illegal.Op2B
import Equiv.Decode.Illegal.Op3F
import Equiv.Decode.Illegal.Op5B
import Equiv.Decode.Illegal.Op5F
import Equiv.Decode.Illegal.Op6B
import Equiv.Decode.Illegal.Op7B
import Equiv.Decode.Illegal.Op7F

namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A word of a major opcode no clause of the decoder has decodes to ILLEGAL. -/
theorem decode_rej_opcode (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x0B#7 ∨ w.extractLsb 6 0 = 0x1F#7 ∨ w.extractLsb 6 0 = 0x2B#7 ∨
      w.extractLsb 6 0 = 0x3F#7 ∨ w.extractLsb 6 0 = 0x5B#7 ∨ w.extractLsb 6 0 = 0x5F#7 ∨
      w.extractLsb 6 0 = 0x6B#7 ∨ w.extractLsb 6 0 = 0x7B#7 ∨ w.extractLsb 6 0 = 0x7F#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  rcases hop with h | h | h | h | h | h | h | h | h
  · exact decode_op0B s w h hp sec hsec
  · exact decode_op1F s w h hp sec hsec
  · exact decode_op2B s w h hp sec hsec
  · exact decode_op3F s w h hp sec hsec
  · exact decode_op5B s w h hp sec hsec
  · exact decode_op5F s w h hp sec hsec
  · exact decode_op6B s w h hp sec hsec
  · exact decode_op7B s w h hp sec hsec
  · exact decode_op7F s w h hp sec hsec

end Equiv.Riscv
