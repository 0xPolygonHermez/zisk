/-
  sail-riscv's decoder on A's AMOs: each op and width a clause of the AMO
  pattern; past it, AMOs (Zaamo) enabled, through misa's A bit.
-/
import Equiv.RiscvDecodeZbkbBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zaamo, enabled by misa's A bit. -/
theorem ce_zaamo (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) (ha : _get_Misa_A v = 1#1) :
    (interpretSailEffects (currentlyEnabled .Ext_Zaamo)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, ha]

theorem bit_matches (x : BitVec 1) : bool_bit_backwards_matches x = true := by
  revert x; decide

end Equiv.Riscv
