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

end Equiv.Riscv
