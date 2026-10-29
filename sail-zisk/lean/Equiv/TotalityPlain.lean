/-
  Totality, the plain instructions: a word the spec decodes to an
  instruction with no memory access and no jump is a step the simulation
  covers (`OneOkT`). For each kind, the instruction its fields make
  encodes to the word, and the step's facts follow from the context.
-/
import Equiv.TotalityBase
import Equiv.ProgramRomSpec

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Transpiler (decode convert opcode funct3 funct7 funct6)

/-! ## The step's facts from the context -/

/-- The ZisK program holds the spec's translation of the word at pc, the
    next word after it, for a word whose translation does not read the one
    after that. -/
theorem ctx_transpiled {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    (hn2 : Transpiler.usesNext2 w = false)
    (hdesc : ∃ ia l, convert pc w (nextWord S.rv pc) ia = some l) :
    Transpiled zrom pc w (nextWord S.rv pc) := by
  obtain ⟨ia, l, hl⟩ := hdesc
  have h := hx.spec pc w hx.code hx.word ⟨ia, l, by
    rw [convert_ignores_next2 hn2 _ _ _ none]; exact hl⟩
  exact h.drop2 ⟨ia, l, hl⟩

/-- A plain instruction is one the spec describes, at any internal address,
    whatever the words after it, and its translation does not read the
    word after the next. -/
theorem convert_some_plain {w : BitVec 32} {i : Transpiler.Inst} (hd : decode w = some i)
    (hg : group i = .plain) (pc ia : BitVec 64) (next next2 : Option (BitVec 32)) :
    ∃ l, convert pc w next ia next2 = some l := by
  cases i <;> simp only [group, reduceCtorEq] at hg <;> simp only [convert, hd] <;>
    (repeat' split) <;> exact ⟨_, rfl⟩

theorem usesNext2_plain {w : BitVec 32} {i : Transpiler.Inst} (hd : decode w = some i)
    (hg : group i = .plain) : Transpiler.usesNext2 w = false := by
  cases i <;> simp only [group, reduceCtorEq] at hg <;> simp [Transpiler.usesNext2, hd]

/-- A plain instruction's word at pc: the ZisK program holds its translation. -/
theorem ctx_transpiled_plain {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {i : Transpiler.Inst} (hd : decode w = some i) (hg : group i = .plain) :
    Transpiled zrom pc w (nextWord S.rv pc) := by
  obtain ⟨l, hl⟩ := convert_some_plain hd hg pc 0 (nextWord S.rv pc) none
  exact ctx_transpiled hx (usesNext2_plain hd hg) ⟨0, l, hl⟩

/-- What every plain instruction's location structure takes, for an
    encoding e of the word at pc: pc 4-aligned, executable at pc and pc + 2,
    e's bytes in memory, the ZisK program holding its translation. -/
theorem ctx_at {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {i : Transpiler.Inst} (hd : decode w = some i) (hg : group i = .plain) {e : BitVec 32} (he : e = w) :
    pc.toNat % 4 = 0 ∧ ExecAt ziskRegions pc ∧ ExecAt ziskRegions (pc + 2) ∧
      S.rv.mem.get? pc.toNat = some (byte e 0) ∧ S.rv.mem.get? (pc.toNat + 1) = some (byte e 1) ∧
      S.rv.mem.get? (pc.toNat + 2) = some (byte e 2) ∧ S.rv.mem.get? (pc.toNat + 3) = some (byte e 3) ∧
      Transpiled zrom pc e (nextWord S.rv pc) := by
  subst he
  have hb := readWord_bytes hx.word
  exact ⟨(hx.aligned pc hx.code), hx.exec pc hx.code, exec_add2 (hx.exec pc hx.code) (hx.aligned pc hx.code), hb.1, hb.2.1, hb.2.2.1,
    hb.2.2.2, ctx_transpiled_plain hx hd hg⟩

theorem bv_of_toNat {n : Nat} {x : BitVec n} {k : Nat} (h : x.toNat = k) : x = BitVec.ofNat n k := by
  subst h; simp

/-! ## R-type -/

/-- What the decoder reads of an R-type op: opcode, funct3, funct7. -/
def _root_.Transpiler.ROp.fields : Transpiler.ROp → Nat × Nat × Nat
  | .add => (51, 0, 0) | .sub => (51, 0, 32) | .sll => (51, 1, 0) | .slt => (51, 2, 0)
  | .sltu => (51, 3, 0) | .xor => (51, 4, 0) | .srl => (51, 5, 0) | .sra => (51, 5, 32)
  | .or => (51, 6, 0) | .and => (51, 7, 0)
  | .addw => (59, 0, 0) | .subw => (59, 0, 32) | .sllw => (59, 1, 0) | .srlw => (59, 5, 0)
  | .sraw => (59, 5, 32)

set_option maxHeartbeats 2000000 in
theorem decode_r_fields {w : BitVec 32} {op : Transpiler.ROp} (h : decode w = some (.r op)) :
    ((opcode w).toNat, (funct3 w).toNat, (funct7 w).toNat) = op.fields := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first | (simp at h; done) | (cases h; rename_i h1 h2; simp_all [Transpiler.ROp.fields])

/-- The base R-type ops, as sail-riscv names them. -/
def _root_.Transpiler.ROp.base : Transpiler.ROp → Option rop
  | .add => some .ADD | .sub => some .SUB | .sll => some .SLL | .slt => some .SLT | .sltu => some .SLTU
  | .xor => some .XOR | .srl => some .SRL | .sra => some .SRA | .or => some .OR | .and => some .AND
  | _ => none

/-- The R-type instruction a word's fields make. -/
def rOfWord (r : rop) (w : BitVec 32) : RInstr :=
  ⟨r, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem rOfWord_encode {w : BitVec 32} {op : Transpiler.ROp} {r : rop} (h : decode w = some (.r op))
    (hb : op.base = some r) : (rOfWord r w).encode = w := by
  have hf := decode_r_fields h
  cases op <;> simp only [Transpiler.ROp.base, reduceCtorEq, Option.some.injEq] at hb <;> subst hb <;>
    simp only [Transpiler.ROp.fields, Prod.mk.injEq] at hf <;> obtain ⟨h1, h2, h3⟩ := hf <;>
    have e1 := bv_of_toNat h1 <;> have e2 := bv_of_toNat h2 <;> have e3 := bv_of_toNat h3 <;>
    simp only [opcode, funct3, funct7, BitVec.extractLsb] at e1 e2 e3 <;>
    simp only [rOfWord, RInstr.encode, rtypeF7, rtypeF3] <;>
    (revert e1 e2 e3; bv_decide)

/-- A base R-type word at pc: a step the simulation covers. -/
theorem step_r {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {op : Transpiler.ROp} {r : rop} (hd : decode w = some (.r op)) (hb : op.base = some r) :
    OneOkT ziskRegions zrom S.rv := by
  have e := rOfWord_encode hd hb
  have hb4 := readWord_bytes hx.word
  rw [← e] at hb4
  have htr := ctx_transpiled_plain hx hd (by rfl)
  rw [← e] at htr
  exact ⟨pc, hx.atPc, .inl ⟨.r (rOfWord r w), ⟨(hx.aligned pc hx.code), hx.exec pc hx.code,
    exec_add2 (hx.exec pc hx.code) (hx.aligned pc hx.code), hb4.1, hb4.2.1, hb4.2.2.1, hb4.2.2.2, htr⟩⟩⟩

/-! ## The other plain kinds, by the fields the decoder reads

Each kind's facts are a predicate on the word, `Fields`, in bit-vector
form; the decoder's case split gives it (`decode_*_fields`), and
`bv_decide` the encoding from it. -/

/-- The word-form R-type ops, as sail-riscv names them. -/
def _root_.Transpiler.ROp.word : Transpiler.ROp → Option ropw
  | .addw => some .ADDW | .subw => some .SUBW | .sllw => some .SLLW | .srlw => some .SRLW
  | .sraw => some .SRAW | _ => none

def rwOfWord (r : ropw) (w : BitVec 32) : RwInstr :=
  ⟨r, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem rwOfWord_encode {w : BitVec 32} {op : Transpiler.ROp} {r : ropw} (h : decode w = some (.r op))
    (hb : op.word = some r) : (rwOfWord r w).encode = w := by
  have hf := decode_r_fields h
  cases op <;> simp only [Transpiler.ROp.word, reduceCtorEq, Option.some.injEq] at hb <;> subst hb <;>
    simp only [Transpiler.ROp.fields, Prod.mk.injEq] at hf <;> obtain ⟨h1, h2, h3⟩ := hf <;>
    have e1 := bv_of_toNat h1 <;> have e2 := bv_of_toNat h2 <;> have e3 := bv_of_toNat h3 <;>
    simp only [opcode, funct3, funct7, BitVec.extractLsb] at e1 e2 e3 <;>
    simp only [rwOfWord, RwInstr.encode, rtypewF7, rtypewF3] <;>
    (revert e1 e2 e3; bv_decide)

/-! ### I-type -/

def _root_.Transpiler.IOp.Fields (w : BitVec 32) : Transpiler.IOp → Prop
  | .addi => opcode w = 19#7 ∧ funct3 w = 0#3 | .slti => opcode w = 19#7 ∧ funct3 w = 2#3
  | .sltiu => opcode w = 19#7 ∧ funct3 w = 3#3 | .xori => opcode w = 19#7 ∧ funct3 w = 4#3
  | .ori => opcode w = 19#7 ∧ funct3 w = 6#3 | .andi => opcode w = 19#7 ∧ funct3 w = 7#3
  | .addiw => opcode w = 27#7 ∧ funct3 w = 0#3

set_option maxHeartbeats 2000000 in
theorem decode_i_fields {w : BitVec 32} {op : Transpiler.IOp} (h : decode w = some (.i op)) : op.Fields w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, Transpiler.IOp.Fields])

def _root_.Transpiler.IOp.base : Transpiler.IOp → Option iop
  | .addi => some .ADDI | .slti => some .SLTI | .sltiu => some .SLTIU | .xori => some .XORI
  | .ori => some .ORI | .andi => some .ANDI | .addiw => none

def iOfWord (r : iop) (w : BitVec 32) : IInstr := ⟨r, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 12⟩

theorem iOfWord_encode {w : BitVec 32} {op : Transpiler.IOp} {r : iop} (h : decode w = some (.i op))
    (hb : op.base = some r) : (iOfWord r w).encode = w := by
  have hf := decode_i_fields h
  cases op <;> simp only [Transpiler.IOp.base, reduceCtorEq, Option.some.injEq] at hb <;> subst hb <;>
    simp only [Transpiler.IOp.Fields, opcode, funct3, BitVec.extractLsb] at hf <;>
    simp only [iOfWord, IInstr.encode, itypeF3] <;> (revert hf; bv_decide)

def iwOfWord (w : BitVec 32) : IwInstr := ⟨w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 12⟩

theorem iwOfWord_encode {w : BitVec 32} (h : decode w = some (.i .addiw)) : (iwOfWord w).encode = w := by
  have hf := decode_i_fields h
  simp only [Transpiler.IOp.Fields, opcode, funct3, BitVec.extractLsb] at hf
  simp only [iwOfWord, IwInstr.encode]; revert hf; bv_decide

/-! ### Shifts by an immediate -/

def _root_.Transpiler.SOp.Fields (w : BitVec 32) : Transpiler.SOp → Prop
  | .slli => opcode w = 19#7 ∧ funct3 w = 1#3 ∧ funct6 w = 0#6
  | .srli => opcode w = 19#7 ∧ funct3 w = 5#3 ∧ funct6 w = 0#6
  | .srai => opcode w = 19#7 ∧ funct3 w = 5#3 ∧ funct6 w = 16#6
  | .slliw => opcode w = 27#7 ∧ funct3 w = 1#3 ∧ funct7 w = 0#7
  | .srliw => opcode w = 27#7 ∧ funct3 w = 5#3 ∧ funct7 w = 0#7
  | .sraiw => opcode w = 27#7 ∧ funct3 w = 5#3 ∧ funct7 w = 32#7

set_option maxHeartbeats 2000000 in
theorem decode_shift_fields {w : BitVec 32} {op : Transpiler.SOp} (h : decode w = some (.shift op)) :
    op.Fields w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, Transpiler.SOp.Fields])

def _root_.Transpiler.SOp.base : Transpiler.SOp → Option sop
  | .slli => some .SLLI | .srli => some .SRLI | .srai => some .SRAI | _ => none

def _root_.Transpiler.SOp.word : Transpiler.SOp → Option sopw
  | .slliw => some .SLLIW | .srliw => some .SRLIW | .sraiw => some .SRAIW | _ => none

def sOfWord (r : sop) (w : BitVec 32) : SInstr := ⟨r, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 6⟩

theorem sOfWord_encode {w : BitVec 32} {op : Transpiler.SOp} {r : sop} (h : decode w = some (.shift op))
    (hb : op.base = some r) : (sOfWord r w).encode = w := by
  have hf := decode_shift_fields h
  cases op <;> simp only [Transpiler.SOp.base, reduceCtorEq, Option.some.injEq] at hb <;> subst hb <;>
    simp only [Transpiler.SOp.Fields, opcode, funct3, funct6, BitVec.extractLsb] at hf <;>
    simp only [sOfWord, SInstr.encode, shiftF3, shiftF6] <;> (revert hf; bv_decide)

def siwOfWord (r : sopw) (w : BitVec 32) : SiwInstr :=
  ⟨r, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem siwOfWord_encode {w : BitVec 32} {op : Transpiler.SOp} {r : sopw} (h : decode w = some (.shift op))
    (hb : op.word = some r) : (siwOfWord r w).encode = w := by
  have hf := decode_shift_fields h
  cases op <;> simp only [Transpiler.SOp.word, reduceCtorEq, Option.some.injEq] at hb <;> subst hb <;>
    simp only [Transpiler.SOp.Fields, opcode, funct3, funct7, BitVec.extractLsb] at hf <;>
    simp only [siwOfWord, SiwInstr.encode, shiftwF3, shiftwF7] <;> (revert hf; bv_decide)

/-! ### M -/

def _root_.Transpiler.MOp.Fields (w : BitVec 32) (op : Transpiler.MOp) : Prop :=
  opcode w = op.opc ∧ funct3 w = op.f3 ∧ funct7 w = 1#7

set_option maxHeartbeats 2000000 in
theorem decode_m_fields {w : BitVec 32} {op : Transpiler.MOp} (h : decode w = some (.m op)) : op.Fields w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, Transpiler.MOp.Fields, Transpiler.MOp.opc,
        Transpiler.MOp.f3])

