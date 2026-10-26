/-
  ZisK's RISC-V machine (ZiskRiscv/Machine.lean) over sail-riscv's Lean, for
  the proofs: the `RiscvModel` instance, as ZiskRiscvRun/Model.lean makes it
  over the executable Lean, and the machine's step where the machine leaves
  the instruction to sail-riscv — what the simulation's steps are about.

  The machine steps sail-riscv with `try_step 0 true`; the step lemmas are
  about `try_step n false`, the same step once the hart is active
  (Equiv/RiscvTryStep.lean).
-/
import Equiv.ProgSpec
import Equiv.RiscvTryStep
import ZiskRiscv.Machine

namespace Equiv.Machine

open Equiv.Riscv Equiv.Sim
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- Where the machine's model points mtvec: outside ZisK's memory, so that a
    trap is seen as one (ZiskRiscvRun/Model.lean). -/
def TRAP_VEC : BitVec 64 := 0x4

/-- The initial ra: the BIOS's return address (contract § 3). -/
def EXIT_ADDR : BitVec 64 := 0x101C

def runM (m : SailM α) (s : RVState) : Except ZiskRiscv.StepError (α × RVState) :=
  match (interpretSailEffects m).run s with
  | .ok a s => .ok (a, s)
  | .error e _ => .error (.ofSail e)

/-- sail-riscv's Lean as the machine's RISC-V model. -/
noncomputable def model : ZiskRiscv.RiscvModel where
  State := RVState
  mem s := s.mem
  setMem s m := { s with mem := m }
  getX s r := (getX s r).getD 0
  setX s r v := setX s r v
  getPC s := (s.regs.get? Register.PC).getD 0
  setPC s v := { s with regs := s.regs.insert Register.PC v }
  step s := do let (_, s) ← runM (try_step 0 true) s; pure s
  trapped s := (s.regs.get? Register.PC).getD 0 == TRAP_VEC
  exitAddr := EXIT_ADDR

/-! ## The machine's step on plain RISC-V -/

/-- The machine's step where the model runs the instruction: not at the exit
    address, plain RISC-V at pc, the model's step taken and not trapped. -/
