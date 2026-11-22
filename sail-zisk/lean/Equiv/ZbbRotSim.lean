/-
  The simulation, one step, on Zbb's other rotations, any registers: rolw
  and rorw, and rori and roriw, by an immediate; and the ZisK op the spec
  translates each to (rol_w, ror_w, and ror and ror_w with the immediate as
  b). As for ror, ZisK's ror_w is a rotation left by 32 minus the amount.
-/
import Equiv.ZbbSim

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)
open Transpiler (ZbbWOp)

/-! ## The ops, both ways -/

theorem shl_w {w} (a : BitVec w) (n : Nat) : a <<< (n : Int) = a <<< n := shl_int a n

theorem shr_w {w} (a : BitVec w) (n : Nat) : a >>> (n : Int) = a >>> n := shr_int a n

theorem zsx32 (x : BitVec 32) : _root_.Zisk.Functions.sign_extend x 64 = sign_extend (m := 64) x := by
  simp [_root_.Zisk.Functions.sign_extend, sign_extend, Sail.BitVec.signExtend]

theorem ext_k5 (b : BitVec 64) : (Sail.BitVec.extractLsb b 4 0).toNat < 32 := by
  simp [Sail.BitVec.extractLsb]; omega

theorem rolw_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_ROL_W ()) a b).1 = zbbWVal .ROLW a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbbWVal, _root_.Zisk.Functions.rotl, rotate_bits_left, Sail.BitVec.length,
    BitVec.toNatInt]
  simp only [rotatel, Sail.BitVec.length]
  rw [zsx32]; congr 1

theorem rorw_val (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_ROR_W ()) a b).1 = zbbWVal .RORW a b := by
  simp [_root_.Zisk.Functions.zisk_exec, zbbWVal, _root_.Zisk.Functions.rotl, rotate_bits_right, Sail.BitVec.length,
    BitVec.toNatInt]
  simp only [rotater, Sail.BitVec.length]
  rw [zsx32]; congr 1
  have hk := ext_k5 b
  generalize (Sail.BitVec.extractLsb b 4 0).toNat = k at hk ⊢
  generalize Sail.BitVec.extractLsb a 31 0 = x
  by_cases h0 : k = 0
  · subst h0
    simp only [if_true]
    show x <<< ((0 : Nat) : Int) ||| x >>> ((32 : Nat) : Int) = x >>> (0 : Nat) ||| x <<< ((32 - 0 : Nat) : Int)
    rw [shl_w, shr_w, shl_w]; simp; bv_decide
  · simp only [h0, if_false]
    rw [show (32 : Int) - ((32 : Int) - (k : Int)) = ((k : Nat) : Int) by omega,
      show (32 : Int) - (k : Int) = ((32 - k : Nat) : Int) by omega,
      show ((32 : Nat) : Int) - (k : Int) = ((32 - k : Nat) : Int) by omega, shr_w, shl_w]
    exact BitVec.or_comm _ _

theorem rori_val (sh : BitVec 6) (a : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_ROR ()) a (sh.setWidth 64)).1 = roriVal sh a := by
  rw [ror_val]
  have e : Sail.BitVec.extractLsb (sh.setWidth 64) (Functions.log2_xlen -i 1) 0
      = Sail.BitVec.extractLsb sh (Functions.log2_xlen -i 1) 0 := by
    simp only [Sail.BitVec.extractLsb, Functions.log2_xlen]; bv_decide
  simp only [zbbVal, roriVal, e]

theorem roriw_val (sh : BitVec 5) (a : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_ROR_W ()) a (sh.setWidth 64)).1 = roriwVal sh a := by
  rw [rorw_val]
  have e : Sail.BitVec.extractLsb (sh.setWidth 64) 4 0 = sh := by
    simp only [Sail.BitVec.extractLsb]; bv_decide
  simp only [zbbWVal, roriwVal, e]

/-! ## Each op: its encoding, sail-riscv's instruction -/

def _root_.Transpiler.ZbbWOp.r : ZbbWOp → bropw_zbb
  | .rolw => .ROLW | .rorw => .RORW

