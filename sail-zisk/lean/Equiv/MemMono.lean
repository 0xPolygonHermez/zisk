/-
  Sail's sequential interpreter is monotone in memory: a program that runs
  on a state runs the same way — the same result, the same writes — on one
  with more bytes defined. The state t extends s by D (`Ext D s t`): the
  same registers, and t's memory s's, with D's bytes where s has none. For
  any program of any architecture (Sail/ArchSemSequential.lean), as
  Equiv/MemGrow.lean.

  It is what relates a run on memory as the machine is given it — sparse,
  an unwritten byte missing — to one on memory as ZisK starts it, every byte
  of its regions defined.
-/
import Equiv.MemGrow

namespace Equiv

open Sail Sail.ArchSem

variable [Arch]

/-- t is s with D's bytes where s has none. -/
structure Ext (D : Nat → Option (BitVec 8)) (s t : SequentialState Arch.register_type) : Prop where
  regs : t.regs = s.regs
  tags : t.tags = s.tags
  cycles : t.cycleCount = s.cycleCount
  out : t.sailOutput = s.sailOutput
  mem : ∀ a, t.mem.get? a = (s.mem.get? a).or (D a)

theorem Ext.insert {D : Nat → Option (BitVec 8)} {s t : SequentialState Arch.register_type} (h : Ext D s t)
    (a : Nat) (v : BitVec 8) :
    Ext D { s with mem := s.mem.insert a v } { t with mem := t.mem.insert a v } := by
  refine ⟨h.regs, h.tags, h.cycles, h.out, fun b => ?_⟩
  have := h.mem b
  simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert] at this ⊢
  split <;> simp_all

