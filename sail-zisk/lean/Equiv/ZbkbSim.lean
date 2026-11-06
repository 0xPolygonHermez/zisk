/-
  The simulation, one step, on Zbkb's packs (pack, packh, packw) and Zbb's
  extensions (sext.b, sext.h, zext.h), any registers, and the ZisK
  instruction the spec translates each to: the op of the same name for the
  packs; signextend_b or _h of rs1 as b, a the immediate 0, for sext.b and
  sext.h; an and with the immediate 0xffff for zext.h.
-/
import Equiv.ZbbRotSim
import Equiv.RiscvZbkb
import Equiv.RiscvDecodeZbkb

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (PackOp SextOp)

/-! ## The ops, both ways -/

theorem pack_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_PACK ()) a b).1 = zbkbVal .PACK a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbkbVal, Functions.xlen_bytes]
  rfl

theorem packh_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_PACK_H ()) a b).1 = zbkbVal .PACKH a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbkbVal, _root_.Zisk.Functions.zero_extend, zero_extend,
    Sail.BitVec.zeroExtend]

theorem packw_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_PACK_W ()) a b).1 = packwVal a b := by
  simp [_root_.Zisk.Functions.zisk_exec, packwVal, _root_.Zisk.Functions.sign_extend, sign_extend,
    Sail.BitVec.signExtend]

theorem zexth_val (a : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_AND ()) a 0xFFFF#64).1 = extVal .ZEXTH a := by
  simp [_root_.Zisk.Functions.zisk_exec, extVal, zero_extend, Sail.BitVec.zeroExtend, Sail.BitVec.extractLsb]
  bv_decide

/-! ## Each op: its encoding, sail-riscv's instruction -/

def _root_.Transpiler.PackOp.r : PackOp → brop_zbkb
  | .pack => .PACK | .packh => .PACKH

def _root_.Transpiler.SextOp.r : SextOp → extop_zbb
  | .sextb => .SEXTB | .sexth => .SEXTH

