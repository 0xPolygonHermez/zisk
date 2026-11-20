/-
  The theorem, ZisK's memory without the execute-only bytes (precondition
  X): as `zisk_total`, ZisK started on σ0, m0 without X's bytes; its runs
  there are its runs on m0, or a step of the machine's run breaks X.
-/
import Equiv.RunX

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- sail-zisk's run from σ ends with code c: on 0 in success, memory m
    without X's bytes (but the input's first 8, which are not memory); else
    in error with c. -/
def ZiskExitsX (X : Nat → Bool) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (σ : Equiv.Zisk.ZState) (c : BitVec 64)
    (m : Mem) : Prop :=
  ∃ σ', ZEnds zrom σ σ' ∧
    (c = 0 → σ'.regs.get? .Zerror = some false ∧ Equiv.normIn σ'.mem = eraseX X (Equiv.normIn m)) ∧
    (c ≠ 0 → σ'.regs.get? .Zerror = some true ∧ σ'.regs.get? .Zerror_code = some c)

set_option maxHeartbeats 8000000 in
theorem zisk_totalX (entry : BitVec 64) (segs : List (BitVec 64 × Array (BitVec 32))) (m0 : Mem)
    (hp : ProgramOk segs (CodeAt segs) m0) (ht : Transpiler.tailOk segs = true)
    (hcx : ∀ pc, CodeAt segs pc → ExecAt ziskRegions pc) (hfull : Full m0)
    (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, CodeAt segs pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w)
    {X : Nat → Bool} (hX : RomX X) (σ0 : Equiv.Zisk.ZState) (hσ0 : FRX X σ0 z0) :
    (∀ N, ∃ k, N ≤ k ∧ Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k ∧
      ∃ z' n σ', ZRun (zromOf segs entry) n z0 z' ∧ Rel (sIter 0 k ⟨s0, fc⟩).rv z' ziskRegions ∧
        ZRun (zromOf segs entry) n σ0 σ' ∧ FRX X σ' z') ∨
    (∃ k c, MachineExits ⟨s0', fc⟩ c (ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∧
      ZiskExitsX X (zromOf segs entry) σ0 c (sIter 0 k ⟨s0, fc⟩).rv.mem) ∨
    (MachineFails ⟨s0', fc⟩ ∧ ZiskFails (zromOf segs entry) σ0) ∨
    (∃ j, Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ j ∧
      PreBroken (zromOf segs entry) (CodeAt segs) j (sIter 0 j ⟨s0, fc⟩)) ∨
    ∃ j, RunOkM ziskRegions (zromOf segs entry) (CodeAt segs) j ⟨s0, fc⟩ 0 ∧ RunInM (CodeAt segs) j ⟨s0, fc⟩ 0 ∧
      BreakXS X (sIter 0 j ⟨s0, fc⟩) := by
  obtain ⟨hb, hspec, -⟩ := hp.rom entry
  have hnx := noCodeRam_of_exec hcx
  have hni := noCodeIn_of_exec hcx
  by_cases hbx : ∃ j, RunOkM ziskRegions (zromOf segs entry) (CodeAt segs) j ⟨s0, fc⟩ 0 ∧
      RunInM (CodeAt segs) j ⟨s0, fc⟩ 0 ∧ BreakXS X (sIter 0 j ⟨s0, fc⟩)
  · exact .inr (.inr (.inr (.inr hbx)))
  -- no step breaks X: ZisK's run on σ0 follows its run on z0
  have follow : ∀ k, RunOkM ziskRegions (zromOf segs entry) (CodeAt segs) k ⟨s0, fc⟩ 0 →
      RunInM (CodeAt segs) k ⟨s0, fc⟩ 0 →
      ∃ z' n σ', ZRun (zromOf segs entry) n z0 z' ∧ Rel (sIter 0 k ⟨s0, fc⟩).rv z' ziskRegions ∧
        SpecRom (zromOf segs entry) (CodeAt segs) (sIter 0 k ⟨s0, fc⟩).rv.mem ∧
        Grows s0.mem (sIter 0 k ⟨s0, fc⟩).rv.mem ∧ ZRun (zromOf segs entry) n σ0 σ' ∧ FRX X σ' z' := by
    intro k hc1 hc2
    obtain ⟨z', n, hzr, hrel, -, hsp, hg, -, hσ⟩ := covered_runX (zromOf segs entry) entry hb ziskRegions
      exit_not_exec trap_not_exec (CodeAt segs) hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ k
      hc1 hc2 hX
    rcases hσ σ0 hσ0 with ⟨σ', hr', h'⟩ | ⟨j, hj, hok, hbr⟩
    · exact ⟨z', n, σ', hzr, hrel, hsp, hg, hr', h'⟩
    · exact absurd ⟨j, hok, fun i hi => hc2 i (by omega), hbr⟩ hbx
  by_cases hinf : ∀ N, ∃ k, N ≤ k ∧ Covered (zromOf segs entry) (CodeAt segs) ⟨s0, fc⟩ k
  · refine .inl fun N => ?_
    obtain ⟨k, hk, hc⟩ := hinf N
    obtain ⟨z', n, σ', hzr, hrel, -, -, hr', h'⟩ := follow k hc.1 hc.2.1
    exact ⟨k, hk, hc, z', n, σ', hzr, hrel, hr', h'⟩
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
    · obtain ⟨z1, n1, σ1, hz1, hrel1, -, hg1, hr1, h1⟩ := follow j hrun hin
      have hfull' : Full (sIter 0 j ⟨s0, fc⟩).rv.mem := by
        have := startMem_full hfull
        rw [← hs0.mem] at this
        exact this.grow hg1
      obtain ⟨σ', he, hz2, hz3⟩ := rel_return_end_x hX _ entry hb hrel1 h1 hret hfull'.outBytes hfull'.inBytes
      exact ⟨σ', ZEnds.of_run hr1 he, fun _ => ⟨hz2, hz3⟩, fun h => absurd rfl h⟩
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
    refine .inr (.inl ⟨j, c, ?_, ?_⟩)
    · rcases hm with ⟨Sk, h, ho⟩ | ⟨msg, h⟩
      · exact .inl ⟨j + 1, Sk, j, h, ho⟩
      · exact .inr ⟨j + 1, msg, h⟩
    · obtain ⟨z1, n1, σ1, hz1', hrel1, hsp1, hg1, hr1, h1⟩ := follow j hrun hin
      have hfull' : Full (sIter 0 j ⟨s0, fc⟩).rv.mem := by
        have := startMem_full hfull
        rw [← hs0.mem] at this
        exact this.grow hg1
      have hrom : zromOf segs entry pc = Transpiler.ecall := (hsp1 pc ECALL hpcc hw ⟨0, _, rfl⟩).one (fun _ => rfl)
      by_cases h0 : c = 0
      · subst h0
        obtain ⟨σ', he, hz2', hz3'⟩ := rel_exit_ok_x hX _ entry hb hrel1 h1 pc hpc hrom h17 h10
          hfull'.outBytes hfull'.inBytes
        exact ⟨σ', ZEnds.of_run hr1 he, fun _ => ⟨hz2', hz3'⟩, fun h => absurd rfl h⟩
      · obtain ⟨σ', he, hz2', hzc⟩ := rel_exit_fail_x (X := X) _ entry hb hrel1 h1 pc hpc hrom h17 c h10 h0
        exact ⟨σ', ZEnds.of_run hr1 he, fun h => absurd h h0, fun _ => ⟨hz2', hzc⟩⟩
  · -- a ZisK call that fails
    obtain ⟨hm, n, z1, e, z', hz1, hz2⟩ := zisk_fail_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc
      hzf hmir _ j hrun hin pc hpc hpcc w hrw c hd hC4 hC8 hfail s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, ziskFails_x hX hσ0 (.inr ⟨n, z1, e, z', hz1, hz2⟩)⟩))
  · -- a load or a store neither side may make
    obtain ⟨hm, n, z1, e, z', hz1, hz2⟩ := zisk_fault_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm
      fc hzf hmir _ j hrun hin hfa s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, ziskFails_x hX hσ0 (.inr ⟨n, z1, e, z', hz1, hz2⟩)⟩))
  · -- a word the transpiler halts on
    obtain ⟨hm, hz⟩ := zisk_halt_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j hrun hin
      hha s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, ziskFails_x hX hσ0 (.inl hz)⟩))
  · -- a sequence the transpiler refuses
    have h4 : (pc + 4).toNat = pc.toNat + 4 := by
      have := exec_rom (hcx pc hpcc); rw [BitVec.toNat_add]; simp; omega
    have h8 : (pc + 8).toNat = pc.toNat + 8 := by
      have := exec_rom (hcx pc hpcc); rw [BitVec.toNat_add]; simp; omega
    obtain ⟨hm, hz⟩ := zisk_malformed_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j
      hrun hin pc hpc hpcc w hrw h4 h8 why hd hdesc hC s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, ziskFails_x hX hσ0 (.inl hz)⟩))
  · -- a jump to a target 2 mod 4
    obtain ⟨hm, hz⟩ := zisk_misjump_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j hrun
      hin hmj s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, ziskFails_x hX hσ0 (.inl hz)⟩))
  · -- the fused pair to a target 2 mod 4
    obtain ⟨hm, hz⟩ := zisk_fusedmis_prog entry _ hcx m0 segs hp s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir _ j
      hrun hin pc hpc i hi ⟨ht0, ht1⟩ s0' hsp hcode
    exact .inr (.inr (.inl ⟨fails hm, ziskFails_x hX hσ0 (.inl hz)⟩))
  · exact .inr (.inr (.inr (.inl ⟨j, hcj, hbr⟩)))

end Equiv.Final
