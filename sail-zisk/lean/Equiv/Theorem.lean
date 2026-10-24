/-
  The theorem, as far as it goes: for a program whose run the simulation
  covers (`RunOkT`: every step an instruction proved, the ZisK program the
  spec's translation at it), started as the contract says on memory as ZisK
  starts it, ZisK's RISC-V machine and sail-zisk running the program with
  the BIOS end the same way: the exit system call with code c, success with
  the same memory — so the same output — when c is 0, the error with code c
  otherwise; or the return to the initial ra, success with the same memory.
-/
import Equiv.Full
import Equiv.Machine

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

/-! ## The state a run reaches -/

/-- The state k steps of sail-riscv's main loop reach from s, from step n. -/
noncomputable def rvIter : Nat → Nat → RVState → RVState
  | _, 0, s => s
  | n, k + 1, s => rvIter (n + 1) k (rvNext n s)

theorem RVRun.iter {n k : Nat} {s s' : RVState} (h : RVRun n k s s') : s' = rvIter n k s := by
  induction h with
  | done => rfl
  | step h1 _ ih => rw [ih, rvIter, rvNext_of h1]

/-! ## The start -/

/-- From the contract's start and ZisK's, the covered run on both sides:
    the machine's steps, and sail-zisk's from the BIOS's start, to related
    states. -/
theorem covered_run (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (m0 : Mem) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (k : Nat) (hrun : RunOkT rs zrom k s0 0) :
    ∃ z' n, RVRun 0 k s0 (rvIter 0 k s0) ∧ ZRun zrom n z0 z' ∧ Rel (rvIter 0 k s0) z' rs ∧
      (PcOk (rvIter 0 k s0) → MRun k ⟨s0, fc⟩ ⟨rvIter 0 k s0, fc⟩) := by
  obtain ⟨st, hst⟩ := hzst
  obtain ⟨z1, r1, hrel1⟩ := start_rel zrom entry hb m0 z0 st hzx hzpc hst hze hzm s0 rs hs0
  obtain ⟨s', z', k', hrv, hzr, hrel, hm⟩ := prog_machine zrom rs hx ht k ⟨s0, fc⟩ z1 0 hrel1 hrun
  have e := RVRun.iter hrv
  subst e
  exact ⟨z', 6 + k', hrv, ZRun.append r1 hzr, hrel, hm⟩

/-! ## The ends -/

/-- The exit system call with code c: the machine's run ends in `exit c`;
    sail-zisk's in success with the same memory when c is 0, else in error
    with code c. -/
theorem exit_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (m0 : Mem) (hfull : Full m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (k : Nat) (hrun : RunOkT rs zrom k s0 0)
    (pc : BitVec 64) (hpc : (rvIter 0 k s0).regs.get? Register.PC = some pc) (hpx : ExecAt rs pc)
    (hw : ZiskRiscv.readWord (rvIter 0 k s0).mem pc.toNat = some ECALL) (hrom : zrom pc = Transpiler.ecall)
    (h17 : getX (rvIter 0 k s0) 17 = some 93) (c : BitVec 64) (h10 : getX (rvIter 0 k s0) 10 = some c) :
    (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) =
        .ok (⟨rvIter 0 k s0, fc⟩, .exit c, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ z'.mem = (rvIter 0 k s0).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) := by
  obtain ⟨z1, n1, hrv, hz1, hrel, hm⟩ :=
    covered_run zrom entry hb rs hx ht m0 s0 hs0 z0 hzx hzpc hzst hze hzm fc k hrun
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  have hfull' : Full (rvIter 0 k s0).mem := by
    have := startMem_full hfull
    rw [← hs0.mem] at this
    exact this.grow (RVRun.grows hrv)
  refine ⟨fun fuel => ?_, ?_⟩
  · have hg : model.getPC (rvIter 0 k s0) = pc := by simp [model, hpc]
    have he := step_exit (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model)
      (by rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hpx))
      (by rw [hg]; exact hw) (by simp [model, h17])
    have hc : model.getX (rvIter 0 k s0) 10 = c := by simp [model, h10]
    rw [hc] at he
    exact run_of hmr he (by simp) fuel
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
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (m0 : Mem) (hfull : Full m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (k : Nat) (hrun : RunOkT rs zrom k s0 0)
    (hpc : (rvIter 0 k s0).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK) :
    (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) =
        .ok (⟨rvIter 0 k s0, fc⟩, .exit 0, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = (rvIter 0 k s0).mem := by
  obtain ⟨z1, n1, hrv, hz1, hrel, hm⟩ :=
    covered_run zrom entry hb rs hx ht m0 s0 hs0 z0 hzx hzpc hzst hze hzm fc k hrun
  have hmr := hm ⟨_, hpc, by decide⟩
  have hfull' : Full (rvIter 0 k s0).mem := by
    have := startMem_full hfull
    rw [← hs0.mem] at this
    exact this.grow (RVRun.grows hrv)
  refine ⟨fun fuel => ?_, ?_⟩
  · have he := step_return (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model) (by simp [model, hpc]; rfl)
    exact run_of hmr he (by simp) fuel
  · obtain ⟨z', he, hz, hzm'⟩ := rel_return_end zrom entry hb _ z1 rs hrel hpc
      hfull'.outBytes hfull'.inBytes
    exact ⟨z', ZEnds.of_run hz1 he, hz, hzm'⟩

end Equiv.Top