def mOfWord (op : Transpiler.MOp) (w : BitVec 32) : MInstr :=
  ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem mOfWord_encode {w : BitVec 32} {op : Transpiler.MOp} (h : decode w = some (.m op)) :
    (mOfWord op w).encode = w := by
  have hf := decode_m_fields h
  cases op <;> simp only [Transpiler.MOp.Fields, Transpiler.MOp.opc, Transpiler.MOp.f3, opcode, funct3, funct7,
    BitVec.extractLsb] at hf <;> simp only [mOfWord, MInstr.encode, Transpiler.MOp.opc, Transpiler.MOp.f3] <;>
    (revert hf; bv_decide)

/-! ### Zba, Zbs, Zbb, Zbkb: their fields and encodings -/

def FieldsShadd (w : BitVec 32) (op : Transpiler.ShAdd) : Prop :=
  opcode w = 51#7 ∧ funct3 w = op.f3 ∧ funct7 w = 16#7

set_option maxHeartbeats 2000000 in
theorem decode_shadd_fields {w : BitVec 32} {op : Transpiler.ShAdd} (h : decode w = some (.shadd op)) : FieldsShadd w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsShadd, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsShadd, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def shaddOfWord (op : Transpiler.ShAdd) (w : BitVec 32) : ZbaInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem shaddOfWord_encode {w : BitVec 32} {op : Transpiler.ShAdd} (h : decode w = some (.shadd op)) :
    (shaddOfWord op w).encode = w := by
  have hf := decode_shadd_fields h
  cases op <;> simp only [FieldsShadd, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [shaddOfWord, ZbaInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsSlliuw (w : BitVec 32) : Prop :=
  opcode w = 27#7 ∧ funct3 w = 1#3 ∧ funct6 w = 2#6

set_option maxHeartbeats 2000000 in
theorem decode_slliuw_fields {w : BitVec 32} (h : decode w = some (.slliuw)) : FieldsSlliuw w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsSlliuw])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsSlliuw])

