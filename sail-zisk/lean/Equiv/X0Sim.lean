/-
  The simulation, one step, on the forms the earlier steps leave out: x0 as
  an operand or as rd, and what the transpiler makes of them (a copyb for
  `add rd, x0, rs2`, `mv` and `li`, a flag for `nop`, no store for rd = x0).
  The ZisK instruction at pc is the spec's translation of the RISC-V word
  there (`Transpiled`), whichever form it takes.
-/
import Equiv.FromSpec
import Equiv.ZiskSpec
import Equiv.RiscvDecodeX0

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv ASrc BSrc)


/-! ## R-type -/

theorem regv_zero (xs : Vector (BitVec 64) 40) {x : BitVec 5} (h : x = 0) : regv xs x.toNat = 0 := by
  subst h; rfl

/-- What the spec emits for an R-type word, any registers, is an op on
    register-or-immediate operands into rd, and its value is sail-riscv's. -/
theorem RInstr.zisk_of_spec (i : RInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) (xs : Vector (BitVec 64) 40) :
    ∃ ra rb zop va vb, zrom pc = Transpiler.mk ra rb zop (Transpiler.storeReg i.rd.toNat) 4 4 ∧
      Equiv.Zisk.PureOp zop ∧ ASrc ra xs va ∧ BSrc rb xs vb ∧
      (_root_.Zisk.Functions.zisk_exec zop va vb).1 =
        rtypeVal i.op (regv xs i.rs1.toNat) (regv xs i.rs2.toNat) := by
  obtain ⟨hop, hf3, hf7, hrd, hr1, hr2⟩ := i.spec_fields
  have l1 := i.rs1.isLt
  have l2 := i.rs2.isLt
  -- The general form: the op on rs1 and rs2.
  have general : zrom pc = Transpiler.registerOp i.encode (ziskOp i.op) → ∃ ra rb zop va vb,
      zrom pc = Transpiler.mk ra rb zop (Transpiler.storeReg i.rd.toNat) 4 4 ∧
      Equiv.Zisk.PureOp zop ∧ ASrc ra xs va ∧ BSrc rb xs vb ∧
      (_root_.Zisk.Functions.zisk_exec zop va vb).1 =
        rtypeVal i.op (regv xs i.rs1.toNat) (regv xs i.rs2.toNat) := fun hz =>
    ⟨_, _, _, _, _, hz.trans (by simp [Transpiler.registerOp, hr1, hr2, hrd]), pureOp i.op,
      Equiv.Zisk.srcA_ASrc xs _ l1, Equiv.Zisk.srcB_BSrc xs _ l2, ziskVal _ _ _⟩
  -- A copyb of one operand (`add`/`or` with x0 as the other).
  have copy : ∀ (k : Nat) (hk : k < 32), (rtypeVal i.op (regv xs i.rs1.toNat) (regv xs i.rs2.toNat) = regv xs k) →
      zrom pc = Transpiler.mk (.A_IMM, 0) (Transpiler.srcB k) (.ZOP_COPYB ())
        (Transpiler.storeReg i.rd.toNat) 4 4 → ∃ ra rb zop va vb,
      zrom pc = Transpiler.mk ra rb zop (Transpiler.storeReg i.rd.toNat) 4 4 ∧
      Equiv.Zisk.PureOp zop ∧ ASrc ra xs va ∧ BSrc rb xs vb ∧
      (_root_.Zisk.Functions.zisk_exec zop va vb).1 =
        rtypeVal i.op (regv xs i.rs1.toNat) (regv xs i.rs2.toNat) := fun k hk hv hz =>
    ⟨_, _, _, _, _, hz, Equiv.Zisk.copyb_pure, Or.inl rfl, Equiv.Zisk.srcB_BSrc xs k hk,
      by rw [Equiv.Zisk.copyb_val]; exact hv.symm⟩
  cases hk : i.op <;> rw [hk] at hf3 hf7 general copy
  case ADD | OR =>
    by_cases z1 : i.rs1 = 0
    · refine copy _ l2 ?_ (h.one fun ia => ?_)
      · simp [hk, rtypeVal, regv_zero xs z1]
      · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, rtypeF3, rtypeF7, hr1, hr2, hrd, z1,
          Transpiler.copyb]
    by_cases z2 : i.rs2 = 0
    · refine copy _ l1 ?_ (h.one fun ia => ?_)
      · simp [hk, rtypeVal, regv_zero xs z2]
      · have e1 := ne_zero_toNat z1
        simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, rtypeF3, rtypeF7, hr1, hr2, hrd, z2, e1,
          Transpiler.copyb]
    · have e1 := ne_zero_toNat z1
      have e2 := ne_zero_toNat z2
      refine general (h.one fun ia => ?_)
      simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, rtypeF3, rtypeF7, hr1, hr2, e1, e2, hk,
        Transpiler.ROp.zop, ziskOp]
  all_goals
    refine general (h.one fun ia => ?_)
    simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf7, rtypeF3, rtypeF7, hk, Transpiler.ROp.zop, ziskOp]

