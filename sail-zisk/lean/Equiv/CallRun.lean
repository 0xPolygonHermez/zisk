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

set_option maxHeartbeats 4000000 in
/-- A ZisK call at pc: the machine's step on it, at a code address (and,
    for a call of more than one word, pc + 4 one too), and ZisK's steps on
    its translation, from related states to related states. -/
theorem callstep_m (C : BitVec 64 → Prop) (S : ZiskRiscv.State model) (z : Equiv.Zisk.ZState)
    (rs : List PMA_Region) (hrel : Rel S.rv z rs) (hf : Equiv.Zisk.FcallRel S.fcall z)
    (hx : ¬ ExecAt rs EXIT_ADDR) (pc : BitVec 64) (hpc : S.rv.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (w : BitVec 32) (hi : WordsAt S.rv rs pc zrom w (nextWord S.rv pc))
    (h4 : (pc + 4).toNat = pc.toNat + 4) (c : ZiskRiscv.Call)
    (hd : ZiskRiscv.decode (Transpiler.fetch2 pc w (nextWord S.rv pc)) pc.toNat = .call c)
    (hC : C pc) (hC4 : c.len ≠ 1 → C (pc + 4)) (hok : CallOk S c) :
    ∃ S' z' k', CStep C S S' ∧ ZRun zrom k' z z' ∧ Rel S'.rv z' rs ∧ Equiv.Zisk.FcallRel S'.fcall z' ∧
      (∀ a, Equiv.OffRam a → ¬ (Equiv.IN_LO ≤ a ∧ a < Equiv.IN_LO + 8) → S'.rv.mem.get? a = S.rv.mem.get? a) ∧
      Grows S.rv.mem S'.rv.mem := by
  have hw := decode_call_wf hd
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  have hrw : ZiskRiscv.readWord S.rv.mem pc.toNat = some w := Equiv.Machine.readWord_of hi.h0 hi.h1 hi.h2 hi.h3
  have hcs : ∀ S', ZiskRiscv.call S c = .ok S' → CStep C S S' := by
    intro S' hc
    refine ⟨?_, by rw [hg]; exact hC, c, hc, fun hl => by rw [hg]; exact ⟨hC4 hl, h4⟩, fun fetch f0 f4 => ?_⟩
    · rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hi.x1)
    · rw [hg] at f0 f4 ⊢
      refine decode_any hd fetch (by rw [f0]; exact hrw) fun hl => ?_
      rw [f4 hl, nextWord, h4]; rfl
  obtain ⟨ia, hz⟩ := transpiled_call hi.tr hd
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
