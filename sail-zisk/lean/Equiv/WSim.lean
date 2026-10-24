/-
  The simulation, one step, on RV64I's word instructions, any registers:
  addw, subw, sllw, srlw, sraw, addiw (sext.w among them), slliw, srliw,
  sraiw, and the ZisK instruction the spec translates each to (the _w ops).
-/
import Equiv.X0Sim
import Equiv.RiscvW
import Equiv.RiscvDecodeW
import Equiv.SimW

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)

/-! ## The instructions -/

structure RwInstr where
  op : ropw
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

def RwInstr.encode (i : RwInstr) : BitVec 32 :=
  rtypewF7 i.op ++ i.rs2 ++ i.rs1 ++ rtypewF3 i.op ++ i.rd ++ 0x3B#7

theorem RwInstr.fields (i : RwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x3B#7 ∧ v.extractLsb 14 12 = rtypewF3 i.op ∧ v.extractLsb 31 25 = rtypewF7 i.op ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, RwInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem RwInstr.spec_fields (i : RwInstr) :
    Transpiler.opcode i.encode = 0x3B#7 ∧ Transpiler.funct3 i.encode = rtypewF3 i.op ∧
    Transpiler.funct7 i.encode = rtypewF7 i.op ∧ Transpiler.rd i.encode = i.rd.toNat ∧
    Transpiler.rs1 i.encode = i.rs1.toNat ∧ Transpiler.rs2 i.encode = i.rs2.toNat := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.funct7, Transpiler.rd, Transpiler.rs1,
    Transpiler.rs2, RwInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · bv_decide
  all_goals congr 1; bv_decide

theorem RwInstr.zisk_of_spec (i : RwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) (ziskOpW i.op)
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  obtain ⟨hop, hf3, hf7, hrd, hr1, hr2⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 hf7 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, rtypewF3, rtypewF7, hr1, hr2, hrd,
      Transpiler.registerOp, Transpiler.ROp.zop, ziskOpW]

structure IwInstr where
  rd : BitVec 5
  rs1 : BitVec 5
  imm : BitVec 12

def IwInstr.encode (i : IwInstr) : BitVec 32 := i.imm ++ i.rs1 ++ 0#3 ++ i.rd ++ 0x1B#7

theorem IwInstr.fields (i : IwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x1B#7 ∧ v.extractLsb 14 12 = 0#3 ∧ v.extractLsb 31 20 = i.imm ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, IwInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem IwInstr.spec_fields (i : IwInstr) :
    Transpiler.opcode i.encode = 0x1B#7 ∧ Transpiler.funct3 i.encode = 0#3 ∧
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
    Transpiler.immI i.encode = i.imm.signExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.rd, Transpiler.rs1, Transpiler.immI,
    IwInstr.encode]
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · bv_decide
  · bv_decide
  · congr 1; bv_decide
  · congr 1; bv_decide
  · bv_decide

theorem addiw_val (a : BitVec 64) (imm : BitVec 12) :
    (_root_.Zisk.Functions.zisk_exec (.ZOP_ADD_W ()) a (imm.signExtend 64)).1 = addiwVal a imm := by
  simp [_root_.Zisk.Functions.zisk_exec, _root_.Zisk.Functions.sx32, addiwVal, _root_.Zisk.Functions.sign_extend,
    Sail.BitVec.signExtend, sign_extend, Sail.BitVec.extractLsb]

/-- What the spec emits for addiw, any registers: add_w(r[rs1], imm) into
    rd, or, for `addiw x0, x0, 0`, a nop; its value, rd not x0, sail-riscv's. -/
