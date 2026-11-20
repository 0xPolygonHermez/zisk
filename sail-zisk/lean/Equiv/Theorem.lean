/-
  The theorem, as far as it goes. A program started as the contract says,
  on memory as ZisK starts it; a ZisK program that holds the BIOS and, at
  every executable word the spec describes, the spec's translation of it
  (`SpecRom`, a hypothesis on the program, with no code in the RAM); and a
  run the simulation covers (`RunOkM`, stated with any ZisK program: of the
  RISC-V side, every step an instruction proved, with its accesses good,
  or a ZisK call; the run is the machine's, `sIter`, lr and sc and the
  calls run its own way).
  Then ZisK's RISC-V machine and sail-zisk running the program with the
  BIOS end the same way: the exit system call with code c, success with
  the same memory — so the same output — when c is 0, the error with code
  c otherwise; or the return to the initial ra, success with the same
  memory.
-/
import Equiv.Full
import Equiv.Machine
import Equiv.SpecRom
import Equiv.RunM
import Equiv.FaultSim
import Equiv.EbreakSim
import Equiv.MalformedSim
import Equiv.MisJumpSim

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
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0) :
    ∃ z' n, ZRun zrom n z0 z' ∧ Rel ((sIter 0 k ⟨s0, fc⟩).rv) z' rs ∧
      Equiv.Zisk.FcallRel (sIter 0 k ⟨s0, fc⟩).fcall z' ∧
      SpecRom zrom C ((sIter 0 k ⟨s0, fc⟩).rv).mem ∧ Grows s0.mem ((sIter 0 k ⟨s0, fc⟩).rv).mem ∧
      (PcOk ((sIter 0 k ⟨s0, fc⟩).rv) → MRunR C k ⟨s0, fc⟩ (sIter 0 k ⟨s0, fc⟩)) := by
  obtain ⟨st, hst⟩ := hzst
  obtain ⟨z1, r1, hrel1, hk1⟩ := start_rel zrom entry hb m0 z0 st hzx hzpc hst hze hzm s0 rs hs0
  have hs : SpecRom zrom C s0.mem := by
    rw [hs0.mem]
    exact hspec.keep ((memWrite_ramOnly _ _ _ arch_writable.1).trans (memWrite_ramOnly _ _ _ mtvec_writable.1)) hnx
  have hmir0 : Mirror ⟨s0, fc⟩ := by
    show Equiv.memRead s0.mem 8 Equiv.IN_LO = some fc.freeInput
    rw [hs0.mem, startMem, mirror_ramOnly ((memWrite_ramOnly _ _ _ arch_writable.1).trans
      (memWrite_ramOnly _ _ _ mtvec_writable.1))]
    exact hmir
  obtain ⟨z', k', hzr, hrel, hfr, hro, hg, -, hm⟩ := prog_romM zr zrom rs C hx ht hnx hni k ⟨s0, fc⟩ z1 0 hrel1
    (fun r hr => (hk1 r hr).trans (hzf r hr)) hmir0 hrun hin hs
  exact ⟨z', 6 + k', ZRun.append r1 hzr, hrel, hfr, hs.keepX hro hnx hni, hg, hm⟩

/-! ## The ends -/

/-- The exit system call with code c: the machine's run ends in `exit c`;
    sail-zisk's in success with the same memory when c is 0, else in error
    with code c. -/
