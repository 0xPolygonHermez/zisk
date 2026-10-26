/-
  The theorem for ZisK's memory map: `exit_end_sparse` and
  `return_end_sparse` with the PMA regions the model's initialisation writes
  (Equiv/Regions.lean). What the general statements take of the regions —
  the exit address and the trap vector not executable, no code in the RAM —
  is then proved (Equiv/RegionFacts.lean), given that the code addresses
  are executable. The machine is given memory holding the code: each code
  address's word (`hcode`), as the ELF loader writes the program's text.
-/
import Equiv.Sparse
import Equiv.RegionFacts

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- Code at executable addresses, in the ROM, is off the input's first 8
    bytes, and so are the two words after. -/
theorem noCodeIn_of_exec {C : BitVec 64 → Prop} (h : ∀ pc, C pc → ExecAt ziskRegions pc) : NoCodeIn C := by
  intro pc hc
  have ⟨e1, e2⟩ := exec_rom (h pc hc)
  have e4 : (pc + 4).toNat = pc.toNat + 4 := by rw [BitVec.toNat_add]; simp; omega
  have e8 : (pc + 8).toNat = pc.toNat + 8 := by rw [BitVec.toNat_add]; simp; omega
  refine ⟨fun i hi => ?_, fun i hi => ?_, fun i hi => ?_⟩ <;> simp only [Equiv.IN_LO]
  · omega
  · rw [e4]; omega
  · rw [e8]; omega

/-- The exit system call with code c. -/
theorem zisk_exit (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (C : BitVec 64 → Prop) (hcx : ∀ pc, C pc → ExecAt ziskRegions pc)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some pc) (hpcc : C pc)
    (hw : ZiskRiscv.readWord ((sIter 0 k ⟨s0, fc⟩).rv).mem pc.toNat = some ECALL)
    (h17 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 17 = some 93) (c : BitVec 64) (h10 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 10 = some c)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w)
    (hw0 : ZiskRiscv.readWord s0'.mem pc.toNat = some ECALL) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit c, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) :=
  exit_end_sparse zrom entry hb ziskRegions exit_not_exec trap_not_exec C (noCodeRam_of_exec hcx) (noCodeIn_of_exec hcx) m0 hfull hspec
    s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin pc hpc (hcx pc hpcc) hpcc hw h17 c h10 s0' hsp hcode hw0

/-- The return to the initial ra. -/
theorem zisk_return (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (C : BitVec 64 → Prop) (hcx : ∀ pc, C pc → ExecAt ziskRegions pc)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit 0, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem :=
  return_end_sparse zrom entry hb ziskRegions exit_not_exec trap_not_exec C (noCodeRam_of_exec hcx) (noCodeIn_of_exec hcx) m0 hfull
    hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin hpc s0' hsp hcode

/-- A ZisK call whose operation fails, not by reading a byte that is not
    there: the machine's run ends in failure or in its own error, and
    sail-zisk's reaches a step that fails. -/
theorem zisk_fail (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (C : BitVec 64 → Prop) (hcx : ∀ pc, C pc → ExecAt ziskRegions pc)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (hpcc : C pc)
    (w : BitVec 32) (hrw : ZiskRiscv.readWord (sIter 0 k ⟨s0, fc⟩).rv.mem pc.toNat = some w) (c : ZiskRiscv.Call)
    (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc)
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc)) pc.toNat = .call c)
    (hC4 : c.len ≠ 1 → C (pc + 4)) (hC8 : c.len = 3 → C (pc + 8))
    (hfail : CallFail (sIter 0 k ⟨s0, fc⟩) c)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ n z1 e z', ZRun zrom n z0 z1 ∧ (zstep zrom).run z1 = .error e z' := by
  have ⟨e1, e2⟩ := exec_rom (hcx pc hpcc)
  have h4 : (pc + 4).toNat = pc.toNat + 4 := by rw [BitVec.toNat_add]; simp; omega
  have h8 : (pc + 8).toNat = pc.toNat + 8 := by rw [BitVec.toNat_add]; simp; omega
  exact fail_end_sparse zrom entry hb ziskRegions exit_not_exec trap_not_exec C (noCodeRam_of_exec hcx)
    (noCodeIn_of_exec hcx) m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin pc hpc (hcx pc hpcc)
    hpcc w hrw h4 h8 c hd hC4 hC8 hfail s0' hsp hcode

end Equiv.Final
