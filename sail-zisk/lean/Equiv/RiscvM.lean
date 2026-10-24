/-
  sail-riscv's M extension, any registers: mul, mulh, mulhsu, mulhu (MUL),
  div, divu (DIV), rem, remu (REM), mulw (MULW), divw, divuw (DIVW), remw,
  remuw (REMW): rd gets what each computes.
-/
import Equiv.RiscvW

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

def mulVal (op : mul_op) (a b : BitVec 64) : BitVec 64 :=
  mult_to_bits_half (l := 64) op.signed_rs1 op.signed_rs2 a b op.result_part

def divVal (u : Bool) (a b : BitVec 64) : BitVec 64 :=
  let x : Int := if u then BitVec.toNatInt a else BitVec.toInt a
  let y : Int := if u then BitVec.toNatInt b else BitVec.toInt b
  let q := if y == 0 then -1 else Int.tdiv x y
  let q := if (!u && q ≥b 2 ^i (64 -i 1)) then -(2 ^i (64 -i 1)) else q
  to_bits_truncate (l := 64) q

def remVal (u : Bool) (a b : BitVec 64) : BitVec 64 :=
  let x : Int := if u then BitVec.toNatInt a else BitVec.toInt a
  let y : Int := if u then BitVec.toNatInt b else BitVec.toInt b
  to_bits_truncate (l := 64) (if y == 0 then x else Int.tmod x y)

def mulwVal (a b : BitVec 64) : BitVec 64 :=
  sign_extend (m := 64) (to_bits_truncate (l := 32)
    ((Sail.BitVec.extractLsb a 31 0).toInt *i (Sail.BitVec.extractLsb b 31 0).toInt))

def divwVal (u : Bool) (a b : BitVec 64) : BitVec 64 :=
  let a32 := Sail.BitVec.extractLsb a 31 0
  let b32 := Sail.BitVec.extractLsb b 31 0
  let x : Int := if u then BitVec.toNatInt a32 else BitVec.toInt a32
  let y : Int := if u then BitVec.toNatInt b32 else BitVec.toInt b32
  let q := if y == 0 then -1 else Int.tdiv x y
  let q := if (!u && q ≥b 2 ^i 31) then -(2 ^i 31) else q
  sign_extend (m := 64) (to_bits_truncate (l := 32) q)

def remwVal (u : Bool) (a b : BitVec 64) : BitVec 64 :=
  let a32 := Sail.BitVec.extractLsb a 31 0
  let b32 := Sail.BitVec.extractLsb b 31 0
  let x : Int := if u then BitVec.toNatInt a32 else BitVec.toInt a32
  let y : Int := if u then BitVec.toNatInt b32 else BitVec.toInt b32
  sign_extend (m := 64) (to_bits_truncate (l := 32) (if y == 0 then x else Int.tmod x y))

section
variable (s : RVState) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
  (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2)
include h1 h2

theorem execute_mul (op : mul_op) :
    (interpretSailEffects (execute_MUL (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (mulVal op v1 v2)) := by
  simp only [execute_MUL]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

theorem execute_div (u : Bool) :
    (interpretSailEffects (execute_DIV (.Regidx rs2) (.Regidx rs1) (.Regidx rd) u)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (divVal u v1 v2)) := by
  simp only [execute_DIV]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

theorem execute_rem (u : Bool) :
    (interpretSailEffects (execute_REM (.Regidx rs2) (.Regidx rs1) (.Regidx rd) u)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (remVal u v1 v2)) := by
  simp only [execute_REM]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

theorem execute_mulw :
    (interpretSailEffects (execute_MULW (.Regidx rs2) (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (mulwVal v1 v2)) := by
  simp only [execute_MULW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

theorem execute_divw (u : Bool) :
    (interpretSailEffects (execute_DIVW (.Regidx rs2) (.Regidx rs1) (.Regidx rd) u)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (divwVal u v1 v2)) := by
  simp only [execute_DIVW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

theorem execute_remw (u : Bool) :
    (interpretSailEffects (execute_REMW (.Regidx rs2) (.Regidx rs1) (.Regidx rd) u)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (remwVal u v1 v2)) := by
  simp only [execute_REMW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

end

end Equiv.Riscv
