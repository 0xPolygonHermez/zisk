/-
  ZisK steps that leave the fcall registers as they are (`FKeep`): a step
  on an instruction whose operation does not write them (`zstep_fkeep`),
  and every instruction of the spec's translation of a word other than a
  CSR instruction is one (`convert_keep`).
-/
import Equiv.Block
import Equiv.ZKeepAll
import Transpiler.Spec

namespace Equiv.Sim

open Equiv.Riscv
open ZiskRiscv

/-- From z to z', the fcall registers as they were. -/
def FKeep (z z' : Equiv.Zisk.ZState) : Prop :=
  ∀ r, Equiv.Zisk.fregs r = true → z'.regs.get? r = z.regs.get? r

theorem FKeep.refl (z : Equiv.Zisk.ZState) : FKeep z z := fun _ _ => rfl

theorem FKeep.trans {z1 z2 z3 : Equiv.Zisk.ZState} (h1 : FKeep z1 z2) (h2 : FKeep z2 z3) : FKeep z1 z3 :=
  fun r hr => (h2 r hr).trans (h1 r hr)

/-- A ZisK step on an instruction whose operation does not write the fcall
    registers leaves them as they are. -/
theorem zstep_fkeep {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {z z' : Equiv.Zisk.ZState} {pc : BitVec 64}
    (hpc : z.regs.get? _root_.Zisk.Register.PC = some pc) (h : Equiv.Zisk.writesF (zrom pc).op = false) {b : Bool}
    (hr : (zstep zrom).run z = .ok b z') : FKeep z z' := by
  rw [zstep_run zrom z pc hpc] at hr
  exact Equiv.Zisk.Keep.execute _ h z b z' hr

/-- The spec's operations write no fcall register. -/
theorem zop_keep :
    (∀ op : Transpiler.ROp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.ShAdd, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.ZbsOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.ZbbOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.ZbbWOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.PackOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.SextOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.ByteOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.CountOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.MOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ op : Transpiler.SOp, Equiv.Zisk.writesF op.zop = false) ∧
    (∀ d (op : Transpiler.AmoOp), Equiv.Zisk.writesF (op.zop d) = false) ∧
    (∀ d, Equiv.Zisk.writesF (Transpiler.aLoad d) = false) := by
  refine ⟨fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_,
    fun op => ?_, fun op => ?_, fun op => ?_, fun op => ?_, fun d op => ?_, fun d => ?_⟩
  all_goals (try cases d) <;> (try cases op) <;> rfl

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- The spec's translation of a word other than a CSR instruction uses no
    operation that writes the fcall registers. -/
theorem convert_keep {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × Zisk.zisk_inst)} (h : Transpiler.convert pc w next ia = some l)
    (hd : Transpiler.decode w ≠ some .csr) : ∀ p ∈ l, Equiv.Zisk.writesF p.2.op = false := by
  unfold Transpiler.convert at h
  rcases e : Transpiler.decode w with _ | i
  · rw [e] at h; cases h
  · rw [e] at h hd
    cases i
    all_goals (try (simp at hd; done))
    all_goals (try (exfalso; exact hd rfl))
    all_goals (dsimp only at h)
    all_goals (try simp only [Transpiler.uw, Transpiler.amo, Transpiler.sc, Transpiler.jalr] at h)
    all_goals ((try split at h) <;> (try split at h) <;> (try split at h) <;> (try split at h) <;>
      (try split at h) <;> (try split at h))
    all_goals (cases h)
    all_goals (intro p hp; simp only [List.mem_cons, List.not_mem_nil, or_false] at hp)
    all_goals (rcases hp with rfl | rfl | rfl | rfl | rfl)
    all_goals (simp only [Transpiler.mk, Transpiler.registerOp, Transpiler.immediateOp, Transpiler.immediateOpOrCopyb,
      Transpiler.copyb, Transpiler.singleOp, Transpiler.zextH, Transpiler.nop, Transpiler.ecall, Transpiler.lui,
      Transpiler.auipc, Transpiler.jal, Transpiler.loadOp, Transpiler.storeOp, Transpiler.branchOp]; try rfl)
    all_goals (try split) <;> (try split) <;> (try rfl) <;> simp [zop_keep]

theorem aLoad_keep (d : Bool) : Equiv.Zisk.writesF (Transpiler.aLoad d) = false := by cases d <;> rfl

theorem amoZop_keep (d : Bool) (op : Transpiler.AmoOp) : Equiv.Zisk.writesF (op.zop d) = false := by
  cases d <;> cases op <;> rfl

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- Every translation of a word other than a CSR instruction has an
    instruction at pc. -/
theorem convert_head {pc : BitVec 64} {w : BitVec 32} {next : Option (BitVec 32)} {ia : BitVec 64}
    {l : List (BitVec 64 × Zisk.zisk_inst)} (h : Transpiler.convert pc w next ia = some l)
    (hd : Transpiler.decode w ≠ some .csr) : ∃ i, (pc, i) ∈ l := by
  unfold Transpiler.convert at h
  rcases e : Transpiler.decode w with _ | i
  · rw [e] at h; cases h
  · rw [e] at h hd
    cases i
    all_goals (try (simp at hd; done))
    all_goals (try (exfalso; exact hd rfl))
    all_goals (dsimp only at h)
    all_goals (try simp only [Transpiler.uw, Transpiler.amo, Transpiler.sc, Transpiler.jalr] at h)
    all_goals ((try split at h) <;> (try split at h) <;> (try split at h) <;> (try split at h) <;>
      (try split at h) <;> (try split at h))
    all_goals (cases h)
    all_goals exact ⟨_, List.mem_cons_self⟩

/-- The spec decodes a CSR instruction only from SYSTEM's opcode. -/
theorem decode_csr_opcode {w : BitVec 32} (h : Transpiler.decode w = some .csr) :
    w.extractLsb' 0 7 = 0x73#7 := by
  have e : (Transpiler.opcode w).toNat = (w.extractLsb' 0 7).toNat := by
    simp [Transpiler.opcode, BitVec.extractLsb, BitVec.extractLsb'_toNat]
  simp only [Transpiler.decode] at h
  generalize ho : (Transpiler.opcode w).toNat = o at h
  apply BitVec.eq_of_toNat_eq
  rw [← e, ho]
  split at h <;> (repeat' (split at h)) <;> simp_all


end Equiv.Sim