theorem exit_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (hw : ZiskRiscv.readWord ((sIter 0 k ⟨s0, fc⟩).rv).mem pc.toNat = some ECALL)
    (h17 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 17 = some 93) (c : BitVec 64) (h10 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 10 = some c) :
    (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) =
        .ok ((sIter 0 k ⟨s0, fc⟩), .exit c, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) := by
  obtain ⟨z1, n1, hz1, hrel, -, hsp, hg, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  have hrom : zrom pc = Transpiler.ecall :=
    (hsp pc ECALL hpcc hw ⟨0, _, rfl⟩).one (fun _ => rfl)
  have hfull' : Full T.rv.mem := by
    have := startMem_full hfull
    rw [← hs0.mem] at this
    exact this.grow hg
  refine ⟨fun fuel => ?_, ?_⟩
  · have hg : model.getPC T.rv = pc := by simp [model, hpc]
    have he := step_exit T
      (by rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hpx))
      (by rw [hg]; exact hw) (by simp [model, h17])
    have hc : model.getX T.rv 10 = c := by simp [model, h10]
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
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK) :
    (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) =
        .ok ((sIter 0 k ⟨s0, fc⟩), .exit 0, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem := by
  obtain ⟨z1, n1, hz1, hrel, -, hsp, hg, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm ⟨_, hpc, by decide⟩
  have hfull' : Full T.rv.mem := by
    have := startMem_full hfull
    rw [← hs0.mem] at this
    exact this.grow hg
  refine ⟨fun fuel => ?_, ?_⟩
  · have he := step_return T (by simp [model, hpc]; rfl)
    exact run_of hmr.toMRun he (by simp) fuel
  · obtain ⟨z', he, hz, hzm'⟩ := rel_return_end zrom entry hb _ z1 rs hrel hpc
      hfull'.outBytes hfull'.inBytes
    exact ⟨z', ZEnds.of_run hz1 he, hz, hzm'⟩

/-- A ZisK call whose operation fails (`CallFail`, at a call the spec
    decodes): the machine's run ends in failure, or in its own error;
    sail-zisk's reaches a step that fails. -/
theorem fail_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (w : BitVec 32) (hrw : ZiskRiscv.readWord (sIter 0 k ⟨s0, fc⟩).rv.mem pc.toNat = some w)
    (h4 : (pc + 4).toNat = pc.toNat + 4) (h8 : (pc + 8).toNat = pc.toNat + 8) (c : ZiskRiscv.Call)
    (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc)
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc)) pc.toNat = .call c)
    (hfail : CallFail (sIter 0 k ⟨s0, fc⟩) c) :
    ((∃ m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) =
        .ok (sIter 0 k ⟨s0, fc⟩, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ n z1 e z', ZRun zrom n z0 z1 ∧ (zstep zrom).run z1 = .error e z' := by
  obtain ⟨z1, n1, hz1, hrel, hfr, hsp, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  have htr := hsp pc w hpcc hrw ⟨0, _, convert_call3 hd 0⟩
  obtain ⟨hstep, ⟨k', z2, e, z', hzr, hze'⟩, -⟩ := callfail_m T z1 rs hrel hfr hx pc hpc zrom w hpx hrw htr h4 h8 c hd hfail
  refine ⟨?_, n1 + k', z2, e, z', ZRun.append hz1 hzr, hze'⟩
  rcases hstep with ⟨m, hs⟩ | ⟨msg, hs⟩
  · exact .inl ⟨m, fun fuel => run_of hmr.toMRun hs (by simp) fuel⟩
  · exact .inr ⟨msg, fun fuel => run_err_of hmr.toMRun hs fuel⟩

/-! ## An access neither side may make -/

/-- A load or a store at pc, any registers, at an address no PMA region
    permits and that ZisK's memory map refuses too. -/
def FaultAt (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (s : RVState) : Prop :=
  ∃ pc, s.regs.get? Register.PC = some pc ∧
    ((∃ i : LdInstr, LdtAt s rs pc zrom i (nextWord s pc) ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧
        AccFault rs (.Load .Data) (v1 + i.imm.signExtend 64) 8 ∧
        _root_.Zisk.Functions.readable (v1 + i.imm.signExtend 64) 8 = false) ∨
     (∃ i : LwInstr, LwtAt s rs pc zrom i (nextWord s pc) ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧
        AccFault rs (.Load .Data) (v1 + i.imm.signExtend 64) i.k.w ∧
        _root_.Zisk.Functions.readable (v1 + i.imm.signExtend 64) i.k.w = false) ∨
     (∃ i : SdInstr, SdtAt s rs pc zrom i (nextWord s pc) ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧
        AccFault rs (.Store .Data) (v1 + (i.immHi ++ i.immLo).signExtend 64) 8 ∧
        _root_.Zisk.Functions.writable (v1 + (i.immHi ++ i.immLo).signExtend 64) 8 = false) ∨
     (∃ i : SwInstr, SwtAt s rs pc zrom i (nextWord s pc) ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧
        AccFault rs (.Store .Data) (v1 + (i.immHi ++ i.immLo).signExtend 64) i.k.w ∧
        _root_.Zisk.Functions.writable (v1 + (i.immHi ++ i.immLo).signExtend 64) i.k.w = false))

/-- The faulting access, one step of each: sail-riscv's to the trap vector,
    a plain word at an executable pc; sail-zisk's failing. -/
theorem fault_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (h : FaultAt rs zrom s) (n : Nat) :
    ∃ pc, s.regs.get? Register.PC = some pc ∧ WordOk s.mem rs pc ∧
      (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
        s'.regs.get? Register.PC = some TRAP_VEC) ∧ ∃ e z', (zstep zrom).run z = .error e z' := by
  obtain ⟨pc, hpc, ⟨i, hi, v1, hv1, hf, hz⟩ | ⟨i, hi, v1, hv1, hf, hz⟩ | ⟨i, hi, v1, hv1, hf, hz⟩ |
    ⟨i, hi, v1, hv1, hf, hz⟩⟩ := h
  · obtain ⟨h1, h2⟩ := ldt_fault s z rs hrel pc hpc zrom i _ hi v1 hv1 hf hz n
    exact ⟨pc, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_ld i).plain⟩, h1, h2⟩
  · obtain ⟨h1, h2⟩ := lwt_fault s z rs hrel pc hpc zrom i _ hi v1 hv1 hf hz n
    exact ⟨pc, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_lw i).plain⟩, h1, h2⟩
  · obtain ⟨h1, h2⟩ := sdt_fault s z rs hrel pc hpc zrom i _ hi v1 hv1 hf hz n
    exact ⟨pc, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_sd i).plain⟩, h1, h2⟩
  · obtain ⟨h1, h2⟩ := swt_fault s z rs hrel pc hpc zrom i _ hi v1 hv1 hf hz n
    exact ⟨pc, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_sw i).plain⟩, h1, h2⟩

