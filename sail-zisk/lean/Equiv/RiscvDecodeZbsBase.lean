/-
  What sail-riscv's decoder on Zbs needs, shared by the lemmas of
  Equiv/DecodeZbs/, one per op and module so that they check in parallel:
  Zbs enabled (through misa's B bit, as Zba), and each op's fields.
-/
import Equiv.RiscvDecodeZbaBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zbs, enabled by misa's B bit. -/
theorem ce_zbs (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) (hb : _get_Misa_B v = 1#1) :
    (interpretSailEffects (currentlyEnabled .Ext_Zbs)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, hb]

def zbsF7 : brop_zbs → BitVec 7
  | .BCLR => 0x24 | .BEXT => 0x24 | .BINV => 0x34 | .BSET => 0x14

def zbsF3 : brop_zbs → BitVec 3
  | .BCLR => 1 | .BEXT => 5 | .BINV => 1 | .BSET => 1

def zbsiF6 : biop_zbs → BitVec 6
  | .BCLRI => 0x12 | .BEXTI => 0x12 | .BINVI => 0x1A | .BSETI => 0x0A

def zbsiF3 : biop_zbs → BitVec 3
  | .BCLRI => 1 | .BEXTI => 5 | .BINVI => 1 | .BSETI => 1

end Equiv.Riscv
