/-
  The simulation, one step, on Zba's single-instruction forms, any
  registers: sh1add, sh2add, sh3add (the ZisK op of the same name) and
  slli.uw (sll_u_w), as the spec translates them.
-/
import Equiv.X0Sim
import Equiv.RiscvZba
import Equiv.RiscvDecodeZba

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (ShAdd)

/-! ## sh1add, sh2add, sh3add -/

/-- The op's shift amount, as sail-riscv's ZBA_RTYPE carries it. -/
def _root_.Transpiler.ShAdd.sh : ShAdd → BitVec 2
  | .sh1 => 1 | .sh2 => 2 | .sh3 => 3

/-- funct3: the shift amount, times two. -/
def _root_.Transpiler.ShAdd.f3 (op : ShAdd) : BitVec 3 := op.sh ++ 0#1

theorem _root_.Transpiler.ShAdd.zval (op : ShAdd) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = zbaVal op.sh a b := by
  cases op <;> simp [Transpiler.ShAdd.zop, Transpiler.ShAdd.sh, _root_.Zisk.Functions.zisk_exec, zbaVal,
    shift_bits_left] <;> bv_decide

theorem _root_.Transpiler.ShAdd.pure (op : ShAdd) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.ShAdd.zop] | decide

structure ZbaInstr where
  op : ShAdd
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def ZbaInstr.encode (i : ZbaInstr) : BitVec 32 :=
  0x10#7 ++ i.rs2 ++ i.rs1 ++ i.op.f3 ++ i.rd ++ 0x33#7

