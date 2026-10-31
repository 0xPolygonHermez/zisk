/-
  sail-riscv's Zbb register ops, any registers (ZBB_RTYPE): andn, orn, xnor,
  max, maxu, min, minu, rol, ror; their word forms rolw, rorw (ZBB_RTYPEW);
  and the rotations by an immediate, rori and roriw. rd gets what each
  computes.
-/
import Equiv.RiscvZbs

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

def zbbVal (op : brop_zbb) (a b : BitVec 64) : BitVec 64 :=
  match op with
  | .ANDN => a &&& ~~~b
  | .ORN => a ||| ~~~b
  | .XNOR => ~~~(a ^^^ b)
  | .MAX => if zopz0zK_s a b then a else b
  | .MAXU => if zopz0zK_u a b then a else b
  | .MIN => if zopz0zI_s a b then a else b
  | .MINU => if zopz0zI_u a b then a else b
  | .ROL => rotate_bits_left a (Sail.BitVec.extractLsb b (Functions.log2_xlen -i 1) 0)
  | .ROR => rotate_bits_right a (Sail.BitVec.extractLsb b (Functions.log2_xlen -i 1) 0)

variable (s : RVState) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
  (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2)

include h1 h2 in
theorem execute_zbb_rtype (op : brop_zbb) :
    (interpretSailEffects (execute_ZBB_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (zbbVal op v1 v2)) := by
  simp only [execute_ZBB_RTYPE]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  cases op <;> rfl

def zbbWVal (op : bropw_zbb) (a b : BitVec 64) : BitVec 64 :=
  sign_extend (m := 64) (match op with
    | .ROLW => rotate_bits_left (Sail.BitVec.extractLsb a 31 0) (Sail.BitVec.extractLsb b 4 0)
    | .RORW => rotate_bits_right (Sail.BitVec.extractLsb a 31 0) (Sail.BitVec.extractLsb b 4 0))

include h1 h2 in
theorem execute_zbb_rtypew (op : bropw_zbb) :
    (interpretSailEffects (execute_ZBB_RTYPEW (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (zbbWVal op v1 v2)) := by
  simp only [execute_ZBB_RTYPEW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [r2]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  cases op <;> rfl

def roriVal (sh : BitVec 6) (a : BitVec 64) : BitVec 64 :=
  rotate_bits_right a (Sail.BitVec.extractLsb sh (Functions.log2_xlen -i 1) 0)

def roriwVal (sh : BitVec 5) (a : BitVec 64) : BitVec 64 :=
  sign_extend (m := 64) (rotate_bits_right (Sail.BitVec.extractLsb a 31 0) sh)

include h1 in
theorem execute_rori (sh : BitVec 6) :
    (interpretSailEffects (execute_RORI sh (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (roriVal sh v1)) := by
  simp only [execute_RORI]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  rfl

include h1 in
theorem execute_roriw (sh : BitVec 5) :
    (interpretSailEffects (execute_RORIW sh (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (roriwVal sh v1)) := by
  simp only [execute_RORIW]
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  simp only [rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only
  rw [wX_run s rd.toNat rd.lt]
  rfl

end Equiv.Riscv
