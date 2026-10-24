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
    (he : ZiskRiscv.readWord (model.mem T.rv) (model.getPC T.rv).toNat = some e) (hop : e.extractLsb' 0 7 ≠ 0x73#7) :
    ZiskRiscv.decode (ZiskRiscv.readWord (model.mem S.rv)) (model.getPC S.rv).toNat = .riscv := by
  rw [getPC_eq h]
  cases hs : ZiskRiscv.readWord (model.mem S.rv) (model.getPC T.rv).toNat with
  | none => simp [ZiskRiscv.decode, hs]
  | some w =>
    have := readWord_ext h.ext hs
    have he' : ZiskRiscv.readWord T.rv.mem (model.getPC T.rv).toNat = some e := he
    rw [he'] at this; cases this
    exact decode_riscv hs hop

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

/-- A run of steps left to sail-riscv, on sparse memory: to a state sparse of
    the full run's, or the machine's error. -/
theorem runs_sparse {k : Nat} {T Tk : ZiskRiscv.State model} (h : MRunR k T Tk) :
    ∀ S, SparseOf S T →
      (∃ Sk, SparseOf Sk Tk ∧ Grows S.rv.mem Sk.rv.mem ∧
        ∀ fuel n, ZiskRiscv.run.go (k + fuel) n S = ZiskRiscv.run.go fuel (n + k) Sk) ∨
      (∃ msg, ∀ fuel n, ZiskRiscv.run.go (k + fuel) n S = .error msg) := by
  induction h with
  | done T => intro S hs; exact .inl ⟨S, hs, Grows.refl _, fun fuel n => by simp⟩
  | @step m T Tk t hT _ ih =>
    intro S hs
    rcases go_sparse hT 4096 S hs with ⟨S1, h1, hs1, hg1⟩ | ⟨msg, hmsg⟩
    · have e1 : ZiskRiscv.step S = .ok (S1, .running) := h1
      rcases ih S1 hs1 with ⟨Sk, hsk, hgk, hrun⟩ | ⟨msg, hmsg⟩
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

/-! ## The theorem on sparse memory -/

open Equiv.Top in
/-- `exit_end` on memory as the machine is given it: the machine started on
    sparse memory s0' (s0 its memory with 0 in the regions where it has
    none, the exit's word there) ends as on full memory — in `exit c`, with
    the same output — or stops with its own error. -/
theorem exit_end_sparse (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (rs : List PMA_Region) (hx : ¬ ExecAt rs EXIT_ADDR) (ht : ¬ ExecAt rs TRAP_VEC)
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkT rs zr k s0 0)
    (hin : RunIn C k s0 0)
    (pc : BitVec 64) (hpc : (rvIter 0 k s0).regs.get? Register.PC = some pc) (hpx : ExecAt rs pc) (hpcc : C pc)
    (hw : ZiskRiscv.readWord (rvIter 0 k s0).mem pc.toNat = some ECALL)
    (h17 : getX (rvIter 0 k s0) 17 = some 93) (c : BitVec 64) (h10 : getX (rvIter 0 k s0) 10 = some c)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) (hw0 : ZiskRiscv.readWord s0'.mem pc.toNat = some ECALL) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit c, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧
      (c = 0 → z'.regs.get? .Zerror = some false ∧ z'.mem = (rvIter 0 k s0).mem) ∧
      (c ≠ 0 → z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c) := by
  refine ⟨?_, (exit_end zrom entry hb rs hx ht C hnx m0 hfull hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k hrun
    hin pc hpc hpx hpcc hw h17 c h10).2⟩
  obtain ⟨-, -, -, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k hrun hin
  have hmr := hm (PcOk.of_exec ht hpc hpx)
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ with ⟨Sk, hsk, hg, hgo⟩ | ⟨msg, hmsg⟩
  · refine .inl ⟨Sk, fun fuel => ?_, output_sparse hsk⟩
    have hgT : model.getPC (rvIter 0 k s0) = pc := by simp [model, hpc]
    have hg' : model.getPC Sk.rv = pc := (getPC_eq hsk).trans hgT
    have hwk : ZiskRiscv.readWord (model.mem Sk.rv) (model.getPC Sk.rv).toNat = some ECALL := by
      rw [hg']
      obtain ⟨w', hw'⟩ := readWord_grows hg hw0
      have := readWord_ext hsk.ext hw'
      have hw2 : ZiskRiscv.readWord (rvIter 0 k s0).mem pc.toNat = some w' := this
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
    (C : BitVec 64 → Prop) (hnx : NoCodeRam C)
    (m0 : Mem) (hfull : Full m0) (hspec : SpecRom zrom C m0) (s0 : RVState) (hs0 : Start s0 rs entry m0)
    (z0 : Equiv.Zisk.ZState) (hzx : z0.regs.get? .Xs = some (Vector.replicate 40 0))
    (hzpc : z0.regs.get? .PC = some Transpiler.ROM_ENTRY) (hzst : ∃ st, z0.regs.get? .Step = some st)
    (hze : z0.regs.get? .Zerror = some false) (hzm : z0.mem = m0)
    (fc : ZiskRiscv.Fcall) (zr : BitVec 64 → _root_.Zisk.zisk_inst) (k : Nat) (hrun : RunOkT rs zr k s0 0)
    (hin : RunIn C k s0 0)
    (hpc : (rvIter 0 k s0).regs.get? Register.PC = some Transpiler.OUTPUT_BLOCK)
    (s0' : RVState) (hsp : Ext zeroFill s0' s0) :
    ((∃ Sk : ZiskRiscv.State model,
        (∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .ok (Sk, .exit 0, k)) ∧
        ZiskRiscv.output Sk = ZiskRiscv.output (⟨rvIter 0 k s0, fc⟩ : ZiskRiscv.State model)) ∨
      ∃ msg, ∀ fuel, ZiskRiscv.run (k + 1 + fuel) (⟨s0', fc⟩ : ZiskRiscv.State model) = .error msg) ∧
    ∃ z', ZEnds zrom z0 z' ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = (rvIter 0 k s0).mem := by
  refine ⟨?_, (return_end zrom entry hb rs hx ht C hnx m0 hfull hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k
    hrun hin hpc).2⟩
  obtain ⟨-, -, -, -, -, -, hm⟩ :=
    covered_run zrom entry hb rs hx ht C hnx m0 hspec s0 hs0 z0 hzx hzpc hzst hze hzm fc zr k hrun hin
  have hmr := hm ⟨_, hpc, by decide⟩
  rcases runs_sparse hmr ⟨s0', fc⟩ ⟨hsp, rfl⟩ with ⟨Sk, hsk, -, hgo⟩ | ⟨msg, hmsg⟩
  · refine .inl ⟨Sk, fun fuel => ?_, output_sparse hsk⟩
    have he := step_return Sk (by rw [getPC_eq hsk]; simp [model, hpc]; rfl)
    simp only [ZiskRiscv.run]
    rw [show k + 1 + fuel = k + (fuel + 1) by omega, hgo, ZiskRiscv.run.go]
    simp [he, bind, Except.bind, pure, Except.pure]
  · exact .inr ⟨msg, fun fuel => by
      simp only [ZiskRiscv.run]; rw [show k + 1 + fuel = k + (fuel + 1) by omega]; exact hmsg _ 0⟩

end Equiv.Sparse
