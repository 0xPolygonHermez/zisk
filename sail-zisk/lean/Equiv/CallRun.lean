/-
  A ZisK call at pc, as the run meets it: the word there the spec decodes
  as a call (with the word after it), the ZisK program holding its
  translation. One step of the machine (`CStep`) and ZisK's one or two
  steps keep the states related.
-/
import Equiv.CallDecode

namespace Equiv.Machine

open Equiv.Riscv Equiv.Sim
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The word after the next one, as the transpiler reads it. -/
noncomputable def nextWord2 (s : RVState) (pc : BitVec 64) : Option (BitVec 32) :=
  ZiskRiscv.readWord s.mem (pc + 8).toNat

set_option maxHeartbeats 4000000 in
/-- A ZisK call at pc: the machine's step on it, at a code address (and,
    for a call of more than one word, pc + 4 one too; for one of three,
    pc + 8), and ZisK's steps on its translation, from related states to
    related states. -/
theorem callstep_m (C : BitVec 64 → Prop) (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState)
    (rs : List PMA_Region) (hrel : Rel S.rv z rs) (hf : Equiv.Zisk.FcallRel S.fcall z)
    (hx : ¬ ExecAt rs EXIT_ADDR) (pc : BitVec 64) (hpc : S.rv.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (w : BitVec 32) (hex : ExecAt rs pc)
    (hrw : ZiskRiscv.readWord S.rv.mem pc.toNat = some w)
    (htr : Transpiled zrom pc w (nextWord S.rv pc) (nextWord2 S.rv pc))
    (h4 : (pc + 4).toNat = pc.toNat + 4) (h8 : (pc + 8).toNat = pc.toNat + 8) (c : ZiskRiscv.Call)
    (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord S.rv pc) (nextWord2 S.rv pc)) pc.toNat = .call c)
    (hC : C pc) (hC4 : c.len ≠ 1 → C (pc + 4)) (hC8 : c.len = 3 → C (pc + 8)) (hok : CallOk S c) :
    ∃ S' z' k', CStep C S S' ∧ ZRun zrom k' z z' ∧ Rel S'.rv z' rs ∧ Equiv.Zisk.FcallRel S'.fcall z' ∧
      (∀ a, Equiv.OffRam a → ¬ (Equiv.IN_LO ≤ a ∧ a < Equiv.IN_LO + 8) → S'.rv.mem.get? a = S.rv.mem.get? a) ∧
      Grows S.rv.mem S'.rv.mem := by
  have hw := decode_call_wf hd
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  have hcs : ∀ S', ZiskRiscv.call S c = .ok S' → CStep C S S' := by
    intro S' hc
    refine ⟨?_, by rw [hg]; exact hC, c, hc, fun hl => by rw [hg]; exact ⟨hC4 hl, h4⟩,
      fun hl => by rw [hg]; exact ⟨hC8 hl, h8⟩, fun fetch f0 f4 f8 => ?_⟩
    · rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hex)
    · rw [hg] at f0 f4 f8 ⊢
      refine decode_any3 hd fetch (by rw [f0]; exact hrw) (fun hl => ?_) fun hl => ?_
      · rw [f4 hl, nextWord, h4]; rfl
      · rw [f8 hl, nextWord2, h8]; rfl
  obtain ⟨ia, hz⟩ := transpiled_call3 htr hd
  cases hcn : c.count with
  | none =>
    simp only [Transpiler.callInsts, hcn, List.mem_singleton, forall_eq] at hz
    obtain ⟨S', z', h1, h2, h3, h4', h5, h6⟩ := callstep1 S z rs hrel hf c hw hcn pc hpc zrom hz hok
    exact ⟨S', z', 1, hcs S' h1, .step h2 (.done _), h3, h4', h5, h6⟩
  | some rc =>
    simp only [Transpiler.callInsts, hcn, List.mem_cons, List.mem_singleton, List.not_mem_nil, or_false,
      forall_eq_or_imp, forall_eq] at hz
    obtain ⟨S', z', h1, h2, h3, h4', h5, h6⟩ := callstep2 S z rs hrel hf c hw rc hcn pc hpc zrom ia hz.1 hz.2 hok
    exact ⟨S', z', 2, hcs S' h1, h2, h3, h4', h5, h6⟩

end Equiv.Machine
