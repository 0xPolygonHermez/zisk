/-
  sail-zisk's operations depend on memory and the fcall registers alone
  (`Frame`): run on two states that agree there — memory off the input's
  first 8 bytes the same; on those bytes, the first state none, or what
  the run wrote; the fcall registers the same — a run that succeeds on
  the first succeeds on the second the same way, to states that agree, the
  second's other registers untouched.

  sail-zisk's Lean is a state monad, which no induction sees into: `Frame`
  is proved of each operation through its code, by lemmas on the
  primitives and the ways to combine them (`frame` tactic). The first
  state is the reference run: on the memory without the input's first 8
  bytes, its success says the operation never reads them (precondition I).
-/
import Equiv.MemNorm
import Equiv.Zisk

namespace Equiv.Zisk

open _root_.Zisk Sail Sail.ConcurrencyInterfaceV1

/-- The fcall registers: the free input, and the current fcall result. -/
def fregs : Register → Bool
  | .FreeInput | .FcallNext | .FcallBase | .FcallSize | .FcallGot => true
  | _ => false

/-- σ and τ agree where an operation may look: each byte σ has, τ has the
    same (σ may lack some: the input's first 8, or bytes nobody wrote); the
    fcall registers. -/
structure FR (σ τ : ZState) : Prop where
  mem : ∀ k v, σ.mem.get? k = some v → τ.mem.get? k = some v
  regs : ∀ r, fregs r = true → σ.regs.get? r = τ.regs.get? r

/-- After runs from σ to σ' and τ to τ': σ's bytes kept, and a byte σ lacks
    before and after left as it was in τ. -/
structure Kept (σ σ' τ τ' : ZState) : Prop where
  grows : ∀ k v, σ.mem.get? k = some v → ∃ v', σ'.mem.get? k = some v'
  same : ∀ k, σ.mem.get? k = none → σ'.mem.get? k = none → τ'.mem.get? k = τ.mem.get? k

/-- A program that depends on memory and the fcall registers alone, and
    writes no other register: a run that succeeds on σ succeeds on τ the
    same way; one that fails on σ reads a byte σ lacks and τ has, or fails
    on τ the same way. -/
def Frame {α : Type} (P : SailM α) : Prop :=
  (∀ σ τ : ZState, FR σ τ → ∀ a σ', P.run σ = .ok a σ' →
    ∃ τ', P.run τ = .ok a τ' ∧ FR σ' τ' ∧ (∀ r, fregs r = false → τ'.regs.get? r = τ.regs.get? r) ∧
      Kept σ σ' τ τ') ∧
  (∀ σ τ : ZState, FR σ τ → ∀ e σ', P.run σ = .error e σ' →
    (∃ k, e = .OutOfMemoryRange k ∧ σ.mem.get? k = none ∧ τ.mem.get? k ≠ none) ∨
    ∃ τ', P.run τ = .error e τ')

theorem Kept.refl (σ τ : ZState) : Kept σ σ τ τ := ⟨fun _ v h => ⟨v, h⟩, fun _ _ _ => rfl⟩

theorem Kept.none {σ σ' τ τ' : ZState} (h : Kept σ σ' τ τ') {k : Nat} (hk : σ'.mem.get? k = none) :
    σ.mem.get? k = none := by
  cases h' : σ.mem.get? k with
  | none => rfl
  | some v => obtain ⟨_, h''⟩ := h.grows k v h'; rw [hk] at h''; cases h''

theorem Kept.trans {σ σ1 σ2 τ τ1 τ2 : ZState} (h1 : Kept σ σ1 τ τ1) (h2 : Kept σ1 σ2 τ1 τ2) : Kept σ σ2 τ τ2 := by
  refine ⟨fun k v h => ?_, fun k hk hk2 => ?_⟩
  · obtain ⟨v1, h1'⟩ := h1.grows k v h; exact h2.grows k v1 h1'
  · have e1 := h2.none hk2
    rw [h2.same k e1 hk2, h1.same k hk e1]

theorem Frame.pure {α : Type} (a : α) : Frame (pure a : SailM α) := by
  refine ⟨fun σ τ h b σ' hr => ?_, fun σ τ h e σ' hr => ?_⟩
  · simp only [EStateM.run, pure, EStateM.pure, EStateM.Result.ok.injEq] at hr
    obtain ⟨rfl, rfl⟩ := hr
    exact ⟨τ, rfl, h, fun _ _ => rfl, Kept.refl _ _⟩
  · cases hr

theorem Frame.bind {α β : Type} {P : SailM α} {f : α → SailM β} (hP : Frame P) (hf : ∀ a, Frame (f a)) :
    Frame (P >>= f) := by
  refine ⟨fun σ τ h b σ' hr => ?_, fun σ τ h e σ' hr => ?_⟩
  · rw [run_bind'] at hr
    cases h1 : P.run σ with
    | error e σ1 => rw [h1] at hr; cases hr
    | ok a σ1 =>
      rw [h1] at hr
      obtain ⟨τ1, ht1, h1', r1, k1⟩ := hP.1 σ τ h a σ1 h1
      obtain ⟨τ', ht', h', r', k'⟩ := (hf a).1 σ1 τ1 h1' b σ' hr
      exact ⟨τ', by rw [run_bind', ht1]; exact ht', h', fun r hr => (r' r hr).trans (r1 r hr), k1.trans k'⟩
  · rw [run_bind'] at hr
    cases h1 : P.run σ with
    | ok a σ1 =>
      rw [h1] at hr
      obtain ⟨τ1, ht1, h1', -, k1⟩ := hP.1 σ τ h a σ1 h1
      rcases (hf a).2 σ1 τ1 h1' e σ' hr with ⟨k, rfl, hk1, hk2⟩ | ⟨τ', ht'⟩
      · have hs := k1.none hk1
        exact .inl ⟨k, rfl, hs, by rw [← k1.same k hs hk1]; exact hk2⟩
      · exact .inr ⟨τ', by rw [run_bind', ht1]; exact ht'⟩
    | error e' σ1 =>
      rw [h1] at hr; cases hr
      rcases hP.2 σ τ h _ _ h1 with h2 | ⟨τ', ht'⟩
      · exact .inl h2
      · exact .inr ⟨τ', by rw [run_bind', ht']⟩

theorem Frame.throw {α : Type} (e : Sail.Error exception) : Frame (throw e : SailM α) := by
  refine ⟨fun σ τ h b σ' hr => (by cases hr), fun σ τ h e' σ' hr => ?_⟩
  cases hr
  exact .inr ⟨τ, rfl⟩

theorem Frame.assert (p : Bool) (msg : String) : Frame (Sail.ConcurrencyInterfaceV1.PreSail.assert p msg :
    SailM Unit) := by
  unfold Sail.ConcurrencyInterfaceV1.PreSail.assert
  split
  · exact Frame.pure _
  · exact Frame.throw _

theorem Frame.ite {α : Type} {c : Prop} [Decidable c] {P Q : SailM α} (hP : Frame P) (hQ : Frame Q) :
    Frame (if c then P else Q) := by
  split
  · exact hP
  · exact hQ

theorem Frame.readReg (r : Register) (hr : fregs r = true) : Frame (Zisk.readReg r) := by
  refine ⟨fun σ τ h a σ' hrun => ?_, fun σ τ h e σ' hrun => ?_⟩
  · rw [readReg_run'] at hrun ⊢
    rw [← h.regs r hr]
    split at hrun
    · cases hrun; exact ⟨τ, rfl, h, fun _ _ => rfl, Kept.refl _ _⟩
    · cases hrun
  · rw [readReg_run'] at hrun ⊢
    rw [← h.regs r hr]
    split at hrun
    · cases hrun
    · cases hrun; exact .inr ⟨τ, rfl⟩

theorem Frame.writeReg (r : Register) (v : RegisterType r) (hr : fregs r = true) : Frame (Zisk.writeReg r v) := by
  refine ⟨fun σ τ h a σ' hrun => ?_, fun σ τ h e σ' hrun => by rw [writeReg_run] at hrun; cases hrun⟩
  rw [writeReg_run] at hrun ⊢
  cases hrun
  refine ⟨_, rfl, ⟨h.mem, fun r' hr' => ?_⟩, fun r' hr' => ?_, ⟨fun _ v h => ⟨v, h⟩, fun _ _ _ => rfl⟩⟩
  · simp only [Std.ExtDHashMap.get?_insert]
    split
    · rfl
    · exact h.regs r' hr'
  · simp only [Std.ExtDHashMap.get?_insert]
    split
    · next e => simp only [beq_iff_eq] at e; subst e; rw [hr] at hr'; cases hr'
    · rfl

theorem readByte_run' (s : ZState) (k : Nat) :
    (Sail.ConcurrencyInterfaceV1.PreSail.readByte k : SailM (BitVec 8)).run s = match s.mem.get? k with
      | some v => .ok v s
      | none => .error (.OutOfMemoryRange k) s := by
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.readByte]
  split <;> simp_all <;> rfl

theorem Frame.readByte (k : Nat) : Frame (Sail.ConcurrencyInterfaceV1.PreSail.readByte k : SailM (BitVec 8)) := by
  refine ⟨fun σ τ h a σ' hrun => ?_, fun σ τ h e σ' hrun => ?_⟩
  · rw [readByte_run'] at hrun ⊢
    cases hv : σ.mem.get? k with
    | none => rw [hv] at hrun; cases hrun
    | some v =>
      rw [hv] at hrun; cases hrun
      rw [h.mem k a hv]; exact ⟨τ, rfl, h, fun _ _ => rfl, Kept.refl _ _⟩
  · rw [readByte_run'] at hrun ⊢
    cases hv : σ.mem.get? k with
    | some v => rw [hv] at hrun; cases hrun
    | none =>
      rw [hv] at hrun; cases hrun
      cases ht : τ.mem.get? k with
      | none => exact .inr ⟨τ, rfl⟩
      | some v => exact .inl ⟨k, rfl, hv, by rw [ht]; simp⟩

theorem writeByte_run (s : ZState) (k : Nat) (v : BitVec 8) :
    (Sail.ConcurrencyInterfaceV1.PreSail.writeByte k v : SailM PUnit).run s = .ok () { s with mem := s.mem.insert k v } :=
  rfl

theorem get?_insert' (m : Mem) (k k' : Nat) (v : BitVec 8) :
    (m.insert k v).get? k' = if k = k' then some v else m.get? k' := by
  simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert, beq_iff_eq]

theorem Frame.writeByte (k : Nat) (v : BitVec 8) :
    Frame (Sail.ConcurrencyInterfaceV1.PreSail.writeByte k v : SailM PUnit) := by
  refine ⟨fun σ τ h a σ' hrun => ?_, fun σ τ h e σ' hrun => by rw [writeByte_run] at hrun; cases hrun⟩
  rw [writeByte_run] at hrun ⊢
  cases hrun
  refine ⟨_, rfl, ⟨fun k' v' hk => ?_, h.regs⟩, fun _ _ => rfl, ⟨fun k' v' hk => ?_, fun k' _ hk => ?_⟩⟩
  · simp only [get?_insert'] at hk ⊢
    split
    · next e => rw [if_pos e] at hk; exact hk
    · next e => rw [if_neg e] at hk; exact h.mem k' v' hk
  · simp only [get?_insert']
    split
    · exact ⟨v, rfl⟩
    · exact ⟨v', hk⟩
  · simp only [get?_insert'] at hk ⊢
    split
    · next e => rw [if_pos e] at hk; cases hk
    · rfl

theorem Frame.readBytes : ∀ (size addr : Nat),
    Frame (Sail.ConcurrencyInterfaceV1.PreSail.readBytes size addr : SailM ((BitVec (8 * size)) × Option Bool))
  | 0, _ => by simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]; exact Frame.pure _
  | 1, a => by
    simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]
    exact Frame.bind (Frame.readByte a) (fun _ => Frame.pure _)
  | n + 2, a => by
    simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]
    exact Frame.bind (Frame.readByte a) (fun _ => Frame.bind (Frame.readBytes (n + 1) (a + 1)) (fun _ => Frame.pure _))

theorem Frame.forM_list {α : Type} (l : List α) (f : α → SailM PUnit) (hf : ∀ a, Frame (f a)) :
    Frame (List.forM l f) := by
  induction l with
  | nil => exact Frame.pure _
  | cons a l ih => exact Frame.bind (hf a) (fun _ => ih)

theorem Frame.writeBytes {n : Nat} (addr : Nat) (v : BitVec (8 * n)) :
    Frame (Sail.ConcurrencyInterfaceV1.PreSail.writeBytes addr v : SailM Bool) := by
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.writeBytes]
  exact Frame.bind (Frame.forM_list _ _ (fun _ => Frame.writeByte _ _)) (fun _ => Frame.pure _)

theorem Frame.loop {β : Type} (range : IntRange) (f : (i : Int) → i ∈ range → β → SailM (ForInStep β))
    (hf : ∀ i h b, Frame (f i h b)) : ∀ b i hs, Frame (IntRange.forIn'.loop range f b i hs) := by
  intro b i hs
  induction b, i, hs using IntRange.forIn'.loop.induct range with
  | case1 b i hs hin ih =>
    rw [IntRange.forIn'.loop.eq_1, dif_pos hin]
    refine Frame.bind (hf i hin b) (fun st => ?_)
    cases st with
    | done b => exact Frame.pure _
    | yield b => exact ih b
  | case2 b i hs hn => rw [IntRange.forIn'.loop.eq_1, dif_neg hn]; exact Frame.pure _

theorem Frame.forIn {β : Type} (range : IntRange) (init : β) (f : Int → β → SailM (ForInStep β))
    (hf : ∀ i b, Frame (f i b)) : Frame (forIn range init f) :=
  Frame.loop range (fun i _ b => f i b) (fun i _ b => hf i b) init range.start _

theorem Frame.read_mem (addr : BitVec 64) (n : Nat) : Frame (Functions.read_mem addr n) := by
  unfold Functions.read_mem
  refine Frame.bind (Frame.assert _ _) (fun _ => Frame.bind (Frame.assert _ _) (fun _ => Frame.ite ?_ ?_))
  · exact Frame.bind (Frame.readReg .FreeInput rfl) (fun _ => Frame.pure _)
  · simp only [Sail.ConcurrencyInterfaceV1.PreSail.read_ram]
    exact Frame.bind (Frame.readBytes _ _) (fun _ => Frame.pure _)

theorem Frame.write_mem (addr : BitVec 64) (n : Nat) (v : BitVec (8 * n)) : Frame (Functions.write_mem addr n v) := by
  unfold Functions.write_mem
  refine Frame.bind (Frame.assert _ _) (fun _ => Frame.bind (Frame.assert _ _) (fun _ => ?_))
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.write_ram]
  exact Frame.bind (Frame.writeBytes _ _) (fun _ => Frame.pure _)

theorem Frame.read_ram (as ds : Nat) (h addr : BitVec as) :
    Frame (Sail.ConcurrencyInterfaceV1.PreSail.read_ram as ds h addr : SailM (BitVec (8 * ds))) := by
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.read_ram]
  exact Frame.bind (Frame.readBytes _ _) (fun _ => Frame.pure _)

theorem Frame.write_ram (as ds : Nat) (h addr : BitVec as) (v : BitVec (8 * ds)) :
    Frame (Sail.ConcurrencyInterfaceV1.PreSail.write_ram as ds h addr v : SailM Unit) := by
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.write_ram]
  exact Frame.bind (Frame.writeBytes _ _) (fun _ => Frame.pure _)

/-- An undefined value: the choice source is trivial, so both choose the same. -/
theorem Frame.choose (p : Sail.Primitive) :
    Frame (Sail.ConcurrencyInterfaceV1.PreSail.choose p : SailM p.reflect) := by
  refine ⟨fun σ τ h a σ' hrun => ?_, fun σ τ h e σ' hrun => ?_⟩
  · simp only [Sail.ConcurrencyInterfaceV1.PreSail.choose, EStateM.run, modifyGet, MonadStateOf.modifyGet,
      EStateM.modifyGet] at hrun ⊢
    cases hrun
    exact ⟨_, rfl, ⟨h.mem, h.regs⟩, fun _ _ => rfl, ⟨fun _ v h => ⟨v, h⟩, fun _ _ _ => rfl⟩⟩
  · simp only [Sail.ConcurrencyInterfaceV1.PreSail.choose, EStateM.run, modifyGet, MonadStateOf.modifyGet,
      EStateM.modifyGet] at hrun
    cases hrun

theorem Frame.dite {α : Type} {c : Prop} [Decidable c] {P : c → SailM α} {Q : ¬ c → SailM α}
    (hP : ∀ h, Frame (P h)) (hQ : ∀ h, Frame (Q h)) : Frame (if h : c then P h else Q h) := by
  split
  · exact hP _
  · exact hQ _

open Lean Elab Tactic Meta in
/-- The rule for the program's head: bind, pure, if, a loop. -/
elab "frame_head" : tactic => withMainContext do
  let tgt ← instantiateMVars (← getMainTarget)
  let P := tgt.appArg!.headBeta
  match P.getAppFn.constName? with
  | some ``Bind.bind => evalTactic (← `(tactic| refine Frame.bind ?_ (fun _ => ?_)))
  | some ``Pure.pure => evalTactic (← `(tactic| exact Frame.pure _))
  | some ``ite => evalTactic (← `(tactic| refine Frame.ite ?_ ?_))
  | some ``dite => evalTactic (← `(tactic| refine Frame.dite (fun _ => ?_) (fun _ => ?_)))
  | some ``ForIn.forIn => evalTactic (← `(tactic| refine Frame.forIn _ _ _ (fun _ _ => ?_)))
  | _ => throwError "frame_head: no rule for the head"

open Lean Elab Tactic Meta in
/-- Unfold the program's head constant once. -/
elab "frame_unfold" : tactic => withMainContext do
  let tgt ← instantiateMVars (← getMainTarget)
  let P := tgt.appArg!.headBeta
  let some n := P.getAppFn.constName? | throwError "frame_unfold: no constant"
  evalTactic (← `(tactic| unfold $(mkIdent n):ident))

/-- Push `Frame` through a program's code: the head's rule (bind, pure, if,
    a loop), else a primitive's, else a match split, a `let` reduced, or the
    head unfolded; a goal it cannot take is left. -/
syntax "frame" : tactic
macro_rules
  | `(tactic| frame) => `(tactic| first
      | (frame_head <;> frame)
      | exact Frame.assert _ _
      | exact Frame.throw _
      | exact Frame.read_mem _ _
      | exact Frame.write_mem _ _ _
      | exact Frame.read_ram _ _ _ _
      | exact Frame.write_ram _ _ _ _ _
      | exact Frame.readBytes _ _
      | exact Frame.writeBytes _ _
      | exact Frame.readByte _
      | exact Frame.writeByte _ _
      | exact Frame.choose _
      | (refine Frame.readReg _ ?_; rfl)
      | (refine Frame.writeReg _ _ ?_; rfl)
      | (split <;> frame)
      | (dsimp only; frame)
      | (frame_unfold; frame)
      | skip)

set_option maxRecDepth 1000000 in
theorem Frame.dma_copy (dst src : BitVec 64) (count : Int) : Frame (Functions.dma_copy dst src count) := by
  unfold Functions.dma_copy
  frame

end Equiv.Zisk
