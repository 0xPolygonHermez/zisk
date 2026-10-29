/-
  Memory as the machine is given it, sparse: an unwritten byte missing,
  which the machine reads as 0, retrying the step with it written 0
  (ZiskRiscv.step). Against the same run on memory as ZisK starts it, every
  byte of its regions defined, 0 where unwritten (`SparseOf`): the sparse
  run takes the same steps, to states sparse of the full run's, or stops
  with the machine's own error (a step needing more than 4096 retries).
  From Sail's interpreter being monotone in memory (Equiv/MemMono.lean).
-/
import Equiv.Theorem
import Equiv.MemMono

namespace Equiv.Sparse

open Equiv.Riscv Equiv.Sim Equiv.Machine
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The bytes full memory has where sparse memory has none: 0 in ZisK's
    regions. -/
def zeroFill (a : Nat) : Option (BitVec 8) := if ZiskRiscv.inRegions a then some 0 else none

/-- S on sparse memory as T on full memory: the same registers and fcall
    state, T's memory S's with 0 in the regions where S has none. -/
structure SparseOf (S T : ZiskRiscv.State model) : Prop where
  ext : Ext zeroFill S.rv T.rv
  fcall : S.fcall = T.fcall

theorem getPC_eq {S T : ZiskRiscv.State model} (h : SparseOf S T) : model.getPC S.rv = model.getPC T.rv := by
  simp [model, h.ext.regs]

theorem getX_eq {S T : ZiskRiscv.State model} (h : SparseOf S T) (r : Nat) :
    model.getX S.rv r = model.getX T.rv r := by
  simp only [model, getX]; rw [h.ext.regs]

theorem readWord_ext {s t : RVState} (h : Ext zeroFill s t) {a : Nat} {e : BitVec 32}
    (hs : ZiskRiscv.readWord s.mem a = some e) : ZiskRiscv.readWord t.mem a = some e := by
  have hb : ∀ b v, s.mem.get? b = some v → t.mem.get? b = some v := by
    intro b v hv; rw [h.mem b, hv]; rfl
  simp only [ZiskRiscv.readWord] at hs ⊢
  cases h0 : s.mem.get? a <;> cases h1 : s.mem.get? (a + 1) <;> cases h2 : s.mem.get? (a + 2) <;>
    cases h3 : s.mem.get? (a + 3) <;> simp_all

/-- A plain RISC-V word at pc on full memory is plain RISC-V on sparse
    memory too: the same word, or none, which the machine leaves to
    sail-riscv as well. -/
theorem decode_sparse {S T : ZiskRiscv.State model} (h : SparseOf S T) {e : BitVec 32}
    (he : ZiskRiscv.readWord (model.mem T.rv) (model.getPC T.rv).toNat = some e) (hop : Plain e) :
    ZiskRiscv.decode (ZiskRiscv.readWord (model.mem S.rv)) (model.getPC S.rv).toNat = .riscv := by
  rw [getPC_eq h]
  cases hs : ZiskRiscv.readWord (model.mem S.rv) (model.getPC T.rv).toNat with
  | none => simp [ZiskRiscv.decode, hs]
  | some w =>
    have := readWord_ext h.ext hs
    have he' : ZiskRiscv.readWord T.rv.mem (model.getPC T.rv).toNat = some e := he
    rw [he'] at this; cases this
    exact hop _ _ hs

/-- The model's step on sparse memory: as on full memory, or a read of a
    byte in the regions the sparse memory lacks. -/
theorem model_step_sparse {s t t' : RVState} (h : Ext zeroFill s t) (ht : model.step t = .ok t') :
    (∃ s', model.step s = .ok s' ∧ Ext zeroFill s' t') ∨
    (∃ a, model.step s = .error (.missing a) ∧ ZiskRiscv.inRegions a = true ∧ s.mem.get? a = none) := by
  have ht' : ∃ b, (interpretSailEffects (try_step 0 true)).run t = .ok b t' := by
    simp only [model, runM, bind, Except.bind] at ht
    split at ht
    · cases ht
    · next p hp =>
      split at hp
      · next b u hb => cases hp; simp [pure, Except.pure] at ht; exact ⟨b, ht ▸ hb⟩
      · cases hp
  obtain ⟨b, hb⟩ := ht'
  cases hs : (interpretSailEffects (try_step 0 true)).run s with
  | ok b' s' =>
    obtain ⟨t'', ht'', hx⟩ := interp_mono _ b' h hs
    rw [hb] at ht''; cases ht''
    exact .inl ⟨s', by simp [model, runM, hs, bind, Except.bind, pure, Except.pure], hx⟩
  | error e s1 =>
    rcases interp_err _ e h hs with ⟨a, rfl, hd, hm⟩ | ⟨t'', ht''⟩
    · refine .inr ⟨a, by simp [model, runM, hs, bind, Except.bind, ZiskRiscv.StepError.ofSail], ?_, hm⟩
      simp only [zeroFill] at hd; split at hd <;> simp_all
    · rw [hb] at ht''; cases ht''

theorem model_step_grows {s s' : RVState} (h : model.step s = .ok s') : Grows s.mem s'.mem := by
  simp only [model, runM, bind, Except.bind] at h
  split at h
  · cases h
  · next p hp =>
    split at hp
    · next b u hb => cases hp; simp [pure, Except.pure] at h; subst h; exact interp_grows _ _ _ _ hb
    · cases hp

/-- Writing a 0 the sparse memory lacks in the regions keeps it sparse of the
    full memory. -/
theorem SparseOf.fill {S T : ZiskRiscv.State model} (h : SparseOf S T) {a : Nat}
    (ha : ZiskRiscv.inRegions a = true) (hm : S.rv.mem.get? a = none) :
    SparseOf { S with rv := model.setMem S.rv ((model.mem S.rv).insert a 0) } T := by
  refine ⟨⟨h.ext.regs, h.ext.tags, h.ext.cycles, h.ext.out, fun b => ?_⟩, h.fcall⟩
  have := h.ext.mem b
  simp only [model] at this ⊢
  simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert] at this hm ⊢
  split
  · next e => simp at e; subst e; rw [this, hm]; simp [zeroFill, ha]
  · exact this

/-- The machine's step on sparse memory, where on full memory it is a step
    left to sail-riscv: the same step, the memory grown, or the machine's
    error (fuel out). -/
