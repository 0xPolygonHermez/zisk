/-
  sail-riscv's Zbs, any registers: bclr, bext, binv, bset (ZBS_RTYPE, the bit
  rs2's low six bits name) and their immediate forms (ZBS_IOP): rd gets
  rs1 with that bit cleared, flipped or set, or the bit itself.
-/
import Equiv.RiscvZba

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- What each single-bit op computes on a and the bit's mask. -/
def zbsOp (op : brop_zbs) (a mask : BitVec 64) : BitVec 64 :=
  match op with
  | .BCLR => a &&& ~~~mask
  | .BEXT => zero_extend (m := 64) (bool_to_bit ((a &&& mask) != 0#64))
  | .BINV => a ^^^ mask
  | .BSET => a ||| mask

def zbsIOp (op : biop_zbs) (a mask : BitVec 64) : BitVec 64 :=
  match op with
  | .BCLRI => a &&& ~~~mask
  | .BEXTI => zero_extend (m := 64) (bool_to_bit ((a &&& mask) != 0#64))
  | .BINVI => a ^^^ mask
  | .BSETI => a ||| mask

/-- The mask for bit `i` (mod 64). -/
def zbsMask (i : BitVec 6) : BitVec 64 := shift_bits_left (zero_extend (m := 64) 1#1) i

def zbsRVal (op : brop_zbs) (a b : BitVec 64) : BitVec 64 := zbsOp op a (zbsMask (Sail.BitVec.extractLsb b 5 0))
def zbsIVal (op : biop_zbs) (sh : BitVec 6) (a : BitVec 64) : BitVec 64 := zbsIOp op a (zbsMask sh)

variable (s : RVState) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
  (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2)

include h1 h2 in
theorem execute_zbs_rtype (op : brop_zbs) :
    (interpretSailEffects (execute_ZBS_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (zbsRVal op v1 v2)) := by
  simp only [execute_ZBS_RTYPE]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  cases op <;> rfl

include h1 in
theorem execute_zbs_iop (op : biop_zbs) (sh : BitVec 6) :
    (interpretSailEffects (execute_ZBS_IOP sh (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (zbsIVal op sh v1)) := by
  simp only [execute_ZBS_IOP]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  cases op <;> rfl

end Equiv.Riscv
