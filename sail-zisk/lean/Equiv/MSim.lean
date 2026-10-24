/-
  The simulation, one step, on the M extension, any registers: mul, mulh,
  mulhsu, mulhu, div, divu, rem, remu, mulw, divw, divuw, remw, remuw, and
  the ZisK instruction the spec translates each to (one arith op, as
  create_register_op emits it).
-/
import Equiv.X0Sim
import Equiv.RiscvM
import Equiv.RiscvDecodeM
import Equiv.RiscvDecodeMul
import Equiv.SimM

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (MOp)

/-! ## Each M op: its encoding, sail-riscv's instruction and value -/

def _root_.Transpiler.MOp.opc : MOp → BitVec 7
  | .mulw | .divw | .divuw | .remw | .remuw => 0x3B
  | _ => 0x33

def _root_.Transpiler.MOp.f3 : MOp → BitVec 3
  | .mul => 0 | .mulh => 1 | .mulhsu => 2 | .mulhu => 3 | .div => 4 | .divu => 5 | .rem => 6 | .remu => 7
  | .mulw => 0 | .divw => 4 | .divuw => 5 | .remw => 6 | .remuw => 7

/-- sail-riscv's instruction for the op. -/
def _root_.Transpiler.MOp.inst (op : MOp) (rd rs1 rs2 : BitVec 5) : instruction :=
  match op with
  | .mul => .MUL (.Regidx rs2, .Regidx rs1, .Regidx rd, MulSel.mul.op)
  | .mulh => .MUL (.Regidx rs2, .Regidx rs1, .Regidx rd, MulSel.mulh.op)
  | .mulhsu => .MUL (.Regidx rs2, .Regidx rs1, .Regidx rd, MulSel.mulhsu.op)
  | .mulhu => .MUL (.Regidx rs2, .Regidx rs1, .Regidx rd, MulSel.mulhu.op)
  | .div => .DIV (.Regidx rs2, .Regidx rs1, .Regidx rd, false)
  | .divu => .DIV (.Regidx rs2, .Regidx rs1, .Regidx rd, true)
  | .rem => .REM (.Regidx rs2, .Regidx rs1, .Regidx rd, false)
  | .remu => .REM (.Regidx rs2, .Regidx rs1, .Regidx rd, true)
  | .mulw => .MULW (.Regidx rs2, .Regidx rs1, .Regidx rd)
  | .divw => .DIVW (.Regidx rs2, .Regidx rs1, .Regidx rd, false)
  | .divuw => .DIVW (.Regidx rs2, .Regidx rs1, .Regidx rd, true)
  | .remw => .REMW (.Regidx rs2, .Regidx rs1, .Regidx rd, false)
  | .remuw => .REMW (.Regidx rs2, .Regidx rs1, .Regidx rd, true)

/-- What the op computes, as sail-riscv has it. -/
def _root_.Transpiler.MOp.val (op : MOp) (a b : BitVec 64) : BitVec 64 :=
  match op with
  | .mul => mulVal MulSel.mul.op a b | .mulh => mulVal MulSel.mulh.op a b
  | .mulhsu => mulVal MulSel.mulhsu.op a b | .mulhu => mulVal MulSel.mulhu.op a b
  | .div => divVal false a b | .divu => divVal true a b | .rem => remVal false a b | .remu => remVal true a b
  | .mulw => mulwVal a b | .divw => divwVal false a b | .divuw => divwVal true a b
  | .remw => remwVal false a b | .remuw => remwVal true a b

