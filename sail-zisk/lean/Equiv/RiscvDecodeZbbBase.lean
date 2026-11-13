/-
  What sail-riscv's decoder on Zbb's register ops needs, shared by the
  lemmas of Equiv/Decode/Zbb/, one per op and module so that they check in
  parallel: Zbb enabled (through misa's B bit, as Zba and Zbs), Zbkb (in the
  profile; andn, orn and xnor are in either), and each op's fields.
-/
import Equiv.RiscvDecodeZbsBase

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zbb, enabled by misa's B bit. -/
theorem ce_zbb (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) (hb : _get_Misa_B v = 1#1) :
    (interpretSailEffects (currentlyEnabled .Ext_Zbb)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, hb]

/-- Zbkb, in the profile, which reads no register. -/
theorem ce_zbkb (s : RVState) :
    (interpretSailEffects (currentlyEnabled .Ext_Zbkb)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports]

def zbbF7 : brop_zbb → BitVec 7
  | .ANDN => 0x20 | .ORN => 0x20 | .XNOR => 0x20 | .MAX => 0x05 | .MAXU => 0x05 | .MIN => 0x05
  | .MINU => 0x05 | .ROL => 0x30 | .ROR => 0x30

def zbbF3 : brop_zbb → BitVec 3
  | .ANDN => 7 | .ORN => 6 | .XNOR => 4 | .MAX => 6 | .MAXU => 7 | .MIN => 4 | .MINU => 5
  | .ROL => 1 | .ROR => 5

def zbbWF3 : bropw_zbb → BitVec 3
  | .ROLW => 1 | .RORW => 5

end Equiv.Riscv