/-- An R-type instruction at pc, any registers: its encoding in executable
    memory there, and the ZisK program holding the spec's translation. -/
structure RtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
/-- One step of each on an R-type instruction, any registers, x0 among
    them, keeps the states related. -/
theorem rtstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RInstr) (next : Option (BitVec 32))
    (hi : RtAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf7, hrdx, hr1x, hr2x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  obtain ⟨ra, rb, zop, va, vb, hrom, hpure, ha, hb, hval⟩ := i.zisk_of_spec hi.tr xs
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have v1 := getX_regv agree l1
  have v2 := getX_regv agree l2
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.RTYPE (.Regidx i.rs2, .Regidx i.rs1, .Regidx i.rd, i.op)) ?_ i.rd.toNat i.rd.isLt
    (rtypeVal i.op (regv xs i.rs1.toNat) (regv xs i.rs2.toNat)) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_rtype0 t _ i.op hop hf3 hf7 htp w hw'
    rw [hr2x, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_rtype_eq, execute_rtype t i.op i.rs1 i.rs2 i.rd _ _ (getX_of_agree he l1 v1)
      (getX_of_agree he l2 v2), insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, hrom]
    refine ⟨_, Equiv.Zisk.execute_mk z ra rb zop hpure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst va vb ha hb,
      ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, hval]

/-! ## I-type -/

theorem storeReg_zero : Transpiler.storeReg 0 = (.ST_NONE, 0, false) := rfl

theorem signExtend_ne_zero {imm : BitVec 12} (h : imm ≠ 0) : imm.signExtend 64 ≠ 0#64 := by
  intro e; apply h; bv_decide

/-- What the spec emits for an I-type word, any registers: an op on
    register-or-immediate operands into rd (nop and hints, with rd = x0,
    store nothing), whose value, when rd is not x0, is sail-riscv's. -/
