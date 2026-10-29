/-
  Totality, the statement: at every step of a program in the contract,
  ZisK's RISC-V machine takes a step the simulation covers, or meets one of
  the ends the theorems prove (Final.lean), or the program breaks one of the
  contract's preconditions there (doc/riscv_contract.md).

  `StepCase` is that disjunction. `KindCases` lists what remains to prove,
  a field for each kind of word the spec's decoder tells apart
  (`Transpiler.decode`); `step_case` assembles them: the return address,
  a pc outside the code, then the word at pc, by its kind. Each field is
  proved in its own module, kind by kind.
-/
import Equiv.Final

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top Equiv.Sparse
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-! ## A step's memory access -/

/-- How a word accesses memory. -/
inductive AccKind where
  | load | store | amo
deriving DecidableEq

/-- A register's value, x0 reading 0. -/
noncomputable def regOf (s : RVState) (r : Nat) : BitVec 64 := (getX s r).getD 0

def LOp.width : Transpiler.LOp → Nat
  | .lb | .lbu => 1 | .lh | .lhu => 2 | .lw | .lwu => 4 | .ld => 8

def SzOp.width : Transpiler.SzOp → Nat
  | .sb => 1 | .sh => 2 | .sw => 4 | .sd => 8

/-- The memory access the word w makes in state s, if it makes one: its
    kind, its address and its width (lr a load, sc a store). -/
noncomputable def access (s : RVState) (w : BitVec 32) : Option (AccKind × BitVec 64 × Nat) :=
  match Transpiler.decode w with
  | some (.load op) => some (.load, regOf s (Transpiler.rs1 w) + Transpiler.immI w, LOp.width op)
  | some (.store op) => some (.store, regOf s (Transpiler.rs1 w) + Transpiler.immS w, SzOp.width op)
  | some (.amo _ d) => some (.amo, regOf s (Transpiler.rs1 w), if d then 8 else 4)
  | some (.lr d) => some (.load, regOf s (Transpiler.rs1 w), if d then 8 else 4)
  | some (.sc d) => some (.store, regOf s (Transpiler.rs1 w), if d then 8 else 4)
  | _ => none

/-- Whether [a, a + n) and [b, b + m) share a byte. -/
def Overlaps (a : BitVec 64) (n : Nat) (b : BitVec 64) (m : Nat) : Prop :=
  a.toNat < b.toNat + m ∧ b.toNat < a.toNat + n

/-! ## The contract's preconditions, broken at a step

Each is stated of the state at the step and the word at its pc, as the
contract states it (doc/riscv_contract.md). -/

/-- J: the pc out of the code, but for the return to the initial ra. -/
def BreakJ (C : BitVec 64 → Prop) (s : RVState) : Prop :=
  ∃ pc, s.regs.get? Register.PC = some pc ∧ ¬ C pc ∧ pc ≠ Transpiler.OUTPUT_BLOCK

/-- E: an `ecall` but the exit system call. -/
def BreakE (s : RVState) (w : BitVec 32) : Prop :=
  w = ECALL ∧ getX s 17 ≠ some 93

/-- F: a word of a float opcode. -/
def BreakF (w : BitVec 32) : Prop :=
  let o := (w.extractLsb' 0 7).toNat
  o = 0x07 ∨ o = 0x27 ∨ o = 0x43 ∨ o = 0x47 ∨ o = 0x4B ∨ o = 0x4F ∨ o = 0x53

/-- C and Z: a CSR instruction outside the contract's forms (§§ 6–8: the
    ZisK calls, `csrs ID, x0`, reading `marchid`, writing a read-only CSR),
    a zkvmcall's among them: those the spec describes, and only those. -/
def BreakCsr (pc : BitVec 64) (w : BitVec 32) (next next2 : Option (BitVec 32)) : Prop :=
  Transpiler.decode w = some .csr ∧ ∀ ia, Transpiler.convert pc w next ia next2 = none

/-- R: an access touching the registers' slots. -/
def BreakR (s : RVState) (w : BitVec 32) : Prop :=
  ∃ k a n, access s w = some (k, a, n) ∧ _root_.Zisk.Functions.hits_registers a n = true

