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

/-- The exit system call with code c. -/
theorem zisk_exit (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (C : BitVec 64 → Prop) (hcx : ∀ pc, C pc → ExecAt ziskRegions pc)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkT ziskRegions zr k s0 0) (hin : RunIn C k s0 0)
    (pc : BitVec 64) (hpc : (rvIter 0 k s0).regs.get? Register.PC = some pc) (hpcc : C pc)
    (hw : ZiskRiscv.readWord (rvIter 0 k s0).mem pc.toNat = some ECALL)
    (h17 : getX (rvIter 0 k s0) 17 = some 93) (c : BitVec 64) (h10 : getX (rvIter 0 k s0) 10 = some c)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w)
    (hw0 : ZiskRiscv.readWord s0'.mem pc.toNat = some ECALL) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit c, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn (rvIter 0 k s0).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) :=
  exit_end_sparse zrom entry hb ziskRegions exit_not_exec trap_not_exec C (noCodeRam_of_exec hcx) m0 hfull hspec
    s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k hrun hin pc hpc (hcx pc hpcc) hpcc hw h17 c h10 s0' hsp hcode hw0

/-- The return to the initial ra. -/
theorem zisk_return (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (C : BitVec 64 → Prop) (hcx : ∀ pc, C pc → ExecAt ziskRegions pc)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkT ziskRegions zr k s0 0) (hin : RunIn C k s0 0)
    (hpc : (rvIter 0 k s0).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit 0, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn (rvIter 0 k s0).mem :=
  return_end_sparse zrom entry hb ziskRegions exit_not_exec trap_not_exec C (noCodeRam_of_exec hcx) m0 hfull
    hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k hrun hin hpc s0' hsp hcode

end Equiv.Final