theorem _root_.Transpiler.ZbbWOp.zval (op : ZbbWOp) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec op.zop a b).1 = zbbWVal op.r a b := by
  cases op
  · exact rolw_val a b
  · exact rorw_val a b

theorem _root_.Transpiler.ZbbWOp.pure (op : ZbbWOp) : Equiv.Zisk.PureOp op.zop := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [Transpiler.ZbbWOp.zop] | decide

theorem pure_ror : Equiv.Zisk.PureOp (.ZOP_ROR ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

theorem pure_ror_w : Equiv.Zisk.PureOp (.ZOP_ROR_W ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

/-! ## Word rotations by a register -/

structure ZbbWInstr where
  op : ZbbWOp
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def ZbbWInstr.encode (i : ZbbWInstr) : BitVec 32 :=
  0x30#7 ++ i.rs2 ++ i.rs1 ++ zbbWF3 i.op.r ++ i.rd ++ 0x3B#7

theorem ZbbWInstr.fields (i : ZbbWInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x3B#7 ∧ v.extractLsb 31 25 = 0x30#7 ∧ v.extractLsb 14 12 = zbbWF3 i.op.r ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, ZbbWInstr.encode, byte]
  cases i.op <;> simp only [Transpiler.ZbbWOp.r, zbbWF3] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem ZbbWInstr.zisk_of_spec (i : ZbbWInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) i.op.zop
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, ZbbWInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, ZbbWInstr.encode]; congr 1; bv_decide
  have hr2 : Transpiler.rs2 i.encode = i.rs2.toNat := by
    simp only [Transpiler.rs2, ZbbWInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x3B#7 := by
    simp only [Transpiler.opcode, ZbbWInstr.encode]; bv_decide
  have hf7 : Transpiler.funct7 i.encode = 0x30#7 := by
    simp only [Transpiler.funct7, ZbbWInstr.encode]; bv_decide
  cases hk : i.op <;>
  · have hf3 : Transpiler.funct3 i.encode = zbbWF3 i.op.r := by
      simp only [Transpiler.funct3, ZbbWInstr.encode]; bv_decide
    rw [hk] at hf3
    simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, zbbWF3, Transpiler.ZbbWOp.r, hr1, hr2, hrd,
      Transpiler.registerOp]

structure ZbbWAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbbWInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_zbb_rtypew_eq (a b c : regidx) (op : bropw_zbb) :
    execute (.ZBB_RTYPEW (a, b, c, op)) = execute_ZBB_RTYPEW a b c op := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on rolw or rorw, any registers, keeps the states
    related. -/
theorem zbbwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : ZbbWInstr) (next : Option (BitVec 32))
    (hi : ZbbWAt s rs pc zrom i next) (n : Nat) :
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
    (.ZBB_RTYPEW (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.op.r)) ?_
    i.rd.toNat i.rd.isLt (zbbWVal i.op.r (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbb_w t _ i.op.r hop hf7 hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hr2x] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_zbb_rtypew_eq, execute_zbb_rtypew t i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ i.op.pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Transpiler.ZbbWOp.zval]

/-! ## Rotations by an immediate -/

structure RoriInstr where
  sh : BitVec 6
  rs1 : BitVec 5
  rd : BitVec 5

def RoriInstr.encode (i : RoriInstr) : BitVec 32 :=
  0x18#6 ++ i.sh ++ i.rs1 ++ 5#3 ++ i.rd ++ 0x13#7

theorem RoriInstr.fields (i : RoriInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x13#7 ∧ v.extractLsb 31 26 = 0x18#6 ∧ v.extractLsb 14 12 = 5#3 ∧
    v.extractLsb 25 20 = i.sh ∧ v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, RoriInstr.encode, byte]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem RoriInstr.zisk_of_spec (i : RoriInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, i.sh.setWidth 64) (.ZOP_ROR ())
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, RoriInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, RoriInstr.encode]; congr 1; bv_decide
  have hsh : Transpiler.shamt i.encode = i.sh.setWidth 64 := by
    simp only [Transpiler.shamt, RoriInstr.encode]; congr 1; bv_decide
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x13#7 := by
    simp only [Transpiler.opcode, RoriInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 5#3 := by
    simp only [Transpiler.funct3, RoriInstr.encode]; bv_decide
  have hf6 : Transpiler.funct6 i.encode = 0x18#6 := by
    simp only [Transpiler.funct6, RoriInstr.encode]; bv_decide
  simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf6, hr1, hrd, hsh, Transpiler.immediateOp]

