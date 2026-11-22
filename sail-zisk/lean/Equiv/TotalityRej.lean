/-
  Totality, the words the spec decodes to nothing: by their opcode, a word
  the transpiler halts on (a 16-bit parcel, a word RISC-V rejects within an
  opcode the profile has or for its opcode, a vector word, a SYSTEM word of
  funct3 4), a float word (precondition F), or a SYSTEM word of funct3 0
  but `ecall` and `ebreak` (`mret`, `wfi`, ...), which the machine refuses.
-/
import Equiv.TotalitySys

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Transpiler (decode convert opcode funct3)

/-- A word the spec halts on at pc: one of the ends. -/
theorem StepCase.ofHalt {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    (hd : decode w = none) (hh : HaltWord w) : StepCase zrom C S :=
  StepCase.ofEnd (.inr (.inr (.inr (.inl ⟨pc, w, hx.atPc, hh,
    ctx_words hx ⟨0, _, by simp only [convert, hd]; rfl⟩⟩))))

theorem opcode_eq {w : BitVec 32} {k : Nat} (h : (opcode w).toNat = k) : w.extractLsb 6 0 = BitVec.ofNat 7 k :=
  bv_of_toNat h

/-- A SYSTEM word of funct3 0 the spec decodes to nothing: neither `ecall`
    nor `ebreak`, so the machine's `system` kind. -/
theorem system_kind {w : BitVec 32} (hd : decode w = none) (ho : (opcode w).toNat = 115)
    (hf : (funct3 w).toNat = 0) : ZiskRiscv.kind (ZiskRiscv.fields w) = .system := by
  have h1 : w ≠ 0x00000073#32 := by
    intro e; subst e; rw [show decode 0x00000073#32 = some .ecall from rfl] at hd; cases hd
  have h2 : w ≠ 0x00100073#32 := by
    intro e; subst e; rw [show decode 0x00100073#32 = some .ebreak from rfl] at hd; cases hd
  have ho' := opcode_eq ho
  have hf' : w.extractLsb 14 12 = 0#3 := bv_of_toNat hf
  unfold ZiskRiscv.kind
  have e1 : (ZiskRiscv.fields w).opcode = 115 := ho
  have e2 : (ZiskRiscv.fields w).funct3 = 0 := hf
  rw [e1, e2]
  simp only [ZiskRiscv.fields, Bool.and_eq_true, beq_iff_eq]
  split
  · next hc =>
    exfalso; apply h1
    obtain ⟨⟨a, b⟩, c⟩ := hc
    have a' := bv_of_toNat a; have b' := bv_of_toNat b; have c' := bv_of_toNat c
    simp only [BitVec.extractLsb] at ho' hf'
    revert a' b' c' ho' hf'; bv_decide
  · split
    · next hc =>
      exfalso; apply h2
      obtain ⟨⟨a, b⟩, c⟩ := hc
      have a' := bv_of_toNat a; have b' := bv_of_toNat b; have c' := bv_of_toNat c
      simp only [BitVec.extractLsb] at ho' hf'
      revert a' b' c' ho' hf'; bv_decide
    · rfl

theorem rvc_of {w : BitVec 32} (h : (opcode w).toNat % 4 ≠ 3) : RvcWord w := by
  intro e; apply h
  have hb : (opcode w).extractLsb 1 0 = 3#2 := by
    simp only [byte] at e; simp only [opcode]; revert e; bv_decide
  have := congrArg BitVec.toNat hb
  simp only [BitVec.extractLsb, BitVec.extractLsb'_toNat, Nat.shiftRight_zero] at this
  simpa using this

set_option maxHeartbeats 4000000 in
/-- `KindCases.rejected`: a word the spec decodes to nothing at pc, the
    step's cases. -/
theorem kind_rejected {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} :
    ∀ S pc w, Ctx zrom C S pc w → decode w = none → StepCase zrom C S := by
  intro S pc w hx hd
  have hlt : (opcode w).toNat < 128 := (opcode w).isLt
  generalize ho : (opcode w).toNat = o at hlt
  rcases (by omega : o % 4 ≠ 3 ∨
      (o = 3 ∨ o = 15 ∨ o = 19 ∨ o = 27 ∨ o = 35 ∨ o = 47 ∨ o = 51 ∨ o = 59 ∨ o = 99 ∨ o = 103) ∨
      (o = 11 ∨ o = 31 ∨ o = 43 ∨ o = 63 ∨ o = 91 ∨ o = 95 ∨ o = 107 ∨ o = 123 ∨ o = 127) ∨
      (o = 87 ∨ o = 119) ∨ (o = 7 ∨ o = 39 ∨ o = 67 ∨ o = 71 ∨ o = 75 ∨ o = 79 ∨ o = 83) ∨ o = 115 ∨
      (o = 23 ∨ o = 55 ∨ o = 111)) with hr | hr | hr | hr | hr | hr | hr
  · exact StepCase.ofHalt hx hd (.inr (.inr (.inr (.inr (.inr (.inr (rvc_of (ho ▸ hr))))))))
  · refine StepCase.ofHalt hx hd (.inr (.inr (.inr (.inl ⟨hd, ?_⟩))))
    unfold RejOpcode; rw [ho]; exact hr
  · refine StepCase.ofHalt hx hd (.inr (.inr (.inl ?_)))
    have e := opcode_eq ho
    simp only [RejWord, e]
    rcases hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> decide
  · refine StepCase.ofHalt hx hd (.inr (.inr (.inr (.inr (.inl ?_)))))
    have e := opcode_eq ho
    simp only [VecWord, e]
    rcases hr with rfl | rfl <;> decide
  · refine StepCase.ofBroken hx (.inr (.inl ?_))
    unfold BreakF; dsimp only
    have : (w.extractLsb' 0 7).toNat = o := ho
    rw [this]; omega
  · subst hr
    have hf8 : (funct3 w).toNat < 8 := (funct3 w).isLt
    generalize hf : (funct3 w).toNat = f at hf8
    rcases (by omega : f = 4 ∨ f = 0 ∨ (f = 1 ∨ f = 2 ∨ f = 3 ∨ f = 5 ∨ f = 6 ∨ f = 7)) with rfl | rfl | hf'
    · exact StepCase.ofHalt hx hd (.inr (.inr (.inr (.inr (.inr (.inl ⟨opcode_eq ho, bv_of_toNat hf⟩))))))
    · have hk := system_kind hd ho hf
      have hall : ∀ fetch : Nat → Option (BitVec 32), fetch pc.toNat = some w →
          ZiskRiscv.decode fetch pc.toNat = .malformed
            "a SYSTEM instruction other than ecall and ebreak (mret, wfi, sfence.vma, ...)" := by
        intro fetch hfe; simp only [ZiskRiscv.decode, hfe, hk]
      exact StepCase.ofEnd (.inr (.inr (.inr (.inr (.inl ⟨pc, w, _, hx.atPc, hx.code, hx.word,
        hall _ (by simp [Transpiler.fetch3]), ⟨0, _, by simp only [convert, hd]; rfl⟩, .inr hall⟩)))))
    · exfalso
      have n1 : (w == 0x00000073#32) = false := by
        simp only [beq_eq_false_iff_ne]; intro e; subst e
        rw [show (funct3 0x00000073#32).toNat = 0 from rfl] at hf; omega
      have n2 : (w == 0x00100073#32) = false := by
        simp only [beq_eq_false_iff_ne]; intro e; subst e
        rw [show (funct3 0x00100073#32).toNat = 0 from rfl] at hf; omega
      simp only [Transpiler.decode, ho, hf, n1, n2] at hd
      rcases hf' with rfl | rfl | rfl | rfl | rfl | rfl <;> simp at hd
  · exfalso
    rcases hr with rfl | rfl | rfl <;> simp only [Transpiler.decode, ho] at hd <;> cases hd

/-- Every kind of word, its step's cases: what `step_case` takes. -/
theorem kind_cases (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (C : BitVec 64 → Prop) : KindCases zrom C :=
  ⟨kind_rejected, kind_plain, kind_jump, kind_mem, kind_sys⟩

end Equiv.Final
