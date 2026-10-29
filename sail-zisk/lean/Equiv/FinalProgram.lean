/-
  The theorems of Final.lean, for the ROM the transpiler builds from the
  program (`zromOf`, Transpiler/Rom.lean): what they ask of their ROM —
  the BIOS, the spec's translations, the halt at an address 2 mod 4 —
  follows from the program's code (`ProgramOk`, Equiv/ProgramRomSpec.lean).
-/
import Equiv.Final
import Equiv.ProgramRomSpec

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The exit system call with code c. -/
theorem zisk_exit_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (hfull : Full m0)
    (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some pc)
    (hpcc : C pc) (hw : ZiskRiscv.readWord ((sIter 0 k ⟨s0, fc⟩).rv).mem pc.toNat = some ECALL)
    (h17 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 17 = some 93) (c : BitVec 64)
    (h10 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 10 = some c) (s0' : RVState)
    (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w)
    (hw0 : ZiskRiscv.readWord s0'.mem pc.toNat = some ECALL) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit c, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds (zromOf segs entry) z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) :=
  zisk_exit (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 hfull (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin pc hpc hpcc hw h17 c h10 s0' hsp hcode hw0

/-- The return to the initial ra. -/
theorem zisk_return_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (hfull : Full m0)
    (s0 : RVState) (hs0 : Start s0 ziskRegions entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0)
    (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK) (s0' : RVState)
    (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit 0, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds (zromOf segs entry) z0 z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem :=
  zisk_return (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 hfull (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin hpc s0' hsp hcode

/-- A ZisK call whose operation fails (by reading a byte that is not there
    only outside ZisK's regions): the machine's run ends in failure or in its own error, and
    sail-zisk's reaches a step that fails. -/
theorem zisk_fail_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (s0 : RVState)
    (hs0 : Start s0 ziskRegions entry m0) (z0 : Equiv.Zisk.ZState)
    (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0)) (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hzst : ∃ st, z0.regs.get? .Step = some st) (hze : z0.regs.get? .Zerror = some false)
    (hzm : z0.mem = m0) (fc : ZiskRiscv.Fcall)
    (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst)
    (k : Nat) (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0) (pc : BitVec 64)
    (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (hpcc : C pc)
    (w : BitVec 32) (hrw : ZiskRiscv.readWord (sIter 0 k ⟨s0, fc⟩).rv.mem pc.toNat = some w)
    (c : ZiskRiscv.Call) (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc)
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc)) pc.toNat = .call c)
    (hC4 : c.len ≠ 1 → C (pc + 4)) (hC8 : c.len = 3 → C (pc + 8))
    (hfail : CallFail (sIter 0 k ⟨s0, fc⟩) c) (s0' : RVState)
    (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ n z1 e z', ZRun (zromOf segs entry) n z0 z1 ∧ (zstep (zromOf segs entry)).run z1 = .error e z' :=
  zisk_fail (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin pc hpc hpcc w hrw c hd hC4 hC8 hfail s0' hsp hcode

/-- A load or a store at an address none of ZisK's regions permits: the
    machine's run ends in failure, or in its own error; sail-zisk's reaches
    a step that fails. -/
theorem zisk_fault_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (s0 : RVState)
    (hs0 : Start s0 ziskRegions entry m0) (z0 : Equiv.Zisk.ZState)
    (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0)) (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hzst : ∃ st, z0.regs.get? .Step = some st) (hze : z0.regs.get? .Zerror = some false)
    (hzm : z0.mem = m0) (fc : ZiskRiscv.Fcall)
    (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst)
    (k : Nat) (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0) (hfa : FaultAtR (zromOf segs entry) (sIter 0 k ⟨s0, fc⟩).rv)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ n z1 e z', ZRun (zromOf segs entry) n z0 z1 ∧ (zstep (zromOf segs entry)).run z1 = .error e z' :=
  zisk_fault (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin hfa s0' hsp hcode

/-- A jump to a target 2 mod 4: the machine's run ends in failure, or in
    its own error; sail-zisk's takes the jump, to an address with no
    instruction, and ends in error there (`misjump_end`). -/
theorem zisk_misjump_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (s0 : RVState)
    (hs0 : Start s0 ziskRegions entry m0) (z0 : Equiv.Zisk.ZState)
    (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0)) (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hzst : ∃ st, z0.regs.get? .Step = some st) (hze : z0.regs.get? .Zerror = some false)
    (hzm : z0.mem = m0) (fc : ZiskRiscv.Fcall)
    (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst)
    (k : Nat) (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0) (hmj : MisJumpAt ziskRegions (zromOf segs entry) (sIter 0 k ⟨s0, fc⟩).rv)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds (zromOf segs entry) z0 z' ∧ z'.regs.get? .Zerror = some true :=
  zisk_misjump (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin hmj (hp.rom entry).2.2 s0' hsp hcode

/-- The fused auipc-jalr pair to a target 2 mod 4: the machine's run ends
    in failure a step later, or in its own error; sail-zisk's takes the
    pair's one instruction, to the target, and ends in error there
    (`fusedmis_end`). -/
theorem zisk_fusedmis_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (s0 : RVState)
    (hs0 : Start s0 ziskRegions entry m0) (z0 : Equiv.Zisk.ZState)
    (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0)) (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hzst : ∃ st, z0.regs.get? .Step = some st) (hze : z0.regs.get? .Zerror = some false)
    (hzm : z0.mem = m0) (fc : ZiskRiscv.Fcall)
    (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst)
    (k : Nat) (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0) (pc : BitVec 64)
    (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (i : FInstr)
    (hi : FtAtM (sIter 0 k ⟨s0, fc⟩).rv ziskRegions pc (zromOf segs entry) i) (htg : (i.T pc)[0] = false ∧ (i.T pc)[1] = true)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 2 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k + 1)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 2 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds (zromOf segs entry) z0 z' ∧ z'.regs.get? .Zerror = some true :=
  zisk_fusedmis (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin pc hpc i hi htg (hp.rom entry).2.2 s0' hsp hcode

/-- `ebreak`, a write to a read-only CSR, or a word RISC-V rejects: the
    machine's run ends in failure, or in its own error; sail-zisk's ends in
    error. -/
theorem zisk_halt_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (s0 : RVState)
    (hs0 : Start s0 ziskRegions entry m0) (z0 : Equiv.Zisk.ZState)
    (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0)) (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hzst : ∃ st, z0.regs.get? .Step = some st) (hze : z0.regs.get? .Zerror = some false)
    (hzm : z0.mem = m0) (fc : ZiskRiscv.Fcall)
    (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst)
    (k : Nat) (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0) (hha : HaltAt ziskRegions (zromOf segs entry) (sIter 0 k ⟨s0, fc⟩).rv)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds (zromOf segs entry) z0 z' ∧ z'.regs.get? .Zerror = some true :=
  zisk_halt (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin hha s0' hsp hcode

/-- A ZisK call sequence the transpiler refuses, or a SYSTEM word of
    funct3 0 but `ecall` and `ebreak` (`mret`, `wfi`, ...), at a code address, the
    two words after it code too, that the spec describes: the machine's
    run ends in failure, or in its own error; sail-zisk's ends in error. -/
theorem zisk_malformed_prog (entry : BitVec 64) (C : BitVec 64 → Prop)
    (hcx : ∀ pc, C pc → ExecAt ziskRegions pc) (m0 : Mem)
    (segs : List (BitVec 64 × Array (BitVec 32))) (hp : ProgramOk segs C m0) (s0 : RVState)
    (hs0 : Start s0 ziskRegions entry m0) (z0 : Equiv.Zisk.ZState)
    (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0)) (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hzst : ∃ st, z0.regs.get? .Step = some st) (hze : z0.regs.get? .Zerror = some false)
    (hzm : z0.mem = m0) (fc : ZiskRiscv.Fcall)
    (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst)
    (k : Nat) (hrun : RunOkM ziskRegions zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0) (pc : BitVec 64)
    (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (hpcc : C pc)
    (w : BitVec 32) (hrw : ZiskRiscv.readWord (sIter 0 k ⟨s0, fc⟩).rv.mem pc.toNat = some w)
    (h4 : (pc + 4).toNat = pc.toNat + 4) (h8 : (pc + 8).toNat = pc.toNat + 8)
    (why : String) (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc)
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc)) pc.toNat = .malformed why)
    (hdesc : ∃ ia l, Transpiler.convert pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc) ia
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc) = some l) (hC4 : C (pc + 4))
    (hC8 : C (pc + 8)) (s0' : RVState)
    (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds (zromOf segs entry) z0 z' ∧ z'.regs.get? .Zerror = some true :=
  zisk_malformed (zromOf segs entry) entry (hp.rom entry).1 C hcx m0 (hp.rom entry).2.1 s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin pc hpc hpcc w hrw h4 h8 why hd hdesc hC4 hC8 s0' hsp hcode

end Equiv.Final
