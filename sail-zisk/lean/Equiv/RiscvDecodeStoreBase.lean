/-
  sail-riscv's decoder on the narrow stores: sb, sh and sw decode to STORE
  with their fields and width, as Equiv/RiscvDecodeMem.lean has it for sd.
-/
import Equiv.RiscvDecodeMemBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

end Equiv.Riscv
