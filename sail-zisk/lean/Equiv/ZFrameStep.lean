/-
  A whole ZisK step depends on its registers and the bytes it reads alone
  (`SFrame`): run from two states with the same registers, the first's
  bytes a part of the second's, a step that succeeds on the first succeeds
  on the second the same way, to states that agree again; one that fails
  on the first reads a byte the first lacks, or fails on the second the
  same way. So ZisK's runs on a memory without some bytes are its runs on
  the whole memory, until a step reads one of them.

  The operations' `Frame` (ZFrame.lean) lifts to `SFrame`: an operation
  writes no register but the fcall ones, which `Frame` follows.
-/
import Equiv.ZFrameAll
import Equiv.Block
import Equiv.BiosEnd

namespace Equiv.Zisk

open _root_.Zisk Sail Sail.ConcurrencyInterfaceV1

/-- σ's bytes a part of τ's, the registers the same. -/
structure FRA (σ τ : ZState) : Prop where
  mem : ∀ k v, σ.mem.get? k = some v → τ.mem.get? k = some v
  regs : σ.regs = τ.regs

theorem FRA.fr {σ τ : ZState} (h : FRA σ τ) : FR σ τ := ⟨h.mem, fun r _ => by rw [h.regs]⟩

/-- Run from σ and from τ, σ's bytes a part of τ's and the registers the
    same: a success on σ is one on τ, the states after agreeing again, the
    bytes as `Kept` says, σ's changed only in the RAM; a failure on σ reads
    a byte σ lacks, or is one on τ. -/
