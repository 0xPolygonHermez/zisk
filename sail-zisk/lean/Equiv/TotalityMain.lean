/-
  The theorem: a program in the contract, run from the start on ZisK's
  RISC-V machine and, transpiled, on sail-zisk, either runs covered as far
  as one likes, sail-zisk keeping pace with the machine (`Rel`); or both
  exit, with the same code (and, on 0, the same memory); or both fail; or
  the program breaks one of the contract's preconditions, at a step after
  covered ones. The machine's own error (a step it cannot take) stands for
  its exit or failure throughout, as in Final.lean.
-/
import Equiv.TotalityRun
import Equiv.FinalProgram

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-! ## The outcomes -/

/-- The machine's run from S ends in `exit c`, its output `out`, or in its
    own error. -/
def MachineExits (S : ZiskRiscv.State model) (c : BitVec 64) (out : List (BitVec 64)) : Prop :=
  (∃ K Sk j, (∀ fuel, ZiskRiscv.run (K + fuel) S = .ok (Sk, .exit c, j)) ∧ ZiskRiscv.output Sk = out) ∨
    ∃ K msg, ∀ fuel, ZiskRiscv.run (K + fuel) S = .error msg

/-- The machine's run from S ends in failure, or in its own error. -/
def MachineFails (S : ZiskRiscv.State model) : Prop :=
  (∃ K Sk m j, ∀ fuel, ZiskRiscv.run (K + fuel) S = .ok (Sk, .failure m, j)) ∨
    ∃ K msg, ∀ fuel, ZiskRiscv.run (K + fuel) S = .error msg

/-- sail-zisk's run from z ends with code c: on 0 in success, memory m (but
    the input's first 8 bytes, which are not memory); else in error with c. -/