theorem IInstr.zisk_of_spec (i : IInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) (xs : Vector (BitVec 64) 40) :
    ∃ ra rb zop va vb, zrom pc = Transpiler.mk ra rb zop (Transpiler.storeReg i.rd.toNat) 4 4 ∧
      Equiv.Zisk.PureOp zop ∧ ASrc ra xs va ∧ BSrc rb xs vb ∧
      (i.rd ≠ 0 → (_root_.Zisk.Functions.zisk_exec zop va vb).1 = itypeVal i.op (regv xs i.rs1.toNat) i.imm) := by
  obtain ⟨hop, hf3, hrd, hr1, himm⟩ := i.spec_fields
  have l1 := i.rs1.isLt
  -- op(r[rs1], imm), the op or (rs1 = x0, for addi, xori, ori) a copyb.
  have opimm : ∀ zop, Equiv.Zisk.PureOp zop →
      (i.rd ≠ 0 → (_root_.Zisk.Functions.zisk_exec zop (regv xs i.rs1.toNat) (i.imm.signExtend 64)).1 =
        itypeVal i.op (regv xs i.rs1.toNat) i.imm) →
      zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, i.imm.signExtend 64) zop
        (Transpiler.storeReg i.rd.toNat) 4 4 → ∃ ra rb zop va vb,
      zrom pc = Transpiler.mk ra rb zop (Transpiler.storeReg i.rd.toNat) 4 4 ∧
      Equiv.Zisk.PureOp zop ∧ ASrc ra xs va ∧ BSrc rb xs vb ∧
      (i.rd ≠ 0 → (_root_.Zisk.Functions.zisk_exec zop va vb).1 = itypeVal i.op (regv xs i.rs1.toNat) i.imm) :=
    fun zop hp hv hz => ⟨_, _, _, _, _, hz, hp, Equiv.Zisk.srcA_ASrc xs _ l1, Or.inl rfl, hv⟩
  have spec : ∀ zop, Equiv.Zisk.PureOp zop →
      (i.rd ≠ 0 → (_root_.Zisk.Functions.zisk_exec zop (regv xs i.rs1.toNat) (i.imm.signExtend 64)).1 =
        itypeVal i.op (regv xs i.rs1.toNat) i.imm) →
      (∀ ia, Transpiler.convert pc i.encode next ia = some [(pc, Transpiler.immediateOp i.encode zop (Transpiler.immI i.encode))]) →
      ∃ ra rb zop va vb,
      zrom pc = Transpiler.mk ra rb zop (Transpiler.storeReg i.rd.toNat) 4 4 ∧
      Equiv.Zisk.PureOp zop ∧ ASrc ra xs va ∧ BSrc rb xs vb ∧
      (i.rd ≠ 0 → (_root_.Zisk.Functions.zisk_exec zop va vb).1 = itypeVal i.op (regv xs i.rs1.toNat) i.imm) :=
    fun zop hp hv hc => opimm zop hp hv ((h.one hc).trans (by simp [Transpiler.immediateOp, hr1, hrd, himm]))
  -- The value of a copyb of the immediate, rs1 being x0.
  have li : i.rs1 = 0 → i.op = .ADDI ∨ i.op = .XORI ∨ i.op = .ORI →
      (_root_.Zisk.Functions.zisk_exec (.ZOP_COPYB ()) (regv xs i.rs1.toNat) (i.imm.signExtend 64)).1 =
        itypeVal i.op (regv xs i.rs1.toNat) i.imm := by
    intro z1 hk
    rw [Equiv.Zisk.copyb_val, regv_zero xs z1]
    rcases hk with hk | hk | hk <;> simp [hk, itypeVal, sign_extend, Sail.BitVec.signExtend]
  have ival : ∀ (op : iop), i.op = op → (i.rd ≠ 0 →
      (_root_.Zisk.Functions.zisk_exec (ziskOp (iopR op)) (regv xs i.rs1.toNat) (i.imm.signExtend 64)).1 =
        itypeVal i.op (regv xs i.rs1.toNat) i.imm) := by
    intro op hk _; rw [hk, ziskIVal]
  cases hk : i.op <;> rw [hk] at hf3 spec opimm li ival
  case ADDI =>
    by_cases zd : i.rd = 0
    · have ed : i.rd.toNat = 0 := by rw [zd]; rfl
      by_cases z1 : i.rs1 = 0
      · -- nop
        have e1 : i.rs1.toNat = 0 := by rw [z1]; rfl
        refine ⟨_, _, _, 0, 0, (h.one (i := Transpiler.nop) fun ia => ?_).trans ?_, Equiv.Zisk.flag_pure, Or.inl rfl, Or.inl rfl,
          fun h' => absurd zd h'⟩
        · simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, ed, e1]
        · simp [Transpiler.nop, ed, storeReg_zero]
      · -- a hint: flag(r[rs1], imm), no store
        have e1 := ne_zero_toNat z1
        refine ⟨_, _, _, regv xs i.rs1.toNat, i.imm.signExtend 64,
          (h.one (i := Transpiler.hint i.encode) fun ia => ?_).trans ?_,
          Equiv.Zisk.flag_pure, Equiv.Zisk.srcA_ASrc xs _ l1, Or.inl rfl, fun h' => absurd zd h'⟩
        · simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, ed, e1]
        · simp [Transpiler.hint, ed, storeReg_zero, hr1, himm]
    have ed := ne_zero_toNat zd
    by_cases hmv : i.imm = 0 ∧ i.rs1 ≠ 0
    · -- mv: copyb(r[rs1])
      have e1 := ne_zero_toNat hmv.2
      have hz : i.imm.signExtend 64 = 0#64 := by rw [hmv.1]; rfl
      refine ⟨_, _, _, 0, regv xs i.rs1.toNat, (h.one (i := Transpiler.copyb i.encode 1) fun ia => ?_).trans ?_,
        Equiv.Zisk.copyb_pure,
        Or.inl rfl, Equiv.Zisk.srcB_BSrc xs _ l1, fun _ => ?_⟩
      · simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, himm, ed, e1, hz]
      · simp [Transpiler.copyb, hr1, hrd]
      · rw [Equiv.Zisk.copyb_val, hmv.1]; simp [itypeVal, sign_extend, Sail.BitVec.signExtend]
    · by_cases z1 : i.rs1 = 0
      · have e1 : i.rs1.toNat = 0 := by rw [z1]; rfl
        refine spec _ Equiv.Zisk.copyb_pure (fun _ => li z1 (Or.inl rfl)) fun ia => ?_
        simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, himm, ed, e1,
          Transpiler.immediateOpOrCopyb]
      · have e1 := ne_zero_toNat z1
        have hnz : i.imm.signExtend 64 ≠ 0#64 := signExtend_ne_zero (fun e => hmv ⟨e, z1⟩)
        refine spec _ (pureOp _) (ival _ rfl) fun ia => ?_
        simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, himm, ed, e1, hnz,
          Transpiler.immediateOpOrCopyb, ziskOp, iopR]
  case XORI | ORI =>
    by_cases z1 : i.rs1 = 0
    · have e1 : i.rs1.toNat = 0 := by rw [z1]; rfl
      refine spec _ Equiv.Zisk.copyb_pure (fun _ => li z1 (by simp)) fun ia => ?_
      simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, himm, e1,
        Transpiler.immediateOpOrCopyb]
    · have e1 := ne_zero_toNat z1
      refine spec _ (pureOp _) (ival _ rfl) fun ia => ?_
      simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, hr1, hrd, himm, e1,
        Transpiler.immediateOpOrCopyb, ziskOp, iopR]
  all_goals
    refine spec _ (pureOp _) (ival _ rfl) fun ia => ?_
    simp [Transpiler.convert, Transpiler.decode, hop, hf3, itypeF3, himm, ziskOp, iopR]