theorem step_riscv {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (rv' : M.State)
    (hx : M.getPC S.rv ≠ M.exitAddr)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .riscv)
    (hs : M.step S.rv = .ok rv') (ht : M.trapped rv' = false) :
    ZiskRiscv.step S = .ok ({ S with rv := rv' }, .running) := by
  have e : ZiskRiscv.tryStep S = .ok ({ S with rv := rv' }, .running) := by
    simp only [ZiskRiscv.tryStep, hd, hs]
    simp [hx, bind, Except.bind, ht, pure, Except.pure]
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- The model's step, from a step of sail-riscv the simulation takes. -/
theorem model_step {s s' : RVState} {rs : List PMA_Region} (hr : StepReady s rs) {n : Nat}
    (h : (interpretSailEffects (try_step n false)).run s = .ok false s') : model.step s = .ok s' := by
  have e := (try_step_any s rs hr 0 true).trans ((try_step_any s rs hr n false).symm.trans h)
  simp [model, runM, e]; rfl

/-! ## The machine's runs -/

/-- `n` steps of the machine, each leaving the run going. -/
inductive MRun : Nat → ZiskRiscv.State model → ZiskRiscv.State model → Prop
  | done (S : ZiskRiscv.State model) : MRun 0 S S
  | step {n : Nat} {S S' S'' : ZiskRiscv.State model} :
      ZiskRiscv.step S = .ok (S', .running) → MRun n S' S'' → MRun (n + 1) S S''

theorem MRun.append {a b : Nat} {S T U : ZiskRiscv.State model} (h1 : MRun a S T) (h2 : MRun b T U) :
    MRun (a + b) S U := by
  induction h1 with
  | done => simpa using h2
  | @step m _ _ _ h r ih =>
    rw [show m + 1 + b = (m + b) + 1 by omega]
    exact .step h (ih h2)

/-- Where a step leaves sail-riscv, not trapped: its pc not the trap vector. -/
def PcOk (s : RVState) : Prop := ∃ p, s.regs.get? Register.PC = some p ∧ p ≠ TRAP_VEC

theorem PcOk.of_exec {rs : List PMA_Region} (ht : ¬ ExecAt rs TRAP_VEC) {s : RVState} {p : BitVec 64}
    (hp : s.regs.get? Register.PC = some p) (hx : ExecAt rs p) : PcOk s :=
  ⟨p, hp, fun e => ht (e ▸ hx)⟩

/-- The machine's step on a step of sail-riscv the simulation takes: the
    instruction executable and plain RISC-V, the exit address not
    executable, and the step not trapped. -/
theorem mstep (S : ZiskRiscv.State model) (rs : List PMA_Region) (hr : StepReady S.rv rs) (pc : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc) (hw : WordOk S.rv.mem rs pc) (hx : ¬ ExecAt rs EXIT_ADDR)
    {n : Nat} {s' : RVState} (hrun : (interpretSailEffects (try_step n false)).run S.rv = .ok false s')
    (ht : PcOk s') : ZiskRiscv.step S = .ok ({ S with rv := s' }, .running) := by
  obtain ⟨hex, e, he, hop⟩ := hw
  obtain ⟨p, hp, hpt⟩ := ht
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  refine step_riscv S s' ?_ ?_ (model_step hr hrun) ?_
  · rw [hg]; intro h; have h' : pc = EXIT_ADDR := h; exact hx (h' ▸ hex)
  · rw [hg]; exact hop _ _ he
  · simp [model, hp, hpt]

/-- The machine's step where the model's step traps: the run's failure. -/
theorem step_trapped {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (rv' : M.State)
    (hx : M.getPC S.rv ≠ M.exitAddr)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .riscv)
    (hs : M.step S.rv = .ok rv') (ht : M.trapped rv' = true) :
    ∃ m, ZiskRiscv.step S = .ok ({ S with rv := rv' }, .failure m) := by
  have e : ZiskRiscv.tryStep S = .ok ({ S with rv := rv' }, .failure "a trap: a fault or an illegal instruction") := by
    simp only [ZiskRiscv.tryStep, hd, hs]
    simp [hx, bind, Except.bind, ht, pure, Except.pure]
  exact ⟨_, by rw [ZiskRiscv.step, ZiskRiscv.step.go, e]⟩

/-- A step of sail-riscv the simulation takes that ends at the trap vector:
    the machine's step, the run's failure. -/
theorem mstep_trap (S : ZiskRiscv.State model) (rs : List PMA_Region) (hr : StepReady S.rv rs) (pc : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc) (hw : WordOk S.rv.mem rs pc) (hx : ¬ ExecAt rs EXIT_ADDR)
    {n : Nat} {s' : RVState} (hrun : (interpretSailEffects (try_step n false)).run S.rv = .ok false s')
    (ht : s'.regs.get? Register.PC = some TRAP_VEC) :
    ∃ m, ZiskRiscv.step S = .ok ({ S with rv := s' }, .failure m) := by
  obtain ⟨hex, e, he, hop⟩ := hw
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  refine step_trapped S s' ?_ ?_ (model_step hr hrun) ?_
  · rw [hg]; intro h; have h' : pc = EXIT_ADDR := h; exact hx (h' ▸ hex)
  · rw [hg]; exact hop _ _ he
  · simp [model, ht]

/-- A machine step left to sail-riscv that traps, from S to its model
    state t: not at the exit address, a plain word at pc, the model's step
    taken, trapped. -/
structure TStep (S : ZiskRiscv.State model) (t : model.State) : Prop where
  pcx : model.getPC S.rv ≠ model.exitAddr
  word : ∃ e : BitVec 32, ZiskRiscv.readWord (model.mem S.rv) (model.getPC S.rv).toNat = some e ∧
    Plain e
  step : model.step S.rv = .ok t
  trap : model.trapped t = true

theorem TStep.mstep {S : ZiskRiscv.State model} {t : model.State} (h : TStep S t) :
    ∃ m, ZiskRiscv.step S = .ok ({ S with rv := t }, .failure m) := by
  obtain ⟨e, he, hop⟩ := h.word
  exact step_trapped S t h.pcx (hop _ _ he) h.step h.trap

theorem mstep_trapT (S : ZiskRiscv.State model) (rs : List PMA_Region) (hr : StepReady S.rv rs) (pc : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc) (hw : WordOk S.rv.mem rs pc) (hx : ¬ ExecAt rs EXIT_ADDR)
    {n : Nat} {s' : RVState} (hrun : (interpretSailEffects (try_step n false)).run S.rv = .ok false s')
    (ht : s'.regs.get? Register.PC = some TRAP_VEC) : TStep S s' := by
  obtain ⟨hex, e, he, hop⟩ := hw
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  refine ⟨?_, ⟨e, by rw [hg]; exact he, hop⟩, model_step hr hrun, by simp [model, ht]⟩
  rw [hg]; intro h; have h' : pc = EXIT_ADDR := h; exact hx (h' ▸ hex)

/-! ## Runs of plain RISC-V steps -/

/-- A machine step left to sail-riscv, from S to its model state t: not at
    the exit address, a word not of SYSTEM's opcode at pc, the model's step
    taken and not trapped. -/
structure RStep (S : ZiskRiscv.State model) (t : model.State) : Prop where
  pcx : model.getPC S.rv ≠ model.exitAddr
  word : ∃ e : BitVec 32, ZiskRiscv.readWord (model.mem S.rv) (model.getPC S.rv).toNat = some e ∧
    Plain e
  step : model.step S.rv = .ok t
  trap : model.trapped t = false

theorem RStep.mstep {S : ZiskRiscv.State model} {t : model.State} (h : RStep S t) :
    ZiskRiscv.step S = .ok ({ S with rv := t }, .running) := by
  obtain ⟨e, he, hop⟩ := h.word
  exact step_riscv S t h.pcx (hop _ _ he) h.step h.trap

/-- A machine step on lr or sc, at a code address (`C`): not at the exit
    address, the model's step on the plain word w' put at pc, not trapped;
    then the word w put back, and register zr (sc's rd) 0 (`mPost`). -/
def SStep (C : BitVec 64 → Prop) (S : ZiskRiscv.State model) (t : model.State) : Prop :=
  model.getPC S.rv ≠ model.exitAddr ∧ C (model.getPC S.rv) ∧
  ∃ w w' zr t1, ZiskRiscv.readWord (model.mem S.rv) (model.getPC S.rv).toNat = some w ∧
    (∀ fetch : Nat → Option (BitVec 32), fetch (model.getPC S.rv).toNat = some w →
      ZiskRiscv.decode fetch (model.getPC S.rv).toNat = .subst w' zr) ∧
    model.step (model.setMem S.rv (ZiskRiscv.writeWord (model.mem S.rv) (model.getPC S.rv).toNat w')) = .ok t1 ∧
    model.trapped t1 = false ∧ t = mPost t1 (model.getPC S.rv).toNat w zr

/-- The machine's post-step, as `mPost`: x0 is no register to zero. -/
theorem post_eq (t1 : RVState) (p : Nat) (w : BitVec 32) (zr : Nat) :
    (if zr != 0 then model.setX (model.setMem t1 (ZiskRiscv.writeWord (model.mem t1) p w)) zr 0
      else model.setMem t1 (ZiskRiscv.writeWord (model.mem t1) p w)) = mPost t1 p w zr := by
  simp only [model, mPost]
  by_cases h : zr = 0
  · subst h; simp [setX]
  · simp [h]

theorem SStep.mstep {C : BitVec 64 → Prop} {S : ZiskRiscv.State model} {t : model.State} (h : SStep C S t) :
    ZiskRiscv.step S = .ok ({ S with rv := t }, .running) := by
  obtain ⟨hx, -, w, w', zr, t1, hw, hd, hs, ht, rfl⟩ := h
  have e : ZiskRiscv.tryStep S = .ok ({ S with rv := mPost t1 (model.getPC S.rv).toNat w zr }, .running) := by
    have htr : model.trapped (model.setMem t1 (ZiskRiscv.writeWord (model.mem t1) (model.getPC S.rv).toNat w)) =
        false := ht
    simp only [ZiskRiscv.tryStep, hd _ hw, hw, hs]
    simp [hx, bind, Except.bind, htr, pure, Except.pure]
    rw [← post_eq]; by_cases h : zr = 0 <;> simp [h]
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- The machine's step on a ZisK call: not at the exit address, the call
    decoded at pc, its run. -/
theorem step_call {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (c : ZiskRiscv.Call) (S' : ZiskRiscv.State M)
    (hx : M.getPC S.rv ≠ M.exitAddr)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .call c)
    (hc : ZiskRiscv.call S c = .ok S') :
    ZiskRiscv.step S = .ok (S', .running) := by
  have e : ZiskRiscv.tryStep S = .ok (S', .running) := by
    simp only [ZiskRiscv.tryStep, hd]
    simp [hx, bind, Except.bind, hc, pure, Except.pure]
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- A machine step on a ZisK call, at a code address (`C`): not at the exit
    address, the call's run; the call the decoder's on any words that agree
    with memory at pc, at pc + 4 for a call of more than one word, and at
    pc + 8 for one of three, code addresses too. -/
def CStep (C : BitVec 64 → Prop) (S S' : ZiskRiscv.State model) : Prop :=
  model.getPC S.rv ≠ model.exitAddr ∧ C (model.getPC S.rv) ∧
  ∃ c : ZiskRiscv.Call, ZiskRiscv.call S c = .ok S' ∧
    (c.len ≠ 1 → C (model.getPC S.rv + 4) ∧ (model.getPC S.rv + 4).toNat = (model.getPC S.rv).toNat + 4) ∧
    (c.len = 3 → C (model.getPC S.rv + 8) ∧ (model.getPC S.rv + 8).toNat = (model.getPC S.rv).toNat + 8) ∧
    ∀ fetch : Nat → Option (BitVec 32),
      fetch (model.getPC S.rv).toNat = ZiskRiscv.readWord (model.mem S.rv) (model.getPC S.rv).toNat →
      (c.len ≠ 1 → fetch ((model.getPC S.rv).toNat + 4) =
        ZiskRiscv.readWord (model.mem S.rv) ((model.getPC S.rv).toNat + 4)) →
      (c.len = 3 → fetch ((model.getPC S.rv).toNat + 8) =
        ZiskRiscv.readWord (model.mem S.rv) ((model.getPC S.rv).toNat + 8)) →
      ZiskRiscv.decode fetch (model.getPC S.rv).toNat = .call c

theorem CStep.mstep {C : BitVec 64 → Prop} {S S' : ZiskRiscv.State model} (h : CStep C S S') :
    ZiskRiscv.step S = .ok (S', .running) := by
  obtain ⟨hx, -, c, hc, -, -, hd⟩ := h
  exact step_call S c S' hx (hd _ rfl (fun _ => rfl) fun _ => rfl) hc

/-- `n` machine steps, each left to sail-riscv or, on lr and sc at a code
    address, run by substitution, or a ZisK call at a code address. -/
inductive MRunR (C : BitVec 64 → Prop) : Nat → ZiskRiscv.State model → ZiskRiscv.State model → Prop
  | done (S : ZiskRiscv.State model) : MRunR C 0 S S
  | step {n : Nat} {S S'' : ZiskRiscv.State model} {t : model.State} :
      RStep S t → MRunR C n { S with rv := t } S'' → MRunR C (n + 1) S S''
  | subst {n : Nat} {S S'' : ZiskRiscv.State model} {t : model.State} :
      SStep C S t → MRunR C n { S with rv := t } S'' → MRunR C (n + 1) S S''
  | call {n : Nat} {S S' S'' : ZiskRiscv.State model} :
      CStep C S S' → MRunR C n S' S'' → MRunR C (n + 1) S S''

theorem MRunR.toMRun {C : BitVec 64 → Prop} {n : Nat} {S S' : ZiskRiscv.State model} (h : MRunR C n S S') :
    MRun n S S' := by
  induction h with
  | done => exact .done _
  | step h1 _ ih => exact .step h1.mstep ih
  | subst h1 _ ih => exact .step h1.mstep ih
  | call h1 _ ih => exact .step h1.mstep ih

theorem MRunR.append {C : BitVec 64 → Prop} {a b : Nat} {S T U : ZiskRiscv.State model} (h1 : MRunR C a S T)
    (h2 : MRunR C b T U) : MRunR C (a + b) S U := by
  induction h1 with
  | done => simpa using h2
  | @step m _ _ _ h r ih =>
    rw [show m + 1 + b = (m + b) + 1 by omega]
    exact .step h (ih h2)
  | @subst m _ _ _ h r ih =>
    rw [show m + 1 + b = (m + b) + 1 by omega]
    exact .subst h (ih h2)
  | @call m _ _ _ h r ih =>
    rw [show m + 1 + b = (m + b) + 1 by omega]
    exact .call h (ih h2)

/-- `mstep`, recording the step. -/
theorem mstepR (S : ZiskRiscv.State model) (rs : List PMA_Region) (hr : StepReady S.rv rs) (pc : BitVec 64)
    (hpc : S.rv.regs.get? Register.PC = some pc) (hw : WordOk S.rv.mem rs pc) (hx : ¬ ExecAt rs EXIT_ADDR)
    {n : Nat} {s' : RVState} (hrun : (interpretSailEffects (try_step n false)).run S.rv = .ok false s')
    (ht : PcOk s') : RStep S s' := by
  obtain ⟨hex, e, he, hop⟩ := hw
  obtain ⟨p, hp, hpt⟩ := ht
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  refine ⟨?_, ⟨e, ?_, hop⟩, model_step hr hrun, ?_⟩
  · rw [hg]; intro h; have h' : pc = EXIT_ADDR := h; exact hx (h' ▸ hex)
  · rw [hg]; exact he
  · simp [model, hp, hpt]

/-- A step of the machine the simulation takes (`MStep`), at an executable
    code address, the exit address not executable and the step not
    trapped, recorded: left to sail-riscv, or run by substitution. -/
theorem _root_.Equiv.Sim.MStep.toRun {C : BitVec 64 → Prop} {rs : List PMA_Region} {n m : Nat} {S S'' : ZiskRiscv.State model}
    {s' : RVState} (h : MStep rs n S.rv s') (hr : StepReady S.rv rs) {pc : BitVec 64}
    (hpc : S.rv.regs.get? Register.PC = some pc) (hex : ExecAt rs pc) (hc : C pc) (hx : ¬ ExecAt rs EXIT_ADDR)
    (ht : PcOk s') (hrest : MRunR C m { S with rv := s' } S'') : MRunR C (m + 1) S S'' := by
  have hg : model.getPC S.rv = pc := by simp [model, hpc]
  cases h with
  | plain pc' e hpc' hw hop hrun =>
    rw [hpc] at hpc'; cases hpc'
    exact .step (mstepR S rs hr pc hpc ⟨hex, e, hw, hop⟩ hx hrun ht) hrest
  | subst pc' w w' zr s1 hpc' hw hd hr1 hrun he =>
    rw [hpc] at hpc'; cases hpc'
    subst he
    obtain ⟨p, hp, hpt⟩ := ht
    have hp1 : s1.regs.get? Register.PC = some p := by
      rw [← hp]; simp only [mPost]; rw [setX_get_other _ _ _ _ rfl]
    refine .subst ⟨?_, ?_, w, w', zr, s1, ?_, ?_, ?_, ?_, ?_⟩ hrest
    · rw [hg]; intro h; have h' : pc = EXIT_ADDR := h; exact hx (h' ▸ hex)
    · rw [hg]; exact hc
    · rw [hg]; exact hw
    · rw [hg]; exact hd
    · rw [hg]; exact model_step hr1 hrun
    · simp [model, hp1, hpt]
    · rw [hg]

/-! ## The machine's end -/

/-- The return to the initial ra: the run ends, with code 0. -/
theorem step_return {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (hx : M.getPC S.rv = M.exitAddr) :
    ZiskRiscv.step S = .ok (S, .exit 0) := by
  have e : ZiskRiscv.tryStep S = .ok (S, .exit 0) := by simp [ZiskRiscv.tryStep, hx]; rfl
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- `ecall`'s word. -/
def ECALL : BitVec 32 := 0x00000073

/-- The exit system call: the run ends, with code a0. -/
theorem step_exit {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (hx : M.getPC S.rv ≠ M.exitAddr)
    (hw : ZiskRiscv.readWord (M.mem S.rv) (M.getPC S.rv).toNat = some ECALL) (h17 : M.getX S.rv 17 = 93) :
    ZiskRiscv.step S = .ok (S, .exit (M.getX S.rv 10)) := by
  have hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .ecall := by
    simp only [ZiskRiscv.decode, hw]; rfl
  have e : ZiskRiscv.tryStep S = .ok (S, .exit (M.getX S.rv 10)) := by
    simp [ZiskRiscv.tryStep, hx, hd, h17]; rfl
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

theorem run_go_of {k : Nat} {S T T' : ZiskRiscv.State model} {st : ZiskRiscv.Status} (h : MRun k S T)
    (he : ZiskRiscv.step T = .ok (T', st)) (hst : st ≠ .running) (fuel n : Nat) :
    ZiskRiscv.run.go (k + 1 + fuel) n S = .ok (T', st, n + k) := by
  induction h generalizing n with
  | done =>
    rw [show 0 + 1 + fuel = fuel + 1 by omega]
    simp only [ZiskRiscv.run.go, he, bind, Except.bind]
    cases st <;> simp_all <;> rfl
  | @step m _ _ _ h1 _ ih =>
    rw [show m + 1 + 1 + fuel = (m + 1 + fuel) + 1 by omega]
    simp only [ZiskRiscv.run.go, h1, bind, Except.bind]
    rw [ih he (n + 1)]; congr 3; omega

/-- A run: steps that leave it going, then one that ends it. -/
theorem run_of {k : Nat} {S T T' : ZiskRiscv.State model} {st : ZiskRiscv.Status} (h : MRun k S T)
    (he : ZiskRiscv.step T = .ok (T', st)) (hst : st ≠ .running) (fuel : Nat) :
    ZiskRiscv.run (k + 1 + fuel) S = .ok (T', st, k) := by
  have := run_go_of h he hst fuel 0
  rw [Nat.zero_add] at this
  exact this

theorem run_go_err {k : Nat} {S T : ZiskRiscv.State model} {msg : String} (h : MRun k S T)
    (he : ZiskRiscv.step T = .error msg) (fuel n : Nat) :
    ZiskRiscv.run.go (k + 1 + fuel) n S = .error msg := by
  induction h generalizing n with
  | done =>
    rw [show 0 + 1 + fuel = fuel + 1 by omega]
    simp [ZiskRiscv.run.go, he, bind, Except.bind]
  | @step m _ _ _ h1 _ ih =>
    rw [show m + 1 + 1 + fuel = (m + 1 + fuel) + 1 by omega]
    simp only [ZiskRiscv.run.go, h1, bind, Except.bind]
    exact ih he (n + 1)

/-- A run: steps that leave it going, then one that is the machine's error. -/
theorem run_err_of {k : Nat} {S T : ZiskRiscv.State model} {msg : String} (h : MRun k S T)
    (he : ZiskRiscv.step T = .error msg) (fuel : Nat) : ZiskRiscv.run (k + 1 + fuel) S = .error msg :=
  run_go_err h he fuel 0

end Equiv.Machine
