/-
  sail-riscv's counts, any registers: clz, ctz, cpop and their word forms
  clzw, ctzw, cpopw. rd gets the count, to 64 bits: Lean's clz for clz and
  ctz, cpop's ones counted by its loop (`count_ones`, run as `popLoop`).
-/
import Equiv.Count

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

variable (s : RVState) (rs1 rd : RegBits) (v1 : BitVec 64) (h1 : getX s rs1.toNat = some v1)

include h1 in
theorem execute_clz :
    (interpretSailEffects (execute_CLZ (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (to_bits (l := 64) (Sail.BitVec.countLeadingZeros v1))) := by
  simp only [execute_CLZ]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

include h1 in
theorem execute_ctz :
    (interpretSailEffects (execute_CTZ (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (to_bits (l := 64) (Sail.BitVec.countTrailingZeros v1))) := by
  simp only [execute_CTZ]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

include h1 in
theorem execute_clzw :
    (interpretSailEffects (execute_CLZW (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS
        (setX s rd.toNat (to_bits (l := 64) (Sail.BitVec.countLeadingZeros (Sail.BitVec.extractLsb v1 31 0)))) := by
  simp only [execute_CLZW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

include h1 in
theorem execute_ctzw :
    (interpretSailEffects (execute_CTZW (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS
        (setX s rd.toNat (to_bits (l := 64) (Sail.BitVec.countTrailingZeros (Sail.BitVec.extractLsb v1 31 0)))) := by
  simp only [execute_CTZW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

include h1 in
theorem execute_cpop :
    (interpretSailEffects (execute_CPOP (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (to_bits (l := 64) (popLoop v1))) := by
  simp only [execute_CPOP]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [count_ones_run]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

include h1 in
theorem execute_cpopw :
    (interpretSailEffects (execute_CPOPW (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (to_bits (l := 64) (popLoop (Sail.BitVec.extractLsb v1 31 0)))) := by
  simp only [execute_CPOPW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [count_ones_run]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

end Equiv.Riscv
