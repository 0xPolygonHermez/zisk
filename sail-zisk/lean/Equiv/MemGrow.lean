/-
  sail-riscv never forgets a byte: its interpreter changes memory only by
  writing bytes, so a byte defined before a run is defined after it. For
  any program of any architecture, by Sail's sequential interpreter
  (Sail/ArchSemSequential.lean).
-/
import Sail.ArchSemSequential

namespace Equiv

open Sail Sail.ArchSem

/-- Every byte defined in m is defined in m'. -/
def Grows (m m' : Std.ExtHashMap Nat (BitVec 8)) : Prop := ∀ a, (m.get? a).isSome → (m'.get? a).isSome

theorem Grows.refl (m : Std.ExtHashMap Nat (BitVec 8)) : Grows m m := fun _ h => h

theorem Grows.trans {m1 m2 m3 : Std.ExtHashMap Nat (BitVec 8)} (h1 : Grows m1 m2) (h2 : Grows m2 m3) :
    Grows m1 m3 := fun a h => h2 a (h1 a h)

theorem Grows.insert (m : Std.ExtHashMap Nat (BitVec 8)) (a : Nat) (v : BitVec 8) : Grows m (m.insert a v) := by
  intro b h
  simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert] at h ⊢
  split <;> simp_all

variable [Arch]

theorem forM_grows (f : Nat × BitVec 8 → ArchSemSeqM ue PUnit)
    (hf : ∀ p s r s', (f p).run s = .ok r s' → Grows s.mem s'.mem) (l : List (Nat × BitVec 8)) :
    ∀ s r s', (List.forM l f).run s = .ok r s' → Grows s.mem s'.mem := by
  induction l with
  | nil =>
    intro s r s' h
    simp only [List.forM, EStateM.run, pure, EStateM.pure] at h
    cases h; exact Grows.refl _
  | cons p l ih =>
    intro s r s' h
    simp only [List.forM, EStateM.run, bind, EStateM.bind] at h
    split at h
    · next r1 s1 h1 => exact (hf p s r1 s1 h1).trans (ih s1 r s' h)
    · cases h

theorem writeBytes_grows {n : Nat} (addr : Nat) (value : BitVec (8 * n))
    (s s' : SequentialState Arch.register_type) (r : PUnit)
    (h : (writeBytes (ue := ue) addr value).run s = .ok r s') : Grows s.mem s'.mem := by
  unfold writeBytes at h
  refine forM_grows _ ?_ _ s r s' h
  rintro ⟨a, v⟩ s r s' h
  simp only [writeByte, modify, modifyGet, MonadStateOf.modifyGet, EStateM.modifyGet, EStateM.run] at h
  cases h; exact Grows.insert _ _ _

theorem readByte_state (addr : Nat) (s s' : SequentialState Arch.register_type) (r : BitVec 8)
    (h : (readByte (ue := ue) addr).run s = .ok r s') : s' = s := by
  unfold readByte at h
  cases hm : s.mem[addr]? <;>
    simp [hm, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get, pure,
      EStateM.pure, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h
  exact h.2.symm

theorem readBytes_state (size : Nat) : ∀ (addr : Nat) (s s' : SequentialState Arch.register_type)
    (r : BitVec (8 * size)), (readBytes (ue := ue) size addr).run s = .ok r s' → s' = s := by
  induction size with
  | zero =>
    intro addr s s' r h
    simp only [readBytes, EStateM.run, pure, EStateM.pure, EStateM.Result.ok.injEq] at h; exact h.2.symm
  | succ n ih =>
    intro addr s s' r h
    cases n with
    | zero =>
      simp only [readBytes, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at h
      split at h
      · next r1 s1 h1 => simp only [EStateM.Result.ok.injEq] at h; rw [← h.2]; exact readByte_state addr s s1 r1 h1
      · cases h
    | succ m =>
      simp only [readBytes, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at h
      split at h
      · next r1 s1 h1 =>
        split at h
        · next r2 s2 h2 =>
          simp only [EStateM.Result.ok.injEq] at h
          rw [← h.2, ih (addr + 1) s1 s2 r2 h2]; exact readByte_state addr s s1 r1 h1
        · cases h
      · cases h

theorem effect_grows (eff : InstructionEffect) (s s' : SequentialState Arch.register_type) (r : eff.ret)
    (h : (interpretInstructionEffect (ue := ue) eff).run s = .ok r s') : Grows s.mem s'.mem := by
  cases eff with
  | memRead req =>
    simp only [interpretInstructionEffect] at h
    by_cases ht : (req.numTag != 0) = true
    · simp [ht, EStateM.run, bind, EStateM.bind, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h
    · simp only [ht, Bool.false_eq_true, if_false, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at h
      split at h
      · next r1 s1 h1 =>
        simp only [EStateM.Result.ok.injEq] at h
        rw [← h.2, readBytes_state _ _ s s1 r1 h1]; exact Grows.refl _
      · cases h
  | memWrite req value tags =>
    simp only [interpretInstructionEffect] at h
    by_cases ht : (req.numTag != 0) = true
    · simp [ht, EStateM.run, bind, EStateM.bind, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h
    · simp only [ht, Bool.false_eq_true, if_false, EStateM.run, bind, EStateM.bind, pure, EStateM.pure] at h
      split at h
      · next r1 s1 h1 =>
        simp only [EStateM.Result.ok.injEq] at h
        rw [← h.2]; exact writeBytes_grows _ _ s s1 r1 h1
      · cases h
  | regRead reg acc =>
    simp only [interpretInstructionEffect] at h
    cases hm : s.regs.get? reg <;>
      simp [hm, EStateM.run, bind, EStateM.bind, get, getThe, MonadStateOf.get, EStateM.get, pure,
        EStateM.pure, throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h
    rw [← h.2]; exact Grows.refl _
  | _ =>
    simp [interpretInstructionEffect, EStateM.run, modify, modifyGet, MonadStateOf.modifyGet,
      EStateM.modifyGet, pure, EStateM.pure, bind, EStateM.bind, get, getThe, MonadStateOf.get,
      EStateM.get] at h
    all_goals first | (obtain ⟨-, rfl⟩ := h; exact Grows.refl _) | (rw [← h.2]; exact Grows.refl _) |
      (subst h; exact Grows.refl _) | skip

theorem interp_grows {ty : Type} (x : PreSailM ue ty) :
    ∀ (s s' : SequentialState Arch.register_type) (r : ty),
      (interpretSailEffects x).run s = .ok r s' → Grows s.mem s'.mem := by
  induction x with
  | pure a =>
    intro s s' r h
    simp only [interpretSailEffects, FreeM.liftM, EStateM.run, pure, EStateM.pure, EStateM.Result.ok.injEq] at h
    rw [← h.2]; exact Grows.refl _
  | impure eff cont ih =>
    intro s s' r h
    simp only [interpretSailEffects, FreeM.liftM, EStateM.run, bind, EStateM.bind] at h
    split at h
    · next r1 s1 h1 =>
      have g1 : Grows s.mem s1.mem := by
        rcases eff with (e | e) | c
        · simp [throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h1
        · exact effect_grows e s s1 r1 h1
        · simp only at h1
          split at h1
          · simp only [pure, EStateM.pure, EStateM.Result.ok.injEq] at h1; rw [← h1.2]; exact Grows.refl _
          · simp [throw, throwThe, MonadExceptOf.throw, EStateM.throw] at h1
      exact g1.trans (ih r1 s1 s' r h)
    · cases h

end Equiv
