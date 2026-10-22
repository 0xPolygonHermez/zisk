/-
  The simulation over straight-line runs of every instruction proved so far:
  R-type, I-type, immediate shifts, `ld` and `sd`.

  A load's or a store's address depends on the registers when it runs, and
  a store changes memory; so what each instruction needs is stated of the
  state sail-riscv reaches when it runs (`RunOk`), along sail-riscv's own run
  (`rvNext`: its step is a function of the state).
-/
import Equiv.StoreSim

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The state sail-riscv's step `n` leads to (the state itself on an error). -/
noncomputable def rvNext (n : Nat) (s : RVState) : RVState :=
  match (interpretSailEffects (try_step n false)).run s with
  | .ok _ s' => s'
  | .error _ s' => s'

theorem rvNext_of {n : Nat} {s s' : RVState} {b : Bool}
    (h : (interpretSailEffects (try_step n false)).run s = .ok b s') : rvNext n s = s' := by
  simp [rvNext, h]

/-- An instruction of the runs proved so far. -/
inductive Op where
  | alu (i : Instr)
  | ld (i : LdInstr)
  | sd (i : SdInstr)

/-- What op `o` needs to run at pc in state s: to be there, in memory and in
    the ZisK program, and, for a load or a store, its address good. -/
def StepOk (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) : Op → Prop
  | .alu i => InstrAt s rs pc zrom i
  | .ld i => LdAt s rs pc zrom i ∧
      ∃ v1 v, getX s i.rs1.toNat = some v1 ∧ LdOk s rs (v1 + i.imm.signExtend 64) v
  | .sd i => SdAt s rs pc zrom i ∧
      ∃ v1 v2, getX s i.rs1.toNat = some v1 ∧ getX s i.rs2.toNat = some v2 ∧
        SdOk rs (v1 + (i.immHi ++ i.immLo).signExtend 64)

/-- Each op of the list good to run in the state sail-riscv reaches. -/
def RunOk (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) :
    List Op → RVState → BitVec 64 → Nat → Prop
  | [], _, _, _ => True
  | o :: os, s, pc, n => StepOk s rs pc zrom o ∧ RunOk rs zrom os (rvNext n s) (pc + 4) (n + 1)

theorem op_step_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (o : Op) (ho : StepOk s rs pc zrom o) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) := by
  match o, ho with
  | .alu i, hi =>
    obtain ⟨s', z', h1, h2, h3, h4, -⟩ := step_rel s z rs hrel pc hpc zrom i hi n
    exact ⟨s', z', h1, h2, h3, h4⟩
  | .ld i, ⟨hi, v1, v, hv1, hok⟩ =>
    obtain ⟨s', z', h1, h2, h3, h4, -⟩ := ldstep_rel s z rs hrel pc hpc zrom i hi v1 hv1 v hok n
    exact ⟨s', z', h1, h2, h3, h4⟩
  | .sd i, ⟨hi, v1, v2, hv1, hv2, hok⟩ =>
    obtain ⟨s', z', h1, h2, h3, h4, -⟩ := sdstep_rel s z rs hrel pc hpc zrom i hi v1 v2 hv1 hv2 hok n
    exact ⟨s', z', h1, h2, h3, h4⟩

/-- The simulation over a straight-line run: from related states at its
    start, when each op is good to run in the state sail-riscv reaches,
    sail-riscv's main loop and ZisK both run through it, and end related,
    at its end. -/
theorem run_sim (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) :
    ∀ (l : List Op) (s : RVState) (z : Equiv.Zisk.ZState) (pc : BitVec 64) (n : Nat),
      Rel s z rs → s.regs.get? Register.PC = some pc → RunOk rs zrom l s pc n →
      ∃ s' z', RVRun n l.length s s' ∧ ZRun zrom l.length z z' ∧ Rel s' z' rs ∧
        s'.regs.get? Register.PC = some (pc + BitVec.ofNat 64 (4 * l.length))
  | [], s, z, pc, n, hrel, hpc, _ => ⟨s, z, .done n s, .done z, hrel, by simpa using hpc⟩
  | o :: os, s, z, pc, n, hrel, hpc, ⟨ho, hos⟩ => by
    obtain ⟨s1, z1, hrun, hzrun, hrel1, hpc1⟩ := op_step_rel s z rs hrel pc hpc zrom o ho n
    rw [rvNext_of hrun] at hos
    obtain ⟨s2, z2, hrun2, hzrun2, hrel2, hpc2⟩ := run_sim zrom rs os s1 z1 (pc + 4) (n + 1) hrel1 hpc1 hos
    refine ⟨s2, z2, .step hrun hrun2, .step hzrun hzrun2, hrel2, ?_⟩
    rw [hpc2]; congr 1
    simp only [List.length_cons]
    rw [BitVec.add_assoc]; congr 1
    apply BitVec.eq_of_toNat_eq; simp; omega

end Equiv.Sim
