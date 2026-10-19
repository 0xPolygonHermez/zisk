/-
  The program's end.

  ZisK ends a program at `ROM_EXIT` (0x1004): `copyb` of `0`, with the
  instruction's `is_end` set, jumping to itself. The model stops there —
  `zisk_execute` returns `true` and leaves the pc. Main does not see the flag:
  `is_end` is not among the ROM's flags, so to Main the end instruction is a
  `copyb` that loops on itself, and the last segment is padded with it. What
  ends the trace is the continuation bus: a global update closes the chain at
  `(0, 1, 0, END_PC, 0, 0)`, so the last segment's exit pc is `END_PC`.

  So the run is proved in two steps. First, the trace runs the program *as
  Main sees it* (`mainView`, every `is_end` cleared) through every segment, and
  the closing pins the pc it ends at to `END_PC` (`run_all_segments`,
  `ChainCloses`). Second, the model's own run of the program follows that run
  until the first end instruction it reaches, which it executes, and stops
  (`runs_to_end`). `run_to_end`: from the boot state, the model runs the
  program to an end instruction and ends.
-/
import Proofs.Segments

open Out.Functions

namespace Correspondence

open Pil.Zisk

/-! ### The model at an end instruction -/

/-- An end instruction with the pilot's sources and an internal op: the model
executes it and stops. -/
theorem execute_end (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ())
    (hstore : inst.store = .ST_NONE) (hend : inst.is_end = true) :
    ∃ σ', zisk_execute inst σ = .ok true σ' := by
  rcases ha with ha | ha <;> rcases hb with hb | hb <;> rcases hop with hop | hop <;>
  cases hsp : inst.store_pc <;>
  · simp [zisk_execute, source_a, source_b, store_c, value_to_store, zisk_exec,
      ha, hb, hop, hstore, hend, hsp, srcA, srcB, bind, EStateM.bind, pure, EStateM.pure,
      readReg_eq, writeReg_run, hpc, hzc, hst, Std.ExtDHashMap.get?_insert]

/-! ### The program as Main sees it -/

/-- An instruction with `is_end` cleared: what Main executes, since the ROM
does not carry the flag. -/
def asMain (inst : zisk_inst) : zisk_inst := { inst with is_end := false }

/-- The program as Main sees it. -/
def mainView (prog : BitVec 64 → Option zisk_inst) : BitVec 64 → Option zisk_inst :=
  fun pc => (prog pc).map asMain

theorem asMain_of_not_end {inst : zisk_inst} (h : inst.is_end = false) : asMain inst = inst := by
  cases inst; simp only [asMain] at h ⊢; rw [h]

/-- Every instruction of `prog` is in the pilot's scope, except that it may
end the program — as an internal op, as ZisK's `ROM_EXIT` is. -/
def PilotProgEnd (prog : BitVec 64 → Option zisk_inst) : Prop :=
  ∀ pc inst, prog pc = some inst →
    Pilot (asMain inst) ∧ Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2 ∧
      (inst.is_end = true → inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ())

theorem PilotProgEnd.mainView {prog : BitVec 64 → Option zisk_inst} (h : PilotProgEnd prog) :
    PilotProg (mainView prog) := by
  intro pc inst' hp
  simp only [Correspondence.mainView, Option.map_eq_some_iff] at hp
  obtain ⟨inst, hinst, rfl⟩ := hp
  obtain ⟨h1, h2, h3, -⟩ := h pc inst hinst
  exact ⟨h1, h2, h3⟩

section Rom

variable {F : Type} [PilField F]

/-- The Rom encodes the program as Main sees it: `is_end` is not in the ROM. -/
theorem RomHolds.mainView {prog : BitVec 64 → Option zisk_inst} {tr : Rom.Trace F}
    (h : RomHolds prog tr) : RomHolds (mainView prog) tr := by
  intro j hj
  obtain ⟨pc, inst, hp, hlt, enc⟩ := h j hj
  exact ⟨pc, asMain inst, by simp [Correspondence.mainView, hp], hlt,
    { line := enc.line, a_offset_imm0 := enc.a_offset_imm0, a_imm1 := enc.a_imm1,
      b_offset_imm0 := enc.b_offset_imm0, b_imm1 := enc.b_imm1, op := enc.op,
      store_offset := enc.store_offset, jmp_offset1 := enc.jmp_offset1,
      jmp_offset2 := enc.jmp_offset2, flags := enc.flags }⟩

end Rom

/-! ### From Main's run to the model's -/

/-- The registers a step reads are present. -/
def HasRegs (σ : State) : Prop :=
  (∃ pc, σ.regs.get? Register.PC = some pc) ∧ (∃ zc, σ.regs.get? Register.Zc = some zc) ∧
    ∃ st, σ.regs.get? Register.Step = some st

theorem Ready.hasRegs {σ : State} {pc zc : BitVec 64} (h : Ready σ pc zc) : HasRegs σ :=
  ⟨⟨pc, h.1⟩, ⟨zc, h.2.1⟩, h.2.2.1⟩

