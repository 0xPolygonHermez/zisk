/-
  What sail-riscv's decoder on Zbkb's packs and Zbb's extensions needs,
  shared by the lemmas of Equiv/Decode/Zbkb/: the packs' and extensions'
  fields.
-/
import Equiv.RiscvDecodeZbbBase

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

def zbkbF3 : brop_zbkb → BitVec 3
  | .PACK => 4 | .PACKH => 7

end Equiv.Riscv
