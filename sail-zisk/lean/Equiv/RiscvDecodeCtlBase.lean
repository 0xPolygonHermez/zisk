/-
  sail-riscv's decoder on control flow: a branch word decodes to BTYPE with
  its fields (by peeling the decoder's clauses; BTYPE comes early).
-/
import Equiv.RiscvDecodeImmBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- funct3 of each branch. -/
def btypeF3 : bop → BitVec 3
  | .BEQ => 0#3 | .BNE => 1#3 | .BLT => 4#3 | .BGE => 5#3 | .BLTU => 6#3 | .BGEU => 7#3

theorem bop_dec (op : bop) : encdec_bop_backwards (btypeF3 op) = pure op := by cases op <;> rfl

theorem bop_matches (op : bop) : encdec_bop_backwards_matches (btypeF3 op) = true := by cases op <;> rfl

/-- The opcode of lui and auipc. -/
def uopEnc : uop → BitVec 7
  | .LUI => 0x37#7 | .AUIPC => 0x17#7

theorem uop_dec (op : uop) : encdec_uop_backwards (uopEnc op) = pure op := by cases op <;> rfl

theorem uop_matches (op : uop) : encdec_uop_backwards_matches (uopEnc op) = true := by cases op <;> rfl

end Equiv.Riscv