def slliuwOfWord (w : BitVec 32) : SlliuwInstr := ⟨w.extractLsb' 20 6, w.extractLsb' 15 5, w.extractLsb' 7 5⟩

theorem slliuwOfWord_encode {w : BitVec 32} (h : decode w = some (.slliuw)) : (slliuwOfWord w).encode = w := by
  have hf := decode_slliuw_fields h
  simp only [FieldsSlliuw, opcode, funct3, funct6, funct7, BitVec.extractLsb] at hf
  simp only [slliuwOfWord, SlliuwInstr.encode]; revert hf; bv_decide

def FieldsZbs (w : BitVec 32) (op : Transpiler.ZbsOp) : Prop :=
  opcode w = 51#7 ∧ funct3 w = zbsF3 op.r ∧ funct7 w = zbsF7 op.r

set_option maxHeartbeats 2000000 in
theorem decode_zbs_fields {w : BitVec 32} {op : Transpiler.ZbsOp} (h : decode w = some (.zbs op)) : FieldsZbs w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsZbs, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsZbs, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def zbsOfWord (op : Transpiler.ZbsOp) (w : BitVec 32) : ZbsInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem zbsOfWord_encode {w : BitVec 32} {op : Transpiler.ZbsOp} (h : decode w = some (.zbs op)) :
    (zbsOfWord op w).encode = w := by
  have hf := decode_zbs_fields h
  cases op <;> simp only [FieldsZbs, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [zbsOfWord, ZbsInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsZbsi (w : BitVec 32) (op : Transpiler.ZbsOp) : Prop :=
  opcode w = 19#7 ∧ funct3 w = zbsiF3 op.i ∧ funct6 w = zbsiF6 op.i

set_option maxHeartbeats 2000000 in
theorem decode_zbsi_fields {w : BitVec 32} {op : Transpiler.ZbsOp} (h : decode w = some (.zbsi op)) : FieldsZbsi w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsZbsi, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsZbsi, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def zbsiOfWord (op : Transpiler.ZbsOp) (w : BitVec 32) : ZbsiInstr := ⟨op, w.extractLsb' 20 6, w.extractLsb' 15 5, w.extractLsb' 7 5⟩

theorem zbsiOfWord_encode {w : BitVec 32} {op : Transpiler.ZbsOp} (h : decode w = some (.zbsi op)) :
    (zbsiOfWord op w).encode = w := by
  have hf := decode_zbsi_fields h
  cases op <;> simp only [FieldsZbsi, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [zbsiOfWord, ZbsiInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsZbb (w : BitVec 32) (op : Transpiler.ZbbOp) : Prop :=
  opcode w = 51#7 ∧ funct3 w = zbbF3 op.r ∧ funct7 w = zbbF7 op.r

set_option maxHeartbeats 2000000 in
theorem decode_zbb_fields {w : BitVec 32} {op : Transpiler.ZbbOp} (h : decode w = some (.zbb op)) : FieldsZbb w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsZbb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsZbb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def zbbOfWord (op : Transpiler.ZbbOp) (w : BitVec 32) : ZbbInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem zbbOfWord_encode {w : BitVec 32} {op : Transpiler.ZbbOp} (h : decode w = some (.zbb op)) :
    (zbbOfWord op w).encode = w := by
  have hf := decode_zbb_fields h
  cases op <;> simp only [FieldsZbb, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [zbbOfWord, ZbbInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsZbbw (w : BitVec 32) (op : Transpiler.ZbbWOp) : Prop :=
  opcode w = 59#7 ∧ funct3 w = zbbWF3 op.r ∧ funct7 w = 48#7

set_option maxHeartbeats 2000000 in
theorem decode_zbbw_fields {w : BitVec 32} {op : Transpiler.ZbbWOp} (h : decode w = some (.zbbw op)) : FieldsZbbw w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsZbbw, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsZbbw, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def zbbwOfWord (op : Transpiler.ZbbWOp) (w : BitVec 32) : ZbbWInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem zbbwOfWord_encode {w : BitVec 32} {op : Transpiler.ZbbWOp} (h : decode w = some (.zbbw op)) :
    (zbbwOfWord op w).encode = w := by
  have hf := decode_zbbw_fields h
  cases op <;> simp only [FieldsZbbw, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [zbbwOfWord, ZbbWInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsRori (w : BitVec 32) : Prop :=
  opcode w = 19#7 ∧ funct3 w = 5#3 ∧ funct6 w = 24#6

set_option maxHeartbeats 2000000 in
theorem decode_rori_fields {w : BitVec 32} (h : decode w = some (.rori)) : FieldsRori w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsRori])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsRori])

def roriOfWord (w : BitVec 32) : RoriInstr := ⟨w.extractLsb' 20 6, w.extractLsb' 15 5, w.extractLsb' 7 5⟩

theorem roriOfWord_encode {w : BitVec 32} (h : decode w = some (.rori)) : (roriOfWord w).encode = w := by
  have hf := decode_rori_fields h
  simp only [FieldsRori, opcode, funct3, funct6, funct7, BitVec.extractLsb] at hf
  simp only [roriOfWord, RoriInstr.encode]; revert hf; bv_decide

def FieldsRoriw (w : BitVec 32) : Prop :=
  opcode w = 27#7 ∧ funct3 w = 5#3 ∧ funct7 w = 48#7

set_option maxHeartbeats 2000000 in
theorem decode_roriw_fields {w : BitVec 32} (h : decode w = some (.roriw)) : FieldsRoriw w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsRoriw])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsRoriw])