structure ItAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : IInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem xsUpd_eq {xs : Vector (BitVec 64) 40} {rd : BitVec 5} {a b : BitVec 64} (h : rd ≠ 0 → a = b) :
    Equiv.Zisk.xsUpd xs rd.toNat a = Equiv.Zisk.xsUpd xs rd.toNat b := by
  by_cases z : rd = 0
  · have : rd.toNat = 0 := by rw [z]; rfl
    simp [Equiv.Zisk.xsUpd, this]
  · rw [h z]

set_option maxHeartbeats 4000000 in
/-- One step of each on an I-type instruction, any registers — nop, hints,
    mv and li among them — keeps the states related. -/
theorem itstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : IInstr) (next : Option (BitVec 32))
    (hi : ItAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
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
    (.ITYPE (i.imm, .Regidx i.rs1, .Regidx i.rd, i.op)) ?_ i.rd.toNat i.rd.isLt
    (itypeVal i.op (regv xs i.rs1.toNat) i.imm) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_itype0 t _ i.op hop hf3 htp w hw'
    rw [himm, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_itype_eq, execute_itype t i.op i.rs1 i.rd i.imm _ (getX_of_agree he l1 v1),
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, hrom]
    refine ⟨_, Equiv.Zisk.execute_mk z ra rb zop hpure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst va vb ha hb,
      ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, xsUpd_eq hval]

/-! ## Immediate shifts -/

theorem SInstr.zisk_of_spec (i : SInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IMM, i.sh.zeroExtend 64) (ziskOp (sopR i.op))
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  obtain ⟨hop, hf3, hf6, hrd, hr1, hsh⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 hf6 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, hf6, shiftF3, shiftF6, hr1, hrd, hsh,
      Transpiler.immediateOp, Transpiler.SOp.zop, ziskOp, sopR]

