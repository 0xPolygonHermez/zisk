/-
  The simulation, one step, on Zbs, any registers: bclr, bext, binv, bset
  and their immediate forms, and the ZisK op of the same name the spec
  translates each to. sail-zisk clears, sets, flips or reads the bit by
  index; sail-riscv masks with 1 shifted to it: the same.
-/
import Equiv.ZbaSim
import Equiv.RiscvZbs
import Equiv.RiscvDecodeZbs

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (ZbsOp)

/-! ## The bit ops, both ways -/

theorem upd0 (a : BitVec 64) (k : Nat) : Sail.BitVec.update a k 0#1 = a &&& ~~~(1#64 <<< k) := by
  simp only [Sail.BitVec.update, Sail.BitVec.updateSubrange']
  have : (BitVec.allOnes 1).zeroExtend 64 = 1#64 := by decide
  rw [this]
  generalize 1#64 <<< k = m
  simp; bv_decide

theorem upd1 (a : BitVec 64) (k : Nat) : Sail.BitVec.update a k 1#1 = a ||| (1#64 <<< k) := by
  simp only [Sail.BitVec.update, Sail.BitVec.updateSubrange']
  have : (BitVec.allOnes 1).zeroExtend 64 = 1#64 := by decide
  have h1 : (1#1).zeroExtend 64 = 1#64 := by decide
  rw [this, h1]
  generalize 1#64 <<< k = m
  bv_decide

theorem bit_mask (a : BitVec 64) (k : Nat) (hk : k < 64) :
    (a &&& (1#64 <<< k) != 0#64) = a.getLsbD k := by
  by_cases h : a.getLsbD k
  · simp only [h]
    simp only [bne_iff_ne, ne_eq]
    intro e
    have := congrArg (fun x => x.getLsbD k) e
    rw [BitVec.getLsbD_eq_getElem hk] at h
    simp [hk] at this
    rw [h] at this; cases this
  · simp only [h, Bool.false_eq_true]
    simp only [bne_eq_false_iff_eq]
    apply BitVec.eq_of_getLsbD_eq; intro i hi
    simp [BitVec.getLsbD_shiftLeft]
    intro ha _ _ he
    have : i = k := by omega
    subst this; exact h ha

theorem and_mask (a : BitVec 64) (k : Nat) (hk : k < 64) :
    a &&& (1#64 <<< k) = if a.getLsbD k then 1#64 <<< k else 0#64 := by
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  by_cases h : a.getLsbD k <;> simp [h, BitVec.getLsbD_shiftLeft] <;> intro _ _ <;>
    (by_cases e : i = k) <;> simp_all <;> omega

theorem ext_k (b : BitVec 64) : (Sail.BitVec.extractLsb b 5 0).toNat < 64 := by
  simp [Sail.BitVec.extractLsb]; omega

theorem bclr_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_BCLR ()) a b).1 = zbsRVal .BCLR a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbsRVal, zbsOp, zbsMask, shift_bits_left, zero_extend, Sail.BitVec.zeroExtend]
  exact upd0 _ _

theorem bset_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_BSET ()) a b).1 = zbsRVal .BSET a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbsRVal, zbsOp, zbsMask, shift_bits_left, zero_extend, Sail.BitVec.zeroExtend]
  exact upd1 _ _

theorem access_k (a : BitVec 64) (k : Nat) (hk : k < 64) : BitVec.access a k = BitVec.ofBool (a.getLsbD k) := by
  simp [BitVec.access, BitVec.getLsbD_eq_getElem hk, hk]