structure RoriAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RoriInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_rori_eq (sh : BitVec 6) (a b : regidx) :
    execute (.RORI (sh, a, b)) = execute_RORI sh a b := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on rori, any registers, keeps the states related. -/
theorem roristep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RoriInstr) (next : Option (BitVec 32))
    (hi : RoriAt s rs pc zrom i next) (n : Nat) :
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
    (.RORI (i.sh, .Regidx i.rs1, .Regidx i.rd)) ?_
    i.rd.toNat i.rd.isLt (roriVal i.sh (regv xs i.rs1.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbb_rori t _ hop hf6 hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hshx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_rori_eq, execute_rori t i.rs1 i.rd _ (getX_of_agree he l1 v1), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ pure_ror i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, rori_val]

structure RoriwInstr where
  sh : BitVec 5
  rs1 : BitVec 5
  rd : BitVec 5

def RoriwInstr.encode (i : RoriwInstr) : BitVec 32 :=
  0x30#7 ++ i.sh ++ i.rs1 ++ 5#3 ++ i.rd ++ 0x1B#7

theorem RoriwInstr.fields (i : RoriwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x1B#7 ∧ v.extractLsb 31 25 = 0x30#7 ∧ v.extractLsb 14 12 = 5#3 ∧
    v.extractLsb 24 20 = i.sh ∧ v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, RoriwInstr.encode, byte]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem RoriwInstr.zisk_of_spec (i : RoriwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, i.sh.setWidth 64) (.ZOP_ROR_W ())
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  have hrd : Transpiler.rd i.encode = i.rd.toNat := by
    simp only [Transpiler.rd, RoriwInstr.encode]; congr 1; bv_decide
  have hr1 : Transpiler.rs1 i.encode = i.rs1.toNat := by
    simp only [Transpiler.rs1, RoriwInstr.encode]; congr 1; bv_decide
  have hsh : Transpiler.shamt i.encode = i.sh.setWidth 64 := by
    simp only [Transpiler.shamt, RoriwInstr.encode]; bv_decide
  refine h.one fun ia => ?_
  have ho : Transpiler.opcode i.encode = 0x1B#7 := by
    simp only [Transpiler.opcode, RoriwInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 5#3 := by
    simp only [Transpiler.funct3, RoriwInstr.encode]; bv_decide
  have hf7 : Transpiler.funct7 i.encode = 0x30#7 := by
    simp only [Transpiler.funct7, RoriwInstr.encode]; bv_decide
  simp [Transpiler.convert, Transpiler.decode, ho, hf3, hf7, hr1, hrd, hsh, Transpiler.immediateOp]

structure RoriwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RoriwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_roriw_eq (sh : BitVec 5) (a b : regidx) :
    execute (.RORIW (sh, a, b)) = execute_RORIW sh a b := by simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on roriw, any registers, keeps the states related. -/
theorem roriwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RoriwInstr) (next : Option (BitVec 32))
    (hi : RoriwAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf7, hf3, hshx, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.RORIW (i.sh, .Regidx i.rs1, .Regidx i.rd)) ?_
    i.rd.toNat i.rd.isLt (roriwVal i.sh (regv xs i.rs1.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ _ ⟨mb, hmb, hmbB⟩
    have := decode_zbb_roriw t _ hop hf7 hf3 htp w hw' mb hmb hmbB
    rw [hrdx, hr1x, hshx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_roriw_eq, execute_roriw t i.rs1 i.rd _ (getX_of_agree he l1 v1), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ pure_ror_w i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, roriw_val]

end Equiv.Sim