structure StAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
theorem ststep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SInstr) (next : Option (BitVec 32))
    (hi : StAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf6, hshx, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have v1 := getX_regv agree l1
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.SHIFTIOP (i.sh, .Regidx i.rs1, .Regidx i.rd, i.op)) ?_ i.rd.toNat i.rd.isLt
    (shiftVal i.op (regv xs i.rs1.toNat) i.sh) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_shiftiop0 t _ i.op hop hf3 hf6 htp w hw'
    rw [hshx, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _ hnext _
    rw [execute_shiftiop_eq, execute_shiftiop t i.op i.rs1 i.rd i.sh _ (getX_of_agree he l1 v1),
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ (pureOp _) i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst _ _
      (Equiv.Zisk.srcA_ASrc xs _ l1) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, ziskSVal]

/-! ## lui and auipc, any rd -/

/-- lui, or an auipc the transpiler does not fuse with the word after it,
    any rd: copyb(0, its value) into rd. -/
theorem UInstr.zisk_of_spec (i : UInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (hnf : ∀ n, next = some n → Transpiler.fuses i.encode n = false)
    (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (.A_IMM, 0) (.B_IMM, utypeVal i.op pc i.imm) (.ZOP_COPYB ())
      (Transpiler.storeReg i.rd.toNat) 4 4 := by
  obtain ⟨hop, hrd, himm⟩ := i.spec_fields
  refine h.one fun ia => ?_
  have hplain : Transpiler.auipc i.encode pc next = Transpiler.auipc.plain i.encode pc := by
    cases next with
    | none => rfl
    | some n => simp [Transpiler.auipc, hnf n rfl]
  cases hk : i.op <;> rw [hk] at hop <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, uopEnc, hrd, himm, hplain,
      Transpiler.lui, Transpiler.auipc.plain, utypeVal, sign_extend, Sail.BitVec.signExtend]

structure UtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : UInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  nf : ∀ n, next = some n → Transpiler.fuses i.encode n = false
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
theorem utstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : UInstr) (next : Option (BitVec 32))
    (hi : UtAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, himm, hrdx, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw (.UTYPE (i.imm, .Regidx i.rd, i.op))
    ?_ i.rd.toNat i.rd.isLt (utypeVal i.op pc' i.imm) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_utype t _ i.op hop htp w hw'
    rw [himm, hrdx] at this; exact this
  · intro t _ _ _ _ _ hnext hother
    rw [execute_utype_eq, execute_utype t i.op i.rd i.imm pc' (other_regs hpc mi hmi hother).1,
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.nf hi.tr]
    refine ⟨_, Equiv.Zisk.execute_mk z _ _ _ Equiv.Zisk.copyb_pure i.rd.toNat i.rd.isLt xs pc' st hx hzpc hst
      _ _ (Or.inl rfl) (Or.inl rfl), ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Equiv.Zisk.copyb_val]

/-! ## Branches, x0 operands among them -/

theorem BInstr.zisk_of_spec (i : BInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) (zbopOp i.op)
      (Transpiler.storeReg 0) i.zjumps.1 i.zjumps.2 := by
  obtain ⟨hop, hf3, hr1, hr2, himm⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.op <;> rw [hk] at hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, btypeF3, hr1, hr2, himm, hk,
      Transpiler.branchOp, BInstr.zjumps, zbopOp, zbopNeg, storeReg_zero]

structure BtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : BInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tal : (pc + i.imm.signExtend 64)[0] = false ∧ (pc + i.imm.signExtend 64)[1] = false
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
/-- One step of each on a branch, any registers (`beqz` and the like among
    them), keeps the states related, both at the branch's target. -/
theorem btstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : BInstr) (next : Option (BitVec 32))
    (hi : BtAt s rs pc zrom i next)
    (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (hv2 : getX s i.rs2.toNat = some v2) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (i.target pc v1 v2) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, h12, h11, h10, h4, hr2x, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hr2 : regv xs i.rs2.toNat = v2 := by
    have := getX_regv agree l2; rw [hv2] at this; exact (Option.some.inj this).symm
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_gen s rs hr pc' hpc _ _ _ _ hw (.BTYPE (i.imm, .Regidx i.rs2, .Regidx i.rs1, i.op))
      (by intro t htp ⟨w, hw'⟩ hmt
          have := decode_btype t _ i.op hop hf3 htp w hw'
          rw [h12, h11, h10, h4, hr2x, hr1x] at this; exact this)
      0 0 (i.target pc' v1 v2)
      (by intro t he _ _ _ _ hnext hother
          rw [execute_btype_eq]
          have hpct : t.regs.get? Register.PC = some pc' := by
            rw [hother _ (by decide) (by decide) (by decide)]; exact hpc
          have hmt : t.regs.get? Register.misa = some mi := by
            rw [hother _ (by decide) (by decide) (by decide)]; exact hmi
          rw [execute_btype t i.op i.rs1 i.rs2 i.imm v1 v2 pc' mi (getX_of_agree he l1 hv1)
            (getX_of_agree he l2 hv2) hpct hmt hi.tal]
          simp only [setX, if_true, BInstr.target]
          split
          · rfl
          · rw [insert_same t _ _ hnext]) n
  have hz := Equiv.Zisk.execute_mkj z _ _ _ (zbop_pure i.op) 0 (by omega) i.zjumps.1 i.zjumps.2 xs pc' st
    hx hzpc hst _ _ (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2)
  refine ⟨s', _, hrun, by rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]; exact hz,
    ⟨hr.keep hwf ho, ⟨xs, ?_, ?_⟩, ⟨_, hpc'', ?_⟩, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, ?_⟩,
    hpc'', hmem⟩
  · simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs, hx]
  · intro k hk1 hk2; rw [hxs' k hk2]; simp only [setX, if_true]; exact agree k hk1 hk2
  · simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
    rw [hr1, hr2, BInstr.target, ← zbop_flag, BInstr.zjumps]
    cases zbopNeg i.op <;> cases (_root_.Zisk.Functions.zisk_exec (zbopOp i.op) v1 v2).2 <;> simp
  · show (z.mem) = s'.mem
    rw [hmem, hzm]

/-! ## Stores, x0 operands among them (`sd zero, off(sp)`) -/

theorem SwInstr.zisk_of_spec' (i : SwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) (.ZOP_COPYB ())
      (.ST_IND, (i.immHi ++ i.immLo).signExtend 64, false) 4 4 (w := i.k.w) := by
  obtain ⟨hop, hf3, hr1, hr2, himm⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.k <;> rw [hk] at hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, SKind.f3, hr1, hr2, himm, hk, Transpiler.storeOp,
      SKind.w]

theorem SdInstr.zisk_of_spec' (i : SdInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (Transpiler.srcB i.rs2.toNat) (.ZOP_COPYB ())
      (.ST_IND, (i.immHi ++ i.immLo).signExtend 64, false) 4 4 (w := 8) := by
  obtain ⟨hop, hf3, hr1, hr2, himm⟩ := i.spec_fields
  refine h.one fun ia => ?_
  simp [Transpiler.convert, Transpiler.decode, hop, hf3, hr1, hr2, himm, Transpiler.storeOp]

/-- A narrow store at pc, any registers. -/
structure SwtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
/-- One step of each on a narrow store, any registers, keeps the states
    related, when the store's address is good to write. -/
theorem swtstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SwInstr) (next : Option (BitVec 32))
    (hi : SwtAt s rs pc zrom i next)
    (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (hv2 : getX s i.rs2.toNat = some v2)
    (hok : SwOk rs (v1 + (i.immHi ++ i.immLo).signExtend 64) i.k.w) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = Equiv.memWrite (n := i.k.w) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat
        (v2.setWidth (8 * i.k.w)) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hhi, hlo, hr2x, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hr2 : regv xs i.rs2.toNat = v2 := by
    have := getX_regv agree l2; rw [hv2] at this; exact (Option.some.inj this).symm
  obtain ⟨hal, ⟨r, hm, hwr⟩, hzw, hzn⟩ := hok
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_store s rs hr pc' hpc _ _ _ _ hw
      (.STORE (i.immHi ++ i.immLo, .Regidx i.rs2, .Regidx i.rs1, (i.k.w : Int)))
      (by intro t htp ⟨w, hw'⟩ hmt
          have := i.k.decode t _ hop hf3 htp w hw'
          rw [hhi, hlo, hr2x, hr1x] at this; exact this)
      (Equiv.memWrite (n := i.k.w) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat (v2.setWidth (8 * i.k.w)))
      (by intro t he htm hmr hsec hwf'
          rw [execute_store_w, Int.toNat_natCast]
          obtain ⟨sv, hsv, hpm⟩ := hr.pmm
          rw [← htm]
          exact i.k.exec t hwf' rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rs2 (i.immHi ++ i.immLo) v1 v2
            (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) r hm hwr hal) n
  have ha := Equiv.Zisk.srcA_ASrc xs _ l1
  have hb := Equiv.Zisk.srcB_BSrc xs _ l2
  rw [hr1] at ha; rw [hr2] at hb
  have hz := Equiv.Zisk.execute_store_mk z _ _ ((i.immHi ++ i.immLo).signExtend 64) i.k.w xs pc' st hx hzpc hst
    v1 v2 ha hb hzw hzn
  refine ⟨s', _, hrun, by rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]; exact hz,
    ⟨hr.keep hwf ho, ⟨xs, ?_, ?_⟩, ⟨_, hpc'', by simp⟩, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩,
    ?_⟩, hpc'', hmem⟩
  · simp [Std.ExtDHashMap.get?_insert, hx]
  · intro k hk1 hk2; rw [hxs' k hk2]; exact agree k hk1 hk2
  · simp only [hmem, hzm]

structure SdtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SdInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
/-- One step of each on `sd`, any registers, keeps the states related, when
    the store's address is good to write. -/
theorem sdtstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SdInstr) (next : Option (BitVec 32))
    (hi : SdtAt s rs pc zrom i next)
    (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (hv2 : getX s i.rs2.toNat = some v2)
    (hok : SdOk rs (v1 + (i.immHi ++ i.immLo).signExtend 64)) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = Equiv.memWrite (n := 8) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat v2 := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hhi, hlo, hr2x, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hr2 : regv xs i.rs2.toNat = v2 := by
    have := getX_regv agree l2; rw [hv2] at this; exact (Option.some.inj this).symm
  obtain ⟨hal, ⟨r, hm, hwr⟩, hzw, hzn⟩ := hok
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_store s rs hr pc' hpc _ _ _ _ hw (.STORE (i.immHi ++ i.immLo, .Regidx i.rs2, .Regidx i.rs1, 8))
      (by intro t htp ⟨w, hw'⟩ hmt
          have := decode_sd t _ hop hf3 htp w hw'
          rw [hhi, hlo, hr2x, hr1x] at this; exact this)
      (Equiv.memWrite (n := 8) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat v2)
      (by intro t he htm hmr hsec hwf'
          rw [execute_store_eq]
          obtain ⟨sv, hsv, hpm⟩ := hr.pmm
          rw [← htm]
          exact execute_sd t hwf' rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rs2 (i.immHi ++ i.immLo) v1 v2
            (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) r hm hwr hal) n
  have ha := Equiv.Zisk.srcA_ASrc xs _ l1
  have hb := Equiv.Zisk.srcB_BSrc xs _ l2
  rw [hr1] at ha; rw [hr2] at hb
  have hz := Equiv.Zisk.execute_store_mk z _ _ ((i.immHi ++ i.immLo).signExtend 64) 8 xs pc' st hx hzpc hst
    v1 v2 ha hb hzw hzn
  refine ⟨s', _, hrun, by rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]; exact hz,
    ⟨hr.keep hwf ho, ⟨xs, ?_, ?_⟩, ⟨_, hpc'', by simp⟩, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩,
    ?_⟩, hpc'', hmem⟩
  · simp [Std.ExtDHashMap.get?_insert, hx]
  · intro k hk1 hk2; rw [hxs' k hk2]; exact agree k hk1 hk2
  · simp only [hmem, hzm]
    congr 1

/-! ## Narrow loads, any registers -/

theorem LwInstr.zisk_of_spec' (i : LwInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IND, i.imm.signExtend 64) i.k.op
      (Transpiler.storeReg i.rd.toNat) 4 4 (w := i.k.w) := by
  obtain ⟨hop, hf3, hrd, hr1, himm⟩ := i.spec_fields
  refine h.one fun ia => ?_
  cases hk : i.k <;> rw [hk] at hf3 <;>
  · simp [Transpiler.convert, Transpiler.decode, hop, hf3, LKind.f3, hr1, hrd, himm, hk, Transpiler.loadOp,
      LKind.op, LKind.w]

structure LwtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LwInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
/-- One step of each on a narrow load, any registers (a load to x0 still
    reads), keeps the states related, when the address is good to read. -/
theorem lwtstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LwInstr) (next : Option (BitVec 32))
    (hi : LwtAt s rs pc zrom i next)
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (v : BitVec (8 * i.k.w))
    (hok : LwOk s rs (v1 + i.imm.signExtend 64) i.k.w v) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  obtain ⟨hu, hwd⟩ := i.k.dec _ hf3
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hvalid := i.k.valid
  rw [← hu, ← hwd] at hvalid
  obtain ⟨hal, ⟨r, hm, hrb⟩, hzr, hzn, hbytes⟩ := hok
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.LOAD (i.imm, .Regidx i.rs1, .Regidx i.rd, i.k.u, (i.k.w : Int))) ?_ i.rd.toNat i.rd.isLt
    (extend_value i.k.u v) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_load0 t _ hop hvalid htp w hw'
    rw [himm, hr1x, hrdx, hu, hwd] at this; exact this
  · intro t he htm hmr hsec hwf hnext _
    rw [execute_load_w, Int.toNat_natCast, insert_same t _ _ hnext]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    have hv := Equiv.riscv_readBytes t i.k.w _ v (htm ▸ hbytes)
    exact i.k.exec t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rd i.imm v1 (getX_of_agree he l1 hv1)
      r hm hrb hal v hv
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]
    have ha := Equiv.Zisk.srcA_ASrc xs _ l1
    rw [hr1] at ha
    have hex := Equiv.Zisk.execute_load_mk z _ i.k.op i.k.pure i.rd.toNat i.rd.isLt _ i.k.w xs pc' st hx hzpc hst
      v1 ha v hzr hzn (i.k.w_ne8 _) (by rw [hzm]; exact hbytes)
    refine ⟨_, hex, ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, i.k.val]