def SFrame {α : Type} (P : SailM α) : Prop :=
  (∀ σ τ : ZState, FRA σ τ → ∀ a σ', P.run σ = .ok a σ' →
    ∃ τ', P.run τ = .ok a τ' ∧ FRA σ' τ' ∧ Kept σ σ' τ τ' ∧ Equiv.RamOnly σ.mem σ'.mem) ∧
  (∀ σ τ : ZState, FRA σ τ → ∀ e σ', P.run σ = .error e σ' →
    (∃ k, e = .OutOfMemoryRange k ∧ σ.mem.get? k = none ∧ τ.mem.get? k ≠ none) ∨ ∃ τ', P.run τ = .error e τ')

/-- An operation's `Frame` is an `SFrame`: it writes no register but the
    fcall ones, on σ as on τ. -/
theorem Frame.sframe {α : Type} {P : SailM α} (h : Frame P) : SFrame P := by
  refine ⟨fun σ τ hs a σ' hr => ?_, fun σ τ hs e σ' hr => ?_⟩
  · obtain ⟨τ', ht, hf, hrt, hk⟩ := h.1 σ τ hs.fr a σ' hr
    obtain ⟨σ'', hs', -, hrs, hks⟩ := h.1 σ σ ⟨fun _ _ x => x, fun _ _ => rfl⟩ a σ' hr
    rw [hr] at hs'; cases hs'
    refine ⟨τ', ht, ⟨hf.mem, ?_⟩, hk, hks.ram⟩
    apply Std.ExtDHashMap.ext_get?; intro r
    cases e : fregs r
    · rw [hrs r e, hrt r e, hs.regs]
    · exact hf.regs r e
  · rcases (h.2 σ τ hs.fr e σ' hr).1 with ⟨k, rfl, hk, hk'⟩ | ⟨τ', ht⟩
    · exact .inl ⟨k, rfl, hk, hk'⟩
    · exact .inr ⟨τ', ht⟩

theorem SFrame.pure {α : Type} (a : α) : SFrame (pure a : SailM α) := (Frame.pure a).sframe

theorem SFrame.bind {α β : Type} {P : SailM α} {f : α → SailM β} (hP : SFrame P) (hf : ∀ a, SFrame (f a)) :
    SFrame (P >>= f) := by
  refine ⟨fun σ τ h b σ' hr => ?_, fun σ τ h e σ' hr => ?_⟩
  · rw [run_bind'] at hr
    cases h1 : P.run σ with
    | error e σ1 => rw [h1] at hr; cases hr
    | ok a σ1 =>
      rw [h1] at hr
      obtain ⟨τ1, ht1, h1', k1, r1⟩ := hP.1 σ τ h a σ1 h1
      obtain ⟨τ', ht', h', k', r'⟩ := (hf a).1 σ1 τ1 h1' b σ' hr
      exact ⟨τ', by rw [run_bind', ht1]; exact ht', h', k1.trans k', r1.trans r'⟩
  · rw [run_bind'] at hr
    cases h1 : P.run σ with
    | ok a σ1 =>
      rw [h1] at hr
      obtain ⟨τ1, ht1, h1', k1, -⟩ := hP.1 σ τ h a σ1 h1
      rcases (hf a).2 σ1 τ1 h1' e σ' hr with ⟨k, rfl, hk, hk'⟩ | ⟨τ', ht'⟩
      · have hs := k1.none hk
        exact .inl ⟨k, rfl, hs, by rw [← k1.same k hs hk]; exact hk'⟩
      · exact .inr ⟨τ', by rw [run_bind', ht1]; exact ht'⟩
    | error e' σ1 =>
      rw [h1] at hr; cases hr
      rcases hP.2 σ τ h _ _ h1 with h2 | ⟨τ', ht'⟩
      · exact .inl h2
      · exact .inr ⟨τ', by rw [run_bind', ht']⟩

theorem SFrame.ite {α : Type} {c : Prop} [Decidable c] {P Q : SailM α} (hP : SFrame P) (hQ : SFrame Q) :
    SFrame (if c then P else Q) := by
  split
  · exact hP
  · exact hQ

theorem SFrame.dite {α : Type} {c : Prop} [Decidable c] {P : c → SailM α} {Q : ¬ c → SailM α}
    (hP : ∀ h, SFrame (P h)) (hQ : ∀ h, SFrame (Q h)) : SFrame (if h : c then P h else Q h) := by
  split
  · exact hP _
  · exact hQ _

theorem SFrame.readReg (r : Register) : SFrame (Zisk.readReg r) := by
  refine ⟨fun σ τ h a σ' hrun => ?_, fun σ τ h e σ' hrun => ?_⟩
  · rw [readReg_run'] at hrun ⊢
    rw [← h.regs]
    split at hrun
    · cases hrun; exact ⟨τ, rfl, h, Kept.refl _ _, Equiv.RamOnly.refl _⟩
    · cases hrun
  · rw [readReg_run'] at hrun ⊢
    rw [← h.regs]
    split at hrun
    · cases hrun
    · cases hrun; exact .inr ⟨τ, rfl⟩

theorem SFrame.writeReg (r : Register) (v : RegisterType r) : SFrame (Zisk.writeReg r v) := by
  refine ⟨fun σ τ h a σ' hrun => ?_, fun σ τ h e σ' hrun => by rw [writeReg_run] at hrun; cases hrun⟩
  rw [writeReg_run] at hrun ⊢
  cases hrun
  exact ⟨_, rfl, ⟨h.mem, by rw [h.regs]⟩, ⟨fun _ v hv => ⟨v, hv⟩, fun _ _ _ => rfl,
    fun r hr => by simp only [Std.ExtDHashMap.get?_insert]; split <;> simp_all, fun _ _ => rfl⟩,
    Equiv.RamOnly.refl _⟩

open Lean Elab Tactic Meta in
/-- The rule for the program's head: bind, pure, if. -/
elab "sframe_head" : tactic => withMainContext do
  let tgt ← instantiateMVars (← getMainTarget)
  let P := tgt.appArg!.headBeta
  match P.getAppFn.constName? with
  | some ``Bind.bind => evalTactic (← `(tactic| refine SFrame.bind ?_ (fun _ => ?_)))
  | some ``Pure.pure => evalTactic (← `(tactic| exact SFrame.pure _))
  | some ``ite => evalTactic (← `(tactic| refine SFrame.ite ?_ ?_))
  | some ``dite => evalTactic (← `(tactic| refine SFrame.dite (fun _ => ?_) (fun _ => ?_)))
  | _ => throwError "sframe_head: no rule for the head"

/-- Push `SFrame` through a program's code, the operations and memory
    accesses by their `Frame`; a goal it cannot take is left. -/
syntax "sframe" : tactic
macro_rules
  | `(tactic| sframe) => `(tactic| first
      | (sframe_head <;> sframe)
      | exact (Frame.exec_mem _ _ _ _).sframe
      | exact (Frame.assert _ _).sframe
      | exact (Frame.read_mem _ _).sframe
      | exact (Frame.write_mem _ _ _).sframe
      | exact (Frame.choose _).sframe
      | (refine (Frame.throw _ ?_).sframe; intro _ h; cases h)
      | exact SFrame.readReg _
      | exact SFrame.writeReg _ _
      | (split <;> sframe)
      | (dsimp only; sframe)
      | (frame_unfold; sframe)
      | skip)

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- A ZisK instruction depends on its registers and the bytes it reads alone. -/
theorem SFrame.execute (inst : zisk_inst) : SFrame (Functions.zisk_execute inst) := by
  unfold Functions.zisk_execute
  sframe

end Equiv.Zisk

namespace Equiv.Sim

open Equiv.Zisk

theorem sframe_zstep (zrom : BitVec 64 → _root_.Zisk.zisk_inst) : SFrame (zstep zrom) := by
  unfold zstep
  exact SFrame.bind (SFrame.readReg _) (fun _ => SFrame.execute _)

/-- σ is τ without the bytes X marks, which are off the RAM: the same
    registers, the same bytes elsewhere. -/
structure FRX (X : Nat → Bool) (σ τ : ZState) : Prop where
  regs : σ.regs = τ.regs
  mem : ∀ k, σ.mem.get? k = if X k then none else τ.mem.get? k

theorem FRX.fra {X : Nat → Bool} {σ τ : ZState} (h : FRX X σ τ) : FRA σ τ :=
  ⟨fun k v hk => by rw [h.mem k] at hk; split at hk; · cases hk
                    · exact hk, h.regs⟩

/-- After a step on each side, σ still τ without X's bytes. -/
theorem FRX.step {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {σ σ' τ τ' : ZState} (h : FRX X σ τ)
    (hf : FRA σ' τ') (hk : Kept σ σ' τ τ') (hr : Equiv.RamOnly σ.mem σ'.mem) : FRX X σ' τ' := by
  refine ⟨hf.regs, fun k => ?_⟩
  by_cases e : X k
  · rw [if_pos e, hr k (hX k e), h.mem k, if_pos e]
  · rw [if_neg e]
    cases h' : σ'.mem.get? k with
    | some v => exact (hf.mem k v h').symm
    | none =>
      have h0 := hk.none h'
      rw [hk.same k h0 h', ← h0, h.mem k, if_neg e]

/-- σ's run reaches a step that reads one of X's bytes. -/
def MissAt (X : Nat → Bool) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (σ : ZState) : Prop :=
  ∃ j σj k σ', ZRun zrom j σ σj ∧ (zstep zrom).run σj = .error (.OutOfMemoryRange k) σ' ∧ X k

/-- A failure on σ that reads a byte σ lacks and τ has: one of X's. -/
theorem FRX.miss {X : Nat → Bool} {σ τ : ZState} (h : FRX X σ τ) {k : Nat} (h1 : σ.mem.get? k = none)
    (h2 : τ.mem.get? k ≠ none) : X k := by
  cases e : X k
  · rw [h.mem k, e] at h1; exact absurd (by simpa using h1) h2
  · rfl

/-- A step on τ that succeeds is one on σ, or σ's reads one of X's bytes. -/
theorem zstep_sub {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst}
    {σ τ τ' : ZState} (h : FRX X σ τ) {b : Bool} (ht : (zstep zrom).run τ = .ok b τ') :
    (∃ σ', (zstep zrom).run σ = .ok b σ' ∧ FRX X σ' τ') ∨
      ∃ k σ', (zstep zrom).run σ = .error (.OutOfMemoryRange k) σ' ∧ X k := by
  cases hs : (zstep zrom).run σ with
  | ok b' σ' =>
    obtain ⟨τ'', ht', hf, hk, hr⟩ := (sframe_zstep zrom).1 σ τ h.fra b' σ' hs
    rw [ht] at ht'; cases ht'
    exact .inl ⟨σ', rfl, h.step hX hf hk hr⟩
  | error e σ' =>
    rcases (sframe_zstep zrom).2 σ τ h.fra e σ' hs with ⟨k, rfl, hk, hk'⟩ | ⟨τ'', ht'⟩
    · exact .inr ⟨k, σ', rfl, h.miss hk hk'⟩
    · rw [ht] at ht'; cases ht'

/-- A step on τ that fails is one on σ. -/
theorem zstep_sub_err {X : Nat → Bool} {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {σ τ τ' : ZState}
    (h : FRX X σ τ) {e : Sail.Error _root_.Zisk.exception} (ht : (zstep zrom).run τ = .error e τ') :
    ∃ e' σ', (zstep zrom).run σ = .error e' σ' := by
  cases hs : (zstep zrom).run σ with
  | ok b' σ' =>
    obtain ⟨τ'', ht', -⟩ := (sframe_zstep zrom).1 σ τ h.fra b' σ' hs
    rw [ht] at ht'; cases ht'
  | error e' σ' => exact ⟨e', σ', rfl⟩

/-- A run on τ is one on σ, σ still τ without X's bytes, or σ's run reads
    one of them. -/
theorem zrun_sub {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} :
    ∀ {n : Nat} {σ τ τ' : ZState}, FRX X σ τ →
    ZRun zrom n τ τ' → (∃ σ', ZRun zrom n σ σ' ∧ FRX X σ' τ') ∨ MissAt X zrom σ
  | _, σ, _, _, h, .done _ => .inl ⟨σ, .done _, h⟩
  | _, σ, _, _, h, .step ht hr => by
    rcases zstep_sub hX h ht with ⟨σ1, hs1, h1⟩ | ⟨k, σ', hs, hk⟩
    · rcases zrun_sub hX h1 hr with ⟨σ', hr', h'⟩ | ⟨j, σj, k, σ', hj, hs', hk⟩
      · exact .inl ⟨σ', .step hs1 hr', h'⟩
      · exact .inr ⟨j + 1, σj, k, σ', .step hs1 hj, hs', hk⟩
    · exact .inr ⟨0, σ, k, σ', .done _, hs, hk⟩

/-- A run on τ that ends is one on σ, σ still τ without X's bytes, or σ's
    run reads one of them. -/
theorem zends_sub {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst}
    {σ τ τ' : ZState} (h : FRX X σ τ) (he : ZEnds zrom τ τ') :
    (∃ σ', ZEnds zrom σ σ' ∧ FRX X σ' τ') ∨ MissAt X zrom σ := by
  obtain ⟨n, τ1, hr, hs⟩ := he
  rcases zrun_sub hX h hr with ⟨σ1, hr', h1⟩ | hm
  · rcases zstep_sub hX h1 hs with ⟨σ', hs', h'⟩ | ⟨k, σ', hs', hk⟩
    · exact .inl ⟨σ', ⟨n, σ1, hr', hs'⟩, h'⟩
    · exact .inr ⟨n, σ1, k, σ', hr', hs', hk⟩
  · exact .inr hm

/-- A run on τ that reaches a failing step is one on σ that does. -/
theorem zfail_sub {X : Nat → Bool} (hX : ∀ k, X k → OffRam k) {zrom : BitVec 64 → _root_.Zisk.zisk_inst}
    {σ τ : ZState} (h : FRX X σ τ) {n : Nat} {τ1 τ' : ZState} {e : Sail.Error _root_.Zisk.exception}
    (hr : ZRun zrom n τ τ1) (hs : (zstep zrom).run τ1 = .error e τ') :
    ∃ n σ1 e' σ', ZRun zrom n σ σ1 ∧ (zstep zrom).run σ1 = .error e' σ' := by
  rcases zrun_sub hX h hr with ⟨σ1, hr', h1⟩ | ⟨j, σj, k, σ', hj, hs', -⟩
  · obtain ⟨e', σ', hs'⟩ := zstep_sub_err h1 hs
    exact ⟨n, σ1, e', σ', hr', hs'⟩
  · exact ⟨j, σj, _, σ', hj, hs'⟩

end Equiv.Sim
