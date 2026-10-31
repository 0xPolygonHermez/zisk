/-
  The simulation, one step, on Zbb's register ops, any registers: andn, orn,
  xnor, max, maxu, min, minu, rol, ror, and the ZisK op of the same name the
  spec translates each to. Max and min compare the other way round on each
  side; ZisK's ror is a rotation left by 64 minus the amount: the same.
-/
import Equiv.ZbsSim
import Equiv.RiscvZbb
import Equiv.RiscvDecodeZbb

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (ZbbOp)

/-! ## The ops, both ways -/

/-- shl_int and shr_int, with the cast the shifts here come with. -/
theorem shl_cast (a : BitVec 64) (n : Nat) : a <<< (n : Int) = a <<< n := shl_int a n

theorem shr_cast (a : BitVec 64) (n : Nat) : a >>> (n : Int) = a >>> n := shr_int a n

theorem rol_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_ROL ()) a b).1 = zbbVal .ROL a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbbVal, _root_.Zisk.Functions.rotl, rotate_bits_left, Sail.BitVec.length,
    BitVec.toNatInt, Functions.log2_xlen]
  simp only [rotatel, Sail.BitVec.length]
  rfl

theorem ror_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_ROR ()) a b).1 = zbbVal .ROR a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbbVal, _root_.Zisk.Functions.rotl, rotate_bits_right, Sail.BitVec.length,
    BitVec.toNatInt, Functions.log2_xlen]
  simp only [rotater, Sail.BitVec.length]
  have hk := ext_k b
  generalize (Sail.BitVec.extractLsb b 5 0).toNat = k at hk ⊢
  by_cases h0 : k = 0
  · subst h0
    simp only [if_true]
    show a <<< ((0 : Nat) : Int) ||| a >>> ((64 : Nat) : Int) = a >>> (0 : Nat) ||| a <<< ((64 - 0 : Nat) : Int)
    rw [shl_cast, shr_cast, shl_cast]; simp; bv_decide
  · simp only [h0, if_false]
    rw [show (64 : Int) - ((64 : Int) - (k : Int)) = ((k : Nat) : Int) by omega,
      show (64 : Int) - (k : Int) = ((64 - k : Nat) : Int) by omega,
      show ((64 : Nat) : Int) - (k : Int) = ((64 - k : Nat) : Int) by omega, shr_cast, shl_cast]
    exact BitVec.or_comm _ _

/-! ## Each op: its encoding, sail-riscv's instruction -/

def _root_.Transpiler.ZbbOp.r : ZbbOp → brop_zbb
  | .andn => .ANDN | .orn => .ORN | .xnor => .XNOR | .max => .MAX | .maxu => .MAXU | .min => .MIN
  | .minu => .MINU | .rol => .ROL | .ror => .ROR

theorem _root_.Transpiler.ZbbOp.zval (op : ZbbOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = zbbVal op.r a b := by
  cases op
  · simp [Transpiler.ZbbOp.zop, Transpiler.ZbbOp.r, _root_.Zisk.Functions.zisk_exec, zbbVal]
  · simp [Transpiler.ZbbOp.zop, Transpiler.ZbbOp.r, _root_.Zisk.Functions.zisk_exec, zbbVal]
  · simp [Transpiler.ZbbOp.zop, Transpiler.ZbbOp.r, _root_.Zisk.Functions.zisk_exec, zbbVal]
  · simp [Transpiler.ZbbOp.zop, Transpiler.ZbbOp.r, _root_.Zisk.Functions.zisk_exec, zbbVal, zopz0zK_s,
      _root_.Zisk.Functions.zopz0zI_s]
  · simp [Transpiler.ZbbOp.zop, Transpiler.ZbbOp.r, _root_.Zisk.Functions.zisk_exec, zbbVal, zopz0zK_u,
      _root_.Zisk.Functions.zopz0zI_u]
  · simp [Transpiler.ZbbOp.zop, Transpiler.ZbbOp.r, _root_.Zisk.Functions.zisk_exec, zbbVal, zopz0zI_s,
      _root_.Zisk.Functions.zopz0zI_s]
  · simp [Transpiler.ZbbOp.zop, Transpiler.ZbbOp.r, _root_.Zisk.Functions.zisk_exec, zbbVal, zopz0zI_u,
      _root_.Zisk.Functions.zopz0zI_u]
  · exact rol_val a b
  · exact ror_val a b

theorem _root_.Transpiler.ZbbOp.pure (op : ZbbOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.ZbbOp.zop] | decide

/-! ## Register forms -/

structure ZbbInstr where
  op : ZbbOp
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def ZbbInstr.encode (i : ZbbInstr) : BitVec 32 :=
  zbbF7 i.op.r ++ i.rs2 ++ i.rs1 ++ zbbF3 i.op.r ++ i.rd ++ 0x33#7

theorem ZbbInstr.fields (i : ZbbInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x33#7 ∧ v.extractLsb 31 25 = zbbF7 i.op.r ∧ v.extractLsb 14 12 = zbbF3 i.op.r ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, ZbbInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.ZbbOp.r, zbbF7, zbbF3] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem ZbbInstr.zisk_of_spec (i : ZbbInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, ZbbInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, ZbbInstr.encode]; congr 1; bv_decide
  have hr2 : Transpiler.rs2 i.encode = i.rs2.toNat := by
    simp only [Transpiler.rs2, ZbbInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  cases hk : i.op <;>
  · have ho : Transpiler.opcode i.encode = 0x33#7 := by
      simp only [Transpiler.opcode, ZbbInstr.encode]; bv_decide
    have hf3 : Transpiler.funct3 i.encode = zbbF3 i.op.r := by
      simp only [Transpiler.funct3, ZbbInstr.encode]; bv_decide
    have hf7 : Transpiler.funct7 i.encode = zbbF7 i.op.r := by
      simp only [Transpiler.funct7, ZbbInstr.encode]; bv_decide
    rw [hk] at hf3 hf7
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, zbbF3, zbbF7, Transpiler.ZbbOp.r, hr1, hr2, hrd,
      Transpiler.registerOp]

structure ZbbAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbbInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_zbb_rtype_eq (a b c : regidx) (op : brop_zbb) :
    execute (.ZBB_RTYPE (a, b, c, op)) = execute_ZBB_RTYPE a b c op := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on a Zbb register op, any registers, keeps the states
    related. -/
theorem zbbstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbbInstr) (next : Option (BitVec 32))
    (hi : ZbbAt s rs pc zrom i next) (n : Nat) :
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
    (.ZBB_RTYPE (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.op.r)) ?_
    i.rd.toNat i.rd.isLt (zbbVal i.op.r (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbb_r t _ i.op.r hop hf7 hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hr2x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zbb_rtype_eq, execute_zbb_rtype t i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.ZbbOp.zval]

end Equiv.Sim
