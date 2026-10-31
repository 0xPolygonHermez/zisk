/-
  sail-riscv's decoder on the RV64I word instructions, any registers: addw,
  subw, sllw, srlw, sraw (RTYPEW), addiw (ADDIW), slliw, srliw, sraiw
  (SHIFTIWOP), as Equiv/RiscvDecode.lean has it for the R-type ones.
-/
import Equiv.RiscvDecodeBase
import Equiv.RiscvW

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

end Equiv.Riscv
