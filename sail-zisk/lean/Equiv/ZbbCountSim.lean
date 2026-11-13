/-
  The simulation, one step, on Zbb's counts, any registers: clz, ctz, cpop
  and their word forms, and the ZisK op of the same name, on rs1 as b (a
  the immediate 0), the spec translates each to. The counts on both sides
  agree by Equiv/Count.lean and Equiv/Count/.
-/
import Equiv.ZbbByteSim
import Equiv.RiscvZbbCount
import Equiv.RiscvDecodeZbbCount
import Equiv.Count.Clz64
import Equiv.Count.Ctz64
import Equiv.Count.Clz32
import Equiv.Count.Ctz32

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (CountOp)

/-! ## The counts, both ways -/

theorem rtb (n : Nat) : (LeanZiskRV.Functions.to_bits (l := 64) n : BitVec 64) = BitVec.ofNat 64 n := by
  simp only [LeanZiskRV.Functions.to_bits, get_slice_int]
  apply BitVec.eq_of_toNat_eq; simp

theorem clz_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_CLZ ()) a b).1
    = LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countLeadingZeros b) := by
  simp only [_root_.Zisk.Functions.zisk_exec, zclz64, rtb, Sail.BitVec.countLeadingZeros, BitVec.ofNat_toNat]
  simp

theorem ctz_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_CTZ ()) a b).1
    = LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countTrailingZeros b) := by
  simp only [_root_.Zisk.Functions.zisk_exec, zctz64, rtb, Sail.BitVec.countTrailingZeros,
    Sail.BitVec.countLeadingZeros, BitVec.ofNat_toNat]
  simp

theorem clzw_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_CLZ_W ()) a b).1
    = LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countLeadingZeros (Sail.BitVec.extractLsb b 31 0)) := by
  simp only [_root_.Zisk.Functions.zisk_exec, rtb, Sail.BitVec.countLeadingZeros, BitVec.ofNat_toNat]
  exact zclz32 _

theorem ctzw_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_CTZ_W ()) a b).1
    = LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countTrailingZeros (Sail.BitVec.extractLsb b 31 0)) := by
  simp only [_root_.Zisk.Functions.zisk_exec, rtb, Sail.BitVec.countTrailingZeros,
    Sail.BitVec.countLeadingZeros, BitVec.ofNat_toNat]
  exact zctz32 _

theorem cpop_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_CPOP ()) a b).1
    = LeanZiskRV.Functions.to_bits (l := 64) (popLoop b) := by
  simp only [_root_.Zisk.Functions.zisk_exec]
  rw [zisk_cpop, riscv_cpop]; rfl

theorem cpopw_val (a b : BitVec 64) : (_root_.Zisk.Functions.zisk_exec (.ZOP_CPOP_W ()) a b).1
    = LeanZiskRV.Functions.to_bits (l := 64) (popLoop (Sail.BitVec.extractLsb b 31 0)) := by
  simp only [_root_.Zisk.Functions.zisk_exec]
  rw [zisk_cpop, riscv_cpop]; rfl

/-! ## Each op: its encoding, sail-riscv's instruction and value -/

def _root_.Transpiler.CountOp.f12 : CountOp → BitVec 12
  | .clz => 0x600 | .ctz => 0x601 | .cpop => 0x602 | .clzw => 0x600 | .ctzw => 0x601 | .cpopw => 0x602

def _root_.Transpiler.CountOp.opc : CountOp → BitVec 7
  | .clz => 0x13 | .ctz => 0x13 | .cpop => 0x13 | .clzw => 0x1B | .ctzw => 0x1B | .cpopw => 0x1B

noncomputable def _root_.Transpiler.CountOp.inst (op : CountOp) (rs1 rd : regidx) : instruction :=
  match op with
  | .clz => .CLZ (rs1, rd) | .ctz => .CTZ (rs1, rd) | .cpop => .CPOP (rs1, rd)
  | .clzw => .CLZW (rs1, rd) | .ctzw => .CTZW (rs1, rd) | .cpopw => .CPOPW (rs1, rd)

def _root_.Transpiler.CountOp.val : CountOp → BitVec 64 → BitVec 64
  | .clz => fun v => LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countLeadingZeros v)
  | .ctz => fun v => LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countTrailingZeros v)
  | .cpop => fun v => LeanZiskRV.Functions.to_bits (l := 64) (popLoop v)
  | .clzw => fun v => LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countLeadingZeros (Sail.BitVec.extractLsb v 31 0))
  | .ctzw => fun v => LeanZiskRV.Functions.to_bits (l := 64) (Sail.BitVec.countTrailingZeros (Sail.BitVec.extractLsb v 31 0))
  | .cpopw => fun v => LeanZiskRV.Functions.to_bits (l := 64) (popLoop (Sail.BitVec.extractLsb v 31 0))

