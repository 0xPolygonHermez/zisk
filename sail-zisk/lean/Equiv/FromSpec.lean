/-
  The ZisK program as the transpiler's spec gives it (Transpiler/Spec.lean):
  what the simulation's steps assumed of the ZisK program at pc — the `rom`
  field of RAt, IAt, ... — derived from the spec's translation of the
  RISC-V word there.

  `Transpiled zrom pc w next`: the ZisK program holds every instruction the
  spec emits for the word w at pc, followed by the word `next`.
-/
import Transpiler.Spec
import Equiv.FusedSim
import Equiv.Run

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The ZisK program holds the transpiler's translation of the word w at pc,
    followed by `next`, with some internal addresses. -/
def Transpiled (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (pc : BitVec 64) (w : BitVec 32)
    (next : Option (BitVec 32)) : Prop :=
  ∃ ia l, Transpiler.convert pc w next ia = some l ∧ ∀ p ∈ l, zrom p.1 = p.2

theorem Transpiled.one {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {w : BitVec 32}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc w next) {i : _root_.Zisk.zisk_inst}
    (hc : ∀ ia, Transpiler.convert pc w next ia = some [(pc, i)]) : zrom pc = i := by
  obtain ⟨ia, l, hl, hz⟩ := h
  rw [hc ia] at hl; cases hl
  exact hz _ (List.mem_singleton_self _)

theorem srcA_ne {r : Nat} (h : r ≠ 0) : Transpiler.srcA r = (.A_REG, BitVec.ofNat 64 r) := by
  simp [Transpiler.srcA, h]

theorem srcB_ne {r : Nat} (h : r ≠ 0) : Transpiler.srcB r = (.B_REG, BitVec.ofNat 64 r) := by
  simp [Transpiler.srcB, h]

theorem storeReg_ne {r : Nat} (h : r ≠ 0) (b : Bool) :
    Transpiler.storeReg r b = (.ST_REG, BitVec.ofNat 64 r, b) := by
  simp [Transpiler.storeReg, h]

/-! ## R-type -/

theorem RInstr.spec_fields (i : RInstr) :
    Transpiler.opcode i.encode = 0x33#7 ∧ Transpiler.funct3 i.encode = rtypeF3 i.op ∧
    Transpiler.funct7 i.encode = rtypeF7 i.op ∧ Transpiler.rd i.encode = i.rd.toNat ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.rs2 i.encode = i.rs2.toNat := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.funct7, Transpiler.rd, Transpiler.rs1,
    Transpiler.rs2, RInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · bv_decide
  all_goals congr 1; bv_decide

/-- An R-type instruction whose registers are not x0: the spec's translation
    is the one RAt assumes. -/
theorem RInstr.rom_of_spec (i : RInstr) (hd : i.rd ≠ 0) (h1 : i.rs1 ≠ 0) (h2 : i.rs2 ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.regInst (ziskOp i.op) i.rd.toNat i.rs1.toNat i.rs2.toNat := by
  obtain ⟨hop, hf3, hf7, hrd, hr1, hr2⟩ := i.spec_fields
  have ed := ne_zero_toNat hd
  have e1 := ne_zero_toNat h1
  have e2 := ne_zero_toNat h2
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 hf7 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, rtypeF3, rtypeF7, hr1, hr2, hrd, e1, e2, ed,
      Transpiler.registerOp, Transpiler.mk, srcA_ne e1, srcB_ne e2, storeReg_ne ed, Transpiler.ROp.zop,
      Equiv.Zisk.regInst, ziskOp]

/-- A link to r: no store for x0. -/
theorem storeReg_link (r : Nat) :
    Transpiler.storeReg r true = (Equiv.Zisk.linkStore r, BitVec.ofNat 64 r, decide (r ≠ 0)) := by
  by_cases h : r = 0 <;> simp [Transpiler.storeReg, Equiv.Zisk.linkStore, h]

/-! ## I-type and immediate shifts -/

theorem IInstr.spec_fields (i : IInstr) :
    Transpiler.opcode i.encode = 0x13#7 ∧ Transpiler.funct3 i.encode = itypeF3 i.op ∧
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
    Transpiler.immI i.encode = i.imm.signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.rd, Transpiler.rs1, Transpiler.immI,
    IInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

/-- An I-type instruction whose registers are not x0, and whose immediate
    is not 0 for addi (`mv`, which the transpiler emits as a copyb): the
    spec's translation is the one IAt assumes. -/
theorem IInstr.rom_of_spec (i : IInstr) (hd : i.rd ≠ 0) (h1 : i.rs1 ≠ 0) (hmv : i.op = .ADDI → i.imm ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.immInst (ziskOp (iopR i.op)) i.rd.toNat i.rs1.toNat (i.imm.signExtend 64) := by
  obtain ⟨hop, hf3, hrd, hr1, himm⟩ := i.spec_fields
  have ed := ne_zero_toNat hd
  have e1 := ne_zero_toNat h1
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 hmv
  · have hz : i.imm.signExtend 64 ≠ 0#64 := by
      have hne := hmv rfl; intro e; apply hne; bv_decide
    simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, himm, e1, ed, hz,
      Transpiler.immediateOpOrCopyb, Transpiler.immediateOp, Transpiler.mk, srcA_ne e1, storeReg_ne ed,
      Equiv.Zisk.immInst, ziskOp, iopR]
  all_goals
    simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, himm, e1, ed,
      Transpiler.immediateOpOrCopyb, Transpiler.immediateOp, Transpiler.mk, srcA_ne e1, storeReg_ne ed,
      Equiv.Zisk.immInst, ziskOp, iopR]

