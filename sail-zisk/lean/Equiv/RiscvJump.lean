/-
  sail-riscv's upper-immediate instructions and jumps: what `execute` does
  for lui, auipc and jal, for any registers.
-/
import Equiv.RiscvCtl

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- What lui and auipc compute, at pc. -/
def utypeVal : uop → BitVec 64 → BitVec 20 → BitVec 64
  | .LUI, _, imm => sign_extend (m := 64) (imm ++ 0x000#12)
  | .AUIPC, pc, imm => pc + sign_extend (m := 64) (imm ++ 0x000#12)

/-- lui and auipc, any rd: rd gets what they compute. -/
theorem execute_utype (s : RVState) (op : uop) (rd : RegBits) (imm : BitVec 20) (pc : BitVec 64)
    (hpc : s.regs.get? Register.PC = some pc) :
    (interpretSailEffects (execute_UTYPE imm (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (utypeVal op pc imm)) := by
  have w := wX_run s rd.toNat rd.lt (utypeVal op pc imm)
  cases op <;>
  · simp only [execute_UTYPE, wX_bits, get_arch_pc, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
    try rw [readReg_run s _ pc hpc]; try dsimp only
    simp only [utypeVal] at w; rw [w]; rfl

/-- A jump to a 4-aligned target: nextPC gets it. -/
theorem jump_to_run (s : RVState) (tgt : BitVec 64) (m : RegisterType Register.misa)
    (hmisa : s.regs.get? Register.misa = some m) (hal : tgt[0] = false ∧ tgt[1] = false) :
    (interpretSailEffects (jump_to tgt)).run s
      = .ok RETIRE_SUCCESS { s with regs := s.regs.insert Register.nextPC tgt } := by
  simp [jump_to, ext_control_check_pc, currentlyEnabled, hartSupports, bit_to_bool, set_next_pc,
    redirect_callback, BitVec.access,
    interpretSailEffects, LeanZiskRV.readReg, Sail.ArchSem.PreSail.readReg, LeanZiskRV.writeReg,
    Sail.ArchSem.PreSail.writeReg, FreeM.liftM, interpretInstructionEffect, hmisa, hal,
    me_run_lift_bind, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert,
    bool_bit_backwards, LeanZiskRV.not]
  all_goals rfl

/-- `jal rd, imm`, to a 4-aligned target: nextPC gets the target, and rd the
    link, the nextPC before it (pc + 4). -/
theorem execute_jal (s : RVState) (rd : RegBits) (imm : BitVec 21) (pc npc : BitVec 64)
    (m : RegisterType Register.misa)
    (hpc : s.regs.get? Register.PC = some pc) (hnpc : s.regs.get? Register.nextPC = some npc)
    (hmisa : s.regs.get? Register.misa = some m)
    (hal : (pc + imm.signExtend 64)[0] = false ∧ (pc + imm.signExtend 64)[1] = false) :
    (interpretSailEffects (execute_JAL imm (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS
          (setX { s with regs := s.regs.insert Register.nextPC (pc + imm.signExtend 64) } rd.toNat npc) := by
  have w := wX_run { s with regs := s.regs.insert Register.nextPC (pc + imm.signExtend 64) } rd.toNat rd.lt npc
  have j := jump_to_run s (pc + imm.signExtend 64) m hmisa hal
  simp only [execute_JAL, get_next_pc, wX_bits, toNatInt_toNat, interp_bind, run_bind', sign_extend,
    Sail.BitVec.signExtend]
  rw [readReg_run s _ npc hnpc]; dsimp only
  rw [readReg_run s _ pc hpc]; dsimp only
  rw [j]; dsimp only
  simp only [RETIRE_SUCCESS, interp_bind, run_bind']
  rw [w]; rfl

/-- Where jalr goes: rs1 + imm, bit 0 cleared. -/
def jalrTarget (v1 : BitVec 64) (imm : BitVec 12) : BitVec 64 :=
  BitVec.update (v1 + imm.signExtend 64) 0 0#1

/-- `jalr rd, imm(rs1)`, to a 4-aligned target, Zicfilp off: nextPC gets the
    target, and rd the link, the nextPC before it. -/
theorem execute_jalr (s : RVState) (rs1 rd : RegBits) (imm : BitVec 12) (v1 npc : BitVec 64)
    (m : RegisterType Register.misa) (h1 : getX s rs1.toNat = some v1)
    (hnpc : s.regs.get? Register.nextPC = some npc)
    (hmisa : s.regs.get? Register.misa = some m) (hp : s.regs.get? Register.cur_privilege = some .Machine)
    (sec : RegisterType Register.mseccfg) (hsec : s.regs.get? Register.mseccfg = some sec)
    (hal : (jalrTarget v1 imm)[0] = false ∧ (jalrTarget v1 imm)[1] = false) :
    (interpretSailEffects (execute_JALR imm (.Regidx rs1) (.Regidx rd))).run s
      = .ok RETIRE_SUCCESS
          (setX { s with regs := s.regs.insert Register.nextPC (jalrTarget v1 imm) } rd.toNat npc) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have w := wX_run { s with regs := s.regs.insert Register.nextPC (jalrTarget v1 imm) } rd.toNat rd.lt npc
  have j := jump_to_run s (jalrTarget v1 imm) m hmisa hal
  simp only [execute_JALR, update_elp_state, get_next_pc, rX_bits, wX_bits, toNatInt_toNat, interp_bind,
    run_bind', sign_extend, Sail.BitVec.signExtend]
  rw [ce_zicfilp s hp sec hsec]; dsimp only
  simp only [Bool.false_eq_true, if_false, interp_pure, run_pure']
  rw [readReg_run s _ npc hnpc]; dsimp only
  rw [r1]; dsimp only
  try (rw [run_pure']; dsimp only)
  have ej : BitVec.update (v1 + BitVec.signExtend 64 imm) 0 0#1 = jalrTarget v1 imm := rfl
  rw [ej, j]; dsimp only
  simp only [RETIRE_SUCCESS, interp_bind, run_bind']
  rw [w]; rfl

end Equiv.Riscv
