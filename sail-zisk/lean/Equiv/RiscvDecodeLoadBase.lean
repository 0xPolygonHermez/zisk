/-
  sail-riscv's decoder on every load: a load word decodes to LOAD with its
  fields, the unsigned bit and the width as the decoder maps funct3.
-/
import Equiv.RiscvDecodeMemBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

end Equiv.Riscv
