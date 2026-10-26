/-
  The simulation, one whole step: sail-riscv's `try_step` on an R-type
  instruction, and sail-zisk's execution of its transpilation, from agreeing
  registers and the same pc, end with agreeing registers and both pcs at
  pc + 4.

  The transpiler puts the ZisK code for the RISC-V instruction at address A
  at ZisK address A too (ZisK's ROM is addressed by RISC-V pc), so the pc
  relation is equality. That the ZisK instruction at pc is the transpiler's
  `regInst` for the word there is the hypothesis this step takes; it is the
  transpiler's correctness, phase 3.
-/
import Equiv.RiscvStep
import Equiv.Sim

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- A step that changes no register but PC, nextPC, minstret,
    minstret_increment and x registers keeps StepReady. -/
theorem _root_.Equiv.Riscv.StepReady.keep {s s' : RVState} {rs : List PMA_Region} (hr : StepReady s rs)
    (hwf : WF s')
    (ho : ∀ r, isXReg r = false → r ≠ Register.PC → r ≠ Register.nextPC → r ≠ Register.minstret →
      r ≠ Register.minstret_increment → s'.regs.get? r = s.regs.get? r) : StepReady s' rs := by
  have e : ∀ r, isXReg r = false → r ≠ Register.PC → r ≠ Register.nextPC → r ≠ Register.minstret →
      r ≠ Register.minstret_increment → s'.regs.get? r = s.regs.get? r := ho
  refine ⟨hwf, ?_, ?_, ?_, ?_, hr.mem.congr ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hr.mip
  · rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hr.mie
  · rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hr.hart
  · rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hr.elp
  · intro r hrm
    simp only [memRegs, List.mem_cons, List.not_mem_nil, or_false] at hrm
    rcases hrm with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;>
      exact e _ (by decide) (by decide) (by decide) (by decide) (by decide)
  · obtain ⟨v, hv, hz⟩ := hr.pmm
    exact ⟨v, by rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hv, hz⟩
  · obtain ⟨v, hv, hz⟩ := hr.misaM
    exact ⟨v, by rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hv, hz⟩
  · obtain ⟨v, hv, hz⟩ := hr.misaB
    exact ⟨v, by rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hv, hz⟩
  · obtain ⟨v, hv, hz⟩ := hr.misaA
    exact ⟨v, by rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hv, hz⟩
  · rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hr.march
  · rw [e _ (by decide) (by decide) (by decide) (by decide) (by decide)]; exact hr.mtvec

set_option maxHeartbeats 4000000 in
/-- One step of each, on an R-type instruction with rs1, rs2, rd ≠ x0: the
    registers agree after, both pcs are pc + 4, and sail-riscv's state is
    still ready for a step, with its memory unchanged. -/
theorem rtype_step_sim (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (op : rop)
    (hop : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 6 0 = 0x33#7)
    (hf3 : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 14 12 = rtypeF3 op)
    (hf7 : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 31 25 = rtypeF7 op)
    (hrd : ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7 : RegBits).toNat ≠ 0)
    (hr1 : ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 19 15 : RegBits).toNat ≠ 0)
    (hr2 : ((b3 ++ b2 ++ (b1 ++ b0)).extractLsb 24 20 : RegBits).toNat ≠ 0) (n : Nat)
    (z : Equiv.Zisk.ZState) (xs : Vector (BitVec 64) 40) (st : BitVec 64)
    (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    (hst : z.regs.get? _root_.Zisk.Register.Step = some st)
    (agree : RegsAgree s xs) :
    let rd : RegBits := (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7
    let rs1 : RegBits := (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 19 15
    let rs2 : RegBits := (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 24 20
    ∃ s' z' xs',
      (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (_root_.Zisk.Functions.zisk_execute (Equiv.Zisk.regInst (ziskOp op) rd.toNat rs1.toNat rs2.toNat)).run z
        = .ok false z' ∧
      z'.regs.get? _root_.Zisk.Register.Xs = some xs' ∧
      RegsAgree s' xs' ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧
      z'.regs.get? _root_.Zisk.Register.PC = some (pc + 4) ∧
      z'.regs.get? _root_.Zisk.Register.Step = some (BitVec.addInt st 1) ∧
      z'.regs.get? _root_.Zisk.Register.Zerror = z.regs.get? _root_.Zisk.Register.Zerror ∧
      StepReady s' rs ∧ s'.mem = s.mem ∧ z'.mem = z.mem := by
  intro rd rs1 rs2
  have e1 : rs1.toNat ≠ 0 := hr1
  have e2 : rs2.toNat ≠ 0 := hr2
  have ed : rd.toNat ≠ 0 := hrd
  have l1 := rs1.lt; have l2 := rs2.lt; have ld := rd.lt
  have a1 := agree rs1.toNat (by omega) l1
  have a2 := agree rs2.toNat (by omega) l2
  have hrd5 : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7 ≠ 0#5 := by
    intro e; apply hrd; show (BitVec.extractLsb 11 7 (b3 ++ b2 ++ (b1 ++ b0))).toNat = 0; rw [e]; rfl
  obtain ⟨s', hrun, hmem, hwf, hpc', hxs', ho⟩ :=
    try_step_rtype s rs hr pc hpc b0 b1 b2 b3 hw op hop hf3 hf7 hrd5 _ _ a1 a2 n
  obtain ⟨-, hag⟩ := rtype_regs s xs op rs1 rs2 rd e1 e2 ed agree
  refine ⟨s', _, vectorUpdate xs rd.toNat
      (_root_.Zisk.Functions.zisk_exec (ziskOp op) xs[(rs1.toNat : Int)]! xs[(rs2.toNat : Int)]!).1, hrun,
    Equiv.Zisk.execute_regop z (ziskOp op) (pureOp op) rd.toNat rs1.toNat rs2.toNat
      (by omega) (by omega) (by omega) xs pc st hx hzpc hst,
    by simp [Std.ExtDHashMap.get?_insert], ?_, hpc', by simp, by simp [Std.ExtDHashMap.get?_insert],
    by simp [Std.ExtDHashMap.get?_insert], hr.keep hwf ho, hmem, rfl⟩
  intro k hk1 hk2
  rw [hxs' k hk2]
  have := hag k hk1 hk2
  simpa only [ziskVal, getElem!_int] using this

end Equiv.Sim