theorem binv_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_BINV ()) a b).1 = zbsRVal .BINV a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbsRVal, zbsOp, zbsMask, shift_bits_left, zero_extend, Sail.BitVec.zeroExtend]
  have hk := ext_k b
  generalize (Sail.BitVec.extractLsb b 5 0).toNat = k at hk ⊢
  rw [access_k a k hk]
  have ham := and_mask a k hk
  by_cases hb : a.getLsbD k
  · simp only [hb, BitVec.ofBool_true, if_true] at ham ⊢
    rw [if_pos (by decide), upd0]
    generalize 1#64 <<< k = m at ham ⊢
    revert ham; bv_decide
  · simp only [hb, BitVec.ofBool_false, Bool.false_eq_true, if_false] at ham ⊢
    rw [if_neg (by decide), upd1]
    generalize 1#64 <<< k = m at ham ⊢
    revert ham; bv_decide

theorem bext_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_BEXT ()) a b).1 = zbsRVal .BEXT a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbsRVal, zbsOp, zbsMask, shift_bits_left, zero_extend, Sail.BitVec.zeroExtend]
  have hk := ext_k b
  generalize (Sail.BitVec.extractLsb b 5 0).toNat = k at hk ⊢
  rw [access_k a k hk, bit_mask a k hk]
  by_cases hb : a.getLsbD k <;> simp [hb, bool_to_bit] <;> rfl


/-! ## Each op: its encodings, sail-riscv's instructions -/

def _root_.Transpiler.ZbsOp.r : ZbsOp → brop_zbs
  | .bclr => .BCLR | .bext => .BEXT | .binv => .BINV | .bset => .BSET

def _root_.Transpiler.ZbsOp.i : ZbsOp → biop_zbs
  | .bclr => .BCLRI | .bext => .BEXTI | .binv => .BINVI | .bset => .BSETI

