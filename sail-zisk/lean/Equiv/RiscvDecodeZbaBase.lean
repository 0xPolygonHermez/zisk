/-
  sail-riscv's decoder on Zba's single-instruction forms: sh1add, sh2add,
  sh3add (ZBA_RTYPE) and slli.uw (SLLIUW). Each clause, past its guard,
  checks that Zba is enabled, as it is in the profile.
-/
import Equiv.RiscvDecodeFenceBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zba, enabled by misa's B bit. -/
theorem ce_zba (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) (hb : _get_Misa_B v = 1#1) :
    (interpretSailEffects (currentlyEnabled .Ext_Zba)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, hb]

end Equiv.Riscv
