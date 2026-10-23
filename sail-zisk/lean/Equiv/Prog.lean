/-
  The simulation over whole runs, control flow included: n steps of
  sail-riscv's main loop and n steps of ZisK from related states end in
  related states, when at each step the instruction at the pc sail-riscv has
  reached is one of those proved (register, immediate, shift, load, store,
  branch) and good to run there. A branch's target is wherever its
  condition, in that state, sends it; so loops are runs too.
-/
import Equiv.BranchSim

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- An instruction of the runs proved so far, branches included. -/
inductive Instr2 where
  | op (o : Op)
  | br (i : BInstr)

/-- What instruction `o` needs to run at pc in state s. -/
def StepOk2 (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) : Instr2 → Prop
  | .op o => StepOk s rs pc zrom o
  | .br i => BAt s rs pc zrom i

/-- For each of the next k steps, from the state sail-riscv reaches, the
    instruction at its pc is one proved and good to run. -/
def RunOk2 (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) :
    Nat → RVState → Nat → Prop
  | 0, _, _ => True
  | k + 1, s, n => (∃ pc o, s.regs.get? Register.PC = some pc ∧ StepOk2 s rs pc zrom o) ∧
      RunOk2 rs zrom k (rvNext n s) (n + 1)

theorem step_rel2 (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (o : Instr2) (ho : StepOk2 s rs pc zrom o) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs := by
  match o, ho with
  | .op o, ho =>
    obtain ⟨s', z', h1, h2, h3, -⟩ := op_step_rel s z rs hrel pc hpc zrom o ho n
    exact ⟨s', z', h1, h2, h3⟩
  | .br i, hi =>
    have l1 : i.rs1.toNat < 32 := i.rs1.isLt
    have l2 : i.rs2.toNat < 32 := i.rs2.isLt
    have e1 := ne_zero_toNat hi.rs1
    have e2 := ne_zero_toNat hi.rs2
    obtain ⟨xs, -, agree⟩ := hrel.regs
    obtain ⟨s', z', h1, h2, h3, -⟩ := brstep_rel s z rs hrel pc hpc zrom i hi _ _
      (agree i.rs1.toNat (by omega) l1) (agree i.rs2.toNat (by omega) l2) n
    exact ⟨s', z', h1, h2, h3⟩

/-- The simulation over k steps: from related states, when each step's
    instruction is one proved and good to run in the state sail-riscv
    reaches, sail-riscv's main loop and ZisK both take k steps and end
    related. -/
theorem prog_sim (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) :
    ∀ (k : Nat) (s : RVState) (z : Equiv.Zisk.ZState) (n : Nat),
      Rel s z rs → RunOk2 rs zrom k s n →
      ∃ s' z', RVRun n k s s' ∧ ZRun zrom k z z' ∧ Rel s' z' rs
  | 0, s, z, n, hrel, _ => ⟨s, z, .done n s, .done z, hrel⟩
  | k + 1, s, z, n, hrel, ⟨⟨pc, o, hpc, ho⟩, hrest⟩ => by
    obtain ⟨s1, z1, hrun, hzrun, hrel1⟩ := step_rel2 s z rs hrel pc hpc zrom o ho n
    rw [rvNext_of hrun] at hrest
    obtain ⟨s2, z2, hrun2, hzrun2, hrel2⟩ := prog_sim zrom rs k s1 z1 (n + 1) hrel1 hrest
    exact ⟨s2, z2, .step hrun hrun2, .step hzrun hzrun2, hrel2⟩

end Equiv.Sim