def roriwOfWord (w : BitVec 32) : RoriwInstr := ⟨w.extractLsb' 20 5, w.extractLsb' 15 5, w.extractLsb' 7 5⟩

theorem roriwOfWord_encode {w : BitVec 32} (h : decode w = some (.roriw)) : (roriwOfWord w).encode = w := by
  have hf := decode_roriw_fields h
  simp only [FieldsRoriw, opcode, funct3, funct6, funct7, BitVec.extractLsb] at hf
  simp only [roriwOfWord, RoriwInstr.encode]; revert hf; bv_decide

def FieldsPack (w : BitVec 32) (op : Transpiler.PackOp) : Prop :=
  opcode w = 51#7 ∧ funct3 w = zbkbF3 op.r ∧ funct7 w = 4#7

set_option maxHeartbeats 2000000 in
theorem decode_pack_fields {w : BitVec 32} {op : Transpiler.PackOp} (h : decode w = some (.pack op)) : FieldsPack w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsPack, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsPack, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def packOfWord (op : Transpiler.PackOp) (w : BitVec 32) : PackInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem packOfWord_encode {w : BitVec 32} {op : Transpiler.PackOp} (h : decode w = some (.pack op)) :
    (packOfWord op w).encode = w := by
  have hf := decode_pack_fields h
  cases op <;> simp only [FieldsPack, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [packOfWord, PackInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsZexth (w : BitVec 32) : Prop :=
  opcode w = 59#7 ∧ funct3 w = 4#3 ∧ funct7 w = 4#7 ∧ w.extractLsb 24 20 = 0#5

set_option maxHeartbeats 2000000 in
theorem decode_zexth_fields {w : BitVec 32} (h : decode w = some (.zexth)) : FieldsZexth w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsZexth, Transpiler.rs2])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsZexth, Transpiler.rs2])