theorem SInstr.spec_fields (i : SInstr) :
    Transpiler.opcode i.encode = 0x13#7 ∧ Transpiler.funct3 i.encode = shiftF3 i.op ∧
    Transpiler.funct6 i.encode = shiftF6 i.op ∧
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
    Transpiler.shamt i.encode = i.sh.zeroExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.funct6, Transpiler.rd, Transpiler.rs1,
    Transpiler.shamt, SInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem SInstr.rom_of_spec (i : SInstr) (hd : i.rd ≠ 0) (h1 : i.rs1 ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.immInst (ziskOp (sopR i.op)) i.rd.toNat i.rs1.toNat (i.sh.zeroExtend 64) := by
  obtain ⟨hop, hf3, hf6, hrd, hr1, hsh⟩ := i.spec_fields
  have ed := ne_zero_toNat hd
  have e1 := ne_zero_toNat h1
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 hf6 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf6, shiftF3, shiftF6, hr1, hrd, hsh, e1, ed,
      Transpiler.immediateOp, Transpiler.mk, srcA_ne e1, storeReg_ne ed, Transpiler.SOp.zop,
      Equiv.Zisk.immInst, ziskOp, sopR]

/-! ## Loads and stores -/

theorem LdInstr.spec_fields (i : LdInstr) :
    Transpiler.opcode i.encode = 0x03#7 ∧ Transpiler.funct3 i.encode = 3#3 ∧
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
    Transpiler.immI i.encode = i.imm.signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.rd, Transpiler.rs1, Transpiler.immI,
    LdInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem LdInstr.rom_of_spec (i : LdInstr) (hd : i.rd ≠ 0) (h1 : i.rs1 ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.loadInst (.ZOP_COPYB ()) i.rd.toNat i.rs1.toNat (i.imm.signExtend 64) 8 := by
  obtain ⟨hop, hf3, hrd, hr1, himm⟩ := i.spec_fields
  have ed := ne_zero_toNat hd
  have e1 := ne_zero_toNat h1
  refine h.one fun ia => ?_
  simp [Transpiler.convert, Transpiler.decode, hop, hf3, hr1, hrd, himm, e1, ed,
    Transpiler.loadOp, Transpiler.mk, srcA_ne e1, storeReg_ne ed, Equiv.Zisk.loadInst]

theorem LwInstr.spec_fields (i : LwInstr) :
    Transpiler.opcode i.encode = 0x03#7 ∧ Transpiler.funct3 i.encode = i.k.f3 ∧
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
    Transpiler.immI i.encode = i.imm.signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.rd, Transpiler.rs1, Transpiler.immI,
    LwInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem LwInstr.rom_of_spec (i : LwInstr) (hd : i.rd ≠ 0) (h1 : i.rs1 ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.loadInst i.k.op i.rd.toNat i.rs1.toNat (i.imm.signExtend 64) i.k.w := by
  obtain ⟨hop, hf3, hrd, hr1, himm⟩ := i.spec_fields
  have ed := ne_zero_toNat hd
  have e1 := ne_zero_toNat h1
  refine h.one fun ia => ?_
  cases hk : i.k <;> rw [hk] at hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, LKind.f3, hr1, hrd, himm, e1, ed, hk,
      Transpiler.loadOp, Transpiler.mk, srcA_ne e1, storeReg_ne ed, Equiv.Zisk.loadInst, LKind.op, LKind.w]

theorem SdInstr.spec_fields (i : SdInstr) :
    Transpiler.opcode i.encode = 0x23#7 ∧ Transpiler.funct3 i.encode = 3#3 ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.rs2 i.encode = i.rs2.toNat ∧
    Transpiler.immS i.encode = (i.immHi ++ i.immLo).signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.rs1, Transpiler.rs2, Transpiler.immS,
    SdInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem SdInstr.rom_of_spec (i : SdInstr) (h1 : i.rs1 ≠ 0) (h2 : i.rs2 ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.storeInst i.rs1.toNat i.rs2.toNat ((i.immHi ++ i.immLo).signExtend 64) 8 := by
  obtain ⟨hop, hf3, hr1, hr2, himm⟩ := i.spec_fields
  have e1 := ne_zero_toNat h1
  have e2 := ne_zero_toNat h2
  refine h.one fun ia => ?_
  simp [Transpiler.convert, Transpiler.decode, hop, hf3, hr1, hr2, himm, e1, e2,
    Transpiler.storeOp, Transpiler.mk, srcA_ne e1, srcB_ne e2, Equiv.Zisk.storeInst]

theorem SwInstr.spec_fields (i : SwInstr) :
    Transpiler.opcode i.encode = 0x23#7 ∧ Transpiler.funct3 i.encode = i.k.f3 ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.rs2 i.encode = i.rs2.toNat ∧
    Transpiler.immS i.encode = (i.immHi ++ i.immLo).signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.rs1, Transpiler.rs2, Transpiler.immS,
    SwInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem SwInstr.rom_of_spec (i : SwInstr) (h1 : i.rs1 ≠ 0) (h2 : i.rs2 ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.storeInst i.rs1.toNat i.rs2.toNat ((i.immHi ++ i.immLo).signExtend 64) i.k.w := by
  obtain ⟨hop, hf3, hr1, hr2, himm⟩ := i.spec_fields
  have e1 := ne_zero_toNat h1
  have e2 := ne_zero_toNat h2
  refine h.one fun ia => ?_
  cases hk : i.k <;> rw [hk] at hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, SKind.f3, hr1, hr2, himm, e1, e2, hk,
      Transpiler.storeOp, Transpiler.mk, srcA_ne e1, srcB_ne e2, Equiv.Zisk.storeInst, SKind.w]

/-! ## Branches and jumps -/

theorem BInstr.spec_fields (i : BInstr) :
    Transpiler.opcode i.encode = 0x63#7 ∧ Transpiler.funct3 i.encode = btypeF3 i.op ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.rs2 i.encode = i.rs2.toNat ∧
    Transpiler.immB i.encode = i.imm.signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.rs1, Transpiler.rs2, Transpiler.immB,
    BInstr.encode, BInstr.imm]
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem BInstr.rom_of_spec (i : BInstr) (h1 : i.rs1 ≠ 0) (h2 : i.rs2 ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.branchInst (zbopOp i.op) i.rs1.toNat i.rs2.toNat i.zjumps.1 i.zjumps.2 := by
  obtain ⟨hop, hf3, hr1, hr2, himm⟩ := i.spec_fields
  have e1 := ne_zero_toNat h1
  have e2 := ne_zero_toNat h2
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, btypeF3, hr1, hr2, himm, e1, e2, hk,
      Transpiler.branchOp, Transpiler.mk, srcA_ne e1, srcB_ne e2, Equiv.Zisk.branchInst, BInstr.zjumps,
      zbopOp, zbopNeg]

theorem UInstr.spec_fields (i : UInstr) :
    Transpiler.opcode i.encode = uopEnc i.op ∧ Transpiler.rd i.encode = i.rd.toNat ∧
    Transpiler.immU i.encode = (i.imm ++ 0x000#12).signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.rd, Transpiler.immU, UInstr.encode]
  refine ⟨?_, ?_, ?_⟩
  · cases i.op <;> simp only [uopEnc] <;> bv_decide
  · congr 1; bv_decide
  · bv_decide

/-- lui, or an auipc the transpiler does not fuse with the word after it:
    the spec's translation is the one UAt assumes. -/
theorem UInstr.rom_of_spec (i : UInstr) (hd : i.rd ≠ 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (hnf : ∀ n, next = some n → Transpiler.fuses i.encode n = false)
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.constInst i.rd.toNat (utypeVal i.op pc i.imm) := by
  obtain ⟨hop, hrd, himm⟩ := i.spec_fields
  have ed := ne_zero_toNat hd
  refine h.one fun ia => ?_
  have hplain : Transpiler.auipc i.encode pc next = Transpiler.auipc.plain i.encode pc := by
    cases next with
    | none => rfl
    | some n => simp [Transpiler.auipc, hnf n rfl]
  cases hk : i.op <;> rw [hk] at hop <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, uopEnc, hrd, himm, ed, hplain,
      Transpiler.lui, Transpiler.auipc.plain, Transpiler.mk, storeReg_ne ed, Equiv.Zisk.constInst,
      utypeVal, sign_extend, Sail.BitVec.signExtend]

theorem JInstr.spec_fields (i : JInstr) :
    Transpiler.opcode i.encode = 0x6f#7 ∧ Transpiler.rd i.encode = i.rd.toNat ∧
    Transpiler.immJ i.encode = i.imm.signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.rd, Transpiler.immJ, JInstr.encode, JInstr.imm]
  refine ⟨?_, ?_, ?_⟩
  · bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem JInstr.rom_of_spec (i : JInstr)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.jalInst i.rd.toNat (i.imm.signExtend 64) := by
  obtain ⟨hop, hrd, himm⟩ := i.spec_fields
  refine h.one fun ia => ?_
  simp [Transpiler.convert, Transpiler.decode, hop, hrd, himm, Transpiler.jal, Transpiler.mk,
    storeReg_link, Equiv.Zisk.jalInst]

theorem JrInstr.spec_fields (i : JrInstr) :
    Transpiler.opcode i.encode = 0x67#7 ∧ Transpiler.rd i.encode = i.rd.toNat ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.immI i.encode = i.imm.signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.rd, Transpiler.rs1, Transpiler.immI, JrInstr.encode]
  refine ⟨?_, ?_, ?_, ?_⟩
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem JrInstr.lsb (i : JrInstr) : (i.imm.signExtend 64)[0] = i.imm[0] := by
  simp [BitVec.getElem_signExtend]

/-- jalr with an even offset, rs1 not x0: the spec's translation is the one
    JrAt assumes. -/
theorem JrInstr.rom_of_spec (i : JrInstr) (h1 : i.rs1 ≠ 0) (heven : i.imm[0] = false)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Equiv.Zisk.jalrInst i.rd.toNat i.rs1.toNat (i.imm.signExtend 64) := by
  obtain ⟨hop, hrd, hr1, himm⟩ := i.spec_fields
  have e1 := ne_zero_toNat h1
  refine h.one fun ia => ?_
  simp [Transpiler.convert, Transpiler.decode, hop, hrd, hr1, himm, i.lsb, heven, e1, Transpiler.jalr,
    Transpiler.mk, srcB_ne e1, storeReg_link, Equiv.Zisk.jalrInst, Transpiler.JALR_MASK,
    Equiv.Zisk.JALR_MASK]

/-- jalr with an odd offset, rs1 not x0: the spec's two instructions are the
    ones JoAt assumes, at the internal address the translation uses. -/
theorem JrInstr.rom_of_spec_odd (i : JrInstr) (h1 : i.rs1 ≠ 0) (hodd : i.imm[0] = true)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {next : Option (BitVec 32)}
    (h : Transpiled zrom pc i.encode next) :
    ∃ ia, zrom pc = Equiv.Zisk.jalrAddInst i.rs1.toNat (i.imm.signExtend 64) (ia - pc) ∧
      zrom ia = Equiv.Zisk.jalrAndInst i.rd.toNat (pc + 4 - ia) := by
  obtain ⟨hop, hrd, hr1, himm⟩ := i.spec_fields
  have e1 := ne_zero_toNat h1
  obtain ⟨ia, l, hl, hz⟩ := h
  simp [Transpiler.convert, Transpiler.decode, hop, hrd, hr1, himm, i.lsb, hodd, Transpiler.jalr] at hl
  subst hl
  simp only [List.mem_cons, List.mem_singleton, forall_eq_or_imp, forall_eq, List.not_mem_nil,
    or_false] at hz
  obtain ⟨hp, hq⟩ := hz
  refine ⟨ia, hp.trans ?_, hq.trans ?_⟩
  · simp [Transpiler.mk, srcB_ne e1, Equiv.Zisk.jalrAddInst]
  · simp [Transpiler.mk, storeReg_link, Equiv.Zisk.jalrAndInst, Transpiler.JALR_MASK, Equiv.Zisk.JALR_MASK]

/-! ## The fused auipc-jalr pair -/

theorem mask_update (x : BitVec 64) : x &&& Transpiler.JALR_MASK = BitVec.update x 0 0#1 := by
  simp only [Transpiler.JALR_MASK, Sail.BitVec.update, Sail.BitVec.updateSubrange']
  bv_decide

theorem jalr_beq : (Transpiler.Inst.jalr == Transpiler.Inst.jalr) = true := rfl

theorem toNat_eq_zero {x : BitVec 5} : x.toNat = 0 ↔ x = 0 := by
  constructor
  · intro h; apply BitVec.eq_of_toNat_eq; simpa using h
  · intro h; subst h; rfl

/-- The fused pair: the spec fuses the auipc with the jalr after it, and its
    instruction is the one FAt assumes. -/
theorem FInstr.rom_of_spec (i : FInstr) (hd : i.rd ≠ 0) (hl : i.rd2 = i.rd ∨ i.rd2 = 0)
    {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    (h : Transpiled zrom pc i.auipc.encode (some i.jalr.encode)) :
    zrom pc = Equiv.Zisk.constJInst i.rd.toNat (i.c pc) (i.T pc - pc) := by
  obtain ⟨uop, urd, uimm⟩ := i.auipc.spec_fields
  obtain ⟨jop, jrd, jr1, jimm⟩ := i.jalr.spec_fields
  simp only [FInstr.auipc, uopEnc] at uop urd uimm
  simp only [FInstr.jalr] at jop jrd jr1 jimm
  have ed := ne_zero_toNat hd
  have hf : Transpiler.fuses i.auipc.encode i.jalr.encode = true := by
    have : i.rd2.toNat = i.rd.toNat ∨ i.rd2.toNat = 0 := by
      rcases hl with e | e <;> simp [e]
    simp [Transpiler.fuses, Transpiler.decode, jop, jr1, jrd, FInstr.auipc, FInstr.jalr, urd, ed, this, jalr_beq]
  refine h.one fun ia => ?_
  simp only [FInstr.auipc, FInstr.jalr] at hf
  simp [Transpiler.convert, Transpiler.decode, uop, Transpiler.auipc, hf, urd, uimm, jrd, jimm, ed,
    Transpiler.mk, storeReg_ne ed, Equiv.Zisk.constJInst, FInstr.c, FInstr.T, FInstr.A, utypeVal,
    jalrTarget, mask_update, toNat_eq_zero, sign_extend, Sail.BitVec.signExtend, FInstr.jalr, FInstr.auipc]

end Equiv.Sim
