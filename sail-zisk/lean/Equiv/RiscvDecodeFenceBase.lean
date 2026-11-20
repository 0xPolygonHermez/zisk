/-
  sail-riscv's decoder on the fences: `fence` (and `fence.tso`, its own
  clause) and `fence.i`. With Zihintpause out of the profile, `pause` is a
  plain fence.
-/
import Equiv.RiscvDecodeImmBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zifencei, in the profile, which reads no register. -/
theorem ce_zifencei (s : RVState) :
    (interpretSailEffects (currentlyEnabled .Ext_Zifencei)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports]

end Equiv.Riscv