theorem _root_.Transpiler.MOp.decode (op : MOp) (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = op.opc) (hf3 : w.extractLsb 14 12 = op.f3) (hf7 : w.extractLsb 31 25 = 1#7)
    (hp : s.regs.get? Register.cur_privilege = some .Machine)
    (sec : RegisterType Register.mseccfg) (hsec : s.regs.get? Register.mseccfg = some sec)
    (hm : ∃ v, s.regs.get? Register.misa = some v ∧ _get_Misa_M v = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (op.inst (w.extractLsb 11 7) (w.extractLsb 19 15) (w.extractLsb 24 20)) s := by
  cases op
  · exact decode_mul s w .mul hop hf3 hf7 hp sec hsec hm
  · exact decode_mul s w .mulh hop hf3 hf7 hp sec hsec hm
  · exact decode_mul s w .mulhsu hop hf3 hf7 hp sec hsec hm
  · exact decode_mul s w .mulhu hop hf3 hf7 hp sec hsec hm
  · exact decode_div s w false hop hf3 hf7 hp sec hsec hm
  · exact decode_div s w true hop hf3 hf7 hp sec hsec hm
  · exact decode_rem s w false hop hf3 hf7 hp sec hsec hm
  · exact decode_rem s w true hop hf3 hf7 hp sec hsec hm
  · exact decode_mulw s w hop hf3 hf7 hp sec hsec hm
  · exact decode_divw s w false hop hf3 hf7 hp sec hsec hm
  · exact decode_divw s w true hop hf3 hf7 hp sec hsec hm
  · exact decode_remw s w false hop hf3 hf7 hp sec hsec hm
  · exact decode_remw s w true hop hf3 hf7 hp sec hsec hm

theorem execute_mul_eq (a b c : regidx) (op : mul_op) :
    execute (.MUL (a, b, c, op)) = execute_MUL a b c op := by simp [execute]
theorem execute_div_eq (a b c : regidx) (u : Bool) :
    execute (.DIV (a, b, c, u)) = execute_DIV a b c u := by simp [execute]
theorem execute_rem_eq (a b c : regidx) (u : Bool) :
    execute (.REM (a, b, c, u)) = execute_REM a b c u := by simp [execute]
theorem execute_mulw_eq (a b c : regidx) :
    execute (.MULW (a, b, c)) = execute_MULW a b c := by simp [execute]
theorem execute_divw_eq (a b c : regidx) (u : Bool) :
    execute (.DIVW (a, b, c, u)) = execute_DIVW a b c u := by simp [execute]
theorem execute_remw_eq (a b c : regidx) (u : Bool) :
    execute (.REMW (a, b, c, u)) = execute_REMW a b c u := by simp [execute]

theorem _root_.Transpiler.MOp.exec (op : MOp) (t : RVState) (rd rs1 rs2 : RegBits) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2) :
    (interpretSailEffects (execute (op.inst rd rs1 rs2))).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (op.val v1 v2)) := by
  cases op <;> simp only [MOp.inst, MOp.val, execute_mul_eq, execute_div_eq, execute_rem_eq, execute_mulw_eq,
    execute_divw_eq, execute_remw_eq]
  · exact execute_mul t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_mul t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_mul t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_mul t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_div t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_div t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_rem t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_rem t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_mulw t rs1 rs2 rd v1 v2 h1 h2
  · exact execute_divw t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_divw t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_remw t rs1 rs2 rd v1 v2 h1 h2 _
  · exact execute_remw t rs1 rs2 rd v1 v2 h1 h2 _

theorem _root_.Transpiler.MOp.zval (op : MOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = op.val a b := by
  cases op
  · exact mul_val a b
  · exact mulh_val a b
  · exact mulhsu_val a b
  · exact mulhu_val a b
  · exact div_val a b
  · exact divu_val a b
  · exact rem_val a b
  · exact remu_val a b
  · exact mulw_val a b
  · exact divw_val a b
  · exact divuw_val a b
  · exact remw_val a b
  · exact remuw_val a b

theorem _root_.Transpiler.MOp.pure (op : MOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [MOp.zop] | decide

/-! ## The instruction and its step -/

structure MInstr where
  op : MOp
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def MInstr.encode (i : MInstr) : BitVec 32 :=
  1#7 ++ i.rs2 ++ i.rs1 ++ i.op.f3 ++ i.rd ++ i.op.opc

theorem MInstr.fields (i : MInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = i.op.opc ∧ v.extractLsb 14 12 = i.op.f3 ∧ v.extractLsb 31 25 = 1#7 ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, MInstr.encode, byte]
  cases i.op <;> simp only [MOp.opc, MOp.f3] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem MInstr.spec_fields (i : MInstr) :
    Transpiler.opcode i.encode = i.op.opc ∧ Transpiler.funct3 i.encode = i.op.f3 ∧
    Transpiler.funct7 i.encode = 1#7 ∧ Transpiler.rd i.encode = i.rd.toNat ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.rs2 i.encode = i.rs2.toNat := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.funct7, Transpiler.rd, Transpiler.rs1,
    Transpiler.rs2, MInstr.encode]
  cases i.op <;> simp only [MOp.opc, MOp.f3] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;>
    first | bv_decide | (congr 1; bv_decide)

theorem MInstr.zisk_of_spec (i : MInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  obtain ⟨hop, hf3, hf7, hrd, hr1, hr2⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hop hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, MOp.opc, MOp.f3, hr1, hr2, hrd,
      Transpiler.registerOp]

structure MAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : MInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
/-- One step of each on an M instruction, any registers, keeps the states
    related. -/
theorem mstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : MInstr) (next : Option (BitVec 32))
    (hi : MAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf7, hrdx, hr1x, hr2x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have v1 := getX_regv agree l1
  have v2 := getX_regv agree l2
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw (i.op.inst i.rd i.rs1 i.rs2) ?_
    i.rd.toNat i.rd.isLt (i.op.val (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := i.op.decode t _ hop hf3 hf7 htp w hw' hmt
    rw [hrdx, hr1x, hr2x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [i.op.exec t i.rd i.rs1 i.rs2 _ _ (getX_of_agree he l1 v1) (getX_of_agree he l2 v2),
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, MOp.zval]

end Equiv.Sim
