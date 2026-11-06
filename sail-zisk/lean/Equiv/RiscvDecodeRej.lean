/-
  sail-riscv's decoder on the words RISC-V rejects within a major opcode
  the profile uses (but SYSTEM): ILLEGAL. The spec's decode giving
  nothing, the word's bits are one of the classes (DecodeRejMap.lean), and
  each class a lemma in Equiv/Decode/Rej/, one per module so that they
  check in parallel. Float opcodes are not among them: the profile has no
  float.
-/
import Equiv.Decode.Rej.Amo0
import Equiv.Decode.Rej.Amo1
import Equiv.Decode.Rej.Amo4
import Equiv.Decode.Rej.Amo5
import Equiv.Decode.Rej.Amo6
import Equiv.Decode.Rej.Amo7
import Equiv.Decode.Rej.Amocas2
import Equiv.Decode.Rej.Amocas3
import Equiv.Decode.Rej.Amof5
import Equiv.Decode.Rej.Branch
import Equiv.Decode.Rej.Jalr
import Equiv.Decode.Rej.Load
import Equiv.Decode.Rej.Lrrs2
import Equiv.Decode.Rej.MiscMem
import Equiv.Decode.Rej.Op
import Equiv.Decode.Rej.Op32
import Equiv.Decode.Rej.Opimm1
import Equiv.Decode.Rej.Opimm32a
import Equiv.Decode.Rej.Opimm32b
import Equiv.Decode.Rej.Opimm32f
import Equiv.Decode.Rej.Opimm5
import Equiv.Decode.Rej.Store
import Equiv.DecodeRejMap

namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- The major opcodes the profile uses, but SYSTEM. -/
def RejOpcode (w : BitVec 32) : Prop :=
  (Transpiler.opcode w).toNat = 3 ∨ (Transpiler.opcode w).toNat = 15 ∨ (Transpiler.opcode w).toNat = 19 ∨ (Transpiler.opcode w).toNat = 27 ∨ (Transpiler.opcode w).toNat = 35 ∨ (Transpiler.opcode w).toNat = 47 ∨ (Transpiler.opcode w).toNat = 51 ∨ (Transpiler.opcode w).toNat = 59 ∨ (Transpiler.opcode w).toNat = 99 ∨ (Transpiler.opcode w).toNat = 103

/-- One class's lemma, whichever the bits are. -/
local macro "rej_class" : tactic => `(tactic| first
    | (apply decode_rej_load <;> assumption)
    | (apply decode_rej_miscmem <;> assumption)
    | (apply decode_rej_opimm1 <;> assumption)
    | (apply decode_rej_opimm5 <;> assumption)
    | (apply decode_rej_opimm32a <;> assumption)
    | (apply decode_rej_opimm32b <;> assumption)
    | (apply decode_rej_opimm32f <;> assumption)
    | (apply decode_rej_store <;> assumption)
    | (apply decode_rej_amo0 <;> assumption)
    | (apply decode_rej_amo1 <;> assumption)
    | (apply decode_rej_amo4 <;> assumption)
    | (apply decode_rej_amo5 <;> assumption)
    | (apply decode_rej_amo6 <;> assumption)
    | (apply decode_rej_amo7 <;> assumption)
    | (apply decode_rej_lrrs2 <;> assumption)
    | (apply decode_rej_amof5 <;> assumption)
    | (apply decode_rej_amocas2 <;> assumption)
    | (apply decode_rej_amocas3 <;> assumption)
    | (apply decode_rej_op <;> assumption)
    | (apply decode_rej_op32 <;> assumption)
    | (apply decode_rej_branch <;> assumption)
    | (apply decode_rej_jalr <;> assumption))

/-- A word of a used opcode the spec decodes to nothing decodes to ILLEGAL. -/
theorem decode_rej_field (s : RVState) (w : BitVec 32) (hd : Transpiler.decode w = none) (ho : RejOpcode w)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hA : _get_Misa_A mv = 1#1)
    (hB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s = .ok (.ILLEGAL w) s := by
  rcases ho with ho | ho | ho | ho | ho | ho | ho | ho | ho | ho
  · rcases Transpiler.rej_3 hd ho with h <;> rej_class
  · rcases Transpiler.rej_15 hd ho with h <;> rej_class
  · rcases Transpiler.rej_19 hd ho with h | h <;> rej_class
  · rcases Transpiler.rej_27 hd ho with h | h | h <;> rej_class
  · rcases Transpiler.rej_35 hd ho with h <;> rej_class
  · rcases Transpiler.rej_47 hd ho with h | h | h | h | h | h | h | h | h | h <;> rej_class
  · rcases Transpiler.rej_51 hd ho with h <;> rej_class
  · rcases Transpiler.rej_59 hd ho with h <;> rej_class
  · rcases Transpiler.rej_99 hd ho with h <;> rej_class
  · rcases Transpiler.rej_103 hd ho with h <;> rej_class

end Equiv.Riscv