def zexthOfWord (w : BitVec 32) : ZexthInstr := ⟨w.extractLsb' 7 5, w.extractLsb' 15 5⟩

theorem zexthOfWord_encode {w : BitVec 32} (h : decode w = some (.zexth)) : (zexthOfWord w).encode = w := by
  have hf := decode_zexth_fields h
  simp only [FieldsZexth, opcode, funct3, funct6, funct7, BitVec.extractLsb] at hf
  simp only [zexthOfWord, ZexthInstr.encode]; revert hf; bv_decide

def FieldsSext (w : BitVec 32) (op : Transpiler.SextOp) : Prop :=
  opcode w = 19#7 ∧ funct3 w = 1#3 ∧ w.extractLsb 31 20 = sextF12 op.r

set_option maxHeartbeats 2000000 in
theorem decode_sext_fields {w : BitVec 32} {op : Transpiler.SextOp} (h : decode w = some (.sext op)) : FieldsSext w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsSext, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsSext, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def sextOfWord (op : Transpiler.SextOp) (w : BitVec 32) : SextInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5⟩

theorem sextOfWord_encode {w : BitVec 32} {op : Transpiler.SextOp} (h : decode w = some (.sext op)) :
    (sextOfWord op w).encode = w := by
  have hf := decode_sext_fields h
  cases op <;> simp only [FieldsSext, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [sextOfWord, SextInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsByte (w : BitVec 32) (op : Transpiler.ByteOp) : Prop :=
  opcode w = 19#7 ∧ funct3 w = 5#3 ∧ w.extractLsb 31 20 = op.f12

set_option maxHeartbeats 2000000 in
theorem decode_byte_fields {w : BitVec 32} {op : Transpiler.ByteOp} (h : decode w = some (.byte op)) : FieldsByte w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsByte, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsByte, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def byteOfWord (op : Transpiler.ByteOp) (w : BitVec 32) : ByteInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5⟩

theorem byteOfWord_encode {w : BitVec 32} {op : Transpiler.ByteOp} (h : decode w = some (.byte op)) :
    (byteOfWord op w).encode = w := by
  have hf := decode_byte_fields h
  cases op <;> simp only [FieldsByte, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [byteOfWord, ByteInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsCount (w : BitVec 32) (op : Transpiler.CountOp) : Prop :=
  opcode w = op.opc ∧ funct3 w = 1#3 ∧ w.extractLsb 31 20 = op.f12

set_option maxHeartbeats 2000000 in
theorem decode_count_fields {w : BitVec 32} {op : Transpiler.CountOp} (h : decode w = some (.count op)) : FieldsCount w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsCount, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsCount, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def countOfWord (op : Transpiler.CountOp) (w : BitVec 32) : CountInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 15 5⟩

theorem countOfWord_encode {w : BitVec 32} {op : Transpiler.CountOp} (h : decode w = some (.count op)) :
    (countOfWord op w).encode = w := by
  have hf := decode_count_fields h
  cases op <;> simp only [FieldsCount, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [countOfWord, CountInstr.encode, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsAdduw (w : BitVec 32) : Prop :=
  opcode w = 59#7 ∧ funct3 w = 0#3 ∧ funct7 w = 4#7

set_option maxHeartbeats 2000000 in
theorem decode_adduw_fields {w : BitVec 32} (h : decode w = some (.adduw)) : FieldsAdduw w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsAdduw])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsAdduw])

def adduwOfWord (w : BitVec 32) : UwInstr := ⟨none, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem adduwOfWord_encode {w : BitVec 32} (h : decode w = some (.adduw)) : (adduwOfWord w).encode = w := by
  have hf := decode_adduw_fields h
  simp only [FieldsAdduw, opcode, funct3, funct6, funct7, BitVec.extractLsb] at hf
  simp only [adduwOfWord, UwInstr.encode, UwInstr.f7, UwInstr.f3]; revert hf; bv_decide

def FieldsShadduw (w : BitVec 32) (op : Transpiler.ShAdd) : Prop :=
  opcode w = 59#7 ∧ funct3 w = op.f3 ∧ funct7 w = 16#7

set_option maxHeartbeats 2000000 in
theorem decode_shadduw_fields {w : BitVec 32} {op : Transpiler.ShAdd} (h : decode w = some (.shadduw op)) : FieldsShadduw w op := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsShadduw, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsShadduw, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc])

