/-
  The theorem, as far as it goes. A program started as the contract says,
  on memory as ZisK starts it; a ZisK program that holds the BIOS and, at
  every executable word the spec describes, the spec's translation of it
  (`SpecRom`, a hypothesis on the program, with no code in the RAM); and a
  run the simulation covers (`RunOkT`, stated with any ZisK program: of the
  RISC-V side, every step an instruction proved, with its accesses good;
  the run is the machine's, `rvIter`, lr and sc run its own way).
  Then ZisK's RISC-V machine and sail-zisk running the program with the
  BIOS end the same way: the exit system call with code c, success with
  the same memory — so the same output — when c is 0, the error with code
  c otherwise; or the return to the initial ra, success with the same
  memory.
-/
import Equiv.Full
import Equiv.Machine
import Equiv.SpecRom

namespace Equiv.Top

open Equiv.Riscv Equiv.Sim Equiv.Machine
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-! ## Memory stays full -/

theorem foldl_insert_grows (l : List (Nat × BitVec 8)) :
    ∀ m : Mem, Grows m (l.foldl (fun m p => m.insert p.1 p.2) m) := by
  induction l with
  | nil => intro m; exact Grows.refl _
  | cons p l ih => intro m; exact (Grows.insert m p.1 p.2).trans (ih _)

theorem memWrite_grows {n : Nat} (m : Mem) (a : Nat) (v : BitVec (8 * n)) : Grows m (memWrite m a v) :=
  foldl_insert_grows _ m

theorem startMem_full {m : Mem} (h : Full m) : Full (startMem m) :=
  h.grow ((memWrite_grows _ _ _).trans (memWrite_grows _ _ _))

theorem RVRun.grows {n k : Nat} {s s' : RVState} (h : RVRun n k s s') : Grows s.mem s'.mem := by
  induction h with
  | done => exact Grows.refl _
  | step h1 _ ih => exact (interp_grows _ _ _ _ h1).trans ih

/-! ## The start -/

/-- From the contract's start and ZisK's, the covered run on both sides:
    the machine's steps, and sail-zisk's from the BIOS's start, to related
    states. -/
theorem covered_run (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkT rs zr k s0 0) (hin : RunIn C k s0 0) :
    ∃ z' n, ZRun zrom n z0 z' ∧ Rel (rvIter 0 k s0) z' rs ∧
      SpecRom zrom C (rvIter 0 k s0).mem ∧ Grows s0.mem (rvIter 0 k s0).mem ∧
      (PcOk (rvIter 0 k s0) → MRunR C k ⟨s0, fc⟩ ⟨rvIter 0 k s0, fc⟩) := by
  obtain ⟨st, hst⟩ := hzst
  obtain ⟨z1, r1, hrel1⟩ := start_rel zrom entry hb m0 z0 st hzx hzpc hst hze hzm s0 rs hs0
  have hs : SpecRom zrom C s0.mem := by
    rw [hs0.mem]
    exact hspec.keep ((memWrite_ramOnly _ _ _ arch_writable.1).trans (memWrite_ramOnly _ _ _ mtvec_writable.1)) hnx
  obtain ⟨z', k', hzr, hrel, hro, hg, hm⟩ := prog_rom zr zrom rs C hx ht hnx k ⟨s0, fc⟩ z1 0 hrel1 hrun hin hs
  exact ⟨z', 6 + k', ZRun.append r1 hzr, hrel, hs.keep hro hnx, hg, hm⟩

/-! ## The ends -/

/-- The exit system call with code c: the machine's run ends in `exit c`;
    sail-zisk's in success with the same memory when c is 0, else in error
    with code c. -/
theorem exit_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkT rs zr k s0 0) (hin : RunIn C k s0 0)
    (pc : BitVec 64) (hpc : (rvIter 0 k s0).regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (hw : ZiskRiscv.readWord (rvIter 0 k s0).mem pc.toNat = some ECALL)
    (h17 : getX (rvIter 0 k s0) 17 = some 93) (c : BitVec 64) (h10 : getX (rvIter 0 k s0) 10 = some c) :
    (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) =
        .ok (⟨rvIter 0 k s0, fc⟩, .exit c, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn (rvIter 0 k s0).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) := by
  obtain ⟨z1, n1, hz1, hrel, hsp, hg, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k hrun hin
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  have hrom : zrom pc = Transpiler.ecall :=
    (hsp pc ECALL hpcc hw ⟨0, _, rfl⟩).one (fun _ => rfl)
  have hfull' : Full (rvIter 0 k s0).mem := by
    have := startMem_full hfull
    rw [← hs0.mem] at this
    exact this.grow hg
  refine ⟨fun fuel => ?_, ?_⟩
  · have hg : model.getPC (rvIter 0 k s0) = pc := by simp [model, hpc]
    have he := step_exit (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model)
      (by rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hpx))
      (by rw [hg]; exact hw) (by simp [model, h17])
    have hc : model.getX (rvIter 0 k s0) 10 = c := by simp [model, h10]
    rw [hc] at he
    exact run_of hmr.toMRun he (by simp) fuel
  · by_cases h0 : c = 0
    · subst h0
      obtain ⟨z', he, hz, hzm'⟩ := rel_exit_ok zrom entry hb _ z1 rs hrel pc hpc hrom h17 h10
        hfull'.outBytes hfull'.inBytes
      exact ⟨z', ZEnds.of_run hz1 he, fun _ => ⟨hz, hzm'⟩, fun h => absurd rfl h⟩
    · obtain ⟨z', he, hz, hzc⟩ := rel_exit_fail zrom entry hb _ z1 rs hrel pc hpc hrom h17 c h10 h0
      exact ⟨z', ZEnds.of_run hz1 he, fun h => absurd h h0, fun _ => ⟨hz, hzc⟩⟩

/-- The return to the initial ra: the machine's run ends in `exit 0`;
    sail-zisk's in success with the same memory. -/
theorem return_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkT rs zr k s0 0) (hin : RunIn C k s0 0)
    (hpc : (rvIter 0 k s0).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK) :
    (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) =
        .ok (⟨rvIter 0 k s0, fc⟩, .exit 0, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn (rvIter 0 k s0).mem := by
  obtain ⟨z1, n1, hz1, hrel, hsp, hg, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k hrun hin
  have hmr := hm ⟨_, hpc, by decide⟩
  have hfull' : Full (rvIter 0 k s0).mem := by
    have := startMem_full hfull
    rw [← hs0.mem] at this
    exact this.grow hg
  refine ⟨fun fuel => ?_, ?_⟩
  · have he := step_return (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model) (by simp [model, hpc]; rfl)
    exact run_of hmr.toMRun he (by simp) fuel
  · obtain ⟨z', he, hz, hzm'⟩ := rel_return_end zrom entry hb _ z1 rs hrel hpc
      hfull'.outBytes hfull'.inBytes
    exact ⟨z', ZEnds.of_run hz1 he, hz, hzm'⟩

end Equiv.Top
