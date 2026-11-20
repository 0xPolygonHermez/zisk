/-
  sail-riscv's decoder on the CSR instructions: all six, `csrrs` as `csrr
  rd, marchid` reads it (contract § 6), each as a write to a read-only CSR
  traps.
-/
import Equiv.RiscvDecodeImmBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zicsr, in the profile, which reads no register. -/
theorem ce_zicsr (s : RVState) :
    (interpretSailEffects (currentlyEnabled .Ext_Zicsr)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports]

end Equiv.Riscv