theorem _root_.Transpiler.PackOp.zval (op : PackOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = zbkbVal op.r a b := by
  cases op
  · exact pack_val a b
  · exact packh_val a b

theorem _root_.Transpiler.SextOp.zval (op : SextOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = extVal op.r b := by
  cases op <;> simp [Transpiler.SextOp.zop, Transpiler.SextOp.r, _root_.Zisk.Functions.zisk_exec, extVal,
    _root_.Zisk.Functions.sign_extend, sign_extend, Sail.BitVec.signExtend]

theorem _root_.Transpiler.PackOp.pure (op : PackOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.PackOp.zop] | decide

theorem _root_.Transpiler.SextOp.pure (op : SextOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.SextOp.zop] | decide

theorem pure_pack_w : Equiv.Zisk.PureOp (.ZOP_PACK_W ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

theorem pure_and : Equiv.Zisk.PureOp (.ZOP_AND ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

/-- The words at pc, the instruction's encoding, and its spec translation:
    what each step below needs of the state. -/
structure WordsAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (e : BitVec 32) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte e 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte e 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte e 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte e 3)
  tr : Transpiled zrom pc e next

/-! ## pack and packh -/

structure PackInstr where
  op : PackOp
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def PackInstr.encode (i : PackInstr) : BitVec 32 :=
  0x04#7 ++ i.rs2 ++ i.rs1 ++ zbkbF3 i.op.r ++ i.rd ++ 0x33#7

theorem PackInstr.fields (i : PackInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x33#7 ∧ v.extractLsb 31 25 = 0x04#7 ∧ v.extractLsb 14 12 = zbkbF3 i.op.r ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, PackInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.PackOp.r, zbkbF3] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem PackInstr.zisk_of_spec (i : PackInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, PackInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, PackInstr.encode]; congr 1; bv_decide
  have hr2 : Transpiler.rs2 i.encode = i.rs2.toNat := by
    simp only [Transpiler.rs2, PackInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x33#7 := by
    simp only [Transpiler.opcode, PackInstr.encode]; bv_decide
  have hf7 : Transpiler.funct7 i.encode = 0x04#7 := by
    simp only [Transpiler.funct7, PackInstr.encode]; bv_decide
  cases hk : i.op <;>
  · have hf3 : Transpiler.funct3 i.encode = zbkbF3 i.op.r := by
      simp only [Transpiler.funct3, PackInstr.encode]; bv_decide
    rw [hk] at hf3
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, zbkbF3, Transpiler.PackOp.r, hr1, hr2, hrd,
      Transpiler.registerOp]

theorem execute_zbkb_rtype_eq (a b c : regidx) (op : brop_zbkb) :
    execute (.ZBKB_RTYPE (a, b, c, op)) = execute_ZBKB_RTYPE a b c op := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on pack or packh, any registers, keeps the states
    related. -/
theorem packstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : PackInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (n : Nat) :
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
    (.ZBKB_RTYPE (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.op.r)) ?_
    i.rd.toNat i.rd.isLt (zbkbVal i.op.r (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbkb_r t _ i.op.r hop hf7 hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hr2x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zbkb_rtype_eq, execute_zbkb_rtype t i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.PackOp.zval]

/-! ## packw, rs2 not x0 (with x0, it is zext.h) -/

structure PackwInstr where
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5
  h2 : rs2 ≠ 0#5

def PackwInstr.encode (i : PackwInstr) : BitVec 32 :=
  0x04#7 ++ i.rs2 ++ i.rs1 ++ 4#3 ++ i.rd ++ 0x3B#7

theorem PackwInstr.fields (i : PackwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x3B#7 ∧ v.extractLsb 31 25 = 0x04#7 ∧ v.extractLsb 14 12 = 4#3 ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, PackwInstr.encode, byte]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem PackwInstr.zisk_of_spec (i : PackwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) (.ZOP_PACK_W ())
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, PackwInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, PackwInstr.encode]; congr 1; bv_decide
  have hr2 : Transpiler.rs2 i.encode = i.rs2.toNat := by
    simp only [Transpiler.rs2, PackwInstr.encode]; congr 1; bv_decide
  have h0 : i.rs2.toNat ≠ 0 := fun e => i.h2 (BitVec.eq_of_toNat_eq e)
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x3B#7 := by
    simp only [Transpiler.opcode, PackwInstr.encode]; bv_decide
  have hf7 : Transpiler.funct7 i.encode = 0x04#7 := by
    simp only [Transpiler.funct7, PackwInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 4#3 := by
    simp only [Transpiler.funct3, PackwInstr.encode]; bv_decide
  simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, hr1, hr2, hrd, h0, Transpiler.registerOp]

theorem execute_packw_eq (a b c : regidx) :
    execute (.ZBKB_PACKW (a, b, c)) = execute_ZBKB_PACKW a b c := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on packw, any registers but rs2 x0, keeps the states
    related. -/
theorem packwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : PackwInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (n : Nat) :
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
    (.ZBKB_PACKW (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd)) ?_
    i.rd.toNat i.rd.isLt (packwVal (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbkb_packw t _ hop hf7 hf3 (by rw [hr2x]; exact i.h2) htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hr2x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_packw_eq, execute_packw t i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ pure_pack_w i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, packw_val]

/-! ## sext.b and sext.h -/

structure SextInstr where
  op : SextOp
  rd : BitVec 5
  rs1 : BitVec 5

def SextInstr.encode (i : SextInstr) : BitVec 32 :=
  sextF12 i.op.r ++ i.rs1 ++ 1#3 ++ i.rd ++ 0x13#7

theorem SextInstr.fields (i : SextInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x13#7 ∧ v.extractLsb 31 20 = sextF12 i.op.r ∧ v.extractLsb 14 12 = 1#3 ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, SextInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.SextOp.r, sextF12] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem SextInstr.zisk_of_spec (i : SextInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (.A_IMM, 0) (Transpiler.srcB i.rs1.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, SextInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, SextInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x13#7 := by
    simp only [Transpiler.opcode, SextInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 1#3 := by
    simp only [Transpiler.funct3, SextInstr.encode]; bv_decide
  cases hk : i.op <;>
  · have hf : i.encode.extractLsb 31 20 = sextF12 i.op.r := by
      simp only [SextInstr.encode]; bv_decide
    have hf6 : Transpiler.funct6 i.encode = 24#6 := by
      simp only [Transpiler.funct6, SextInstr.encode]; cases i.op <;> simp only [Transpiler.SextOp.r, sextF12] <;>
        bv_decide
    rw [hk] at hf
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf6, hf, sextF12, Transpiler.SextOp.r, hr1, hrd,
      Transpiler.singleOp]

theorem execute_zbb_extop_eq (a b : regidx) (op : extop_zbb) :
    execute (.ZBB_EXTOP (a, b, op)) = execute_ZBB_EXTOP a b op := by simp [execute]

theorem SextOp.r_ne (op : SextOp) : op.r ≠ .ZEXTH := by cases op <;> simp [Transpiler.SextOp.r]

set_option maxHeartbeats 4000000 in
/-- One step of each on sext.b or sext.h, any registers, keeps the states
    related. -/
theorem sextstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SextInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf, hf3, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.ZBB_EXTOP (.Regidx i.rs1, .Regidx i.rd, i.op.r)) ?_
    i.rd.toNat i.rd.isLt (extVal i.op.r (regv xs i.rs1.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbb_sext t _ i.op.r (SextOp.r_ne _) hop hf hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zbb_extop_eq, execute_zbb_extop t i.rs1 i.rd _ (getX_of_agree he l1 v1), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Or.inl rfl) (Equiv.Zisk.srcB_BSrc xs _ l1), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.SextOp.zval]

/-! ## zext.h -/

structure ZexthInstr where
  rd : BitVec 5
  rs1 : BitVec 5

def ZexthInstr.encode (i : ZexthInstr) : BitVec 32 :=
  0x080#12 ++ i.rs1 ++ 4#3 ++ i.rd ++ 0x3B#7

theorem ZexthInstr.fields (i : ZexthInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x3B#7 ∧ v.extractLsb 31 20 = 0x080#12 ∧ v.extractLsb 14 12 = 4#3 ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, ZexthInstr.encode, byte]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem ZexthInstr.zisk_of_spec (i : ZexthInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, 0xFFFF) (.ZOP_AND ())
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, ZexthInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, ZexthInstr.encode]; congr 1; bv_decide
  have hr2 : Transpiler.rs2 i.encode = 0 := by
    simp only [Transpiler.rs2, ZexthInstr.encode]
    rw [show (0x080#12 ++ i.rs1 ++ 4#3 ++ i.rd ++ 0x3B#7).extractLsb 24 20 = 0#5 by bv_decide]; rfl
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x3B#7 := by
    simp only [Transpiler.opcode, ZexthInstr.encode]; bv_decide
  have hf7 : Transpiler.funct7 i.encode = 0x04#7 := by
    simp only [Transpiler.funct7, ZexthInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 4#3 := by
    simp only [Transpiler.funct3, ZexthInstr.encode]; bv_decide
  simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, hr1, hr2, hrd, Transpiler.zextH]

set_option maxHeartbeats 4000000 in
/-- One step of each on zext.h, any registers, keeps the states related. -/
theorem zexthstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZexthInstr) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf, hf3, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.ZBB_EXTOP (.Regidx i.rs1, .Regidx i.rd, .ZEXTH)) ?_
    i.rd.toNat i.rd.isLt (extVal .ZEXTH (regv xs i.rs1.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbb_zexth t _ hop hf hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zbb_extop_eq, execute_zbb_extop t i.rs1 i.rd _ (getX_of_agree he l1 v1), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ pure_and i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, zexth_val]

end Equiv.Sim
