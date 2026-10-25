/-
  sail-riscv's Zba, the single-instruction forms, any registers: sh1add,
  sh2add, sh3add (ZBA_RTYPE: rs1 shifted by 1, 2 or 3, plus rs2) and
  slli.uw (SLLIUW: rs1's low word, zero-extended, shifted): rd gets what
  each computes.
-/
import Equiv.RiscvM

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

def zbaVal (sh : BitVec 2) (a b : BitVec 64) : BitVec 64 := shift_bits_left a sh + b

def slliuwVal (sh : BitVec 6) (a : BitVec 64) : BitVec 64 :=
  shift_bits_left (zero_extend (m := 64) (Sail.BitVec.extractLsb a 31 0)) sh

variable (s : RVState) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
  (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2)

include h1 h2 in
theorem execute_zba_rtype (sh : BitVec 2) :
    (interpretSailEffects (execute_ZBA_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) sh)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (zbaVal sh v1 v2)) := by
  simp only [execute_ZBA_RTYPE]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

include h1 in
theorem execute_slliuw (sh : BitVec 6) :
    (interpretSailEffects (execute_SLLIUW sh (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (slliuwVal sh v1)) := by
  simp only [execute_SLLIUW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]; rfl

end Equiv.Riscv
