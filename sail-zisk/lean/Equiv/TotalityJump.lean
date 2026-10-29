/-
  Totality, the jumps: jal, jalr and the branches. Each is a step the
  simulation covers, but for a jump to a target 2 mod 4 (a branch, taken),
  one of the ends (`MisJumpAt`). A target out of the code is a step all the
  same: the pc leaves the code at the next step (precondition J).
-/
import Equiv.TotalityPlain
import Equiv.MisJumpSim

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Transpiler (decode convert opcode funct3)

/-- Every register's value, in a state ready to step. -/
theorem getX_ready {s : RVState} {rs : List PMA_Region} (h : StepReady s rs) (r : Nat) :
    ∃ v, getX s r = some v := by
  unfold getX; split
  · exact ⟨0, rfl⟩
  · rw [WF.get? h.wf]; exact ⟨_, rfl⟩

/-- A jump's facts from the context: as `ctx_at`, the jump in the jump group. -/
theorem ctx_at_jump {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {i : Transpiler.Inst} (hd : decode w = some i) (hg : group i = .jump) {e : BitVec 32} (he : e = w) :
    pc.toNat % 4 = 0 ∧ ExecAt ziskRegions pc ∧ ExecAt ziskRegions (pc + 2) ∧
      S.rv.mem.get? pc.toNat = some (byte e 0) ∧ S.rv.mem.get? (pc.toNat + 1) = some (byte e 1) ∧
      S.rv.mem.get? (pc.toNat + 2) = some (byte e 2) ∧ S.rv.mem.get? (pc.toNat + 3) = some (byte e 3) ∧
      Transpiled zrom pc e (nextWord S.rv pc) := by
  rw [he]
  have hb := readWord_bytes hx.word
  have hn2 : Transpiler.usesNext2 w = false := by
    cases i <;> simp only [group, reduceCtorEq] at hg <;> simp [Transpiler.usesNext2, hd]
  have hdesc : ∃ ia l, convert pc w (nextWord S.rv pc) ia = some l := by
    refine ⟨0, ?_⟩
    cases i <;> simp only [group, reduceCtorEq] at hg <;> simp only [convert, hd] <;> (repeat' split) <;>
      exact ⟨_, rfl⟩
  exact ⟨hx.aligned pc hx.code, hx.exec pc hx.code, exec_add2 (hx.exec pc hx.code) (hx.aligned pc hx.code),
    hb.1, hb.2.1, hb.2.2.1, hb.2.2.2, ctx_transpiled hx hn2 hdesc⟩

/-- An aligned pc plus an even offset: an even address. -/
theorem lsb_add_even {pc x : BitVec 64} (hal : pc.toNat % 4 = 0) (hx : x[0] = false) : (pc + x)[0] = false := by
  have h0 : pc[0] = false := by
    have := Transpiler.lsb_eq pc; rw [show pc.getLsbD 0 = pc[0] from rfl] at this; rw [this]; simp; omega
  revert h0 hx; bv_decide

/-- The step's cases, from a covered step. -/
theorem StepCase.ofOneOkT {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} (h : OneOkT ziskRegions zrom S.rv) : StepCase zrom C S :=
  .inl (.inl h)

/-- The step's cases, from a jump to a target 2 mod 4. -/
theorem StepCase.ofMisJump {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} (h : MisJumpAt ziskRegions zrom S.rv) : StepCase zrom C S :=
  .inr (.inr (.inr (.inl (.inr (.inr (.inr (.inr (.inr (.inl h)))))))))

/-! ## jal -/

set_option maxHeartbeats 2000000 in
theorem decode_op_jal {w : BitVec 32} (h : decode w = some .jal) : opcode w = 111#7 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first | (simp at h; done) | (cases h; simp_all [BitVec.toNat_eq])

def jOfWord (w : BitVec 32) : JInstr :=
  ⟨w.extractLsb' 7 5, w.extractLsb' 31 1, w.extractLsb' 12 8, w.extractLsb' 20 1, w.extractLsb' 21 10⟩

theorem jOfWord_encode {w : BitVec 32} (h : decode w = some .jal) : (jOfWord w).encode = w := by
  have hf := decode_op_jal h
  simp only [opcode, BitVec.extractLsb] at hf
  simp only [jOfWord, JInstr.encode]; revert hf; bv_decide

theorem jOfWord_lsb (w : BitVec 32) : ((jOfWord w).imm.signExtend 64)[0] = false := by
  simp only [jOfWord, JInstr.imm]; bv_decide

/-! ## jalr -/

def jrOfWord (w : BitVec 32) : JrInstr := ⟨w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 12⟩

theorem jrOfWord_encode {w : BitVec 32} (h : decode w = some .jalr) : (jrOfWord w).encode = w := by
  have hf := decode_jalr_fields h
  simp only [opcode, funct3, BitVec.extractLsb] at hf
  simp only [jrOfWord, JrInstr.encode]; revert hf; bv_decide

/-! ## The branches -/

def _root_.Transpiler.BOp.r : Transpiler.BOp → bop
  | .beq => .BEQ | .bne => .BNE | .blt => .BLT | .bge => .BGE | .bltu => .BLTU | .bgeu => .BGEU

set_option maxHeartbeats 2000000 in
theorem decode_branch_fields {w : BitVec 32} {op : Transpiler.BOp} (h : decode w = some (.branch op)) :
    opcode w = 99#7 ∧ funct3 w = btypeF3 op.r := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1; simp_all [BitVec.toNat_eq, Transpiler.BOp.r, btypeF3])
    | (cases h; simp_all [BitVec.toNat_eq, Transpiler.BOp.r, btypeF3])

def bOfWord (op : Transpiler.BOp) (w : BitVec 32) : BInstr :=
  ⟨op.r, w.extractLsb' 15 5, w.extractLsb' 20 5, w.extractLsb' 31 1, w.extractLsb' 7 1, w.extractLsb' 25 6,
    w.extractLsb' 8 4⟩

theorem bOfWord_encode {w : BitVec 32} {op : Transpiler.BOp} (h : decode w = some (.branch op)) :
    (bOfWord op w).encode = w := by
  have hf := decode_branch_fields h
  cases op <;> simp only [opcode, funct3, BitVec.extractLsb, Transpiler.BOp.r, btypeF3] at hf <;>
    simp only [bOfWord, BInstr.encode, Transpiler.BOp.r, btypeF3] <;> (revert hf; bv_decide)

theorem bOfWord_lsb (op : Transpiler.BOp) (w : BitVec 32) : ((bOfWord op w).imm.signExtend 64)[0] = false := by
  simp only [bOfWord, BInstr.imm]; bv_decide

/-! ## The jump group, assembled -/

/-- `KindCases.jump`: a jump at pc, the step's cases. -/
theorem kind_jump {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} :
    ∀ S pc w i, Ctx zrom C S pc w → decode w = some i → group i = .jump → StepCase zrom C S := by
  intro S pc w i hx hd hg
  have hal := hx.aligned pc hx.code
  cases i with
  | jal =>
    have e := jOfWord_encode hd
    have h := ctx_at_jump hx hd hg e
    have ht0 := lsb_add_even hal (jOfWord_lsb w)
    cases ht1 : (pc + (jOfWord w).imm.signExtend 64)[1] with
    | false =>
      exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inl ⟨.j (jOfWord w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, ⟨ht0, ht1⟩, h.2.2.2.2.2.2.2⟩⟩
    | true =>
      exact StepCase.ofMisJump ⟨pc, _, hx.atPc, ht0, ht1,
        .inl ⟨jOfWord w, _, ⟨h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩, rfl⟩⟩
  | jalr =>
    have e := jrOfWord_encode hd
    have h := ctx_at_jump hx hd hg e
    have hjr : JrtAt S.rv ziskRegions pc zrom (jrOfWord w) (nextWord S.rv pc) :=
      ⟨h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩
    obtain ⟨v1, hv1⟩ := getX_ready hx.ready (jrOfWord w).rs1.toNat
    have ht0 := jalrTarget_lsb v1 (jrOfWord w).imm
    cases ht1 : (jalrTarget v1 (jrOfWord w).imm)[1] with
    | false =>
      cases he : (jrOfWord w).imm[0] with
      | false =>
        exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inl ⟨.jr (jrOfWord w), hjr, he, v1, hv1, ht0, ht1⟩⟩
      | true =>
        exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inr (.inl ⟨jrOfWord w, hjr, he, v1, hv1, ht0, ht1⟩)⟩
    | true =>
      exact StepCase.ofMisJump ⟨pc, _, hx.atPc, ht0, ht1, .inr (.inl ⟨jrOfWord w, _, hjr, v1, hv1, rfl⟩)⟩
  | branch op =>
    have e := bOfWord_encode hd
    have h := ctx_at_jump hx hd hg e
    have ht0 := lsb_add_even hal (bOfWord_lsb op w)
    obtain ⟨v1, hv1⟩ := getX_ready hx.ready (bOfWord op w).rs1.toNat
    obtain ⟨v2, hv2⟩ := getX_ready hx.ready (bOfWord op w).rs2.toNat
    by_cases hmis : btypeTaken (bOfWord op w).op v1 v2 = true ∧
        (pc + (bOfWord op w).imm.signExtend 64)[1] = true
    · exact StepCase.ofMisJump ⟨pc, _, hx.atPc, ht0, hmis.2,
        .inr (.inr ⟨bOfWord op w, _, ⟨h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1, h.2.2.2.2.2.2.2⟩, v1, v2, hv1, hv2, hmis.1, rfl⟩)⟩
    · refine StepCase.ofOneOkT ⟨pc, hx.atPc, .inl ⟨.br (bOfWord op w), h.1, h.2.1, h.2.2.1, h.2.2.2.1, h.2.2.2.2.1, h.2.2.2.2.2.1, h.2.2.2.2.2.2.1,
        fun v1' v2' h1 h2 htk => ?_, h.2.2.2.2.2.2.2⟩⟩
      rw [hv1] at h1; rw [hv2] at h2
      cases h1; cases h2
      refine ⟨ht0, ?_⟩
      cases h1' : (pc + (bOfWord op w).imm.signExtend 64)[1]
      · rfl
      · exact absurd ⟨htk, h1'⟩ hmis
  | _ => simp [group] at hg

end Equiv.Final