def ZiskExits (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (z : Equiv.Zisk.ZState) (c : BitVec 64) (m : Mem) :
    Prop :=
  ∃ z', ZEnds zrom z z' ∧
    (c = 0 → z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn m) ∧
    (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c)

/-- sail-zisk's run from z ends in error, or reaches a step that fails. -/
def ZiskFails (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (z : Equiv.Zisk.ZState) : Prop :=
  (∃ z', ZEnds zrom z z' ∧ z'.regs.get? .Zerror = some true) ∨
    ∃ n z1 e z', ZRun zrom n z z1 ∧ (zstep zrom).run z1 = .error e z'

/-- A precondition broken at the run's step n, S its state: J, one at the
    word at pc, I by a ZisK call, or marchid's cell changed. -/
def PreBroken (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (n : Nat)
    (S : ZiskRiscv.State model) : Prop :=
  BreakJ C S.rv ∨
  (∃ pc w, S.rv.regs.get? Register.PC = some pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some w ∧
    Broken S.rv pc w) ∨
  BreakIC S ∨ BreakCell zrom C n S

/-! ## The theorem -/

set_option maxHeartbeats 4000000 in
/-- A program in the contract — its code segments `segs` in the ROM
    (`tailOk`), loaded in m0 with ZisK's regions defined (`Full`), started as
    the BIOS starts it, the machine on the sparse memory the ELF gives
    (`s0'`), sail-zisk on the ROM the transpiler builds: the run is covered
    as far as one likes, sail-zisk keeping pace; or the machine and
    sail-zisk exit with the same code; or both fail; or the program breaks
    a precondition, its steps before covered. -/
theorem zisk_total (entry : BitVec 64) (segs : List (BitVec 64 × Array (BitVec 32))) (m0 : Mem)
    (hp : ProgramOk segs (CodeAt segs) m0) (ht : Transpiler.tailOk segs = true)
    (hcx : ∀ pc, CodeAt segs pc → ExecAt ziskRegions pc) (hfull : Full m0)
    (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, CodeAt segs pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    (∀ N, ∃ k, N ≤ k ∧ Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k ∧
      ∃ z' n, ZRun (zromOf segs entry) n z0 z' ∧ Rel (sIter 0 k ⟨s0, fc⟩).rv z' ziskRegions) ∨
    (∃ k c, MachineExits ⟨s0', fc⟩ c (ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∧
      ZiskExits (zromOf segs entry) z0 c (sIter 0 k ⟨s0, fc⟩).rv.mem) ∨
    (MachineFails ⟨s0', fc⟩ ∧ ZiskFails (zromOf segs entry) z0) ∨
    ∃ j, Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ j ∧
      PreBroken (zromOf segs entry) (CodeAt segs) j (sIter 0 j ⟨s0, fc⟩) := by
  obtain ⟨hb, hspec, -⟩ := hp.rom entry
  have hnx := noCodeRam_of_exec hcx
  have hni := noCodeIn_of_exec hcx
  by_cases hinf : ∀ N, ∃ k, N ≤ k ∧ Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k
  · refine .inl fun N => ?_
    obtain ⟨k, hk, hc⟩ := hinf N
    obtain ⟨z', n, hzr, hrel, -⟩ := covered_run (zromOf segs entry) entry hb ziskRegions exit_not_exec
      trap_not_exec (CodeAt segs) hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ k hc.1 hc.2.1
    exact ⟨k, hk, hc, z', n, hzr, hrel⟩
  obtain ⟨N, hN⟩ := Classical.not_forall.1 hinf
  rcases run_cases entry segs m0 hp ht hcx hfull s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir N with
    hc | ⟨j, hcj, hst⟩
  · exact absurd hc hN
  have hrun := hcj.1
  have hin := hcj.2.1
  have fails : ∀ {K i : Nat},
      ((∃ Sk m, ∀ fuel, ZiskRiscv.run (K + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, i)) ∨
        ∃ msg, ∀ fuel, ZiskRiscv.run (K + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) →
      MachineFails ⟨s0', fc⟩ := by
    intro K i h
    rcases h with ⟨Sk, m, h⟩ | ⟨msg, h⟩
    · exact .inl ⟨K, Sk, m, _, h⟩
    · exact .inr ⟨K, msg, h⟩
  rcases hst with hret | (⟨pc, hpc, hpcc, hw, h17⟩ | ⟨pc, w, c, hpc, hpcc, hrw, hd, hC4, hC8, hfail⟩ | hfa |
    hha | ⟨pc, w, why, hpc, hpcc, hrw, hd, hdesc, hC⟩ | hmj | ⟨pc, i, hpc, hi, ht0, ht1⟩) | hbr
  · -- the return to the initial ra
    obtain ⟨hm, hz⟩ := zisk_return_prog entry _ hcx m0 segs hp hfull s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _
      j hrun hin hret s0' hsp hcode
    refine .inr (.inl ⟨j, 0, ?_, ?_⟩)
    · rcases hm with ⟨Sk, h, ho⟩ | ⟨msg, h⟩
      · exact .inl ⟨j + 1, Sk, j, h, ho⟩
      · exact .inr ⟨j + 1, msg, h⟩
    · obtain ⟨z', hz1, hz2, hz3⟩ := hz
      exact ⟨z', hz1, fun _ => ⟨hz2, hz3⟩, fun h => absurd rfl h⟩
  · -- the exit system call
    obtain ⟨z', hrel, -, -, hro⟩ := covered_run_ram (zromOf segs entry) entry hb ziskRegions exit_not_exec
      trap_not_exec (CodeAt segs) hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j hrun hin
    obtain ⟨c, h10⟩ := getX_ready hrel.ready 10
    have hw0 : ZiskRiscv.readWord s0'.mem pc.toNat = some ECALL := by
      obtain ⟨w', hw'⟩ := hcode pc hpcc
      have h1 := readWord_ext hsp hw'
      rw [code_word hnx hni hs0.mem (RamOnlyX.refl _) hpcc] at h1
      rw [code_word hnx hni hs0.mem hro hpcc, h1] at hw
      rw [hw'] ; exact hw
    obtain ⟨hm, z', hz1, hz2, hz3⟩ := zisk_exit_prog entry _ hcx m0 segs hp hfull s0 hs0 z0 hzx hzpc hzst hze hzm
      fc hzf hmir _ j hrun hin pc hpc hpcc hw h17 c h10 s0' hsp hcode hw0
    refine .inr (.inl ⟨j, c, ?_, z', hz1, hz2, hz3⟩)
    rcases hm with ⟨Sk, h, ho⟩ | ⟨msg, h⟩
    · exact .inl ⟨j + 1, Sk, j, h, ho⟩
    · exact .inr ⟨j + 1, msg, h⟩
  · -- a ZisK call that fails
    obtain ⟨hm, n, z1, e, z', hz1, hz2⟩ := zisk_fail_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc
      hzf hmir _ j hrun hin pc hpc hpcc w hrw c hd hC4 hC8 hfail s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, .inr ⟨n, z1, e, z', hz1, hz2⟩⟩))
  · -- a load or a store neither side may make
    obtain ⟨hm, n, z1, e, z', hz1, hz2⟩ := zisk_fault_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm
      fc hzf hmir _ j hrun hin hfa s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, .inr ⟨n, z1, e, z', hz1, hz2⟩⟩))
  · -- a word the transpiler halts on
    obtain ⟨hm, hz⟩ := zisk_halt_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j hrun hin
      hha s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, .inl hz⟩))
  · -- a sequence the transpiler refuses
    have h4 : (pc + 4).toNat = pc.toNat + 4 := by
      have := exec_rom (hcx pc hpcc); rw [BitVec.toNat_add]; simp; omega
    have h8 : (pc + 8).toNat = pc.toNat + 8 := by
      have := exec_rom (hcx pc hpcc); rw [BitVec.toNat_add]; simp; omega
    obtain ⟨hm, hz⟩ := zisk_malformed_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j
      hrun hin pc hpc hpcc w hrw h4 h8 why hd hdesc hC s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, .inl hz⟩))
  · -- a jump to a target 2 mod 4
    obtain ⟨hm, hz⟩ := zisk_misjump_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j hrun
      hin hmj s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, .inl hz⟩))
  · -- the fused pair to a target 2 mod 4
    obtain ⟨hm, hz⟩ := zisk_fusedmis_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j
      hrun hin pc hpc i hi ⟨ht0, ht1⟩ s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, .inl hz⟩))
  · exact .inr (.inr (.inr ⟨j, hcj, hbr⟩))

end Equiv.Final
