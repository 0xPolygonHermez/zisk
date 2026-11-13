/-
  sail-riscv's Zbkb packs (pack, packh: ZBKB_RTYPE; packw: ZBKB_PACKW) and
  Zbb's extensions (sext.b, sext.h, zext.h: ZBB_EXTOP), any registers. rd
  gets what each computes.
-/
import Equiv.RiscvZbb

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

def zbkbVal (op : brop_zbkb) (a b : BitVec 64) : BitVec 64 :=
  match op with
  | .PACK =>
    (Sail.BitVec.extractLsb b ((Functions.xlen_bytes *i 4) -i 1) 0) +++ (Sail.BitVec.extractLsb a ((Functions.xlen_bytes *i 4) -i 1) 0)
  | .PACKH => zero_extend (m := 64) ((Sail.BitVec.extractLsb b 7 0) +++ (Sail.BitVec.extractLsb a 7 0))

def packwVal (a b : BitVec 64) : BitVec 64 :=
  sign_extend (m := 64) ((Sail.BitVec.extractLsb b 15 0) +++ (Sail.BitVec.extractLsb a 15 0))

def extVal (op : extop_zbb) (a : BitVec 64) : BitVec 64 :=
  match op with
  | .SEXTB => sign_extend (m := 64) (Sail.BitVec.extractLsb a 7 0)
  | .SEXTH => sign_extend (m := 64) (Sail.BitVec.extractLsb a 15 0)
  | .ZEXTH => zero_extend (m := 64) (Sail.BitVec.extractLsb a 15 0)

variable (s : RVState) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
  (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2)

include h1 h2 in
theorem execute_zbkb_rtype (op : brop_zbkb) :
    (interpretSailEffects (execute_ZBKB_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (zbkbVal op v1 v2)) := by
  simp only [execute_ZBKB_RTYPE]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  cases op <;> rfl

include h1 h2 in
theorem execute_packw :
    (interpretSailEffects (execute_ZBKB_PACKW (.Regidx rs2) (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (packwVal v1 v2)) := by
  simp only [execute_ZBKB_PACKW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [Functions.xlen, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, beq_self_eq_true, if_true, rX_bits, wX_bits, toNatInt_toNat, interp_bind,
    interp_pure, run_bind', run_pure']
  rw [show (interpretSailEffects (FreeM.pure () : SailM Unit)).run s = .ok () s from rfl]; dsimp only
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  rfl

include h1 in
theorem execute_zbb_extop (op : extop_zbb) :
    (interpretSailEffects (execute_ZBB_EXTOP (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (extVal op v1)) := by
  simp only [execute_ZBB_EXTOP]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  cases op <;> rfl

end Equiv.Riscv
