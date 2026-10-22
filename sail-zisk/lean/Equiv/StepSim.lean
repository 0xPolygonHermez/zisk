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

set_option maxHeartbeats 4000000 in
/-- One step of each, on an R-type instruction with rs1, rs2, rd ≠ x0. -/
theorem rtype_step_sim (s : RVState) (h : WF s) (hmip : s.regs.get? Register.mip = some 0)
    (hmie : s.regs.get? Register.mie = some 0)
    (hhs : s.regs.get? Register.hart_state = some (.HART_ACTIVE ()))
    (help : s.regs.get? Register.elp = some (landing_pad_bits_backwards .NO_LP_EXPECTED))
    (rs : List PMA_Region) (hs : MemReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc) (hal : pc.toNat % 4 = 0)
    (r1 r2 : PMA_Region) (hm1 : matching_pma_region rs (.Physaddr pc) 2 = some r1)
    (hm2 : matching_pma_region rs (.Physaddr (pc + 2)) 2 = some r2)
    (hx1 : r1.attributes.executable = true) (hx2 : r2.attributes.executable = true)
    (b0 b1 b2 b3 : BitVec 8)
    (h0 : s.mem.get? pc.toNat = some b0) (h1 : s.mem.get? (pc.toNat + 1) = some b1)
    (h2 : s.mem.get? (pc.toNat + 2) = some b2) (h3 : s.mem.get? (pc.toNat + 3) = some b3)
    (hc : b0.extractLsb 1 0 = 3#2) (op : rop)
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
      z'.regs.get? _root_.Zisk.Register.PC = some (pc + 4) := by
  intro rd rs1 rs2
  have e1 : rs1.toNat ≠ 0 := hr1
  have e2 : rs2.toNat ≠ 0 := hr2
  have ed : rd.toNat ≠ 0 := hrd
  have l1 := rs1.lt; have l2 := rs2.lt; have ld := rd.lt
  have a1 := agree rs1.toNat (by omega) l1
  have a2 := agree rs2.toNat (by omega) l2
  have hrd5 : (b3 ++ b2 ++ (b1 ++ b0)).extractLsb 11 7 ≠ 0#5 := by
    intro e; apply hrd; show (BitVec.extractLsb 11 7 (b3 ++ b2 ++ (b1 ++ b0))).toNat = 0; rw [e]; rfl
  obtain ⟨s', hrun, -, -, hpc', hxs', -⟩ :=
    try_step_rtype s h hmip hmie hhs help rs hs pc hpc hal r1 r2 hm1 hm2 hx1 hx2 b0 b1 b2 b3 h0 h1 h2 h3 hc op
      hop hf3 hf7 hrd5 _ _ a1 a2 n
  obtain ⟨-, hag⟩ := rtype_regs s xs op rs1 rs2 rd e1 e2 ed agree
  refine ⟨s', _, vectorUpdate xs rd.toNat
      (_root_.Zisk.Functions.zisk_exec (ziskOp op) xs[(rs1.toNat : Int)]! xs[(rs2.toNat : Int)]!).1, hrun,
    Equiv.Zisk.execute_regop z (ziskOp op) (pureOp op) rd.toNat rs1.toNat rs2.toNat
      (by omega) (by omega) (by omega) xs pc st hx hzpc hst,
    by simp [Std.ExtDHashMap.get?_insert], ?_, hpc', by simp⟩
  intro k hk1 hk2
  rw [hxs' k hk2]
  have := hag k hk1 hk2
  simpa only [ziskVal, getElem!_int] using this

end Equiv.Sim
