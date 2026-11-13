/-
  sail-zisk's programs that leave the fcall registers as they are (`Keep`):
  every operation but the three that write them (fcall, fcall_get,
  dma_inputcpy), and so every ZisK step of another instruction. What keeps
  ZisK's fcall registers the machine's fcall state across the steps that
  are not ZisK calls.
-/
import Equiv.ZFrameAll

namespace Equiv.Zisk

open _root_.Zisk Sail Sail.ConcurrencyInterfaceV1

/-- A run of P leaves the fcall registers as they were. -/
def Keep {α : Type} (P : SailM α) : Prop :=
  ∀ σ a σ', P.run σ = .ok a σ' → ∀ r, fregs r = true → σ'.regs.get? r = σ.regs.get? r

theorem Keep.pure {α : Type} (a : α) : Keep (pure a : SailM α) := by
  intro σ b σ' hr; simp only [EStateM.run, Pure.pure, EStateM.pure, EStateM.Result.ok.injEq] at hr
  obtain ⟨-, rfl⟩ := hr; intro _ _; rfl

theorem Keep.bind {α β : Type} {P : SailM α} {f : α → SailM β} (hP : Keep P) (hf : ∀ a, Keep (f a)) :
    Keep (P >>= f) := by
  intro σ b σ' hr r h
  rw [run_bind'] at hr
  cases h1 : P.run σ with
  | error e σ1 => rw [h1] at hr; cases hr
  | ok a σ1 => rw [h1] at hr; rw [hf a σ1 b σ' hr r h, hP σ a σ1 h1 r h]

theorem Keep.throw {α : Type} (e : Sail.Error exception) : Keep (MonadExcept.throw e : SailM α) := by
  intro σ b σ' hr; cases hr

theorem Keep.ite {α : Type} {c : Prop} [Decidable c] {P Q : SailM α} (hP : Keep P) (hQ : Keep Q) :
    Keep (if c then P else Q) := by
  split
  · exact hP
  · exact hQ

theorem Keep.dite {α : Type} {c : Prop} [Decidable c] {P : c → SailM α} {Q : ¬ c → SailM α}
    (hP : ∀ h, Keep (P h)) (hQ : ∀ h, Keep (Q h)) : Keep (if h : c then P h else Q h) := by
  split
  · exact hP _
  · exact hQ _

theorem Keep.assert (p : Bool) (msg : String) :
    Keep (Sail.ConcurrencyInterfaceV1.PreSail.assert p msg : SailM Unit) := by
  unfold Sail.ConcurrencyInterfaceV1.PreSail.assert
  split
  · exact Keep.pure _
  · exact Keep.throw _

theorem Keep.readReg (r : Register) : Keep (Zisk.readReg r) := by
  intro σ a σ' hr q _
  rw [readReg_run'] at hr
  split at hr
  · cases hr; rfl
  · cases hr

theorem Keep.writeReg (r : Register) (v : RegisterType r) (hr : fregs r = false) : Keep (Zisk.writeReg r v) := by
  intro σ a σ' hrun q hq
  rw [writeReg_run] at hrun; cases hrun
  simp only [Std.ExtDHashMap.get?_insert]
  split
  · next e => simp only [beq_iff_eq] at e; subst e; rw [hr] at hq; cases hq
  · rfl

theorem Keep.readByte (k : Nat) : Keep (Sail.ConcurrencyInterfaceV1.PreSail.readByte k : SailM (BitVec 8)) := by
  intro σ a σ' hrun q _
  rw [readByte_run'] at hrun
  split at hrun
  · cases hrun; rfl
  · cases hrun

theorem Keep.writeByte (k : Nat) (v : BitVec 8) :
    Keep (Sail.ConcurrencyInterfaceV1.PreSail.writeByte k v : SailM PUnit) := by
  intro σ a σ' hrun q _
  cases hrun; rfl

theorem Keep.choose (p : Sail.Primitive) : Keep (Sail.ConcurrencyInterfaceV1.PreSail.choose p : SailM p.reflect) := by
  intro σ a σ' hrun q _
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.choose, EStateM.run, modifyGet, MonadStateOf.modifyGet,
    EStateM.modifyGet] at hrun
  cases hrun; rfl

theorem Keep.loop {β : Type} (range : IntRange) (f : (i : Int) → i ∈ range → β → SailM (ForInStep β))
    (hf : ∀ i h b, Keep (f i h b)) : ∀ b i hs, Keep (IntRange.forIn'.loop range f b i hs) := by
  intro b i hs
  induction b, i, hs using IntRange.forIn'.loop.induct range with
  | case1 b i hs hin ih =>
    rw [IntRange.forIn'.loop.eq_1, dif_pos hin]
    refine Keep.bind (hf i hin b) (fun st => ?_)
    cases st with
    | done b => exact Keep.pure _
    | yield b => exact ih b
  | case2 b i hs hn => rw [IntRange.forIn'.loop.eq_1, dif_neg hn]; exact Keep.pure _

theorem Keep.forIn {β : Type} (range : IntRange) (init : β) (f : Int → β → SailM (ForInStep β))
    (hf : ∀ i b, Keep (f i b)) : Keep (forIn range init f) :=
  Keep.loop range (fun i _ b => f i b) (fun i _ b => hf i b) init range.start _

theorem Keep.forM_list {α : Type} (l : List α) (f : α → SailM PUnit) (hf : ∀ a, Keep (f a)) :
    Keep (List.forM l f) := by
  induction l with
  | nil => exact Keep.pure _
  | cons a l ih => exact Keep.bind (hf a) (fun _ => ih)

theorem Keep.readBytes : ∀ (size addr : Nat),
    Keep (Sail.ConcurrencyInterfaceV1.PreSail.readBytes size addr : SailM ((BitVec (8 * size)) × Option Bool))
  | 0, _ => by simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]; exact Keep.pure _
  | 1, a => by
    simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]
    exact Keep.bind (Keep.readByte a) (fun _ => Keep.pure _)
  | n + 2, a => by
    simp only [Sail.ConcurrencyInterfaceV1.PreSail.readBytes]
    exact Keep.bind (Keep.readByte a) (fun _ => Keep.bind (Keep.readBytes (n + 1) (a + 1)) (fun _ => Keep.pure _))

theorem Keep.writeBytes {n : Nat} (addr : Nat) (v : BitVec (8 * n)) :
    Keep (Sail.ConcurrencyInterfaceV1.PreSail.writeBytes addr v : SailM Bool) := by
  simp only [Sail.ConcurrencyInterfaceV1.PreSail.writeBytes]
  exact Keep.bind (Keep.forM_list _ _ (fun _ => Keep.writeByte _ _)) (fun _ => Keep.pure _)

open Lean Elab Tactic Meta in
/-- The rule for the program's head, for `Keep`. -/
elab "keep_head" : tactic => withMainContext do
  let tgt ← instantiateMVars (← getMainTarget)
  let P := tgt.appArg!.headBeta
  match P.getAppFn.constName? with
  | some ``Bind.bind => evalTactic (← `(tactic| refine Keep.bind ?_ (fun _ => ?_)))
  | some ``Pure.pure => evalTactic (← `(tactic| exact Keep.pure _))
  | some ``ite => evalTactic (← `(tactic| refine Keep.ite ?_ ?_))
  | some ``dite => evalTactic (← `(tactic| refine Keep.dite (fun _ => ?_) (fun _ => ?_)))
  | some ``ForIn.forIn => evalTactic (← `(tactic| refine Keep.forIn _ _ _ (fun _ _ => ?_)))
  | some ``List.forM => evalTactic (← `(tactic| refine Keep.forM_list _ _ (fun _ => ?_)))
  | _ => throwError "keep_head: no rule for the head"

/-- Push `Keep` through a program's code; a goal it cannot take is left. -/
syntax "keep" : tactic
macro_rules
  | `(tactic| keep) => `(tactic| first
      | (keep_head <;> keep)
      | exact Keep.assert _ _
      | exact Keep.throw _
      | exact Keep.readBytes _ _
      | exact Keep.writeBytes _ _
      | exact Keep.readByte _
      | exact Keep.writeByte _ _
      | exact Keep.choose _
      | exact Keep.readReg _
      | (refine Keep.writeReg _ _ ?_; rfl)
      | (split <;> keep)
      | (dsimp only; keep)
      | (frame_unfold; keep)
      | skip)

/-- The operations that write the fcall registers. -/
def writesF : zisk_op → Bool
  | .ZOP_FCALL () | .ZOP_FCALL_GET () | .ZOP_DMA_INPUTCPY () => true
  | _ => false


end Equiv.Zisk