/-- A load or a store neither side may make (`FaultAt`): the machine's run
    ends in failure (sail-riscv's access fault, its trap); sail-zisk's
    reaches a step that fails. -/
theorem fault_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hfa : FaultAt rs zrom (sIter 0 k ⟨s0, fc⟩).rv) :
    (∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∧
    ∃ n z1 e z', ZRun zrom n z0 z1 ∧ (zstep zrom).run z1 = .error e z' := by
  obtain ⟨z1, n1, hz1, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨pc, hpc, hw, ⟨s', hrun', hpc'⟩, e, z', hze'⟩ := fault_step T.rv z1 rs hrel zrom hfa 0
  have hmr := hm (PcOk.of_exec ht hpc hw.1)
  obtain ⟨m, hs⟩ := mstep_trap T rs hrel.ready pc hpc hw hx hrun' hpc'
  exact ⟨⟨_, m, fun fuel => run_of hmr.toMRun hs (by simp) fuel⟩, n1, z1, e, z', hz1, hze'⟩

/-! ## A jump to a target 2 mod 4 -/

/-- A jump to a target 2 mod 4 (`MisJumpAt`): the machine's run ends in
    failure (sail-riscv traps, an instruction address misaligned); sail-zisk's
    takes the jump, to an address the transpiler puts no instruction at, and
    ends in error there (`HaltMis`). -/
theorem misjump_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hmj : MisJumpAt rs zrom (sIter 0 k ⟨s0, fc⟩).rv) (hmh : HaltMis zrom) :
    (∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  obtain ⟨z1, n1, hz1, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨pc, tgt, hpc, hw, ht0, ht1, ⟨s', hrun', hpc'⟩, k', z', hz', hst', hzt⟩ :=
    misjump_step T.rv z1 rs hrel zrom hmj 0
  have hmr := hm (PcOk.of_exec ht hpc hw.1)
  obtain ⟨m, hs⟩ := mstep_trap T rs hrel.ready pc hpc hw hx hrun' hpc'
  obtain ⟨z'', he, herr⟩ := misjump_halt hmh ht0 ht1 hzt hst'
  exact ⟨⟨_, m, fun fuel => run_of hmr.toMRun hs (by simp) fuel⟩, z'',
    ZEnds.of_run (ZRun.append hz1 hz') ⟨0, _, .done _, he⟩, herr⟩

/-- The fused auipc-jalr pair to a target 2 mod 4: the machine's run ends
    in failure a step later (the auipc, then sail-riscv's trap on the
    jalr); sail-zisk's takes its one instruction for the pair, to the
    target, and ends in error there (`HaltMis`). -/
theorem fusedmis_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (i : FInstr)
    (hi : FtAtM (sIter 0 k ⟨s0, fc⟩).rv rs pc zrom i) (htg : (i.T pc)[0] = false ∧ (i.T pc)[1] = true)
    (hmh : HaltMis zrom) :
    (∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 2 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k + 1)) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  obtain ⟨z1, n1, hz1, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨s1, hrun1, hr1, hpc1, hw1, hw2, ⟨s2, hrun2, hpc2⟩, z', hz', hst', hzt⟩ :=
    fusedmis_step T.rv z1 rs hrel pc hpc zrom i hi htg 0
  have hmr := hm (PcOk.of_exec ht hpc hi.x1)
  have hR := mstepR T rs hrel.ready pc hpc hw1 hx hrun1 (PcOk.of_exec ht hpc1 hi.x3)
  have hmr1 := hmr.append (.step hR (.done _))
  obtain ⟨m, hs⟩ := mstep_trap { T with rv := s1 } rs hr1 (pc + 4) hpc1 hw2 hx hrun2 hpc2
  obtain ⟨z'', he, herr⟩ := misjump_halt hmh htg.1 htg.2 hzt hst'
  exact ⟨⟨_, m, fun fuel => by
      rw [show k + 2 + fuel = (k + 1) + 1 + fuel by omega]; exact run_of hmr1.toMRun hs (by simp) fuel⟩,
    z'', ZEnds.of_run (ZRun.append hz1 (.step hz' (.done _))) ⟨0, _, .done _, he⟩, herr⟩

/-! ## A word the transpiler halts on -/

/-- `ebreak`, a write to a read-only CSR (`unimp` among them), or a word
    RISC-V rejects, at pc (`HaltWord`). -/
def HaltAt (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (s : RVState) : Prop :=
  ∃ pc e, s.regs.get? Register.PC = some pc ∧ HaltWord e ∧ WordsAt s rs pc zrom e (nextWord s pc)

/-- `ebreak`, a write to a read-only CSR, or a word RISC-V rejects: the
    machine's run ends in failure (sail-riscv traps, on the breakpoint or
    the illegal instruction); sail-zisk's ends in error, on the halt. -/
theorem halt_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hha : HaltAt rs zrom (sIter 0 k ⟨s0, fc⟩).rv) :
    (∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  obtain ⟨z1, n1, hz1, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨pc, e, hpc, he, hi⟩ := hha
  obtain ⟨⟨s', hrun', hpc'⟩, z', hze', hzerr⟩ := halt_step T.rv z1 rs hrel pc hpc zrom _ e he hi 0
  have hw : WordOk T.rv.mem rs pc := ⟨hi.x1, e, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, he.plain⟩
  have hmr := hm (PcOk.of_exec ht hpc hi.x1)
  obtain ⟨m, hs⟩ := mstep_trap T rs hrel.ready pc hpc hw hx hrun' hpc'
  exact ⟨⟨_, m, fun fuel => run_of hmr.toMRun hs (by simp) fuel⟩, z', ZEnds.of_run hz1 ⟨0, z1, .done _, hze'⟩,
    hzerr⟩

/-! ## What the transpiler refuses -/

/-- A ZisK call sequence the transpiler refuses, or a SYSTEM word of
    funct3 0 but `ecall` and `ebreak` (`malformed`), at a code
    address, that the spec describes: the machine's run ends in failure;
    sail-zisk's in error, on the halt. -/
theorem malformed_end (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (hmir : Equiv.memRead m0 8 Equiv.IN_LO = some fc.freeInput) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (w : BitVec 32) (hrw : ZiskRiscv.readWord (sIter 0 k ⟨s0, fc⟩).rv.mem pc.toNat = some w)
    (h4 : (pc + 4).toNat = pc.toNat + 4) (h8 : (pc + 8).toNat = pc.toNat + 8) (why : String)
    (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc)
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc)) pc.toNat = .malformed why)
    (hdesc : ∃ ia l, Transpiler.convert pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc) ia
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc) = some l) :
    (∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0, fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  obtain ⟨z1, n1, hz1, hrel, -, hsp, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf hmir zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  obtain ⟨ia, l, hl, hz⟩ := hsp pc w hpcc hrw hdesc
  have hrom : zrom pc = Transpiler.halt := by
    rw [convert_malformed3 hd hl] at hz
    exact hz _ (List.mem_singleton_self _)
  have hg : model.getPC T.rv = pc := by simp [model, hpc]
  have hdm : ZiskRiscv.decode (ZiskRiscv.readWord (model.mem T.rv)) (model.getPC T.rv).toNat = .malformed why := by
    rw [hg]
    show ZiskRiscv.decode (ZiskRiscv.readWord T.rv.mem) pc.toNat = _
    rw [decode_fetch3 _ hrw (by rw [← h4]; try rfl) (by rw [← h8]; try rfl)]
    exact hd
  have hxm : model.getPC T.rv ≠ model.exitAddr := by
    rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hpx)
  obtain ⟨m, hs⟩ := step_malformed T hxm hdm
  obtain ⟨-, -, ⟨pc', hpc', hzpc'⟩, ⟨st, hst, -⟩, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨z', hze', hzerr⟩ := Equiv.Zisk.execute_halt0 z1 st hst
  refine ⟨⟨_, m, fun fuel => run_of hmr.toMRun hs (by simp) fuel⟩, z', ZEnds.of_run hz1 ⟨0, z1, .done _, ?_⟩, hzerr⟩
  rw [zstep_run zrom z1 pc' hzpc', hrom]
  exact hze'

end Equiv.Top