/-- I: an access touching the free input's 8 bytes, but an 8-byte load of
    them. -/
def BreakI (s : RVState) (w : BitVec 32) : Prop :=
  ∃ k a n, access s w = some (k, a, n) ∧ Overlaps a n _root_.Zisk.Functions.INPUT_ADDR 8 ∧
    ¬ (k = .load ∧ a = _root_.Zisk.Functions.INPUT_ADDR ∧ n = 8)

/-- A: an AMO across a 16-byte boundary. -/
def BreakA (s : RVState) (w : BitVec 32) : Prop :=
  ∃ a n, access s w = some (.amo, a, n) ∧ a.toNat / 16 ≠ (a.toNat + n - 1) / 16

/-- L: an `lr` whose bytes include its own instruction word. -/
def BreakL (s : RVState) (pc : BitVec 64) (w : BitVec 32) : Prop :=
  (∃ d, Transpiler.decode w = some (.lr d)) ∧ ∃ a n, access s w = some (.load, a, n) ∧ Overlaps a n pc 4

/-- C, on memory: a store or AMO touching `marchid`'s cell. -/
def BreakMarchid (s : RVState) (w : BitVec 32) : Prop :=
  ∃ k a n, access s w = some (k, a, n) ∧ k ≠ .load ∧ Overlaps a n MARCHID_CELL 8

/-- A precondition broken at the step, the word w at pc. -/
def Broken (s : RVState) (pc : BitVec 64) (w : BitVec 32) : Prop :=
  BreakE s w ∨ BreakF w ∨ BreakCsr pc w (nextWord s pc) (nextWord2 s pc) ∨ BreakR s w ∨
    BreakI s w ∨ BreakA s w ∨ BreakL s pc w ∨ BreakMarchid s w

/-! ## The ends the theorems prove, at a step -/

/-- One of Final.lean's ends at the step, its hypotheses as the theorem
    takes them: the exit system call, a ZisK call that fails, a load or
    store neither side may make, a word the transpiler halts on, a sequence
    it refuses, a jump to a target 2 mod 4, the fused pair to one. -/
def EndAt (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (S : ZiskRiscv.State model) : Prop :=
  (∃ pc, S.rv.regs.get? Register.PC = some pc ∧ C pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some ECALL ∧
      getX S.rv 17 = some 93) ∨
  (∃ pc w c, S.rv.regs.get? Register.PC = some pc ∧ C pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some w ∧
      ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord S.rv pc) (nextWord2 S.rv pc)) pc.toNat = .call c ∧
      (c.len ≠ 1 → C (pc + 4)) ∧ (c.len = 3 → C (pc + 8)) ∧ CallFail S c) ∨
  FaultAtR zrom S.rv ∨
  HaltAt ziskRegions zrom S.rv ∨
  (∃ pc w why, S.rv.regs.get? Register.PC = some pc ∧ C pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some w ∧
      ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord S.rv pc) (nextWord2 S.rv pc)) pc.toNat = .malformed why ∧
      (∃ ia l, Transpiler.convert pc w (nextWord S.rv pc) ia (nextWord2 S.rv pc) = some l) ∧
      C (pc + 4) ∧ C (pc + 8)) ∨
  MisJumpAt ziskRegions zrom S.rv ∨
  (∃ pc i, S.rv.regs.get? Register.PC = some pc ∧ FtAtM S.rv ziskRegions pc zrom i ∧
      (i.T pc)[0] = false ∧ (i.T pc)[1] = true)

/-! ## The step's cases -/

/-- At a step: a step the simulation covers (`OneOkM`, the fused pair two
    at a time), the return to the initial ra, one of the ends, or a
    precondition broken (J, or one at the word at pc). -/