theorem _root_.Transpiler.CountOp.zval (op : CountOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = op.val b := by
  cases op
  · exact clz_val a b
  · exact ctz_val a b
  · exact cpop_val a b
  · exact clzw_val a b
  · exact ctzw_val a b
  · exact cpopw_val a b

theorem _root_.Transpiler.CountOp.pure (op : CountOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.CountOp.zop] | decide

theorem decode_count (s : RVState) (w : BitVec 32) (op : CountOp) (hop : w.extractLsb 6 0 = op.opc)
    (hf : w.extractLsb 31 20 = op.f12) (hf3 : w.extractLsb 14 12 = 1#3)
    (hp : s.regs.get? Register.cur_privilege = some .Machine)
    (sec : RegisterType Register.mseccfg) (hsec : s.regs.get? Register.mseccfg = some sec)
    (mv : RegisterType Register.misa) (hmv : s.regs.get? Register.misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (op.inst (.Regidx (w.extractLsb 19 15)) (.Regidx (w.extractLsb 11 7))) s := by
  cases op
  · exact decode_zbb_clz s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_ctz s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_cpop s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_clzw s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_ctzw s w hop hf hf3 hp sec hsec mv hmv hmB
  · exact decode_zbb_cpopw s w hop hf hf3 hp sec hsec mv hmv hmB

theorem execute_count (s : RVState) (rs1 rd : RegBits) (v1 : BitVec 64) (h1 : getX s rs1.toNat = some v1)
    (op : CountOp) :
    (interpretSailEffects (execute (op.inst (.Regidx rs1) (.Regidx rd)))).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (op.val v1)) := by
  cases op <;> simp only [Transpiler.CountOp.inst, Transpiler.CountOp.val]
  · rw [show execute (.CLZ (.Regidx rs1, .Regidx rd)) = execute_CLZ (.Regidx rs1) (.Regidx rd) by simp [execute]]
    exact execute_clz s rs1 rd v1 h1
  · rw [show execute (.CTZ (.Regidx rs1, .Regidx rd)) = execute_CTZ (.Regidx rs1) (.Regidx rd) by simp [execute]]
    exact execute_ctz s rs1 rd v1 h1
  · rw [show execute (.CPOP (.Regidx rs1, .Regidx rd)) = execute_CPOP (.Regidx rs1) (.Regidx rd) by simp [execute]]
    exact execute_cpop s rs1 rd v1 h1
  · rw [show execute (.CLZW (.Regidx rs1, .Regidx rd)) = execute_CLZW (.Regidx rs1) (.Regidx rd) by simp [execute]]
    exact execute_clzw s rs1 rd v1 h1
  · rw [show execute (.CTZW (.Regidx rs1, .Regidx rd)) = execute_CTZW (.Regidx rs1) (.Regidx rd) by simp [execute]]
    exact execute_ctzw s rs1 rd v1 h1
  · rw [show execute (.CPOPW (.Regidx rs1, .Regidx rd)) = execute_CPOPW (.Regidx rs1) (.Regidx rd) by simp [execute]]
    exact execute_cpopw s rs1 rd v1 h1

/-! ## The step -/

structure CountInstr where
  op : CountOp
  rd : BitVec 5
  rs1 : BitVec 5

def CountInstr.encode (i : CountInstr) : BitVec 32 :=
  i.op.f12 ++ i.rs1 ++ 1#3 ++ i.rd ++ i.op.opc

theorem CountInstr.fields (i : CountInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = i.op.opc ∧ v.extractLsb 31 20 = i.op.f12 ∧ v.extractLsb 14 12 = 1#3 ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, CountInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.CountOp.f12, Transpiler.CountOp.opc] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem CountInstr.zisk_of_spec (i : CountInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (.A_IMM, 0) (Transpiler.srcB i.rs1.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, CountInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, CountInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  have hf3 : Transpiler.funct3 i.encode = 1#3 := by
    simp only [Transpiler.funct3, CountInstr.encode]; bv_decide
  cases hk : i.op <;>
  · have ho : Transpiler.opcode i.encode = i.op.opc := by
      simp only [Transpiler.opcode, CountInstr.encode]; bv_decide
    have hf : i.encode.extractLsb 31 20 = i.op.f12 := by
      simp only [CountInstr.encode]; bv_decide
    have hf6 : Transpiler.funct6 i.encode = (i.op.f12.extractLsb 11 6) := by
      simp only [Transpiler.funct6, CountInstr.encode]; bv_decide
    rw [hk] at hf hf6 ho
    simp only [Transpiler.CountOp.f12, Transpiler.CountOp.opc] at hf hf6 ho
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf6, hf, hr1, hrd, Transpiler.singleOp]

set_option maxHeartbeats 4000000 in
/-- One step of each on a count, any registers, keeps the states
    related. -/
theorem countstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : CountInstr) (next : Option (BitVec 32))
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
    have := decode_count t _ i.op hop hf hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_count t i.rs1 i.rd _ (getX_of_agree he l1 v1), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Or.inl rfl) (Equiv.Zisk.srcB_BSrc xs _ l1), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.CountOp.zval]

end Equiv.Sim