theorem ld_f3 (x : BitVec 32) (h : x.extractLsb 14 12 = 3#3) :
    x.extractLsb 14 14 = 0#1 ∧ x.extractLsb 13 12 = 3#2 := by
  constructor <;> bv_decide

theorem LdInstr.zisk_of_spec' (i : LdInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (Transpiler.srcA i.rs1.toNat) (.B_IND, i.imm.signExtend 64) (.ZOP_COPYB ())
      (Transpiler.storeReg i.rd.toNat) 4 4 (w := 8) := by
  obtain ⟨hop, hf3, hrd, hr1, himm⟩ := i.spec_fields
  refine h.one fun ia => ?_
  simp [Transpiler.convert, Transpiler.decode, hop, hf3, hr1, hrd, himm, Transpiler.loadOp]

structure LdtAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LdInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

set_option maxHeartbeats 4000000 in
/-- One step of each on `ld`, any registers, keeps the states related, when
    the address is good to read. -/
theorem ldtstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LdInstr) (next : Option (BitVec 32))
    (hi : LdtAt s rs pc zrom i next)
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (v : BitVec 64)
    (hok : LdOk s rs (v1 + i.imm.signExtend 64) v) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  obtain ⟨h14, h13⟩ := ld_f3 _ hf3
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  obtain ⟨hal, ⟨r, hm, hrb⟩, hzr, hzn, hzi, hbytes⟩ := hok
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
    (.LOAD (i.imm, .Regidx i.rs1, .Regidx i.rd, false, 8)) ?_ i.rd.toNat i.rd.isLt v (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_load0 t _ hop (by rw [h13, h14]; decide) htp w hw'
    rw [himm, hr1x, hrdx, h13, h14] at this; exact this
  · intro t he htm hmr hsec hwf hnext _
    rw [execute_load_eq, insert_same t _ _ hnext]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    have hv := Equiv.riscv_readBytes t 8 _ v (htm ▸ hbytes)
    exact execute_ld t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rd i.imm v1 (getX_of_agree he l1 hv1)
      r hm hrb hal v hv
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]
    have ha := Equiv.Zisk.srcA_ASrc xs _ l1
    rw [hr1] at ha
    have hex := Equiv.Zisk.execute_load_mk z _ (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure i.rd.toNat i.rd.isLt _ 8
      xs pc' st hx hzpc hst v1 ha v hzr hzn (by simp [hzi]) (by rw [hzm]; exact hbytes)
    refine ⟨_, hex, ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx, Equiv.Zisk.copyb_val]

end Equiv.Sim