def StepCase (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (S : ZiskRiscv.State model) : Prop :=
  OneOkM ziskRegions zrom C S ∨
  (∃ pc i, S.rv.regs.get? Register.PC = some pc ∧ FtAt S.rv ziskRegions pc zrom i) ∨
  S.rv.regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK ∨
  EndAt zrom C S ∨
  BreakJ C S.rv ∨
  (∃ pc w, S.rv.regs.get? Register.PC = some pc ∧ ZiskRiscv.readWord S.rv.mem pc.toNat = some w ∧
      Broken S.rv pc w)

/-- What a step at a code address has to go on: its pc and word, the
    regions' facts, sail-riscv's state ready, the ZisK program the spec's
    translation of the code, memory as ZisK starts it. -/
structure Ctx (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) (S : ZiskRiscv.State model)
    (pc : BitVec 64) (w : BitVec 32) : Prop where
  atPc : S.rv.regs.get? Register.PC = some pc
  code : C pc
  word : ZiskRiscv.readWord S.rv.mem pc.toNat = some w
  exec : ∀ p, C p → ExecAt ziskRegions p
  ready : StepReady S.rv ziskRegions
  spec : SpecRom zrom C S.rv.mem
  full : Full S.rv.mem
  halt : HaltMis zrom

/-- The kinds of instruction the spec describes, as the proof takes them. -/
inductive Group where
  /-- No memory access, no jump: ALU ops, their word forms, M, Zba, Zbb,
      Zbs, Zbkb, lui, auipc, the fences. -/
  | plain
  /-- jal, jalr, the branches. -/
  | jump
  /-- Loads, stores, AMOs, lr and sc. -/
  | mem
  /-- ecall, ebreak, the CSR instructions. -/
  | sys

def group : Transpiler.Inst → Group
  | .jal | .jalr | .branch _ => .jump
  | .load _ | .store _ | .amo _ _ | .lr _ | .sc _ => .mem
  | .ecall | .ebreak | .csr => .sys
  | _ => .plain

/-- What remains to prove: at a code address, for each kind of word the
    spec's decoder tells apart, the step's cases. -/
structure KindCases (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) : Prop where
  /-- A word the spec decodes to nothing. -/
  rejected : ∀ S pc w, Ctx zrom C S pc w → Transpiler.decode w = none → StepCase zrom C S
  plain : ∀ S pc w i, Ctx zrom C S pc w → Transpiler.decode w = some i → group i = .plain → StepCase zrom C S
  jump : ∀ S pc w i, Ctx zrom C S pc w → Transpiler.decode w = some i → group i = .jump → StepCase zrom C S
  mem : ∀ S pc w i, Ctx zrom C S pc w → Transpiler.decode w = some i → group i = .mem → StepCase zrom C S
  sys : ∀ S pc w i, Ctx zrom C S pc w → Transpiler.decode w = some i → group i = .sys → StepCase zrom C S

/-- The step's cases, from the per-kind ones: the return address, a pc out
    of the code, then the word at pc. -/
theorem step_case {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} (K : KindCases zrom C)
    (S : ZiskRiscv.State model) (pc : BitVec 64) (hpc : S.rv.regs.get? Register.PC = some pc)
    (hcode : ∀ p, C p → ∃ w, ZiskRiscv.readWord S.rv.mem p.toNat = some w)
    (hctx : ∀ w, C pc → ZiskRiscv.readWord S.rv.mem pc.toNat = some w → Ctx zrom C S pc w) :
    StepCase zrom C S := by
  by_cases hr : pc = Transpiler.OUTPUT_BLOCK
  · subst hr; exact .inr (.inr (.inl hpc))
  by_cases hc : C pc
  · obtain ⟨w, hw⟩ := hcode pc hc
    have hx := hctx w hc hw
    cases hd : Transpiler.decode w with
    | none => exact K.rejected S pc w hx hd
    | some i =>
      cases hg : group i
      · exact K.plain S pc w i hx hd hg
      · exact K.jump S pc w i hx hd hg
      · exact K.mem S pc w i hx hd hg
      · exact K.sys S pc w i hx hd hg
  · exact .inr (.inr (.inr (.inr (.inl ⟨pc, hpc, hc, hr⟩))))

end Equiv.Final