theorem IwInstr.zisk_of_spec (i : IwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) (xs : Vector (BitVec 64) 40) :
    ∃ ra rb zop va vb, zrom pc = Transpiler.mk ra rb zop (Transpiler.storeReg i.rd.toNat) 4 4 ∧
      Equiv.Zisk.PureOp zop ∧ ASrc ra xs va ∧ BSrc rb xs vb ∧
      (i.rd ≠ 0 → (_root_.Zisk.Functions.zisk_exec zop va vb).1 = addiwVal (regv xs i.rs1.toNat) i.imm) := by
  obtain ⟨hop, hf3, hrd, hr1, himm⟩ := i.spec_fields
  have l1 := i.rs1.isLt
  by_cases hnop : i.rd = 0 ∧ i.rs1 = 0 ∧ i.imm = 0
  · have ed : i.rd.toNat = 0 := by rw [hnop.1]; rfl
    have e1 : i.rs1.toNat = 0 := by rw [hnop.2.1]; rfl
    have hz : i.imm.signExtend 64 = 0#64 := by rw [hnop.2.2]; rfl
    refine ⟨_, _, _, 0, 0, (h.one (i := Transpiler.nop) fun ia => ?_).trans ?_, Equiv.Zisk.flag_pure,
      Or.inl rfl, Or.inl rfl, fun h' => absurd hnop.1 h'⟩
    · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hrd, hr1, himm, ed, e1, hz]
    · simp [Transpiler.nop, ed, storeReg_zero]
  · refine ⟨_, _, _, regv xs i.rs1.toNat, i.imm.signExtend 64,
      (h.one (i := Transpiler.immediateOp i.encode (.ZOP_ADD_W ()) (Transpiler.immI i.encode)) fun ia => ?_).trans ?_,
      ?_, Equiv.Zisk.srcA_ASrc xs _ l1, Or.inl rfl, fun _ => addiw_val _ _⟩
    · have hc : ¬ (i.rd.toNat = 0 ∧ i.rs1.toNat = 0 ∧ i.imm.signExtend 64 = 0#64) := by
        intro ⟨a, b, c⟩; apply hnop
        refine ⟨?_, ?_, ?_⟩
        · apply BitVec.eq_of_toNat_eq; simpa using a
        · apply BitVec.eq_of_toNat_eq; simpa using b
        · revert c; generalize i.imm = x; intro c; bv_decide
      simp [Transpiler.convert, Transpiler.decode, hop, hf3, hrd, hr1, himm]
      intro a b c; exact absurd ⟨a, b, c⟩ hc
    · simp [Transpiler.immediateOp, hr1, hrd, himm]
    · refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

structure SiwInstr where
  op : sopw
  rd : BitVec 5
  rs1 : BitVec 5
  sh : BitVec 5

def SiwInstr.encode (i : SiwInstr) : BitVec 32 :=
  shiftwF7 i.op ++ i.sh ++ i.rs1 ++ shiftwF3 i.op ++ i.rd ++ 0x1B#7

theorem SiwInstr.fields (i : SiwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x1B#7 ∧ v.extractLsb 14 12 = shiftwF3 i.op ∧ v.extractLsb 31 25 = shiftwF7 i.op ∧
    v.extractLsb 24 20 = i.sh ∧ v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, SiwInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

theorem SiwInstr.spec_fields (i : SiwInstr) :
    Transpiler.opcode i.encode = 0x1B#7 ∧ Transpiler.funct3 i.encode = shiftwF3 i.op ∧
    Transpiler.funct7 i.encode = shiftwF7 i.op ∧
    Transpiler.rd i.encode = i.rd.toNat ∧ Transpiler.rs1 i.encode = i.rs1.toNat ∧
    Transpiler.shamt i.encode = i.sh.zeroExtend 64 := by
  simp only [Transpiler.opcode, Transpiler.funct3, Transpiler.funct7, Transpiler.rd,
    Transpiler.rs1, Transpiler.shamt, SiwInstr.encode]
  cases i.op <;> simp only [shiftwF3, shiftwF7] <;> refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;>
    first | bv_decide | (congr 1; bv_decide)

/-- funct7 0 (slliw): bits 31:20 are below 32, so none of clzw, ctzw, cpopw,
    and funct6 is 0, not slli.uw's 2. -/
theorem f7_zero (x : BitVec 32) (h : x.extractLsb 31 25 = 0#7) :
    (x.extractLsb 31 20).toNat < 32 ∧ (x.extractLsb 31 26).toNat = 0 := by
  have a : x.extractLsb 31 20 < 32#12 := by bv_decide
  have b : x.extractLsb 31 26 = 0#6 := by bv_decide
  exact ⟨by simpa [BitVec.lt_def] using a, by simp [b]⟩

def ziskOpSW : sopw → _root_.Zisk.zisk_op
  | .SLLIW => .ZOP_SLL_W () | .SRLIW => .ZOP_SRL_W () | .SRAIW => .ZOP_SRA_W ()

theorem ziskSWVal (op : sopw) (a : BitVec 64) (sh : BitVec 5) :
    (_root_.Zisk.Functions.zisk_exec (ziskOpSW op) a (sh.zeroExtend 64)).1 = shiftiwVal op a sh := by
  cases op <;> simp [ziskOpSW, shiftiwVal, _root_.Zisk.Functions.zisk_exec,
    _root_.Zisk.Functions.sign_extend, Sail.BitVec.signExtend, sign_extend, Sail.BitVec.extractLsb,
    shift_bits_left, shift_bits_right, shift_bits_right_arith, _root_.Zisk.Functions.sra_bits,
    Sail.BitVec.toNatInt]
  all_goals
    have h32 : ((sh.toNat : Int) % 32) = ((sh.toNat : Nat) : Int) := by have := sh.isLt; omega
  case SLLIW => rw [h32, ← Int.ofNat_eq_natCast, shl_int]
  case SRLIW => rw [h32, ← Int.ofNat_eq_natCast, shr_int]
  case SRAIW =>
    rw [h32, ← Int.ofNat_eq_natCast]
    erw [shr_int]
    try simp only [Int.ofNat_eq_natCast, Int.toNat_natCast]
    rw [← BitVec.ushiftRight_eq', ← BitVec.sshiftRight_eq']
    bv_decide

theorem SiwInstr.zisk_of_spec (i : SiwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, i.sh.zeroExtend 64) (ziskOpSW i.op)
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  obtain ⟨hop, hf3, hf7, hrd, hr1, hsh⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 hf7
  case SLLIW =>
    obtain ⟨h20, h26⟩ := f7_zero i.encode hf7
    simp at h20 h26
    have n : ¬ (i.encode.toNat >>> 20 % 4096 = 1536 ∨ i.encode.toNat >>> 20 % 4096 = 1537 ∨
        i.encode.toNat >>> 20 % 4096 = 1538) := by omega
    simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, shiftwF3, shiftwF7, hr1, hrd, hsh,
      Transpiler.funct6, h26, n, Transpiler.immediateOp, Transpiler.SOp.zop, ziskOpSW]
  all_goals
    simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, shiftwF3, shiftwF7, hr1, hrd, hsh,
      Transpiler.immediateOp, Transpiler.SOp.zop, ziskOpSW]

/-! ## The steps -/

structure RwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem execute_rtypew_eq (a b c : regidx) (op : ropw) :
    execute (.RTYPEW (a, b, c, op)) = execute_RTYPEW a b c op := by simp [execute]

theorem execute_addiw_eq (imm : BitVec 12) (a b : regidx) :
    execute (.ADDIW (imm, a, b)) = execute_ADDIW imm a b := by simp [execute]

theorem execute_shiftiwop_eq (sh : BitVec 5) (a b : regidx) (op : sopw) :
    execute (.SHIFTIWOP (sh, a, b, op)) = execute_SHIFTIWOP sh a b op := by simp [execute]

set_option maxHeartbeats 4000000 in
theorem rwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RwInstr) (next : Option (BitVec 32))
    (hi : RwAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf7, hrdx, hr1x, hr2x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have v1 := getX_regv agree l1
  have v2 := getX_regv agree l2
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.RTYPEW (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.op)) ?_ i.rd.toNat i.rd.isLt
    (rtypewVal i.op (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_rtypew t _ i.op hop hf3 hf7 htp w hw'
    rw [hr2x, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_rtypew_eq, execute_rtypew t i.op i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ ?_ i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    · cases i.op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [ziskOpW] | decide
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, ziskWVal]

structure IwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : IwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
theorem iwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : IwInstr) (next : Option (BitVec 32))
    (hi : IwAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  obtain ⟨ra, rb, zop, va, vb, hrom, hpure, ha, hb, hval⟩ := i.zisk_of_spec hi.tr xs
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.ADDIW (i.imm, .Regidx i.rs1, .Regidx i.rd)) ?_ i.rd.toNat i.rd.isLt
    (addiwVal (regv xs i.rs1.toNat) i.imm) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_addiw t _ hop hf3 htp w hw'
    rw [himm, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_addiw_eq, execute_addiw t i.rs1 i.rd i.imm _ (getX_of_agree he l1 v1), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, hrom]
    refine ⟨_, Equiv.Zisk.execute_mk z ra rb zop hpure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst va vb ha hb,
      ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, xsUpd_eq hval]

structure SiwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SiwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
theorem siwstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SiwInstr) (next : Option (BitVec 32))
    (hi : SiwAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf7, hshx, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.SHIFTIWOP (i.sh, .Regidx i.rs1, .Regidx i.rd, i.op)) ?_ i.rd.toNat i.rd.isLt
    (shiftiwVal i.op (regv xs i.rs1.toNat) i.sh) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_shiftiwop t _ i.op hop hf3 hf7 htp w hw'
    rw [hshx, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_shiftiwop_eq, execute_shiftiwop t i.op i.rs1 i.rd i.sh _ (getX_of_agree he l1 v1),
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ ?_ i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    · cases i.op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp [ziskOpSW] | decide
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, ziskSWVal]

end Equiv.Sim