theorem _root_.Transpiler.ZbsOp.zval (op : ZbsOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = zbsRVal op.r a b := by
  cases op
  · exact bclr_val a b
  · exact bext_val a b
  · exact binv_val a b
  · exact bset_val a b

theorem zbs_imm (op : ZbsOp) (sh : BitVec 6) (a : BitVec 64) : zbsRVal op.r a (sh.setWidth 64) = zbsIVal op.i sh a := by
  have e : Sail.BitVec.extractLsb (sh.setWidth 64) 5 0 = sh := by
    simp only [Sail.BitVec.extractLsb]; bv_decide
  cases op <;> simp [zbsRVal, zbsIVal, zbsOp, zbsIOp, Transpiler.ZbsOp.r, Transpiler.ZbsOp.i, e]

theorem _root_.Transpiler.ZbsOp.pure (op : ZbsOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.ZbsOp.zop] | decide

theorem _root_.Transpiler.ZbsOp.f7r (op : ZbsOp) : zbsF7 op.r = (match op with
    | .bclr => 0x24#7 | .bext => 0x24#7 | .binv => 0x34#7 | .bset => 0x14#7) := by cases op <;> rfl

/-! ## Register forms -/

structure ZbsInstr where
  op : ZbsOp
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def ZbsInstr.encode (i : ZbsInstr) : BitVec 32 :=
  zbsF7 i.op.r ++ i.rs2 ++ i.rs1 ++ zbsF3 i.op.r ++ i.rd ++ 0x33#7

theorem ZbsInstr.fields (i : ZbsInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x33#7 ∧ v.extractLsb 31 25 = zbsF7 i.op.r ∧ v.extractLsb 14 12 = zbsF3 i.op.r ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, ZbsInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.ZbsOp.r, zbsF7, zbsF3] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem ZbsInstr.zisk_of_spec (i : ZbsInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, ZbsInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, ZbsInstr.encode]; congr 1; bv_decide
  have hr2 : Transpiler.rs2 i.encode = i.rs2.toNat := by
    simp only [Transpiler.rs2, ZbsInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  cases hk : i.op <;>
  · have ho : Transpiler.opcode i.encode = 0x33#7 := by
      simp only [Transpiler.opcode, ZbsInstr.encode]; bv_decide
    have hf3 : Transpiler.funct3 i.encode = zbsF3 i.op.r := by
      simp only [Transpiler.funct3, ZbsInstr.encode]; bv_decide
    have hf7 : Transpiler.funct7 i.encode = zbsF7 i.op.r := by
      simp only [Transpiler.funct7, ZbsInstr.encode]; bv_decide
    rw [hk] at hf3 hf7
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, zbsF3, zbsF7, Transpiler.ZbsOp.r, hr1, hr2, hrd,
      Transpiler.registerOp]

structure ZbsAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbsInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_zbs_rtype_eq (a b c : regidx) (op : brop_zbs) :
    execute (.ZBS_RTYPE (a, b, c, op)) = execute_ZBS_RTYPE a b c op := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on a Zbs register op, any registers, keeps the states
    related. -/
theorem zbsstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbsInstr) (next : Option (BitVec 32))
    (hi : ZbsAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf7, hf3, hrdx, hr1x, hr2x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have v1 := getX_regv agree l1
  have v2 := getX_regv agree l2
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.ZBS_RTYPE (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.op.r)) ?_
    i.rd.toNat i.rd.isLt (zbsRVal i.op.r (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbs_r t _ i.op.r hop hf7 hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hr2x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zbs_rtype_eq, execute_zbs_rtype t i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.ZbsOp.zval]

/-! ## Immediate forms -/

structure ZbsiInstr where
  op : ZbsOp
  sh : BitVec 6
  rs1 : BitVec 5
  rd : BitVec 5

def ZbsiInstr.encode (i : ZbsiInstr) : BitVec 32 :=
  zbsiF6 i.op.i ++ i.sh ++ i.rs1 ++ zbsiF3 i.op.i ++ i.rd ++ 0x13#7

theorem ZbsiInstr.fields (i : ZbsiInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x13#7 ∧ v.extractLsb 31 26 = zbsiF6 i.op.i ∧ v.extractLsb 14 12 = zbsiF3 i.op.i ∧
    v.extractLsb 25 20 = i.sh ∧ v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, ZbsiInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.ZbsOp.i, zbsiF6, zbsiF3] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem ZbsiInstr.zisk_of_spec (i : ZbsiInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, i.sh.setWidth 64) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, ZbsiInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, ZbsiInstr.encode]; congr 1; bv_decide
  have hsh : Transpiler.shamt i.encode = i.sh.setWidth 64 := by
    simp only [Transpiler.shamt, ZbsiInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  cases hk : i.op <;>
  · have ho : Transpiler.opcode i.encode = 0x13#7 := by
      simp only [Transpiler.opcode, ZbsiInstr.encode]; bv_decide
    have hf3 : Transpiler.funct3 i.encode = zbsiF3 i.op.i := by
      simp only [Transpiler.funct3, ZbsiInstr.encode]; bv_decide
    have hf6 : Transpiler.funct6 i.encode = zbsiF6 i.op.i := by
      simp only [Transpiler.funct6, ZbsiInstr.encode]; bv_decide
    rw [hk] at hf3 hf6
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf6, zbsiF3, zbsiF6, Transpiler.ZbsOp.i, hr1, hrd, hsh,
      Transpiler.immediateOp]

structure ZbsiAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbsiInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_zbs_iop_eq (sh : BitVec 6) (a b : regidx) (op : biop_zbs) :
    execute (.ZBS_IOP (sh, a, b, op)) = execute_ZBS_IOP sh a b op := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on a Zbs immediate op, any registers, keeps the states
    related. -/
theorem zbsistep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbsiInstr) (next : Option (BitVec 32))
    (hi : ZbsiAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf6, hf3, hshx, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.ZBS_IOP (i.sh, .Regidx i.rs1, .Regidx i.rd, i.op.i)) ?_
    i.rd.toNat i.rd.isLt (zbsIVal i.op.i i.sh (regv xs i.rs1.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbs_i t _ i.op.i hop hf6 hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hshx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zbs_iop_eq, execute_zbs_iop t i.rs1 i.rd _ (getX_of_agree he l1 v1),
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.ZbsOp.zval, zbs_imm]

end Equiv.Sim
