/-
  The simulation, one step, on the byte ops, any registers: Zbb's rev8 and
  orc.b, Zbkb's brev8, and the ZisK op of the same name, on rs1 as b (a the
  immediate 0), the spec translates each to. Both sides loop over the
  bytes (and brev8 over each byte's bits); unrolled, bv_decide finds them
  equal.
-/
import Equiv.ZbkbSim
import Equiv.RiscvZbbByte
import Equiv.RiscvDecodeZbbByte

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (ByteOp)

/-! ## The ops, both ways -/

theorem rev8_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_REV8 ()) a b).1 = rev8 b := by
  simp [_root_.Zisk.Functions.zisk_exec, rev8, Sail.BitVec.length]
  unroll
  simp only [BitVec.updateSubrange, Sail.BitVec.extractLsb, _root_.Zisk.Functions.zeros, zeros]

theorem orcb_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_ORC_B ()) a b).1 = orcbVal b := by
  simp [_root_.Zisk.Functions.zisk_exec, orcbVal]
  unroll
  simp only [BitVec.updateSubrange, BitVec.updateSubrange', Sail.BitVec.extractLsb, _root_.Zisk.Functions.zeros,
    zeros, bv_zero]
  bv_decide

set_option maxRecDepth 100000 in
theorem brev8_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_BREV8 ()) a b).1 = brev8 b := by
  simp [_root_.Zisk.Functions.zisk_exec, brev8, Sail.BitVec.length]
  unroll
  simp only [BitVec.updateSubrange, Sail.BitVec.extractLsb, _root_.Zisk.Functions.zeros, zeros, bv_zero]
  simp only [reverse_bits8]
  simp only [rb8, BitVec.updateSubrange', BitVec.update, acc_eq, bv_zero]
  bv_decide

/-! ## Each op: its encoding, sail-riscv's instruction and value -/

def _root_.Transpiler.ByteOp.f12 : ByteOp → BitVec 12
  | .rev8 => 0x6B8 | .orcb => 0x287 | .brev8 => 0x687

noncomputable def _root_.Transpiler.ByteOp.inst (op : ByteOp) (rs1 rd : regidx) : instruction :=
  match op with
  | .rev8 => .REV8 (rs1, rd) | .orcb => .ORCB (rs1, rd) | .brev8 => .BREV8 (rs1, rd)

def _root_.Transpiler.ByteOp.val : ByteOp → BitVec 64 → BitVec 64
  | .rev8 => rev8 | .orcb => orcbVal | .brev8 => brev8

theorem _root_.Transpiler.ByteOp.zval (op : ByteOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = op.val b := by
  cases op
  · exact rev8_val a b
  · exact orcb_val a b
  · exact brev8_val a b

theorem _root_.Transpiler.ByteOp.pure (op : ByteOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.ByteOp.zop] | decide

theorem decode_byte (s : RVState) (w : BitVec 32) (op : ByteOp) (hop : w.extractLsb 6 0 = 0x13#7)
    (hf : w.extractLsb 31 20 = op.f12) (hf3 : w.extractLsb 14 12 = 5#3)
    (hp : s.regs.get? Register.cur_privilege = some .Machine)
    (sec : RegisterType Register.mseccfg) (hsec : s.regs.get? Register.mseccfg = some sec)
    (mv : RegisterType Register.misa) (hmv : s.regs.get? Register.misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (op.inst (.Regidx (w.extractLsb 19 15)) (.Regidx (w.extractLsb 11 7))) s := by
  cases op
  · exact decode_zbb_rev8 s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_orcb s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbkb_brev8 s w hop hf hf3 hp sec hsec mv hmv hmB

theorem execute_byte (s : RVState) (rs1 rd : RegBits) (v1 : BitVec 64) (h1 : getX s rs1.toNat = some v1)
    (op : ByteOp) :
    (interpretSailEffects (execute (op.inst (.Regidx rs1) (.Regidx rd)))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (op.val v1)) := by
  cases op
  · simp only [Transpiler.ByteOp.inst, Transpiler.ByteOp.val]
    rw [show execute (.REV8 (.Regidx rs1, .Regidx rd)) = execute_REV8 (.Regidx rs1) (.Regidx rd) by
      simp [execute]]
    exact execute_rev8 s rs1 rd v1 h1
  · simp only [Transpiler.ByteOp.inst, Transpiler.ByteOp.val]
    rw [show execute (.ORCB (.Regidx rs1, .Regidx rd)) = execute_ORCB (.Regidx rs1) (.Regidx rd) by
      simp [execute]]
    exact execute_orcb s rs1 rd v1 h1
  · simp only [Transpiler.ByteOp.inst, Transpiler.ByteOp.val]
    rw [show execute (.BREV8 (.Regidx rs1, .Regidx rd)) = execute_BREV8 (.Regidx rs1) (.Regidx rd) by
      simp [execute]]
    exact execute_brev8 s rs1 rd v1 h1

/-! ## The step -/

structure ByteInstr where
  op : ByteOp
  rd : BitVec 5
  rs1 : BitVec 5

def ByteInstr.encode (i : ByteInstr) : BitVec 32 :=
  i.op.f12 ++ i.rs1 ++ 5#3 ++ i.rd ++ 0x13#7

theorem ByteInstr.fields (i : ByteInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x13#7 ∧ v.extractLsb 31 20 = i.op.f12 ∧ v.extractLsb 14 12 = 5#3 ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, ByteInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.ByteOp.f12] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem ByteInstr.zisk_of_spec (i : ByteInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (.A_IMM, 0) (Transpiler.srcB i.rs1.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, ByteInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, ByteInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x13#7 := by
    simp only [Transpiler.opcode, ByteInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 5#3 := by
    simp only [Transpiler.funct3, ByteInstr.encode]; bv_decide
  cases hk : i.op <;>
  · have hf : i.encode.extractLsb 31 20 = i.op.f12 := by
      simp only [ByteInstr.encode]; bv_decide
    have hf6 : Transpiler.funct6 i.encode = (i.op.f12.extractLsb 11 6) := by
      simp only [Transpiler.funct6, ByteInstr.encode]; bv_decide
    rw [hk] at hf hf6
    simp only [Transpiler.ByteOp.f12] at hf hf6
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf6, hf, hr1, hrd, Transpiler.singleOp]

set_option maxHeartbeats 4000000 in
/-- One step of each on rev8, orc.b or brev8, any registers, keeps the
    states related. -/
theorem bytestep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ByteInstr) (next : Option (BitVec 32))
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
    (i.op.inst (.Regidx i.rs1) (.Regidx i.rd)) ?_
    i.rd.toNat i.rd.isLt (i.op.val (regv xs i.rs1.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_byte t _ i.op hop hf hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_byte t i.rs1 i.rd _ (getX_of_agree he l1 v1), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Or.inl rfl) (Equiv.Zisk.srcB_BSrc xs _ l1), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.ByteOp.zval]

end Equiv.Sim