/-- A step of an instruction in the pilot's scope keeps the registers. -/
theorem HasRegs.step {inst : zisk_inst} (hi : Pilot inst) {σ σ' : State} (hσ : HasRegs σ)
    (hrun : zisk_execute inst σ = .ok false σ') : HasRegs σ' := by
  obtain ⟨⟨pc, hpc⟩, ⟨zc, hzc⟩, ⟨st, hst⟩⟩ := hσ
  obtain ⟨ha, hb, hop, hstore, hset, hend⟩ := hi
  have key : ∃ σ'', zisk_execute inst σ = .ok false σ'' ∧ HasRegs σ'' := by
    rcases hop with h | h | h | h | h | h
    · obtain ⟨σ'', h1, h2, -, h4, h5⟩ :=
        execute_internal inst σ pc zc st hpc hzc hst ha hb (.inl h) hstore hset hend
      exact ⟨σ'', h1, ⟨_, h4⟩, ⟨_, h2⟩, ⟨_, h5⟩⟩
    · obtain ⟨σ'', h1, h2, -, h4, h5⟩ :=
        execute_internal inst σ pc zc st hpc hzc hst ha hb (.inr h) hstore hset hend
      exact ⟨σ'', h1, ⟨_, h4⟩, ⟨_, h2⟩, ⟨_, h5⟩⟩
    all_goals
      obtain ⟨σ'', h1, h2, -, h4, h5⟩ :=
        execute_ext inst σ pc zc st hpc hzc hst ha hb (by simp [h]) hstore hset hend
      exact ⟨σ'', h1, ⟨_, h4⟩, ⟨_, h2⟩, ⟨_, h5⟩⟩
  obtain ⟨σ'', h1, h2⟩ := key
  rw [hrun] at h1
  cases h1
  exact h2

/-- **The model follows Main's run to the end.** If Main's view of the
program runs `n` steps from `σ` to a state about to execute an end
instruction, then the model's own run of the program reaches an end
instruction within those steps, executes it, and stops. -/
theorem runs_to_end (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProgEnd prog)
    {n : Nat} {σ σ' : State} (hrun : Runs (mainView prog) n σ σ') (hσ : HasRegs σ)
    (hend : ∃ pc e, σ'.regs.get? Register.PC = some pc ∧ prog pc = some e ∧ e.is_end = true) :
    ∃ m σm pce e σe, m ≤ n ∧ Runs prog m σ σm ∧ σm.regs.get? Register.PC = some pce ∧
      prog pce = some e ∧ e.is_end = true ∧ zisk_execute e σm = .ok true σe := by
  induction hrun with
  | refl σ =>
    obtain ⟨pc, e, hpc, he, hie⟩ := hend
    obtain ⟨-, ⟨zc, hzc⟩, ⟨st, hst⟩⟩ := hσ
    obtain ⟨⟨ha, hb, hop, hstore, -, -⟩, -, -, hend'⟩ := hprog pc e he
    obtain ⟨σe, hex⟩ := execute_end e σ pc zc st hpc hzc hst ha hb (hend' hie) hstore hie
    exact ⟨0, σ, pc, e, σe, Nat.le_refl 0, .refl σ, hpc, he, hie, hex⟩
  | @step n σ₀ σ₁ σ₂ pc inst' hpc hinst hrun' _ ih =>
    simp only [mainView, Option.map_eq_some_iff] at hinst
    obtain ⟨inst, hp, rfl⟩ := hinst
    obtain ⟨hpil, -, -, hend'⟩ := hprog pc inst hp
    cases hie : inst.is_end
    · rw [asMain_of_not_end hie] at hrun' hpil
      obtain ⟨m, σm, pce, e, σe, hm, hruns, h1, h2, h3, h4⟩ := ih (hσ.step hpil hrun') hend
      exact ⟨m + 1, σm, pce, e, σe, by omega, .step hpc hp hrun' hruns, h1, h2, h3, h4⟩
    · obtain ⟨-, ⟨zc, hzc⟩, ⟨st, hst⟩⟩ := hσ
      obtain ⟨ha, hb, -, hstore, -, -⟩ := hpil
      obtain ⟨σe, hex⟩ := execute_end inst σ₀ pc zc st hpc hzc hst ha hb (hend' hie) hstore hie
      exact ⟨0, σ₀, pc, inst, σe, Nat.zero_le _, .refl σ₀, hpc, hp, hie, hex⟩

/-! ### The end of the chain -/

section Chain

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]
variable {xr : Rom.Ctx F} {tr : Rom.Trace F} {ps : List (Prover F)}

/-- The tuple the global update closes the chain with: segment 0, the last,
step 0, the end pc, `c = 0`. -/
def endTuple (fin : BitVec 64) : List F := [0, 1, 0, ((fin.toNat : Nat) : F), 0, 0]

/-- What a sound continuation bus guarantees with the global update that
closes it: the last segment proves the end tuple. -/
def ChainCloses (fin : BitVec 64) (segs : List (Segment xr tr ps)) : Prop :=
  ∀ (h : 0 < segs.length) (m : Nat),
    tupleAt Main.bus715.tuple (segs[segs.length - 1]'(by omega)).x
      (segs[segs.length - 1]'(by omega)).t 0 m = (endTuple fin).getD m 0

/-- **Every segment.** Along the chain, all its segments force
`len·4·numRows` steps of the model from the boot state, and the last
segment's exit airvals encode the state they reach. -/
theorem run_all_segments (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    (boot : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (hlen : 0 < segs.length) (σ₀ : State) (h₀ : Ready σ₀ boot 0) :
    ∃ σ pc zc, Runs prog (segs.length * segSteps) σ₀ σ ∧ Ready σ pc zc ∧
      (segs[segs.length - 1]'(by omega)).Exit pc zc := by
  obtain ⟨σ, pc, zc, hruns, hσ, hin⟩ :=
    run_segments prog hprog hr hrom hpr boot segs hc σ₀ h₀ (segs.length - 1) (by omega)
  obtain ⟨σ', pc', zc', hrun, hσ', hex⟩ :=
    (segs[segs.length - 1]'(by omega)).run prog hprog hr hrom hpr σ pc zc hσ (hin (by omega))
  refine ⟨σ', pc', zc', ?_, hσ', hex⟩
  have := hruns.append hrun
  rwa [← Nat.succ_mul, Nat.succ_eq_add_one, Nat.sub_add_cancel hlen] at this

/-- **A run from boot to the end.** Along a chain of segments from the boot
state that the global update closes at `fin`, where the program has its end
instruction, the model runs the program from the boot state to an end
instruction — within the steps the segments hold — executes it, and stops.

What remains assumed: the soundness of every bus involved (the continuation
bus, opened and closed, as `ChainHolds` and `ChainCloses`; the ROM and
operation-bus lookups of every segment), the contents of the Rom and of
`__L1__`, that every prover on the operation bus computes each op
(`OpsProved`, which `System.ok` gives), and the pilot's scope, now with end
instructions. -/
theorem run_to_end (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProgEnd prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    (boot fin : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (hcl : ChainCloses fin segs) (hlen : 0 < segs.length) (hfin : fin.toNat < 2 ^ 32)
    (hfe : ∃ e, prog fin = some e ∧ e.is_end = true) (σ₀ : State) (h₀ : Ready σ₀ boot 0) :
    ∃ m σ pc e σe, m ≤ segs.length * segSteps ∧ Runs prog m σ₀ σ ∧
      σ.regs.get? Register.PC = some pc ∧ prog pc = some e ∧ e.is_end = true ∧
      zisk_execute e σ = .ok true σe := by
  obtain ⟨σ, pc, zc, hruns, hσ, hex⟩ :=
    run_all_segments (mainView prog) hprog.mainView hr hrom.mainView hpr boot segs hc hlen σ₀ h₀
  -- The closing: the last exit pc is `fin`.
  have e3 := hcl hlen 3
  simp only [tupleAt, Main.bus715, endTuple, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at e3
  have hpcfin : pc = fin := by
    have h' : ((pc.toNat : Nat) : F) = ((fin.toNat : Nat) : F) := hex.1.symm.trans e3
    have := natCast_inj (F := F) (by have := hσ.2.2.2; unfold goldilocks; omega)
      (by unfold goldilocks; omega) h'
    exact BitVec.eq_of_toNat_eq this
  obtain ⟨e, he, hie⟩ := hfe
  exact runs_to_end prog hprog hruns h₀.hasRegs ⟨pc, e, hσ.1, hpcfin ▸ he, hie⟩

/-- **A run from boot to the end, over ZisK's provers.** `run_to_end` with the
operation bus answered by a system's provers (`Provers.lean`). -/
theorem run_to_end_system (S : System F) (hT : Tables S.xv S.tv)
    (hF : ∀ p ∈ frops S.x0 S.t0, FropsOk p)
    (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProgEnd prog)
    {xr : Rom.Ctx F} {tr : Rom.Trace F} (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    (boot fin : BitVec 64) (segs : List (Segment xr tr S.provers)) (hc : ChainHolds boot segs)
    (hcl : ChainCloses fin segs) (hlen : 0 < segs.length) (hfin : fin.toNat < 2 ^ 32)
    (hfe : ∃ e, prog fin = some e ∧ e.is_end = true) (σ₀ : State) (h₀ : Ready σ₀ boot 0) :
    ∃ m σ pc e σe, m ≤ segs.length * segSteps ∧ Runs prog m σ₀ σ ∧
      σ.regs.get? Register.PC = some pc ∧ prog pc = some e ∧ e.is_end = true ∧
      zisk_execute e σ = .ok true σe :=
  run_to_end prog hprog hr hrom (S.ok hT hF) boot fin segs hc hcl hlen hfin hfe σ₀ h₀

end Chain

end Correspondence