theorem go_sparse {T : ZiskRiscv.State model} {t : model.State} (hT : RStep T t) :
    ∀ (f : Nat) (S : ZiskRiscv.State model), SparseOf S T →
      (∃ S', ZiskRiscv.step.go f S = .ok (S', .running) ∧ SparseOf S' { T with rv := t } ∧
        Grows S.rv.mem S'.rv.mem) ∨ ∃ msg, ZiskRiscv.step.go f S = .error msg
  | 0, S, _ => .inr ⟨_, rfl⟩
  | f + 1, S, h => by
    have hx : model.getPC S.rv ≠ model.exitAddr := by rw [getPC_eq h]; exact hT.pcx
    obtain ⟨e, he, hop⟩ := hT.word
    have hd := decode_sparse h he hop
    rcases model_step_sparse h.ext hT.step with ⟨s', hs', hx'⟩ | ⟨a, ha, hreg, hm⟩
    · have htr : model.trapped s' = false := by
        have := hT.trap; simp only [model] at this ⊢; rw [← hx'.regs]; exact this
      have e1 : ZiskRiscv.tryStep S = .ok ({ S with rv := s' }, .running) := by
        simp only [ZiskRiscv.tryStep, hd, hs']
        simp [hx, bind, Except.bind, htr, pure, Except.pure]
      refine .inl ⟨{ S with rv := s' }, by rw [ZiskRiscv.step.go, e1], ⟨hx', h.fcall⟩, model_step_grows hs'⟩
    · have e1 : ZiskRiscv.tryStep S = .error (.missing a) := by
        simp only [ZiskRiscv.tryStep, hd, ha]
        simp [hx, bind, Except.bind, pure, Except.pure]
      have hf := h.fill hreg hm
      rcases go_sparse hT f _ hf with ⟨S', hS', hsp, hg⟩ | ⟨msg, hmsg⟩
      · refine .inl ⟨S', by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hS', hsp, ?_⟩
        exact (Grows.insert _ a 0).trans hg
      · exact .inr ⟨msg, by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hmsg⟩

/-- The machine's step on sparse memory, where on full memory it is a step
    left to sail-riscv that traps: the run's failure, or the machine's
    error (fuel out). -/
theorem go_sparse_trap {T : ZiskRiscv.State model} {t : model.State} (hT : TStep T t) :
    ∀ (f : Nat) (S : ZiskRiscv.State model), SparseOf S T →
      (∃ S' m, ZiskRiscv.step.go f S = .ok (S', .failure m)) ∨ ∃ msg, ZiskRiscv.step.go f S = .error msg
  | 0, S, _ => .inr ⟨_, rfl⟩
  | f + 1, S, h => by
    have hx : model.getPC S.rv ≠ model.exitAddr := by rw [getPC_eq h]; exact hT.pcx
    obtain ⟨e, he, hop⟩ := hT.word
    have hd := decode_sparse h he hop
    rcases model_step_sparse h.ext hT.step with ⟨s', hs', hx'⟩ | ⟨a, ha, hreg, hm⟩
    · have htr : model.trapped s' = true := by
        have := hT.trap; simp only [model] at this ⊢; rw [← hx'.regs]; exact this
      have e1 : ZiskRiscv.tryStep S =
          .ok ({ S with rv := s' }, .failure "a trap: a fault or an illegal instruction") := by
        simp only [ZiskRiscv.tryStep, hd, hs']
        simp [hx, bind, Except.bind, htr, pure, Except.pure]
      exact .inl ⟨_, _, by rw [ZiskRiscv.step.go, e1]⟩
    · have e1 : ZiskRiscv.tryStep S = .error (.missing a) := by
        simp only [ZiskRiscv.tryStep, hd, ha]
        simp [hx, bind, Except.bind, pure, Except.pure]
      have hf := h.fill hreg hm
      rcases go_sparse_trap hT f _ hf with ⟨S', m, hS'⟩ | ⟨msg, hmsg⟩
      · exact .inl ⟨S', m, by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hS'⟩
      · exact .inr ⟨msg, by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hmsg⟩

theorem readWord_grows {m m' : Mem} (g : Grows m m') {a : Nat} {w : BitVec 32}
    (h : ZiskRiscv.readWord m a = some w) : ∃ w', ZiskRiscv.readWord m' a = some w' := by
  simp only [ZiskRiscv.readWord] at h ⊢
  have i0 := g a; have i1 := g (a + 1); have i2 := g (a + 2); have i3 := g (a + 3)
  cases h0 : m.get? a <;> cases h1 : m.get? (a + 1) <;> cases h2 : m.get? (a + 2) <;>
    cases h3 : m.get? (a + 3) <;> simp only [h0, h1, h2, h3] at h <;> try cases h
  simp only [h0, h1, h2, h3, Option.isSome_some, forall_const] at i0 i1 i2 i3
  obtain ⟨b0, e0⟩ := Option.isSome_iff_exists.1 i0
  obtain ⟨b1, e1⟩ := Option.isSome_iff_exists.1 i1
  obtain ⟨b2, e2⟩ := Option.isSome_iff_exists.1 i2
  obtain ⟨b3, e3⟩ := Option.isSome_iff_exists.1 i3
  exact ⟨b3 ++ b2 ++ b1 ++ b0, by simp only [e0, e1, e2, e3]; rfl⟩

/-- Every code address's word in memory: what the machine is given holds the
    program's code. -/
def CodeIn (C : BitVec 64 → Prop) (S : ZiskRiscv.State model) : Prop :=
  ∀ pc, C pc → ∃ w, ZiskRiscv.readWord S.rv.mem pc.toNat = some w

theorem CodeIn.grow {C : BitVec 64 → Prop} {S S' : ZiskRiscv.State model} (h : CodeIn C S)
    (g : Grows S.rv.mem S'.rv.mem) : CodeIn C S' := fun pc hc => by
  obtain ⟨w, hw⟩ := h pc hc
  exact readWord_grows g hw

theorem _root_.Equiv.Ext.writeWord {s t : RVState} (h : Ext zeroFill s t) (p : Nat) (w : BitVec 32) :
    Ext zeroFill { s with mem := ZiskRiscv.writeWord s.mem p w } { t with mem := ZiskRiscv.writeWord t.mem p w } := by
  refine ⟨h.regs, h.tags, h.cycles, h.out, fun b => ?_⟩
  have := h.mem b
  show (ZiskRiscv.writeWord t.mem p w).get? b = ((ZiskRiscv.writeWord s.mem p w).get? b).or (zeroFill b)
  rw [Equiv.writeWord_get, Equiv.writeWord_get]
  by_cases e0 : b = p
  · simp [e0]
  by_cases e1 : b = p + 1
  · simp [e1]
  by_cases e2 : b = p + 2
  · simp [e2]
  by_cases e3 : b = p + 3
  · simp [e3]
  simp only [e0, e1, e2, e3, if_false]; exact this

theorem _root_.Equiv.Ext.setX {s t : RVState} (h : Ext zeroFill s t) (k : Nat) (v : BitVec 64) :
    Ext zeroFill (setX s k v) (setX t k v) := by
  unfold Equiv.Riscv.setX
  split
  · exact h
  · exact ⟨by simp only; rw [h.regs], h.tags, h.cycles, h.out, h.mem⟩

theorem _root_.Equiv.Ext.mPost {s t : RVState} (h : Ext zeroFill s t) (p : Nat) (w : BitVec 32) (zr : Nat) :
    Ext zeroFill (mPost s p w zr) (mPost t p w zr) :=
  (h.writeWord p w).setX zr 0

/-- The machine's step on sparse memory, where on full memory it is lr or
    sc run by substitution: the same step, the memory grown, or the
    machine's error (fuel out). The word at pc is in the sparse memory (the
    code is), so the machine decodes it the same. -/
theorem go_sparse_subst {C : BitVec 64 → Prop} {T : ZiskRiscv.State model} {t : model.State} (hT : SStep C T t) :
    ∀ (f : Nat) (S : ZiskRiscv.State model), SparseOf S T → CodeIn C S →
      (∃ S', ZiskRiscv.step.go f S = .ok (S', .running) ∧ SparseOf S' { T with rv := t } ∧
        Grows S.rv.mem S'.rv.mem) ∨ ∃ msg, ZiskRiscv.step.go f S = .error msg
  | 0, S, _, _ => .inr ⟨_, rfl⟩
  | f + 1, S, h, hcd => by
    obtain ⟨hx, hc, w, w', zr, t1, hw, hd, hs, htr, ht⟩ := hT
    have hg := getPC_eq h
    have hx' : model.getPC S.rv ≠ model.exitAddr := by rw [hg]; exact hx
    obtain ⟨ws, hws⟩ := hcd _ hc
    have hwT := readWord_ext h.ext hws
    have hwT' : ZiskRiscv.readWord T.rv.mem (model.getPC T.rv).toNat = some w := hw
    rw [hwT'] at hwT; cases hwT
    have hwS : ZiskRiscv.readWord (model.mem S.rv) (model.getPC S.rv).toNat = some w := by rw [hg]; exact hws
    have hdS := hd (ZiskRiscv.readWord (model.mem S.rv)) hws
    rw [← hg] at hdS
    have hE : Ext zeroFill { S.rv with mem := ZiskRiscv.writeWord S.rv.mem (model.getPC S.rv).toNat w' }
        { T.rv with mem := ZiskRiscv.writeWord T.rv.mem (model.getPC T.rv).toNat w' } := by
      rw [hg]; exact h.ext.writeWord _ w'
    rcases model_step_sparse hE hs with ⟨s1, hs1, hx1⟩ | ⟨a, ha, hreg, hm⟩
    · have htr1 : model.trapped (model.setMem s1 (ZiskRiscv.writeWord (model.mem s1) (model.getPC S.rv).toNat w)) =
          false := by
        have := htr; simp only [model] at this ⊢; rw [← hx1.regs]; exact this
      have e1 : ZiskRiscv.tryStep S =
          .ok ({ S with rv := mPost s1 (model.getPC S.rv).toNat w zr }, .running) := by
        simp only [ZiskRiscv.tryStep, hdS, hwS]
        have hs1' : model.step (model.setMem S.rv (ZiskRiscv.writeWord (model.mem S.rv)
            (model.getPC S.rv).toNat w')) = .ok s1 := hs1
        simp only [hs1']
        simp [hx', bind, Except.bind, htr1, pure, Except.pure]
        rw [← post_eq]; by_cases hz : zr = 0 <;> simp [hz]
      refine .inl ⟨{ S with rv := mPost s1 (model.getPC S.rv).toNat w zr }, by rw [ZiskRiscv.step.go, e1],
        ⟨?_, h.fcall⟩, ?_⟩
      · show Ext zeroFill (mPost s1 (model.getPC S.rv).toNat w zr) t
        rw [ht, hg]; exact hx1.mPost _ w zr
      · have e : (mPost s1 (model.getPC S.rv).toNat w zr).mem =
            ZiskRiscv.writeWord s1.mem (model.getPC S.rv).toNat w := by
          simp only [mPost, setX]; split <;> rfl
        show Grows S.rv.mem (mPost s1 (model.getPC S.rv).toNat w zr).mem
        rw [e]
        exact ((writeWord_grows _ _ _).trans (model_step_grows hs1)).trans (writeWord_grows _ _ _)
    · have e1 : ZiskRiscv.tryStep S = .error (.missing a) := by
        simp only [ZiskRiscv.tryStep, hdS, hwS]
        have ha' : model.step (model.setMem S.rv (ZiskRiscv.writeWord (model.mem S.rv)
            (model.getPC S.rv).toNat w')) = .error (.missing a) := ha
        simp only [ha']
        simp [hx', bind, Except.bind, pure, Except.pure]
      have hm' : S.rv.mem.get? a = none := by
        have hm2 : (ZiskRiscv.writeWord S.rv.mem (model.getPC S.rv).toNat w').get? a = none := hm
        rw [Equiv.writeWord_get] at hm2
        split at hm2 <;> try (cases hm2)
        split at hm2 <;> try (cases hm2)
        split at hm2 <;> try (cases hm2)
        split at hm2 <;> try (cases hm2)
        exact hm2
      have hf := h.fill hreg hm'
      have hcd' : CodeIn C { S with rv := model.setMem S.rv ((model.mem S.rv).insert a 0) } :=
        hcd.grow (Grows.insert _ a 0)
      rcases go_sparse_subst ⟨hx, hc, w, w', zr, t1, hw, hd, hs, htr, ht⟩ f _ hf hcd' with
        ⟨S', hS', hsp, hg'⟩ | ⟨msg, hmsg⟩
      · refine .inl ⟨S', by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hS', hsp, ?_⟩
        exact (Grows.insert _ a 0).trans hg'
      · exact .inr ⟨msg, by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hmsg⟩

/-! ## ZisK calls on sparse memory -/

theorem mext_insert {m m' : Mem} (h : ∀ k, m'.get? k = (m.get? k).or (zeroFill k)) (a : Nat) (v : BitVec 8) :
    ∀ k, (m'.insert a v).get? k = ((m.insert a v).get? k).or (zeroFill k) := by
  intro b
  have := h b
  simp only [Std.ExtHashMap.get?_eq_getElem?, Std.ExtHashMap.getElem?_insert] at this ⊢
  split
  · rfl
  · exact this

theorem mext_foldl {m m' : Mem} (h : ∀ k, m'.get? k = (m.get? k).or (zeroFill k)) (l : List Nat)
    (f : Nat → Nat) (g : Nat → BitVec 8) :
    ∀ k, (l.foldl (fun m i => m.insert (f i) (g i)) m').get? k =
      ((l.foldl (fun m i => m.insert (f i) (g i)) m).get? k).or (zeroFill k) := by
  induction l generalizing m m' with
  | nil => exact h
  | cons i l ih => exact ih (mext_insert h _ _)

theorem mext_writeDword {m m' : Mem} (h : ∀ k, m'.get? k = (m.get? k).or (zeroFill k)) (a : Nat) (v : BitVec 64) :
    ∀ k, (ZiskRiscv.writeDword m' a v).get? k = ((ZiskRiscv.writeDword m a v).get? k).or (zeroFill k) :=
  mext_foldl h _ _ _

theorem _root_.Equiv.Ext.call_post {s t : RVState} {mS mT : Mem} (h : Ext zeroFill s t)
    (hm : ∀ k, mT.get? k = (mS.get? k).or (zeroFill k)) (rd : Nat) (c pc : BitVec 64) :
    Ext zeroFill (callPost s mS rd c pc) (callPost t mT rd c pc) := by
  have h2 := (show Ext zeroFill { s with mem := mS } { t with mem := mT } from
    ⟨h.regs, h.tags, h.cycles, h.out, hm⟩).setX rd c
  have hr : (callPost t mT rd c pc).regs = (callPost s mS rd c pc).regs := by
    show (setX _ rd c).regs.insert Register.PC pc = (setX _ rd c).regs.insert Register.PC pc
    rw [h2.regs]
  refine ⟨hr, h2.tags, h2.cycles, h2.out, fun k => ?_⟩
  rw [callPost_mem, callPost_mem]; exact hm k

theorem callMem_grows (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) : Grows S.rv.mem (callMem S c) := by
  cases hc : c.count with
  | none => simp only [callMem, hc]; exact Grows.refl _
  | some r => simp only [callMem, hc, writeDword_eq]; exact memWrite_grows' _ _ _

/-- A ZisK call on sparse memory, where on full memory it runs: the same
    call, the memory grown; or a read of a byte in the regions the sparse
    memory lacks (`Frame`: an operation that fails on less memory reads a
    byte it lacks, or fails on more). -/
theorem call_sparse {S T T' : ZiskRiscv.State model} (h : SparseOf S T) (c : ZiskRiscv.Call)
    (hc : ZiskRiscv.call T c = .ok T') :
    (∃ S', ZiskRiscv.call S c = .ok S' ∧ SparseOf S' T' ∧ Grows S.rv.mem S'.rv.mem) ∨
    (∃ a, ZiskRiscv.call S c = .error (.missing a) ∧ ZiskRiscv.inRegions a = true ∧ S.rv.mem.get? a = none) := by
  have harg : ∀ x, S.argValue x = T.argValue x := by
    intro x; cases x <;> simp only [ZiskRiscv.State.argValue, getX_eq h, h.fcall]
  have hcm : ∀ k, (callMem T c).get? k = ((callMem S c).get? k).or (zeroFill k) := by
    cases hc : c.count with
    | none => simp only [callMem, hc]; exact h.ext.mem
    | some r => simp only [callMem, hc, getX_eq h r]; exact mext_writeDword h.ext.mem _ _
  have hFR : Equiv.Zisk.FR (ZiskRiscv.ziskState (callMem S c) S.fcall) (ZiskRiscv.ziskState (callMem T c) T.fcall) := by
    refine ⟨fun k v hk => ?_, fun r _ => by rw [h.fcall]; rfl⟩
    have hk' : (callMem S c).get? k = some v := hk
    show (callMem T c).get? k = some v
    rw [hcm k, hk']; rfl
  cases hS : (_root_.Zisk.Functions.zisk_exec_mem c.op (S.argValue c.a) (S.argValue c.b)
      (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0)).run
      (ZiskRiscv.ziskState (callMem S c) S.fcall) with
  | ok r σ' =>
    obtain ⟨τ', hτ, hfr, -, hk⟩ := (Equiv.Zisk.Frame.exec_mem _ _ _ _).1 _ _ hFR r σ' hS
    have hcS := call_run S c _ _ _ (Equiv.Zisk.runOp_ok hS)
    rw [harg c.a, harg c.b] at hτ
    have hcT := call_run T c _ _ _ (Equiv.Zisk.runOp_ok hτ)
    rw [hcT] at hc; cases hc
    rw [call_post_eq] at hcS ⊢
    have hfo : Equiv.Zisk.fcallOf σ' = Equiv.Zisk.fcallOf τ' := Equiv.Zisk.fcallOf_eq hfr.regs
    have hbase : ∀ k, τ'.mem.get? k = (σ'.mem.get? k).or (zeroFill k) := by
      intro k
      cases hs : σ'.mem.get? k with
      | some v => rw [hfr.mem k v hs]; rfl
      | none =>
        have h0 := hk.none hs
        rw [hk.same k h0 hs]
        have h0' : (callMem S c).get? k = none := h0
        show (callMem T c).get? k = _
        rw [hcm k, h0']
    refine .inl ⟨_, hcS, ⟨?_, by rw [hfo]⟩, ?_⟩
    · rw [harg c.a, harg c.b, getPC_eq h, hfo]
      exact h.ext.call_post (mext_writeDword hbase _ _) _ _ _
    · show Grows S.rv.mem (callPost _ _ _ _ _).mem
      rw [callPost_mem, writeDword_eq]
      refine ((callMem_grows S c).trans fun k hk' => ?_).trans (memWrite_grows' _ _ _)
      obtain ⟨v, hv⟩ := Option.isSome_iff_exists.1 hk'
      obtain ⟨v', hv'⟩ := hk.grows k v hv
      rw [hv']; rfl
  | error e σ' =>
    rcases (Equiv.Zisk.Frame.exec_mem _ _ _ _).2 _ _ hFR e σ' hS with ⟨k, rfl, hk1, hk2⟩ | ⟨τ', hτ⟩
    · have hk1' : (callMem S c).get? k = none := hk1
      have hk2' : (callMem T c).get? k ≠ none := hk2
      rw [hcm k, hk1'] at hk2'
      refine .inr ⟨k, call_err S c _ (Equiv.Zisk.runOp_err hS), ?_, ?_⟩
      · simp only [Option.none_or, zeroFill] at hk2'; split at hk2' <;> simp_all
      · cases hm : S.rv.mem.get? k with
        | none => rfl
        | some v =>
          have := callMem_grows S c k (by rw [hm]; rfl)
          rw [hk1'] at this; cases this
    · rw [harg c.a, harg c.b] at hτ
      rw [call_err T c _ (Equiv.Zisk.runOp_err hτ)] at hc; cases hc

/-- The machine's step on sparse memory, where on full memory it is a ZisK
    call: the same step, the memory grown, or the machine's error (fuel
    out). The words the decoder reads are in the sparse memory (the code
    is), so it decodes the same call. -/
theorem go_sparse_call {C : BitVec 64 → Prop} {T T' : ZiskRiscv.State model} (hT : CStep C T T') :
    ∀ (f : Nat) (S : ZiskRiscv.State model), SparseOf S T → CodeIn C S →
      (∃ S', ZiskRiscv.step.go f S = .ok (S', .running) ∧ SparseOf S' T' ∧
        Grows S.rv.mem S'.rv.mem) ∨ ∃ msg, ZiskRiscv.step.go f S = .error msg
  | 0, S, _, _ => .inr ⟨_, rfl⟩
  | f + 1, S, h, hcd => by
    obtain ⟨hx, hcc, c, hcall, hC4, hC8, hd⟩ := hT
    have hg := getPC_eq h
    have hx' : model.getPC S.rv ≠ model.exitAddr := by rw [hg]; exact hx
    have hw : ∀ p, C p → ZiskRiscv.readWord S.rv.mem p.toNat = ZiskRiscv.readWord T.rv.mem p.toNat := by
      intro p hp
      obtain ⟨ws, hws⟩ := hcd p hp
      rw [hws, readWord_ext h.ext hws]
    have hdS : ZiskRiscv.decode (ZiskRiscv.readWord (model.mem S.rv)) (model.getPC S.rv).toNat = .call c := by
      rw [hg]
      refine hd _ (hw _ hcc) (fun hl => ?_) fun hl => ?_
      · obtain ⟨h4, e4⟩ := hC4 hl
        have := hw _ h4; rw [e4] at this; exact this
      · obtain ⟨h8, e8⟩ := hC8 hl
        have := hw _ h8; rw [e8] at this; exact this
    rcases call_sparse h c hcall with ⟨S', hS', hsp, hgr⟩ | ⟨a, ha, hreg, hm⟩
    · refine .inl ⟨S', ?_, hsp, hgr⟩
      have e1 : ZiskRiscv.tryStep S = .ok (S', .running) := by
        simp only [ZiskRiscv.tryStep, hdS]
        simp [hx', bind, Except.bind, hS', pure, Except.pure]
      rw [ZiskRiscv.step.go, e1]
    · have e1 : ZiskRiscv.tryStep S = .error (.missing a) := by
        simp only [ZiskRiscv.tryStep, hdS]
        simp [hx', bind, Except.bind, ha, pure, Except.pure]
      have hf := h.fill hreg hm
      have hcd' : CodeIn C { S with rv := model.setMem S.rv ((model.mem S.rv).insert a 0) } :=
        hcd.grow (Grows.insert _ a 0)
      rcases go_sparse_call ⟨hx, hcc, c, hcall, hC4, hC8, hd⟩ f _ hf hcd' with ⟨S', hS', hsp, hg'⟩ | ⟨msg, hmsg⟩
      · refine .inl ⟨S', by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hS', hsp, ?_⟩
        exact (Grows.insert _ a 0).trans hg'
      · exact .inr ⟨msg, by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hmsg⟩

/-- A ZisK call on sparse memory, where on full memory it fails: the same
    failure, or a read of a byte in the regions the sparse memory lacks. -/
theorem call_sparse_fail {S T : ZiskRiscv.State model} (h : SparseOf S T) (c : ZiskRiscv.Call)
    {err : ZiskRiscv.StepError} (hc : ZiskRiscv.call T c = .error err) :
    (∃ a, ZiskRiscv.call S c = .error (.missing a) ∧ ZiskRiscv.inRegions a = true ∧ S.rv.mem.get? a = none) ∨
    ZiskRiscv.call S c = .error err := by
  have harg : ∀ x, S.argValue x = T.argValue x := by
    intro x; cases x <;> simp only [ZiskRiscv.State.argValue, getX_eq h, h.fcall]
  have hcm : ∀ k, (callMem T c).get? k = ((callMem S c).get? k).or (zeroFill k) := by
    cases hc : c.count with
    | none => simp only [callMem, hc]; exact h.ext.mem
    | some r => simp only [callMem, hc, getX_eq h r]; exact mext_writeDword h.ext.mem _ _
  have hFR : Equiv.Zisk.FR (ZiskRiscv.ziskState (callMem S c) S.fcall) (ZiskRiscv.ziskState (callMem T c) T.fcall) := by
    refine ⟨fun k v hk => ?_, fun r _ => by rw [h.fcall]; rfl⟩
    have hk' : (callMem S c).get? k = some v := hk
    show (callMem T c).get? k = some v
    rw [hcm k, hk']; rfl
  cases hS : (_root_.Zisk.Functions.zisk_exec_mem c.op (S.argValue c.a) (S.argValue c.b)
      (if _root_.Zisk.Functions.zisk_op_is_precompiled c.op then c.ext else 0)).run
      (ZiskRiscv.ziskState (callMem S c) S.fcall) with
  | ok r σ' =>
    obtain ⟨τ', hτ, -⟩ := (Equiv.Zisk.Frame.exec_mem _ _ _ _).1 _ _ hFR r σ' hS
    rw [harg c.a, harg c.b] at hτ
    rw [call_run T c _ _ _ (Equiv.Zisk.runOp_ok hτ)] at hc; cases hc
  | error e σ' =>
    rcases (Equiv.Zisk.Frame.exec_mem _ _ _ _).2 _ _ hFR e σ' hS with ⟨k, rfl, hk1, hk2⟩ | ⟨τ', hτ⟩
    · have hk1' : (callMem S c).get? k = none := hk1
      have hk2' : (callMem T c).get? k ≠ none := hk2
      rw [hcm k, hk1'] at hk2'
      refine .inl ⟨k, call_err S c _ (Equiv.Zisk.runOp_err hS), ?_, ?_⟩
      · simp only [Option.none_or, zeroFill] at hk2'; split at hk2' <;> simp_all
      · cases hm : S.rv.mem.get? k with
        | none => rfl
        | some v =>
          have := callMem_grows S c k (by rw [hm]; rfl)
          rw [hk1'] at this; cases this
    · rw [harg c.a, harg c.b] at hτ
      rw [call_err T c _ (Equiv.Zisk.runOp_err hτ)] at hc
      injection hc with hc; subst hc
      exact .inr (call_err S c _ (Equiv.Zisk.runOp_err hS))

/-- The machine's step on sparse memory, where on full memory it is a ZisK
    call that fails, by a missing byte only outside ZisK's regions: a
    failure, or the machine's error. -/
theorem go_sparse_fail {C : BitVec 64 → Prop} {T : ZiskRiscv.State model} (hx : model.getPC T.rv ≠ model.exitAddr)
    (hcc : C (model.getPC T.rv)) (c : ZiskRiscv.Call)
    (hC4 : c.len ≠ 1 → C (model.getPC T.rv + 4) ∧ (model.getPC T.rv + 4).toNat = (model.getPC T.rv).toNat + 4)
    (hC8 : c.len = 3 → C (model.getPC T.rv + 8) ∧ (model.getPC T.rv + 8).toNat = (model.getPC T.rv).toNat + 8)
    (hd : ∀ fetch : Nat → Option (BitVec 32),
      fetch (model.getPC T.rv).toNat = ZiskRiscv.readWord (model.mem T.rv) (model.getPC T.rv).toNat →
      (c.len ≠ 1 → fetch ((model.getPC T.rv).toNat + 4) =
        ZiskRiscv.readWord (model.mem T.rv) ((model.getPC T.rv).toNat + 4)) →
      (c.len = 3 → fetch ((model.getPC T.rv).toNat + 8) =
        ZiskRiscv.readWord (model.mem T.rv) ((model.getPC T.rv).toNat + 8)) →
      ZiskRiscv.decode fetch (model.getPC T.rv).toNat = .call c)
    {err : ZiskRiscv.StepError} (hc : ZiskRiscv.call T c = .error err)
    (hne : ∀ a, err = .missing a → ZiskRiscv.inRegions a = false) :
    ∀ (f : Nat) (S : ZiskRiscv.State model), SparseOf S T → CodeIn C S →
      (∃ S' m, ZiskRiscv.step.go f S = .ok (S', .failure m)) ∨ ∃ msg, ZiskRiscv.step.go f S = .error msg
  | 0, S, _, _ => .inr ⟨_, rfl⟩
  | f + 1, S, h, hcd => by
    have hg := getPC_eq h
    have hx' : model.getPC S.rv ≠ model.exitAddr := by rw [hg]; exact hx
    have hw : ∀ p, C p → ZiskRiscv.readWord S.rv.mem p.toNat = ZiskRiscv.readWord T.rv.mem p.toNat := by
      intro p hp
      obtain ⟨ws, hws⟩ := hcd p hp
      rw [hws, readWord_ext h.ext hws]
    have hdS : ZiskRiscv.decode (ZiskRiscv.readWord (model.mem S.rv)) (model.getPC S.rv).toNat = .call c := by
      rw [hg]
      refine hd _ (hw _ hcc) (fun hl => ?_) fun hl => ?_
      · obtain ⟨h4, e4⟩ := hC4 hl
        have := hw _ h4; rw [e4] at this; exact this
      · obtain ⟨h8, e8⟩ := hC8 hl
        have := hw _ h8; rw [e8] at this; exact this
    rcases call_sparse_fail h c hc with ⟨a, ha, hreg, hm⟩ | hS
    · have e1 : ZiskRiscv.tryStep S = .error (.missing a) := by
        simp only [ZiskRiscv.tryStep, hdS]
        simp [hx', bind, Except.bind, ha, pure, Except.pure]
      have hf := h.fill hreg hm
      have hcd' : CodeIn C { S with rv := model.setMem S.rv ((model.mem S.rv).insert a 0) } :=
        hcd.grow (Grows.insert _ a 0)
      rcases go_sparse_fail hx hcc c hC4 hC8 hd hc hne f _ hf hcd' with ⟨S', m, hs⟩ | ⟨msg, hmsg⟩
      · exact .inl ⟨S', m, by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hs⟩
      · exact .inr ⟨msg, by rw [ZiskRiscv.step.go, e1]; simp only [hreg, if_true]; exact hmsg⟩
    · have e1 : ZiskRiscv.tryStep S = .error err := by
        simp only [ZiskRiscv.tryStep, hdS]
        simp [hx', bind, Except.bind, hS, pure, Except.pure]
      rw [ZiskRiscv.step.go, e1]
      cases err with
      | missing a => simp only [hne a rfl, Bool.false_eq_true, if_false]; exact .inl ⟨S, _, rfl⟩
      | assertion m => exact .inl ⟨S, m, rfl⟩
      | other m => exact .inr ⟨m, rfl⟩

/-- A run of the machine's steps, on sparse memory holding the code: to a
    state sparse of the full run's, or the machine's error. -/
theorem runs_sparse {C : BitVec 64 → Prop} {k : Nat} {T Tk : ZiskRiscv.State model} (h : MRunR C k T Tk) :
    ∀ S, SparseOf S T → CodeIn C S →
      (∃ Sk, SparseOf Sk Tk ∧ Grows S.rv.mem Sk.rv.mem ∧
        ∀ fuel n, ZiskRiscv.run.go (k + fuel) n S = ZiskRiscv.run.go fuel (n + k) Sk) ∨
      (∃ msg, ∀ fuel n, ZiskRiscv.run.go (k + fuel) n S = .error msg) := by
  have step : ∀ {m : Nat} {T1 Tk : ZiskRiscv.State model} (S : ZiskRiscv.State model),
      ((∃ S1, ZiskRiscv.step.go 4096 S = .ok (S1, .running) ∧ SparseOf S1 T1 ∧
        Grows S.rv.mem S1.rv.mem) ∨ ∃ msg, ZiskRiscv.step.go 4096 S = .error msg) → CodeIn C S →
      (∀ S, SparseOf S T1 → CodeIn C S →
        (∃ Sk, SparseOf Sk Tk ∧ Grows S.rv.mem Sk.rv.mem ∧
          ∀ fuel n, ZiskRiscv.run.go (m + fuel) n S = ZiskRiscv.run.go fuel (n + m) Sk) ∨
        (∃ msg, ∀ fuel n, ZiskRiscv.run.go (m + fuel) n S = .error msg)) →
      (∃ Sk, SparseOf Sk Tk ∧ Grows S.rv.mem Sk.rv.mem ∧
        ∀ fuel n, ZiskRiscv.run.go (m + 1 + fuel) n S = ZiskRiscv.run.go fuel (n + (m + 1)) Sk) ∨
      (∃ msg, ∀ fuel n, ZiskRiscv.run.go (m + 1 + fuel) n S = .error msg) := by
    intro m T1 Tk S hgo hcd ih
    rcases hgo with ⟨S1, h1, hs1, hg1⟩ | ⟨msg, hmsg⟩
    · have e1 : ZiskRiscv.step S = .ok (S1, .running) := h1
      rcases ih S1 hs1 (hcd.grow hg1) with ⟨Sk, hsk, hgk, hrun⟩ | ⟨msg, hmsg⟩
      · refine .inl ⟨Sk, hsk, hg1.trans hgk, fun fuel n => ?_⟩
        rw [show m + 1 + fuel = (m + fuel) + 1 by omega, ZiskRiscv.run.go]
        simp only [e1, bind, Except.bind]
        rw [hrun, show n + 1 + m = n + (m + 1) by omega]
      · refine .inr ⟨msg, fun fuel n => ?_⟩
        rw [show m + 1 + fuel = (m + fuel) + 1 by omega, ZiskRiscv.run.go]
        simp only [e1, bind, Except.bind]
        exact hmsg fuel (n + 1)
    · have e1 : ZiskRiscv.step S = .error msg := hmsg
      refine .inr ⟨msg, fun fuel n => ?_⟩
      rw [show m + 1 + fuel = (m + fuel) + 1 by omega, ZiskRiscv.run.go]
      simp [e1, bind, Except.bind]
  induction h with
  | done T => intro S hs _; exact .inl ⟨S, hs, Grows.refl _, fun fuel n => by simp⟩
  | @step m T Tk t hT _ ih =>
    intro S hs hcd
    exact step S (go_sparse hT 4096 S hs) hcd ih
  | @subst m T Tk t hT _ ih =>
    intro S hs hcd
    exact step S (go_sparse_subst hT 4096 S hs hcd) hcd ih
  | @call m T T' Tk hT _ ih =>
    intro S hs hcd
    exact step S (go_sparse_call hT 4096 S hs hcd) hcd ih

/-- The output the machine reads is the same from sparse memory as from
    full: a byte the sparse memory lacks reads 0, as full memory has it. -/
theorem output_sparse {S T : ZiskRiscv.State model} (h : SparseOf S T) :
    ZiskRiscv.output S = ZiskRiscv.output T := by
  have hb : ∀ a, (model.mem T.rv).getD a 0 = (model.mem S.rv).getD a 0 := by
    intro a
    have := h.ext.mem a
    simp only [Std.ExtHashMap.get?_eq_getElem?] at this
    simp only [model, Std.ExtHashMap.getD_eq_getD_getElem?]
    rw [this]
    cases hs : S.rv.mem[a]? <;> simp [zeroFill]
    split <;> rfl
  simp only [ZiskRiscv.output, ZiskRiscv.readDword, hb]

/-! ## The theorem on sparse memory -/

open Equiv.Top in
/-- `exit_end` on memory as the machine is given it: the machine started on
    sparse memory s0' (s0 its memory with 0 in the regions where it has
    none, the exit's word there) ends as on full memory — in `exit c`, with
    the same output — or stops with its own error. -/
theorem exit_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (hw : ZiskRiscv.readWord ((sIter 0 k ⟨s0, fc⟩).rv).mem pc.toNat = some ECALL)
    (h17 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 17 = some 93) (c : BitVec 64) (h10 : getX ((sIter 0 k ⟨s0, fc⟩).rv) 10 = some c)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w)
    (hw0 : ZiskRiscv.readWord s0'.mem pc.toNat = some ECALL) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit c, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) := by
  refine ⟨?_, (exit_end zrom entry hb rs hx ht C hnx hni m0 hfull hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun
    hin pc hpc hpx hpcc hw h17 c h10).2⟩
  obtain ⟨-, -, -, -, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, hg, hgo⟩ | ⟨msg, hmsg⟩
  · refine .inl ⟨Sk, fun fuel => ?_, output_sparse hsk⟩
    have hgT : model.getPC T.rv = pc := by simp [model, hpc]
    have hg' : model.getPC Sk.rv = pc := (getPC_eq hsk).trans hgT
    have hwk : ZiskRiscv.readWord (model.mem Sk.rv) (model.getPC Sk.rv).toNat = some ECALL := by
      rw [hg']
      obtain ⟨w', hw'⟩ := readWord_grows hg hw0
      have := readWord_ext hsk.ext hw'
      have hw2 : ZiskRiscv.readWord T.rv.mem pc.toNat = some w' := this
      rw [hw] at hw2; cases hw2; exact hw'
    have h17' : model.getX Sk.rv 17 = 93 := by rw [getX_eq hsk]; simp [model, h17]
    have he := step_exit Sk (by rw [hg']; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hpx)) hwk h17'
    have hc : model.getX Sk.rv 10 = c := by rw [getX_eq hsk]; simp [model, h10]
    rw [hc] at he
    simp only [ZiskRiscv.run]
    rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
    simp [he, bind, Except.bind, pure, Except.pure]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

open Equiv.Top in
/-- `return_end` on sparse memory, as `exit_end_sparse`. -/
theorem return_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0)
    (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hpc : ((sIter 0 k ⟨s0, fc⟩).rv).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0)
    (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit 0, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (sIter 0 k ⟨s0, fc⟩)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn ((sIter 0 k ⟨s0, fc⟩).rv).mem := by
  refine ⟨?_, (return_end zrom entry hb rs hx ht C hnx hni m0 hfull hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k
    hrun hin hpc).2⟩
  obtain ⟨-, -, -, -, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm ⟨_, hpc, by decide⟩
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, -, hgo⟩ | ⟨msg, hmsg⟩
  · refine .inl ⟨Sk, fun fuel => ?_, output_sparse hsk⟩
    have he := step_return Sk (by rw [getPC_eq hsk]; simp [model, hpc]; rfl)
    simp only [ZiskRiscv.run]
    rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
    simp [he, bind, Except.bind, pure, Except.pure]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

open Equiv.Top in
/-- `fail_end` on sparse memory, as `exit_end_sparse`: the machine's run
    ends in failure, or in its own error. -/
theorem fail_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (w : BitVec 32) (hrw : ZiskRiscv.readWord (sIter 0 k ⟨s0, fc⟩).rv.mem pc.toNat = some w)
    (h4 : (pc + 4).toNat = pc.toNat + 4) (h8 : (pc + 8).toNat = pc.toNat + 8) (c : ZiskRiscv.Call)
    (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc)
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc)) pc.toNat = .call c)
    (hC4 : c.len ≠ 1 → C (pc + 4)) (hC8 : c.len = 3 → C (pc + 8))
    (hfail : CallFail (sIter 0 k ⟨s0, fc⟩) c)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ n z1 e z', ZRun zrom n z0 z1 ∧ (zstep zrom).run z1 = .error e z' := by
  refine ⟨?_, (fail_end zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun
    hin pc hpc hpx hpcc w hrw h4 h8 c hd hfail).2⟩
  obtain ⟨z1, -, -, hrel, hfr, hspr, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  have htr := hspr pc w hpcc hrw ⟨0, _, convert_call3 hd 0⟩
  obtain ⟨-, -, err, hc, hne⟩ := callfail_m T z1 rs hrel hfr hx pc hpc zrom w hpx hrw htr h4 h8 c hd hfail
  have hg : model.getPC T.rv = pc := by simp [model, hpc]
  have hxT : model.getPC T.rv ≠ model.exitAddr := by
    rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hpx)
  have hdT : ∀ fetch : Nat → Option (BitVec 32),
      fetch (model.getPC T.rv).toNat = ZiskRiscv.readWord (model.mem T.rv) (model.getPC T.rv).toNat →
      (c.len ≠ 1 → fetch ((model.getPC T.rv).toNat + 4) =
        ZiskRiscv.readWord (model.mem T.rv) ((model.getPC T.rv).toNat + 4)) →
      (c.len = 3 → fetch ((model.getPC T.rv).toNat + 8) =
        ZiskRiscv.readWord (model.mem T.rv) ((model.getPC T.rv).toNat + 8)) →
      ZiskRiscv.decode fetch (model.getPC T.rv).toNat = .call c := by
    intro fetch f0 f4 f8
    rw [hg] at f0 f4 f8 ⊢
    refine decode_any3 hd fetch (by rw [f0]; exact hrw) (fun hl => ?_) fun hl => ?_
    · rw [f4 hl, nextWord, h4]; rfl
    · rw [f8 hl, nextWord2, h8]; rfl
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, hgk, hgo⟩ | ⟨msg, hmsg⟩
  · have hcdk : CodeIn C Sk := CodeIn.grow (S := ⟨s0', fc⟩) hcode hgk
    rcases go_sparse_fail hxT (by rw [hg]; exact hpcc) c (fun hl => by rw [hg]; exact ⟨hC4 hl, h4⟩)
        (fun hl => by rw [hg]; exact ⟨hC8 hl, h8⟩) hdT hc hne 4096 Sk hsk hcdk with ⟨S', m, hs⟩ | ⟨msg, hs⟩
    · refine .inl ⟨S', m, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .ok (S', .failure m) := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind, pure, Except.pure]
    · refine .inr ⟨msg, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .error msg := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

open Equiv.Top in
/-- `fault_end` on sparse memory, as `exit_end_sparse`: the machine's run
    ends in failure, or stops with its own error. -/
theorem fault_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hfa : FaultAt rs zrom (sIter 0 k ⟨s0, fc⟩).rv)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ n z1 e z', ZRun zrom n z0 z1 ∧ (zstep zrom).run z1 = .error e z' := by
  refine ⟨?_, (fault_end zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun
    hin hfa).2⟩
  obtain ⟨z1, -, -, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨pc, hpc, hw, ⟨s', hrun', hpc'⟩, -⟩ := fault_step T.rv z1 rs hrel zrom hfa 0
  have hmr := hm (PcOk.of_exec ht hpc hw.1)
  have hT := mstep_trapT T rs hrel.ready pc hpc hw hx hrun' hpc'
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, -, hgo⟩ | ⟨msg, hmsg⟩
  · rcases go_sparse_trap hT 4096 Sk hsk with ⟨S', m, hs⟩ | ⟨msg, hs⟩
    · refine .inl ⟨S', m, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .ok (S', .failure m) := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind, pure, Except.pure]
    · refine .inr ⟨msg, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .error msg := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

open Equiv.Top in
/-- `misjump_end` on sparse memory, as `exit_end_sparse`: the machine's run
    ends in failure, or stops with its own error. -/
theorem misjump_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hmj : MisJumpAt rs zrom (sIter 0 k ⟨s0, fc⟩).rv) (hmh : HaltMis zrom)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  refine ⟨?_, (misjump_end zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k
    hrun hin hmj hmh).2⟩
  obtain ⟨z1, -, -, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨pc, -, hpc, hw, -, -, ⟨s', hrun', hpc'⟩, -⟩ := misjump_step T.rv z1 rs hrel zrom hmj 0
  have hmr := hm (PcOk.of_exec ht hpc hw.1)
  have hT := mstep_trapT T rs hrel.ready pc hpc hw hx hrun' hpc'
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, -, hgo⟩ | ⟨msg, hmsg⟩
  · rcases go_sparse_trap hT 4096 Sk hsk with ⟨S', m, hs⟩ | ⟨msg, hs⟩
    · refine .inl ⟨S', m, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .ok (S', .failure m) := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind, pure, Except.pure]
    · refine .inr ⟨msg, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .error msg := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

open Equiv.Top in
/-- `fusedmis_end` on sparse memory, as `exit_end_sparse`: the machine's
    run ends in failure, or stops with its own error. -/
theorem fusedmis_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC) (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (i : FInstr)
    (hi : FtAtM (sIter 0 k ⟨s0, fc⟩).rv rs pc zrom i) (htg : (i.T pc)[0] = false ∧ (i.T pc)[1] = true)
    (hmh : HaltMis zrom)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 2 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k + 1)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 2 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  refine ⟨?_, (fusedmis_end zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k
    hrun hin pc hpc i hi htg hmh).2⟩
  obtain ⟨z1, -, -, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨s1, hrun1, hr1, hpc1, hw1, hw2, ⟨s2, hrun2, hpc2⟩, -⟩ := fusedmis_step T.rv z1 rs hrel pc hpc zrom i hi htg 0
  have hmr := hm (PcOk.of_exec ht hpc hi.x1)
  have hR := mstepR T rs hrel.ready pc hpc hw1 hx hrun1 (PcOk.of_exec ht hpc1 hi.x3)
  have hmr1 := hmr.append (.step hR (.done _))
  have hT := mstep_trapT { T with rv := s1 } rs hr1 (pc + 4) hpc1 hw2 hx hrun2 hpc2
  rcases runs_sparse hmr1 ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, -, hgo⟩ | ⟨msg, hmsg⟩
  · rcases go_sparse_trap hT 4096 Sk hsk with ⟨S', m, hs⟩ | ⟨msg, hs⟩
    · refine .inl ⟨S', m, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .ok (S', .failure m) := hs
      simp only [ZiskRiscv.run]
      rw [show k + 2 + fuel = (k + 1) + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind, pure, Except.pure]
    · refine .inr ⟨msg, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .error msg := hs
      simp only [ZiskRiscv.run]
      rw [show k + 2 + fuel = (k + 1) + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 2 + fuel = (k + 1) + (fuel + 1) by omega]; exact hmsg _ 0⟩

open Equiv.Top in
/-- `halt_end` on sparse memory, as `exit_end_sparse`. -/
theorem halt_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (hha : HaltAt rs zrom (sIter 0 k ⟨s0, fc⟩).rv)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  refine ⟨?_, (halt_end zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun
    hin hha).2⟩
  obtain ⟨z1, -, -, hrel, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  obtain ⟨pc, e, hpc, he, hi⟩ := hha
  obtain ⟨⟨s', hrun', hpc'⟩, -⟩ := halt_step T.rv z1 rs hrel pc hpc zrom _ e he hi 0
  have hw : WordOk T.rv.mem rs pc := ⟨hi.x1, e, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, he.plain⟩
  have hmr := hm (PcOk.of_exec ht hpc hi.x1)
  have hT := mstep_trapT T rs hrel.ready pc hpc hw hx hrun' hpc'
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, -, hgo⟩ | ⟨msg, hmsg⟩
  · rcases go_sparse_trap hT 4096 Sk hsk with ⟨S', m, hs⟩ | ⟨msg, hs⟩
    · refine .inl ⟨S', m, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .ok (S', .failure m) := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind, pure, Except.pure]
    · refine .inr ⟨msg, fun fuel => ?_⟩
      have hs' : ZiskRiscv.step Sk = .error msg := hs
      simp only [ZiskRiscv.run]
      rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
      simp [hs', bind, Except.bind]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

/-- The machine's step on sparse memory, where on full memory it meets a
    sequence or a word the transpiler refuses, the words it reads code: the failure. -/
theorem go_sparse_malformed {C : BitVec 64 → Prop} {T : ZiskRiscv.State model} {why : String}
    (hx : model.getPC T.rv ≠ model.exitAddr) (hcc : C (model.getPC T.rv))
    (hC4 : C (model.getPC T.rv + 4) ∧ (model.getPC T.rv + 4).toNat = (model.getPC T.rv).toNat + 4)
    (hC8 : C (model.getPC T.rv + 8) ∧ (model.getPC T.rv + 8).toNat = (model.getPC T.rv).toNat + 8)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (model.mem T.rv)) (model.getPC T.rv).toNat = .malformed why)
    (f : Nat) (S : ZiskRiscv.State model) (h : SparseOf S T) (hcd : CodeIn C S) :
    ∃ m, ZiskRiscv.step.go (f + 1) S = .ok (S, .failure m) := by
  have hg := getPC_eq h
  have hx' : model.getPC S.rv ≠ model.exitAddr := by rw [hg]; exact hx
  have hw : ∀ p, C p → ZiskRiscv.readWord S.rv.mem p.toNat = ZiskRiscv.readWord T.rv.mem p.toNat := by
    intro p hp
    obtain ⟨ws, hws⟩ := hcd p hp
    rw [hws, readWord_ext h.ext hws]
  have hdS : ZiskRiscv.decode (ZiskRiscv.readWord (model.mem S.rv)) (model.getPC S.rv).toNat = .malformed why := by
    rw [hg]
    obtain ⟨w, hwT⟩ := hcd _ hcc
    rw [hw _ hcc] at hwT
    have h4 := hw _ hC4.1
    have h8 := hw _ hC8.1
    rw [hC4.2] at h4
    rw [hC8.2] at h8
    have e1 := decode_fetch3 (ZiskRiscv.readWord (model.mem S.rv)) (by rw [← hwT]; exact hw _ hcc) h4 h8
    have e2 := decode_fetch3 (ZiskRiscv.readWord (model.mem T.rv)) hwT rfl rfl
    exact e1.trans (e2.symm.trans hd)
  have e1 : ZiskRiscv.tryStep S = .ok (S, .failure s!"what the transpiler refuses: {why}") := by
    simp only [ZiskRiscv.tryStep, hdS]
    simp [hx', pure, Except.pure]; rfl
  exact ⟨_, by rw [ZiskRiscv.step.go, e1]⟩

open Equiv.Top in
/-- `malformed_end` on sparse memory, as `exit_end_sparse`, the two words
    after pc code too. -/
theorem malformed_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C) (hni : NoCodeIn C)
    (m0 : Mem) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (hzf : Equiv.Zisk.FcallRel fc z0) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat)
    (hrun : RunOkM rs zr C k ⟨s0, fc⟩ 0) (hin : RunInM C k ⟨s0, fc⟩ 0)
    (pc : BitVec 64) (hpc : (sIter 0 k ⟨s0, fc⟩).rv.regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (w : BitVec 32) (hrw : ZiskRiscv.readWord (sIter 0 k ⟨s0, fc⟩).rv.mem pc.toNat = some w)
    (h4 : (pc + 4).toNat = pc.toNat + 4) (h8 : (pc + 8).toNat = pc.toNat + 8) (why : String)
    (hd : ZiskRiscv.decode (Transpiler.fetch3 pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc)
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc)) pc.toNat = .malformed why)
    (hdesc : ∃ ia l, Transpiler.convert pc w (nextWord (sIter 0 k ⟨s0, fc⟩).rv pc) ia
      (nextWord2 (sIter 0 k ⟨s0, fc⟩).rv pc) = some l)
    (hC4 : C (pc + 4)) (hC8 : C (pc + 8))
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hcode : ∀ pc, C pc → ∃ w, ZiskRiscv.readWord s0'.mem pc.toNat = some w) :
    ((∃ Sk m, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .failure m, k)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some true := by
  refine ⟨?_, (malformed_end zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k
    hrun hin pc hpc hpx hpcc w hrw h4 h8 why hd hdesc).2⟩
  obtain ⟨-, -, -, -, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx hni m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc hzf zr k hrun hin
  generalize sIter 0 k ⟨s0, fc⟩ = T at *
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  have hg : model.getPC T.rv = pc := by simp [model, hpc]
  have hxT : model.getPC T.rv ≠ model.exitAddr := by
    rw [hg]; intro e; have e' : pc = EXIT_ADDR := e; exact hx (e' ▸ hpx)
  have hdT : ZiskRiscv.decode (ZiskRiscv.readWord (model.mem T.rv)) (model.getPC T.rv).toNat = .malformed why := by
    rw [hg]
    show ZiskRiscv.decode (ZiskRiscv.readWord T.rv.mem) pc.toNat = _
    rw [decode_fetch3 _ hrw (by rw [← h4]; try rfl) (by rw [← h8]; try rfl)]
    exact hd
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ hcode with ⟨Sk, hsk, hgk, hgo⟩ | ⟨msg, hmsg⟩
  · have hcdk : CodeIn C Sk := CodeIn.grow (S := ⟨s0', fc⟩) hcode hgk
    obtain ⟨m, hs⟩ := go_sparse_malformed hxT (by rw [hg]; exact hpcc) (by rw [hg]; exact ⟨hC4, h4⟩)
      (by rw [hg]; exact ⟨hC8, h8⟩) hdT 4095 Sk hsk hcdk
    refine .inl ⟨Sk, m, fun fuel => ?_⟩
    have hs' : ZiskRiscv.step Sk = .ok (Sk, .failure m) := hs
    simp only [ZiskRiscv.run]
    rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
    simp [hs', bind, Except.bind, pure, Except.pure]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

end Equiv.Sparse
