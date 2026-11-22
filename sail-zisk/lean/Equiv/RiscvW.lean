/-
  sail-riscv's RV64I word instructions: addw, subw, sllw, srlw, sraw
  (RTYPEW), addiw (ADDIW), slliw, srliw, sraiw (SHIFTIWOP), any registers:
  rd gets the 32-bit result, sign-extended.
-/
import Equiv.RiscvImm

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- What an RTYPEW instruction computes from rs1's and rs2's values. -/
def rtypewVal : ropw → BitVec 64 → BitVec 64 → BitVec 64
  | .ADDW, a, b => sign_extend (m := 64) (Sail.BitVec.extractLsb a 31 0 + Sail.BitVec.extractLsb b 31 0)
  | .SUBW, a, b => sign_extend (m := 64) (Sail.BitVec.extractLsb a 31 0 - Sail.BitVec.extractLsb b 31 0)
  | .SLLW, a, b => sign_extend (m := 64) (shift_bits_left (Sail.BitVec.extractLsb a 31 0)
      (Sail.BitVec.extractLsb (Sail.BitVec.extractLsb b 31 0) 4 0))
  | .SRLW, a, b => sign_extend (m := 64) (shift_bits_right (Sail.BitVec.extractLsb a 31 0)
      (Sail.BitVec.extractLsb (Sail.BitVec.extractLsb b 31 0) 4 0))
  | .SRAW, a, b => sign_extend (m := 64) (shift_bits_right_arith (Sail.BitVec.extractLsb a 31 0)
      (Sail.BitVec.extractLsb (Sail.BitVec.extractLsb b 31 0) 4 0))

theorem execute_rtypew (s : RVState) (op : ropw) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2) :
    (interpretSailEffects (execute_RTYPEW (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (rtypewVal op v1 v2)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  have w := wX_run s rd.toNat rd.lt (rtypewVal op v1 v2)
  cases op <;>
  · simp only [execute_RTYPEW, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
      run_pure']
    rw [r1]; dsimp only
    rw [r2]; dsimp only
    simp only [rtypewVal] at w; rw [w]; rfl

/-- What addiw computes from rs1's value. -/
def addiwVal (a : BitVec 64) (imm : BitVec 12) : BitVec 64 :=
  sign_extend (m := 64) (Sail.BitVec.extractLsb (a + sign_extend (m := 64) imm) 31 0)

theorem execute_addiw (s : RVState) (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) :
    (interpretSailEffects (execute_ADDIW imm (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (addiwVal v1 imm)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have w := wX_run s rd.toNat rd.lt (addiwVal v1 imm)
  simp only [execute_ADDIW, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
    run_pure']
  rw [r1]; dsimp only
  simp only [addiwVal] at w; rw [w]; rfl

/-- What a SHIFTIWOP instruction computes from rs1's value. -/
def shiftiwVal : sopw → BitVec 64 → BitVec 5 → BitVec 64
  | .SLLIW, a, sh => sign_extend (m := 64) (shift_bits_left (Sail.BitVec.extractLsb a 31 0) sh)
  | .SRLIW, a, sh => sign_extend (m := 64) (shift_bits_right (Sail.BitVec.extractLsb a 31 0) sh)
  | .SRAIW, a, sh => sign_extend (m := 64) (shift_bits_right_arith (Sail.BitVec.extractLsb a 31 0) sh)

theorem execute_shiftiwop (s : RVState) (op : sopw) (rs1 rd : RegBits) (sh : BitVec 5) (v1 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) :
    (interpretSailEffects (execute_SHIFTIWOP sh (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (shiftiwVal op v1 sh)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have w := wX_run s rd.toNat rd.lt (shiftiwVal op v1 sh)
  cases op <;>
  · simp only [execute_SHIFTIWOP, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
      run_pure']
    rw [r1]; dsimp only
    simp only [shiftiwVal] at w; rw [w]; rfl

/-- funct3 and funct7 of each RTYPEW op. -/
def rtypewF3 : ropw → BitVec 3
  | .ADDW => 0 | .SUBW => 0 | .SLLW => 1 | .SRLW => 5 | .SRAW => 5

def rtypewF7 : ropw → BitVec 7
  | .SUBW => 0x20 | .SRAW => 0x20 | _ => 0

/-- funct3 and funct7 of each SHIFTIWOP op. -/
def shiftwF3 : sopw → BitVec 3
  | .SLLIW => 1 | .SRLIW => 5 | .SRAIW => 5

def shiftwF7 : sopw → BitVec 7
  | .SRAIW => 0x20 | _ => 0

end Equiv.Riscv