theorem readByte_mono {D : Nat → Option (BitVec 8)} {s t s' : SequentialState Arch.register_type} (h : Ext D s t)
    (a : Nat) (v : BitVec 8) (hr : (readByte (ue := ue) a).run s = .ok v s') :
    s' = s ∧ (readByte (ue := ue) a).run t = .ok v t := by
  unfold readByte at hr ⊢
  have hm := h.mem a
  cases hs : s.mem[a]? <;>
    simp [hs, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get, pure,
      EStateM.pure, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr
  obtain ⟨rfl, rfl⟩ := hr
  simp only [Std.ExtHashMap.get?_eq_getElem?, hs, Option.some_or] at hm
  refine ⟨rfl, ?_⟩
  simp [hm, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get, pure, EStateM.pure]

theorem readBytes_mono {D : Nat → Option (BitVec 8)} (size : Nat) : ∀ (addr : Nat)
    {s t s' : SequentialState Arch.register_type} (_ : Ext D s t) (r : BitVec (8 * size)),
    (readBytes (ue := ue) size addr).run s = .ok r s' → s' = s ∧ (readBytes (ue := ue) size addr).run t = .ok r t := by
  induction size with
  | zero =>
    intro addr s t s' h r hr
    simp only [readBytes, EStateM.run, pure, EStateM.pure, EStateM.Result.ok.injEq] at hr ⊢
    refine ⟨hr.2.symm, ?_⟩; simp_all
  | succ n ih =>
    intro addr s t s' h r hr
    cases n with
    | zero =>
      simp only [readBytes, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        obtain ⟨e1, h1'⟩ := readByte_mono h addr r1 h1
        simp only [EStateM.Result.ok.injEq] at hr
        subst e1
        refine ⟨hr.2.symm, ?_⟩
        simp only [EStateM.run] at h1'
        rw [h1']; simp only [EStateM.Result.ok.injEq, and_true]; exact hr.1
      · cases hr
    | succ m =>
      simp only [readBytes, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        obtain ⟨e1, h1'⟩ := readByte_mono h addr r1 h1
        subst e1
        split at hr
        · next r2 s2 h2 =>
          obtain ⟨e2, h2'⟩ := ih (addr + 1) h r2 h2
          subst e2
          simp only [EStateM.Result.ok.injEq] at hr
          refine ⟨hr.2.symm, ?_⟩
          simp only [EStateM.run] at h1' h2'
          rw [h1']; dsimp only; rw [h2']; simp only [EStateM.Result.ok.injEq, and_true]; exact hr.1
        · cases hr
      · cases hr

theorem forM_mono {D : Nat → Option (BitVec 8)} (f : Nat × BitVec 8 → ArchSemSeqM ue PUnit)
    (hf : ∀ p {s t s' : SequentialState Arch.register_type} (r : PUnit), Ext D s t → (f p).run s = .ok r s' →
      ∃ t', (f p).run t = .ok r t' ∧ Ext D s' t') (l : List (Nat × BitVec 8)) :
    ∀ {s t s' : SequentialState Arch.register_type} (r : PUnit), Ext D s t → (List.forM l f).run s = .ok r s' →
      ∃ t', (List.forM l f).run t = .ok r t' ∧ Ext D s' t' := by
  induction l with
  | nil =>
    intro s t s' r h hr
    simp only [List.forM, EStateM.run, pure, EStateM.pure, EStateM.Result.ok.injEq] at hr ⊢
    obtain ⟨-, rfl⟩ := hr
    exact ⟨t, by simp, h⟩
  | cons p l ih =>
    intro s t s' r h hr
    simp only [List.forM, EStateM.run, bind, EStateM.bind] at hr ⊢
    split at hr
    · next r1 s1 h1 =>
      obtain ⟨t1, ht1, h1'⟩ := hf p r1 h h1
      obtain ⟨t', ht', h'⟩ := ih r h1' hr
      simp only [EStateM.run] at ht1 ht'
      exact ⟨t', by rw [ht1]; exact ht', h'⟩
    · cases hr

theorem writeBytes_mono {D : Nat → Option (BitVec 8)} {n : Nat} (addr : Nat) (value : BitVec (8 * n))
    {s t s' : SequentialState Arch.register_type} (r : PUnit) (h : Ext D s t)
    (hr : (writeBytes (ue := ue) addr value).run s = .ok r s') :
    ∃ t', (writeBytes (ue := ue) addr value).run t = .ok r t' ∧ Ext D s' t' := by
  unfold writeBytes at hr ⊢
  refine forM_mono _ ?_ _ r h hr
  rintro ⟨a, v⟩ s t s' r h hr
  simp only [writeByte, modify, modifyGet, MonadStateOf.modifyGet, EStateM.modifyGet, EStateM.run] at hr ⊢
  cases hr
  exact ⟨_, rfl, h.insert a v⟩

theorem effect_mono {D : Nat → Option (BitVec 8)} (eff : InstructionEffect)
    {s t s' : SequentialState Arch.register_type} (r : eff.ret) (h : Ext D s t)
    (hr : (interpretInstructionEffect (ue := ue) eff).run s = .ok r s') :
    ∃ t', (interpretInstructionEffect (ue := ue) eff).run t = .ok r t' ∧ Ext D s' t' := by
  cases eff with
  | memRead req =>
    simp only [interpretInstructionEffect] at hr ⊢
    by_cases ht : (req.numTag != 0) = true
    · simp [ht, EStateM.run, bind, EStateM.bind, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr
    · simp only [ht, Bool.false_eq_true, if_false, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        obtain ⟨e1, h1'⟩ := readBytes_mono _ _ h r1 h1
        subst e1
        simp only [EStateM.Result.ok.injEq] at hr
        obtain ⟨rfl, rfl⟩ := hr
        simp only [EStateM.run] at h1'
        exact ⟨t, by rw [h1'], h⟩
      · cases hr
  | memWrite req value tags =>
    simp only [interpretInstructionEffect] at hr ⊢
    by_cases ht : (req.numTag != 0) = true
    · simp [ht, EStateM.run, bind, EStateM.bind, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr
    · simp only [ht, Bool.false_eq_true, if_false, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        obtain ⟨t1, ht1, h1'⟩ := writeBytes_mono _ _ r1 h h1
        simp only [EStateM.Result.ok.injEq] at hr
        obtain ⟨rfl, rfl⟩ := hr
        simp only [EStateM.run] at ht1
        exact ⟨t1, by rw [ht1], h1'⟩
      · cases hr
  | regRead reg acc =>
    simp only [interpretInstructionEffect] at hr ⊢
    have e := h.regs
    cases hm : s.regs.get? reg <;>
      simp [hm, e, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get, pure,
        EStateM.pure, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr ⊢
    first | (obtain ⟨rfl, rfl⟩ := hr) | (obtain ⟨-, rfl⟩ := hr)
    exact ⟨_, by simp, h⟩
  | regWrite reg racc value =>
    simp only [interpretInstructionEffect, EStateM.run, modify, modifyGet, MonadStateOf.modifyGet,
      EStateM.modifyGet] at hr ⊢
    cases hr
    exact ⟨_, rfl, by rw [h.regs], h.tags, h.cycles, h.out, h.mem⟩
  | clockCycle =>
    simp only [interpretInstructionEffect, EStateM.run, modify, modifyGet, MonadStateOf.modifyGet,
      EStateM.modifyGet] at hr ⊢
    cases hr
    exact ⟨_, rfl, h.regs, h.tags, by simp [h.cycles], h.out, h.mem⟩
  | getCycleCount =>
    simp only [interpretInstructionEffect, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get,
      EStateM.get, pure, EStateM.pure, EStateM.Result.ok.injEq] at hr ⊢
    obtain ⟨rfl, rfl⟩ := hr
    exact ⟨t, by simp [h.cycles], h⟩
  | printMessage msg =>
    simp only [interpretInstructionEffect, EStateM.run, modify, modifyGet, MonadStateOf.modifyGet,
      EStateM.modifyGet] at hr ⊢
    cases hr
    exact ⟨_, rfl, h.regs, h.tags, h.cycles, by simp [h.out], h.mem⟩
  | _ =>
    simp only [interpretInstructionEffect, EStateM.run, pure, EStateM.pure, EStateM.Result.ok.injEq] at hr ⊢
    first | (obtain ⟨rfl, rfl⟩ := hr) | (obtain ⟨-, rfl⟩ := hr)
    exact ⟨t, by simp, h⟩

/-- Monotone in memory: a program that runs on s runs the same way on any t
    extending it, to a state extending s's result. -/
theorem interp_mono {D : Nat → Option (BitVec 8)} {ty : Type} (x : PreSailM ue ty) :
    ∀ {s t s' : SequentialState Arch.register_type} (r : ty), Ext D s t →
      (interpretSailEffects x).run s = .ok r s' →
      ∃ t', (interpretSailEffects x).run t = .ok r t' ∧ Ext D s' t' := by
  induction x with
  | pure a =>
    intro s t s' r h hr
    simp only [interpretSailEffects, FreeM.liftM, EStateM.run, pure, EStateM.pure,
      EStateM.Result.ok.injEq] at hr ⊢
    obtain ⟨rfl, rfl⟩ := hr
    exact ⟨t, by simp, h⟩
  | impure eff cont ih =>
    intro s t s' r h hr
    rcases eff with (e | e) | c
    · simp [interpretSailEffects, FreeM.liftM, EStateM.run, bind, EStateM.bind, throw, throwThe,
        MonadExceptOf.throw, EStateM.throw] at hr
    · simp only [interpretSailEffects, FreeM.liftM, EStateM.run, bind, EStateM.bind] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        obtain ⟨t1, ht1, h1'⟩ := effect_mono e r1 h h1
        obtain ⟨t', ht', h'⟩ := ih r1 r h1' hr
        simp only [EStateM.run] at ht1
        rw [ht1]; exact ⟨t', ht', h'⟩
      · cases hr
    · simp only [interpretSailEffects, FreeM.liftM, EStateM.run, bind, EStateM.bind] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        split at h1
        · next hc =>
          cases h1
          obtain ⟨t', ht', h'⟩ := ih _ r h hr
          refine ⟨t', ?_, h'⟩
          simp only [hc, dite_true]
          exact ht'
        · simp [throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h1
      · cases hr

/-! ## Failures -/

/-- What a failure on s is on t: the same, or a read of a byte s lacks and D
    gives. -/
def ErrOn (D : Nat → Option (BitVec 8)) (s : SequentialState Arch.register_type) (e : Error ue) (ok : Prop) :
    Prop :=
  (∃ a, e = .OutOfMemoryRange a ∧ (D a).isSome ∧ s.mem.get? a = none) ∨ ok

theorem readByte_err {D : Nat → Option (BitVec 8)} {s t s' : SequentialState Arch.register_type} (h : Ext D s t)
    (a : Nat) (e : Error ue) (hr : (readByte (ue := ue) a).run s = .error e s') :
    ErrOn D s e (∃ t', (readByte (ue := ue) a).run t = .error e t') := by
  unfold readByte at hr ⊢
  have hm := h.mem a
  cases hs : s.mem[a]? <;>
    simp [hs, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get, pure,
      EStateM.pure, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr
  obtain ⟨rfl, -⟩ := hr
  simp only [Std.ExtHashMap.get?_eq_getElem?, hs, Option.none_or] at hm
  cases hd : D a with
  | some v => exact .inl ⟨a, rfl, by simp [hd], by simpa using hs⟩
  | none =>
    refine .inr ⟨t, ?_⟩
    simp [hm, hd, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get,
      throw, throwThe, MonadExceptOf.throw, EStateM.throw]

theorem readBytes_err {D : Nat → Option (BitVec 8)} (size : Nat) : ∀ (addr : Nat)
    {s t s' : SequentialState Arch.register_type} (_ : Ext D s t) (e : Error ue),
    (readBytes (ue := ue) size addr).run s = .error e s' →
      ErrOn D s e (∃ t', (readBytes (ue := ue) size addr).run t = .error e t') := by
  induction size with
  | zero => intro addr s t s' h e hr; simp [readBytes, EStateM.run, pure, EStateM.pure] at hr
  | succ n ih =>
    intro addr s t s' h e hr
    cases n with
    | zero =>
      simp only [readBytes, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr ⊢
      split at hr
      · cases hr
      · next e1 s1 h1 =>
        cases hr
        rcases readByte_err h addr _ h1 with h' | ⟨t', ht'⟩
        · exact .inl h'
        · simp only [EStateM.run] at ht'; exact .inr ⟨t', by rw [ht']⟩
    | succ m =>
      simp only [readBytes, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        obtain ⟨e1, h1'⟩ := readByte_mono h addr r1 h1
        subst e1
        split at hr
        · cases hr
        · next e2 s2 h2 =>
          cases hr
          simp only [EStateM.run] at h1'
          rw [h1']; dsimp only
          rcases ih (addr + 1) h _ h2 with h' | ⟨t', ht'⟩
          · exact .inl h'
          · simp only [EStateM.run] at ht'; exact .inr ⟨t', by rw [ht']⟩
      · next e1 s1 h1 =>
        cases hr
        rcases readByte_err h addr _ h1 with h' | ⟨t', ht'⟩
        · exact .inl h'
        · simp only [EStateM.run] at ht'; exact .inr ⟨t', by rw [ht']⟩

theorem forM_write_ok (l : List (Nat × BitVec 8)) :
    ∀ s : SequentialState Arch.register_type,
      ∃ s', (List.forM l (fun (p : Nat × BitVec 8) => writeByte (ue := ue) p.1 p.2)).run s = .ok () s' := by
  induction l with
  | nil => intro s; exact ⟨s, rfl⟩
  | cons p l ih =>
    intro s
    obtain ⟨s', h⟩ := ih { s with mem := s.mem.insert p.1 p.2 }
    exact ⟨s', by simp only [List.forM, EStateM.run, bind, EStateM.bind] at h ⊢; exact h⟩

theorem writeBytes_ok {n : Nat} (addr : Nat) (value : BitVec (8 * n)) (s : SequentialState Arch.register_type) :
    ∃ s', (writeBytes (ue := ue) addr value).run s = .ok () s' := by
  unfold writeBytes
  obtain ⟨s', h⟩ := forM_write_ok (ue := ue)
    (List.ofFn (λ i : Fin n => (addr + i.val, value.extractLsb' (8 * i.val) 8))) s
  exact ⟨s', h⟩

theorem effect_err {D : Nat → Option (BitVec 8)} (eff : InstructionEffect)
    {s t s' : SequentialState Arch.register_type} (e : Error ue) (h : Ext D s t)
    (hr : (interpretInstructionEffect (ue := ue) eff).run s = .error e s') :
    ErrOn D s e (∃ t', (interpretInstructionEffect (ue := ue) eff).run t = .error e t') := by
  cases eff with
  | memRead req =>
    simp only [interpretInstructionEffect] at hr ⊢
    by_cases ht : (req.numTag != 0) = true
    · simp [ht, EStateM.run, bind, EStateM.bind, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr ⊢
      exact .inr hr.1
    · simp only [ht, Bool.false_eq_true, if_false, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr ⊢
      split at hr
      · cases hr
      · next e1 s1 h1 =>
        cases hr
        rcases readBytes_err _ _ h _ h1 with h' | ⟨t', ht'⟩
        · exact .inl h'
        · simp only [EStateM.run] at ht'; exact .inr ⟨t', by rw [ht']⟩
  | memWrite req value tags =>
    simp only [interpretInstructionEffect] at hr ⊢
    by_cases ht : (req.numTag != 0) = true
    · simp [ht, EStateM.run, bind, EStateM.bind, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr ⊢
      exact .inr hr.1
    · simp only [ht, Bool.false_eq_true, if_false, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at hr
      obtain ⟨s1, h1⟩ := writeBytes_ok (ue := ue) req.address.toNat value s
      simp only [EStateM.run] at h1
      rw [h1] at hr; cases hr
  | regRead reg acc =>
    simp only [interpretInstructionEffect] at hr ⊢
    have e' := h.regs
    cases hm : s.regs.get? reg <;>
      simp [hm, e', EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get, pure,
        EStateM.pure, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at hr ⊢
    exact .inr hr.1
  | _ =>
    simp [interpretInstructionEffect, EStateM.run, modify, modifyGet, MonadStateOf.modifyGet,
      EStateM.modifyGet, pure, EStateM.pure, bind, EStateM.bind, get, getThe, MonadStateOf.get,
      EStateM.get] at hr

/-- A program that fails on s fails the same way on t, or failed reading a
    byte s lacks and D gives. -/
theorem interp_err {D : Nat → Option (BitVec 8)} {ty : Type} (x : PreSailM ue ty) :
    ∀ {s t s' : SequentialState Arch.register_type} (e : Error ue), Ext D s t →
      (interpretSailEffects x).run s = .error e s' →
      ErrOn D s e (∃ t', (interpretSailEffects x).run t = .error e t') := by
  induction x with
  | pure a =>
    intro s t s' e h hr
    simp [interpretSailEffects, FreeM.liftM, EStateM.run, pure, EStateM.pure] at hr
  | impure eff cont ih =>
    intro s t s' e h hr
    rcases eff with (e0 | e0) | c
    · simp [interpretSailEffects, FreeM.liftM, EStateM.run, bind, EStateM.bind, throw, throwThe,
        MonadExceptOf.throw, EStateM.throw] at hr ⊢
      exact .inr hr.1
    · simp only [interpretSailEffects, FreeM.liftM, EStateM.run, bind, EStateM.bind] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        obtain ⟨t1, ht1, h1'⟩ := effect_mono e0 r1 h h1
        simp only [EStateM.run] at ht1
        rw [ht1]
        rcases ih r1 e h1' hr with ⟨a, ha, hd, hm⟩ | h'
        · refine .inl ⟨a, ha, hd, ?_⟩
          have g := effect_grows e0 s s1 r1 h1 a
          cases hs : s.mem.get? a with
          | none => rfl
          | some v =>
            have g' := g (by rw [hs]; rfl)
            rw [hm] at g'; cases g'
        · exact .inr h'
      · next e1 s1 h1 =>
        cases hr
        rcases effect_err e0 _ h h1 with h' | ⟨t', ht'⟩
        · exact .inl h'
        · simp only [EStateM.run] at ht'; exact .inr ⟨t', by rw [ht']⟩
    · simp only [interpretSailEffects, FreeM.liftM, EStateM.run, bind, EStateM.bind] at hr ⊢
      split at hr
      · next r1 s1 h1 =>
        split at h1
        · next hc =>
          cases h1
          simp only [hc, dite_true]
          exact ih _ e h hr
        · simp [throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h1
      · next e1 s1 h1 =>
        cases hr
        split at h1
        · cases h1
        · next hc =>
          simp [throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h1
          refine .inr ⟨t, ?_⟩
          simp [hc, throw, throwThe, MonadExceptOf.throw, EStateM.throw, h1.1]

end Equiv
