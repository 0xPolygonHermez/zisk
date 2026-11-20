/-
  sail-riscv's control flow: what `execute` does for the conditional
  branches, for any registers. A taken branch sets nextPC (jump_to); one not
  taken leaves the state as it is.
-/
import Equiv.RiscvStep

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Whether a branch is taken, from rs1 and rs2. -/
def btypeTaken : bop → BitVec 64 → BitVec 64 → Bool
  | .BEQ, a, b => a == b
  | .BNE, a, b => a != b
  | .BLT, a, b => zopz0zI_s a b
  | .BGE, a, b => zopz0zKzJ_s a b
  | .BLTU, a, b => zopz0zI_u a b
  | .BGEU, a, b => zopz0zKzJ_u a b

/-- Any branch, any registers: taken, to a 4-aligned target, nextPC gets
    the target; not taken, nothing changes, whatever the target. -/
theorem execute_btype (s : RVState) (op : bop) (rs1 rs2 : RegBits) (imm : BitVec 13) (v1 v2 : BitVec 64)
    (pc : BitVec 64) (m : RegisterType Register.misa)
    (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2)
    (hpc : s.regs.get? Register.PC = some pc) (hmisa : s.regs.get? Register.misa = some m)
    (hal : btypeTaken op v1 v2 = true →
      (pc + imm.signExtend 64)[0] = false ∧ (pc + imm.signExtend 64)[1] = false) :
    (interpretSailEffects (execute_BTYPE imm (.Regidx rs2) (.Regidx rs1) op)).run s
      = .ok RETIRE_SUCCESS (if btypeTaken op v1 v2
          then { s with regs := s.regs.insert Register.nextPC (pc + imm.signExtend 64) } else s) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  cases op <;>
  · simp only [execute_BTYPE, rX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
    rw [r1]; dsimp only; rw [r2]; dsimp only
    simp only [btypeTaken] at hal ⊢
    split
    · next ht =>
        have hal := hal ht
        simp [jump_to, ext_control_check_pc, currentlyEnabled, hartSupports, bit_to_bool, set_next_pc,
          redirect_callback, sign_extend, Sail.BitVec.signExtend, BitVec.access,
          interpretSailEffects, LeanZiskRV.readReg, Sail.ArchSem.PreSail.readReg, LeanZiskRV.writeReg,
          Sail.ArchSem.PreSail.writeReg, FreeM.liftM, interpretInstructionEffect, hpc, hmisa, hal,
          me_run_lift_bind, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert,
          bool_bit_backwards, LeanZiskRV.not]
        all_goals rfl
    · simp

end Equiv.Riscv