def shadduwOfWord (op : Transpiler.ShAdd) (w : BitVec 32) : UwInstr := ⟨some op, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem shadduwOfWord_encode {w : BitVec 32} {op : Transpiler.ShAdd} (h : decode w = some (.shadduw op)) :
    (shadduwOfWord op w).encode = w := by
  have hf := decode_shadduw_fields h
  cases op <;> simp only [FieldsShadduw, opcode, funct3, funct6, funct7, BitVec.extractLsb, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf <;>
    simp only [shadduwOfWord, UwInstr.encode, UwInstr.f7, UwInstr.f3, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, zbsF3, zbsF7, zbsiF3, zbsiF6, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, zbbF3, zbbF7, Transpiler.ZbbOp.r, zbbWF3, Transpiler.ZbbWOp.r, zbkbF3, Transpiler.PackOp.r, sextF12, Transpiler.SextOp.r, Transpiler.ByteOp.f12, Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> (revert hf; bv_decide)

def FieldsPackw (w : BitVec 32) : Prop :=
  opcode w = 59#7 ∧ funct3 w = 4#3 ∧ funct7 w = 4#7 ∧ w.extractLsb' 20 5 ≠ 0#5

set_option maxHeartbeats 2000000 in
theorem decode_packw_fields {w : BitVec 32} (h : decode w = some .packw) : FieldsPackw w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsPackw, Transpiler.rs2, BitVec.extractLsb])
    | (cases h; simp_all [BitVec.toNat_eq, FieldsPackw, Transpiler.rs2, BitVec.extractLsb])

def packwOfWord (w : BitVec 32) (h : w.extractLsb' 20 5 ≠ 0#5) : PackwInstr :=
  ⟨w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5, h⟩

theorem packwOfWord_encode {w : BitVec 32} (h : decode w = some .packw) :
    (packwOfWord w (decode_packw_fields h).2.2.2).encode = w := by
  have hf := decode_packw_fields h
  simp only [FieldsPackw, opcode, funct3, funct7, BitVec.extractLsb] at hf
  simp only [packwOfWord, PackwInstr.encode]; revert hf; bv_decide

/-! ### The fences, lui and auipc -/

def FieldsFence (w : BitVec 32) : Prop := opcode w = 15#7 ∧ (funct3 w = 0#3 ∨ funct3 w = 1#3)

set_option maxHeartbeats 2000000 in
theorem decode_fence_fields {w : BitVec 32} (h : decode w = some .fence) : FieldsFence w := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [BitVec.toNat_eq, FieldsFence]; try omega)
    | (cases h; simp_all [BitVec.toNat_eq, FieldsFence]; try omega)

def fenceOfWord (w : BitVec 32) : FenceInstr :=
  ⟨w.extractLsb' 20 12, w.extractLsb' 15 5, w.extractLsb' 12 3 == 1#3, w.extractLsb' 7 5⟩

theorem fenceOfWord_encode {w : BitVec 32} (h : decode w = some .fence) : (fenceOfWord w).encode = w := by
  have hf := decode_fence_fields h
  simp only [FieldsFence, opcode, funct3, BitVec.extractLsb] at hf
  simp only [fenceOfWord, FenceInstr.encode, FenceInstr.f3]
  rcases hf with ⟨h1, h2 | h2⟩ <;> simp only [h2] <;> (revert h1 h2; bv_decide)

set_option maxHeartbeats 2000000 in
theorem decode_u_opcode {w : BitVec 32} (h : decode w = some .lui ∨ decode w = some .auipc) :
    opcode w = 55#7 ∨ opcode w = 23#7 := by
  rcases h with h | h <;> simp only [decode] at h <;> repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1; simp_all [BitVec.toNat_eq])
    | (cases h; simp_all [BitVec.toNat_eq])

/-! ### The plain instructions but lui and auipc, assembled -/

