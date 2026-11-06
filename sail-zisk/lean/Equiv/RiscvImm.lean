/-
  sail-riscv's immediate instructions: what `execute` does for the I-type
  arithmetic (addi, slti, sltiu, xori, ori, andi) and the immediate shifts
  (slli, srli, srai), for any registers — as Equiv/Riscv.lean does for the
  R-type.
-/
import Equiv.Riscv

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- What an I-type instruction computes from rs1 and its immediate. -/
def itypeVal : iop → BitVec 64 → BitVec 12 → BitVec 64
  | .ADDI, a, i => a + sign_extend (m := 64) i
  | .SLTI, a, i => zero_extend (m := 64) (bool_to_bit (zopz0zI_s a (sign_extend (m := 64) i)))
  | .SLTIU, a, i => zero_extend (m := 64) (bool_to_bit (zopz0zI_u a (sign_extend (m := 64) i)))
  | .ANDI, a, i => a &&& sign_extend (m := 64) i
  | .ORI, a, i => a ||| sign_extend (m := 64) i
  | .XORI, a, i => a ^^^ sign_extend (m := 64) i

/-- Any I-type instruction, any registers: rd gets what it computes. -/
theorem execute_itype (s : RVState) (op : iop) (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) :
    (interpretSailEffects (execute_ITYPE imm (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (itypeVal op v1 imm)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have w := wX_run s rd.toNat rd.lt (itypeVal op v1 imm)
  cases op <;>
  · simp only [execute_ITYPE, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
      run_pure']
    rw [r1]; dsimp only
    simp only [itypeVal] at w; rw [w]; rfl

/-- What an immediate shift computes from rs1 and its shift amount. -/
def shiftVal : sop → BitVec 64 → BitVec 6 → BitVec 64
  | .SLLI, a, sh => shift_bits_left a (Sail.BitVec.extractLsb sh (Functions.log2_xlen -i 1) 0)
  | .SRLI, a, sh => shift_bits_right a (Sail.BitVec.extractLsb sh (Functions.log2_xlen -i 1) 0)
  | .SRAI, a, sh => shift_bits_right_arith a (Sail.BitVec.extractLsb sh (Functions.log2_xlen -i 1) 0)

/-- Any immediate shift, any registers: rd gets what it computes. -/
theorem execute_shiftiop (s : RVState) (op : sop) (rs1 rd : RegBits) (sh : BitVec 6) (v1 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) :
    (interpretSailEffects (execute_SHIFTIOP sh (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (shiftVal op v1 sh)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have w := wX_run s rd.toNat rd.lt (shiftVal op v1 sh)
  cases op <;>
  · simp only [execute_SHIFTIOP, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
      run_pure']
    rw [r1]; dsimp only
    simp only [shiftVal] at w; rw [w]; rfl

end Equiv.Riscv