theorem ZbaInstr.fields (i : ZbaInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x33#7 ∧ v.extractLsb 31 25 = 0x10#7 ∧ v.extractLsb 12 12 = 0#1 ∧
    v.extractLsb 14 13 = i.op.sh ∧ v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    v.extractLsb 24 20 = i.rs2 ∧ (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, ZbaInstr.encode, byte, Transpiler.ShAdd.f3]
  cases i.op <;> simp only [Transpiler.ShAdd.sh] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem ZbaInstr.spec_fields (i : ZbaInstr) :
    Transpiler.opcode i.encode = 0x33#7 ∧ Transpiler.funct3 i.encode = i.op.f3 ∧
    Transpiler.funct7 i.encode = 0x10#7 ∧ Transpiler.rd i.encode = i.rd.toNat ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.rs2 i.encode = i.rs2.toNat := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.funct7, Transpiler.rd, Transpiler.rs1,
    Transpiler.rs2, ZbaInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> first | bv_decide | (congr 1; bv_decide)

theorem ZbaInstr.zisk_of_spec (i : ZbaInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  obtain ⟨hop, hf3, hf7, hrd, hr1, hr2⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, Transpiler.ShAdd.f3, Transpiler.ShAdd.sh, hr1, hr2,
      hrd, Transpiler.registerOp]

structure ZbaAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbaInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_zba_rtype_eq (a b c : regidx) (sh : BitVec 2) :
    execute (.ZBA_RTYPE (a, b, c, sh)) = execute_ZBA_RTYPE a b c sh := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on sh1add, sh2add or sh3add, any registers, keeps the
    states related. -/
theorem zbastep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbaInstr) (next : Option (BitVec 32))
    (hi : ZbaAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf7, h12, hsh, hrdx, hr1x, hr2x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have hsh0 : (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 14 13 ≠ 0#2 := by
    rw [hsh]; cases i.op <;> decide
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have v1 := getX_regv agree l1
  have v2 := getX_regv agree l2
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.ZBA_RTYPE (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.op.sh)) ?_
    i.rd.toNat i.rd.isLt (zbaVal i.op.sh (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zba t _ hop hf7 h12 hsh0 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hr2x, hsh] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zba_rtype_eq, execute_zba_rtype t i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.ShAdd.zval]

/-! ## slli.uw -/

theorem sll_u_w_val (sh : BitVec 6) (a : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_SLL_U_W ()) a (sh.setWidth 64)).1 = slliuwVal sh a := by
  simp [_root_.Zisk.Functions.zisk_exec, slliuwVal, shift_bits_left, zero_extend, Sail.BitVec.zeroExtend,
    Sail.BitVec.extractLsb]
  simp only [Zisk.Functions.zero_extend, Sail.BitVec.zeroExtend, BitVec.toNatInt]
  have e : (BitVec.extractLsb 5 0 sh).toNat = sh.toNat := by
    simp [BitVec.extractLsb, BitVec.extractLsb']
  show BitVec.shiftLeft _ (BitVec.extractLsb 5 0 sh).toNat = _
  rw [e]; rfl

theorem sll_u_w_pure : Equiv.Zisk.PureOp (.ZOP_SLL_U_W ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

structure SlliuwInstr where
  sh : BitVec 6
  rs1 : BitVec 5
  rd : BitVec 5

def SlliuwInstr.encode (i : SlliuwInstr) : BitVec 32 :=
  2#6 ++ i.sh ++ i.rs1 ++ 1#3 ++ i.rd ++ 0x1B#7

theorem SlliuwInstr.fields (i : SlliuwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x1B#7 ∧ v.extractLsb 14 12 = 1#3 ∧ v.extractLsb 31 26 = 2#6 ∧
    v.extractLsb 25 20 = i.sh ∧ v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, SlliuwInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem SlliuwInstr.zisk_of_spec (i : SlliuwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, i.sh.setWidth 64) (.ZOP_SLL_U_W ())
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have ho : Transpiler.opcode i.encode = 0x1B#7 := by simp only [Transpiler.opcode, SlliuwInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 1#3 := by simp only [Transpiler.funct3, SlliuwInstr.encode]; bv_decide
  have hf6 : Transpiler.funct6 i.encode = 2#6 := by simp only [Transpiler.funct6, SlliuwInstr.encode]; bv_decide
  have h12 : i.encode.extractLsb 31 20 ≠ 0x600#12 ∧ i.encode.extractLsb 31 20 ≠ 0x601#12 ∧
      i.encode.extractLsb 31 20 ≠ 0x602#12 := by
    simp only [SlliuwInstr.encode]; refine ⟨?_, ?_, ?_⟩ <;> bv_decide
  obtain ⟨e0, e1, e2⟩ := h12
  have n0 : (i.encode.extractLsb 31 20).toNat ≠ 0x600 := fun e => e0 (BitVec.eq_of_toNat_eq (by simpa using e))
  have n1 : (i.encode.extractLsb 31 20).toNat ≠ 0x601 := fun e => e1 (BitVec.eq_of_toNat_eq (by simpa using e))
  have n2 : (i.encode.extractLsb 31 20).toNat ≠ 0x602 := fun e => e2 (BitVec.eq_of_toNat_eq (by simpa using e))
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, SlliuwInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, SlliuwInstr.encode]; congr 1; bv_decide
  have hsh : Transpiler.shamt i.encode = i.sh.setWidth 64 := by
    simp only [Transpiler.shamt, SlliuwInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  simp at n0 n1 n2
  simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf6, n0, n1, n2, hrd, hr1, hsh, Transpiler.immediateOp]

structure SlliuwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SlliuwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_slliuw_eq (sh : BitVec 6) (a b : regidx) :
    execute (.SLLIUW (sh, a, b)) = execute_SLLIUW sh a b := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on slli.uw, any registers, keeps the states related. -/
theorem slliuwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SlliuwInstr) (next : Option (BitVec 32))
    (hi : SlliuwAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf6, hshx, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.SLLIUW (i.sh, .Regidx i.rs1, .Regidx i.rd)) ?_
    i.rd.toNat i.rd.isLt (slliuwVal i.sh (regv xs i.rs1.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_slliuw t _ hop hf3 hf6 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hshx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_slliuw_eq, execute_slliuw t i.rs1 i.rd _ (getX_of_agree he l1 v1) i.sh,
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ sll_u_w_pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, sll_u_w_val]

end Equiv.Sim