/-- A plain word at pc, but lui and auipc: a step the simulation covers. -/
theorem step_plain_other {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {i : Transpiler.Inst} (hd : decode w = some i) (hg : group i = .plain)
    (hu : i ≠ .lui ∧ i ≠ .auipc) : OneOkT ziskRegions zrom S.rv := by
  cases i with
  | r op =>
    cases hb : op.base with
    | some r => exact step_r hx hd hb
    | none =>
      obtain ⟨r, hr⟩ : ∃ r, op.word = some r := by
        cases op <;> simp_all [Transpiler.ROp.base, Transpiler.ROp.word]
      have h := ctx_at hx hd (by rfl) (rwOfWord_encode hd hr)
      exact ⟨pc, hx.atPc, .inl ⟨.rw (rwOfWord r w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | i op =>
    cases hb : op.base with
    | some r =>
        have h := ctx_at hx hd (by rfl) (iOfWord_encode hd hb)
        exact ⟨pc, hx.atPc, .inl ⟨.i (iOfWord r w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
    | none =>
      have hop : op = .addiw := by cases op <;> simp_all [Transpiler.IOp.base]
      subst hop
      have h := ctx_at hx hd (by rfl) (iwOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.iw (iwOfWord w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | shift op =>
    cases hb : op.base with
    | some r =>
        have h := ctx_at hx hd (by rfl) (sOfWord_encode hd hb)
        exact ⟨pc, hx.atPc, .inl ⟨.sh (sOfWord r w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
    | none =>
      obtain ⟨r, hr⟩ : ∃ r, op.word = some r := by
        cases op <;> simp_all [Transpiler.SOp.base, Transpiler.SOp.word]
      have h := ctx_at hx hd (by rfl) (siwOfWord_encode hd hr)
      exact ⟨pc, hx.atPc, .inl ⟨.siw (siwOfWord r w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | m op =>
      have h := ctx_at hx hd (by rfl) (mOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.m (mOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | fence =>
      have h := ctx_at hx hd (by rfl) (fenceOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.fence (fenceOfWord w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | shadd op =>
      have h := ctx_at hx hd (by rfl) (shaddOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.zba (shaddOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | slliuw =>
      have h := ctx_at hx hd (by rfl) (slliuwOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.slliuw (slliuwOfWord w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | zbs op =>
      have h := ctx_at hx hd (by rfl) (zbsOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.zbs (zbsOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | zbsi op =>
      have h := ctx_at hx hd (by rfl) (zbsiOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.zbsi (zbsiOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | zbb op =>
      have h := ctx_at hx hd (by rfl) (zbbOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.zbb (zbbOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | zbbw op =>
      have h := ctx_at hx hd (by rfl) (zbbwOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.zbbw (zbbwOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | rori =>
      have h := ctx_at hx hd (by rfl) (roriOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.rori (roriOfWord w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | roriw =>
      have h := ctx_at hx hd (by rfl) (roriwOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.roriw (roriwOfWord w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | pack op =>
      have h := ctx_at hx hd (by rfl) (packOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.pack (packOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | packw =>
      have h := ctx_at hx hd (by rfl) (packwOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.packw (packwOfWord w (decode_packw_fields hd).2.2.2), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | sext op =>
      have h := ctx_at hx hd (by rfl) (sextOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.sext (sextOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | zexth =>
      have h := ctx_at hx hd (by rfl) (zexthOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.zexth (zexthOfWord w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | byte op =>
      have h := ctx_at hx hd (by rfl) (byteOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.byte (byteOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | count op =>
      have h := ctx_at hx hd (by rfl) (countOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inl ⟨.count (countOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩⟩
  | adduw =>
      have h := ctx_at hx hd (by rfl) (adduwOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inr (.inr (.inl ⟨adduwOfWord w, h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩))⟩
  | shadduw op =>
      have h := ctx_at hx hd (by rfl) (shadduwOfWord_encode hd)
      exact ⟨pc, hx.atPc, .inr (.inr (.inl ⟨shadduwOfWord op w, h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩))⟩
  | lui => exact absurd rfl hu.1
  | auipc => exact absurd rfl hu.2
  | _ => simp [group] at hg

/-! ### lui and auipc, the fused pair among them -/

set_option maxHeartbeats 2000000 in
theorem decode_op_lui {w : BitVec 32} (h : decode w = some .lui) : opcode w = 55#7 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first | (simp at h; done) | (cases h; simp_all [BitVec.toNat_eq])

set_option maxHeartbeats 2000000 in
theorem decode_op_auipc {w : BitVec 32} (h : decode w = some .auipc) : opcode w = 23#7 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first | (simp at h; done) | (cases h; simp_all [BitVec.toNat_eq])

set_option maxHeartbeats 2000000 in
theorem decode_jalr_fields {w : BitVec 32} (h : decode w = some .jalr) : opcode w = 103#7 ∧ funct3 w = 0#3 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1; simp_all [BitVec.toNat_eq])
    | (cases h; simp_all [BitVec.toNat_eq])

def uOfWord (op : uop) (w : BitVec 32) : UInstr := ⟨op, w.extractLsb' 7 5, w.extractLsb' 12 20⟩

theorem uOfWord_lui {w : BitVec 32} (h : decode w = some .lui) : (uOfWord .LUI w).encode = w := by
  have hf := decode_op_lui h
  simp only [opcode, BitVec.extractLsb] at hf
  simp only [uOfWord, UInstr.encode, uopEnc]; revert hf; bv_decide

theorem uOfWord_auipc {w : BitVec 32} (h : decode w = some .auipc) : (uOfWord .AUIPC w).encode = w := by
  have hf := decode_op_auipc h
  simp only [opcode, BitVec.extractLsb] at hf
  simp only [uOfWord, UInstr.encode, uopEnc]; revert hf; bv_decide

/-- What `fuses` says of the word after an auipc: a jalr, through the
    auipc's rd, not x0, linking to rd or x0. -/
theorem fuses_facts {w n : BitVec 32} (hf : Transpiler.fuses w n = true) :
    decode n = some .jalr ∧ Transpiler.rd w ≠ 0 ∧ Transpiler.rs1 n = Transpiler.rd w ∧
      (Transpiler.rd n = Transpiler.rd w ∨ Transpiler.rd n = 0) := by
  simp only [Transpiler.fuses, Bool.and_eq_true, decide_eq_true_eq, Bool.or_eq_true] at hf
  obtain ⟨⟨⟨h1, h2⟩, h3⟩, h4⟩ := hf
  refine ⟨?_, h2, h3, h4⟩
  cases hn : decode n with
  | none => rw [hn] at h1; cases h1
  | some i => rw [hn] at h1; cases i <;> first | rfl | cases h1

/-- The fused pair a word and the next make. -/
def fOfWords (w n : BitVec 32) : FInstr :=
  ⟨w.extractLsb' 7 5, w.extractLsb' 12 20, n.extractLsb' 7 5, n.extractLsb' 20 12⟩

theorem fOfWords_jalr {w n : BitVec 32} (hf : Transpiler.fuses w n = true) : (fOfWords w n).jalr.encode = n := by
  obtain ⟨hj, -, hr, -⟩ := fuses_facts hf
  have hjf := decode_jalr_fields hj
  have hr' : n.extractLsb' 15 5 = w.extractLsb' 7 5 := by
    apply BitVec.eq_of_toNat_eq; simpa [Transpiler.rs1, Transpiler.rd, BitVec.extractLsb] using hr
  simp only [opcode, funct3, BitVec.extractLsb] at hjf
  simp only [fOfWords, FInstr.jalr, JrInstr.encode]; revert hjf hr'; bv_decide

theorem fOfWords_rd {w n : BitVec 32} (hf : Transpiler.fuses w n = true) : (fOfWords w n).rd ≠ 0 := by
  obtain ⟨-, h, -, -⟩ := fuses_facts hf
  intro e; apply h
  have := congrArg BitVec.toNat e
  simpa [fOfWords, Transpiler.rd, BitVec.extractLsb] using this

theorem fOfWords_link {w n : BitVec 32} (hf : Transpiler.fuses w n = true) :
    (fOfWords w n).rd2 = (fOfWords w n).rd ∨ (fOfWords w n).rd2 = 0 := by
  obtain ⟨-, -, -, h⟩ := fuses_facts hf
  rcases h with h | h
  · left; apply BitVec.eq_of_toNat_eq; simpa [fOfWords, Transpiler.rd, BitVec.extractLsb] using h
  · right; apply BitVec.eq_of_toNat_eq; simpa [fOfWords, Transpiler.rd, BitVec.extractLsb] using h

theorem jalrTarget_lsb (v : BitVec 64) (imm : BitVec 12) : (jalrTarget v imm)[0] = false := by
  simp only [jalrTarget, Sail.BitVec.update, Sail.BitVec.updateSubrange']; bv_decide

/-- lui at pc, or auipc that does not fuse with the word after it: a step
    the simulation covers. -/
theorem step_u_plain {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {i : Transpiler.Inst} (hd : decode w = some i) (op : uop) (he : (uOfWord op w).encode = w)
    (hg : group i = .plain)
    (hnf : op = .AUIPC → ∀ n, nextWord S.rv pc = some n → Transpiler.fuses w n = false) :
    OneOkT ziskRegions zrom S.rv := by
  have h := ctx_at hx hd hg he
  exact ⟨pc, hx.atPc, .inl ⟨.u (uOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1,
    fun hk n hn => by rw [he]; exact hnf hk n hn, h.2.2.2.2.2.2.2⟩⟩

/-- lui and auipc at pc: a step the simulation covers; or, auipc fused with
    the jalr after it, the pair, covered, its target 4-aligned, or one of
    the ends, its target 2 mod 4. -/
theorem step_u {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {i : Transpiler.Inst} (hd : decode w = some i) (hu : i = .lui ∨ i = .auipc) : StepCase zrom C S := by
  rcases hu with rfl | rfl
  · exact .inl (.inl (step_u_plain hx hd .LUI (uOfWord_lui hd) rfl (fun hk => by cases hk)))
  have ha := uOfWord_auipc hd
  cases hn : nextWord S.rv pc with
  | none => exact .inl (.inl (step_u_plain hx hd .AUIPC ha rfl (fun _ n h => by rw [hn] at h; cases h)))
  | some n =>
    cases hf : Transpiler.fuses w n with
    | false =>
      exact .inl (.inl (step_u_plain hx hd .AUIPC ha rfl (fun _ n' h => by
        rw [hn] at h; cases h; exact hf)))
    | true =>
      -- the pair: the jalr after it in the code too
      have hc4 : C (pc + 4) := hx.next1 (by simp [Transpiler.usesNext, hd])
      have hal := hx.aligned pc hx.code
      have hal4 := hx.aligned _ hc4
      have hj := fOfWords_jalr hf
      have hau : (fOfWords w n).auipc.encode = w := ha
      have hb := readWord_bytes hx.word
      rw [← hau] at hb
      have hn' : ZiskRiscv.readWord S.rv.mem (pc + 4).toNat = some n := hn
      have hg := readWord_bytes hn'
      rw [← hj] at hg
      -- the ZisK program: the spec's translation of the auipc, the jalr after it
      have htr : Transpiled zrom pc (fOfWords w n).auipc.encode (some (fOfWords w n).jalr.encode) := by
        obtain ⟨l, hl⟩ := convert_some_plain hd (by rfl) pc 0 (some n) none
        have h := hx.spec pc w hx.code hx.word ⟨0, l, by
          rw [convert_ignores_next2 (usesNext2_plain hd (by rfl)) _ _ _ none, hn']; exact hl⟩
        rw [hn'] at h
        rw [hau, hj]; exact h.drop2 ⟨0, l, hl⟩
      have hx1 := hx.exec pc hx.code
      have hx3 := hx.exec _ hc4
      cases ht : ((fOfWords w n).T pc)[1] with
      | false =>
        exact .inr (.inl ⟨pc, fOfWords w n, hx.atPc, ⟨hal, hx1, exec_add2 hx1 hal, hx3, exec_add2 hx3 hal4,
          hb.1, hb.2.1, hb.2.2.1, hb.2.2.2, hg.1, hg.2.1, hg.2.2.1, hg.2.2.2, fOfWords_rd hf, fOfWords_link hf,
          ⟨jalrTarget_lsb _ _, ht⟩, htr⟩⟩)
      | true =>
        exact .inr (.inr (.inr (.inl (.inr (.inr (.inr (.inr (.inr (.inr (⟨pc, fOfWords w n, hx.atPc,
          ⟨hal, hx1, exec_add2 hx1 hal, hx3, exec_add2 hx3 hal4,
            hb.1, hb.2.1, hb.2.2.1, hb.2.2.2, hg.1, hg.2.1, hg.2.2.1, hg.2.2.2, fOfWords_rd hf, fOfWords_link hf,
            htr⟩, jalrTarget_lsb _ _, ht⟩))))))))))

/-- `KindCases.plain`: a plain word at pc, the step's cases. -/
theorem kind_plain {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} :
    ∀ S pc w i, Ctx zrom C S pc w → decode w = some i → group i = .plain → StepCase zrom C S := by
  intro S pc w i hx hd hg
  by_cases hu : i = .lui ∨ i = .auipc
  · exact step_u hx hd hu
  · exact .inl (.inl (step_plain_other hx hd hg ⟨fun e => hu (.inl e), fun e => hu (.inr e)⟩))

end Equiv.Final
