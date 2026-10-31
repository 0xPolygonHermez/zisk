/-
  sail-riscv's byte ops, any registers: Zbb's rev8 and orc.b, Zbkb's brev8.
  rd gets what each computes: rev8 and brev8 pure functions of rs1's value,
  orc.b its loop over the bytes, here as a pure function too.
-/
import Equiv.Loops

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- orc.b's loop, as sail-riscv's execute writes it, pure. -/
def orcbVal (v : BitVec 64) : BitVec 64 := Id.run do
  let mut r : BitVec 64 := zeros (n := 64)
  for i in [0:(64 -i 8):8]i do
    r := Sail.BitVec.updateSubrange r (i +i 7) i
      (if (Sail.BitVec.extractLsb v (i +i 7) i == zeros (n := (((i +i 7) -i i) +i 1))) then 0x00#8 else 0xFF#8)
  pure r

variable (s : RVState) (rs1 rd : RegBits) (v1 : BitVec 64) (h1 : getX s rs1.toNat = some v1)

include h1 in
theorem execute_rev8 :
    (interpretSailEffects (execute_REV8 (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (rev8 v1)) := by
  simp only [execute_REV8]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

include h1 in
theorem execute_brev8 :
    (interpretSailEffects (execute_BREV8 (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (brev8 v1)) := by
  simp only [execute_BREV8]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]

include h1 in
theorem execute_orcb :
    (interpretSailEffects (execute_ORCB (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (orcbVal v1)) := by
  simp only [execute_ORCB]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  simp only [forIn, ForIn'.forIn', IntRange.forIn', Functions.xlen]
  simp (disch := decide) only [loop_in, loop_out]
  simp only [interp_bind, interp_pure, run_bind', run_pure']
  rw [wX_run s rd.toNat rd.lt]
  congr 2
  simp only [orcbVal]
  unroll

end Equiv.Riscv
