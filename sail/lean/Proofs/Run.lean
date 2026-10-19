/-
  Many steps: Main's trace runs the model.

  `Slot.lean` proves one lane against one step, given that the lane reads the
  model's `c` and sits at the model's pc. This file discharges those two
  hypotheses for the step after it, so steps chain: a position in Main (a row
  and a lane) *encodes* a state when its pc column is the state's pc and the
  `c` it reads is the state's `c`; one step from an encoded state leaves the
  next position encoding the model's next state (`step_internal`); and by
  induction, from an encoded start the trace forces `n` steps of the model,
  ending encoded `n` positions on (`run_internal`).

  The model's run is a conclusion, not a hypothesis. The ROM lookup at each
  position finds the program's instruction at the encoded pc, so the trace
  determines which instruction the model executes, and the step's constraints
  that it executes it as `zisk_execute` does.

  Two runs are proved. `run_internal` covers programs of `copyb` and `flag`,
  which Main computes itself, so it needs only the ROM lookup. `run_pilot`
  covers all six modeled opcodes, the external `add`, `ltu`, `eq` and `and`
  through the operation bus, and needs, besides, the bus's guarantee at every
  lane and that every prover it can answer from computes each op
  (`OpsProved`). Both share one induction, `run_of_step`.

  Scope: immediate or `c` sources, no store, no `set_pc`, jumps that stay in
  the 32-bit pc range, within one segment: a run does not cross from the last
  row of a segment to the first row of the next, which goes through the
  continuation bus and its airvals.
-/
import Proofs.ExtPilot

open Out.Functions

namespace Correspondence

open Pil.Zisk

/-! ### Positions -/

/-- A step's place in Main: a row, and one of its four lanes. -/
structure Pos where
  row : Int
  lane : Nat
  lt : lane < 4

/-- The step after: the next lane, or lane 0 of the next row. -/
def Pos.next (s : Pos) : Pos :=
  if h : s.lane = 3 then ⟨s.row + 1, 0, by omega⟩ else ⟨s.row, s.lane + 1, by have := s.lt; omega⟩

/-- The position `k` steps after `s`. -/
def Pos.after (s : Pos) : Nat → Pos
  | 0 => s
  | k + 1 => s.next.after k

section Encoding

variable {F : Type} [PilField F]

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- The `c` the step at `s` reads: the lane before it, or for lane 0 the
`previous_c` Main computes, from the previous row or the previous segment. -/
def prevC (x : Main.Ctx F) (t : Main.Trace F) (s : Pos) (k : Nat) : F :=
  if s.lane = 0 then
    (if k = 0 then Main.Main_previous_c_0_0 x t s.row else Main.Main_previous_c_0_1 x t s.row)
  else (t s.row).c (s.lane - 1) k

/-- Position `s` is about to run from a state with pc `pc` and `c` `zc`. -/
def Encodes (x : Main.Ctx F) (t : Main.Trace F) (s : Pos) (pc zc : BitVec 64) : Prop :=
  (t s.row).pc s.lane = ((pc.toNat : Nat) : F) ∧ prevC x t s 0 = lo zc ∧ prevC x t s 1 = hi zc

/-- The step at `s` and the next one are in the same segment: if `s` is the
last lane of its row, the next row is not the first of a segment. -/
def InSegment (t : Main.Trace F) (s : Pos) : Prop :=
  s.lane = 3 → (t (s.row + 1)).__L1__ = 0

/-- The constraints of the step at `s`, with its neighbours. -/
theorem laneHolds_at {x : Main.Ctx F} {t : Main.Trace F} (hm : ∀ i, Main.everyRow x t i)
    {s : Pos} (hs : InSegment t s) :
    LaneHolds (t s.row) s.lane (prevC x t s 0) (prevC x t s 1) ((t s.next.row).pc s.next.lane) := by
  obtain ⟨i, l, hl⟩ := s
  rcases l with _ | _ | _ | _ | l
  · exact laneHolds_0 (hm i)
  · exact laneHolds_1 (hm i)
  · exact laneHolds_2 (hm i)
  · exact laneHolds_3 (hm i) (hm (i + 1)) (hs rfl)
  · omega

/-- The step after `s` reads the `c` that `s` leaves. -/
theorem prevC_next {x : Main.Ctx F} {t : Main.Trace F} {s : Pos} (hs : InSegment t s) (k : Nat)
    (hk : k = 0 ∨ k = 1) : prevC x t s.next k = (t s.row).c s.lane k := by
  obtain ⟨i, l, hl⟩ := s
  rcases l with _ | _ | _ | _ | l
  · rfl
  · rfl
  · rfl
  · have := previous_c_0 (x := x) (t := t) (i := i + 1) (hs rfl)
    simp only [Int.add_sub_cancel] at this
    rcases hk with rfl | rfl
    · exact this.1
    · exact this.2
  · omega

end Encoding

/-! ### The pilot's scope, over a whole program -/

/-- An instruction the pilot covers: `copyb` or `flag`, immediate or `c`
sources, no store, no `set_pc`, not the last. -/
structure Internal (inst : zisk_inst) : Prop where
  a : inst.a_src = .A_IMM ∨ inst.a_src = .A_C
  b : inst.b_src = .B_IMM ∨ inst.b_src = .B_C
  op : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ()
  store : inst.store = .ST_NONE
  set_pc : inst.set_pc = false
  is_end : inst.is_end = false

/-- A jump from `pc` by `j` lands in the 32-bit pc range. -/
def Lands (pc j : BitVec 64) : Prop :=
  0 ≤ (pc.toNat : Int) + j.toInt ∧ (pc.toNat : Int) + j.toInt < 2 ^ 32

theorem Lands.noWrap {pc j : BitVec 64} (h : Lands pc j) : NoWrap pc j := by
  unfold Lands at h; unfold NoWrap; omega

theorem Lands.lt {pc j : BitVec 64} (h : Lands pc j) : (pc + j).toNat < 2 ^ 32 := by
  have hz : (((pc + j).toNat : Nat) : Int) = (pc.toNat : Int) + j.toInt := by
    rw [BitVec.toNat_add]
    have := BitVec.toInt_eq_toNat_cond j
    have := j.isLt
    have := pc.isLt
    unfold Lands at h
    split at * <;> omega
  unfold Lands at h
  omega

/-- Every instruction of `prog` is one the pilot covers, with both jumps
landing in the 32-bit pc range. -/
def InternalProg (prog : BitVec 64 → Option zisk_inst) : Prop :=
  ∀ pc inst, prog pc = some inst → Internal inst ∧ Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2

/-- The model runs `n` steps of `prog` from `σ` to `σ'`, each executing the
program's instruction at the current pc. -/
inductive Runs (prog : BitVec 64 → Option zisk_inst) : Nat → State → State → Prop
  | refl (σ : State) : Runs prog 0 σ σ
  | step {n : Nat} {σ σ' σ'' : State} {pc : BitVec 64} {inst : zisk_inst} :
      σ.regs.get? Register.PC = some pc → prog pc = some inst →
      zisk_execute inst σ = .ok false σ' → Runs prog n σ' σ'' → Runs prog (n + 1) σ σ''

/-- What a state needs for the next step: a pc in the 32-bit range, a `c`
and a step counter. -/
def Ready (σ : State) (pc zc : BitVec 64) : Prop :=
  σ.regs.get? Register.PC = some pc ∧ σ.regs.get? Register.Zc = some zc ∧
    (∃ st, σ.regs.get? Register.Step = some st) ∧ pc.toNat < 2 ^ 32

/-! ### Steps -/

section Run

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- **One step, at any position.** If position `s` encodes a ready state, the
step there executes the program's instruction at its pc — which the ROM lookup
finds — and the next position encodes the state `zisk_execute` leaves. -/
theorem step_internal (prog : BitVec 64 → Option zisk_inst) (hprog : InternalProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) (hm : ∀ i, Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : ∀ l, l < 4 → LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr)
    (s : Pos) (hs : InSegment tm s) (σ : State) (pc zc : BitVec 64) (hσ : Ready σ pc zc)
    (henc : Encodes xm tm s pc zc) :
    ∃ inst σ' pc' zc', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' zc' ∧ Encodes xm tm s.next pc' zc' := by
  obtain ⟨hpc, hzc, ⟨st, hst⟩, hpc32⟩ := hσ
  obtain ⟨hcol, hp0, hp1⟩ := henc
  obtain ⟨inst, hinst, hslot⟩ :=
    rom_lane prog xm tm s.row (hm s.row) xr tr hr s.lt (hbus s.lane s.lt) hrom pc hpc32 hcol
  obtain ⟨⟨ha, hb, hop, hstore, hset, hend⟩, hl1, hl2⟩ := hprog pc inst hinst
  obtain ⟨σ', c, f, pc', hrun, hc, -, hp, hst', hnext, hc0, hc1, -, hnp⟩ :=
    lane_internal (laneHolds_at hm hs) inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend
      hslot ⟨hp0, hp1⟩ ⟨hl1.noWrap, hl2.noWrap⟩
  refine ⟨inst, σ', pc', c, hinst, hrun, ⟨hp, hc, ⟨_, hst'⟩, ?_⟩, hnp, ?_, ?_⟩
  · rcases hnext with rfl | rfl
    · exact hl1.lt
    · exact hl2.lt
  · rw [prevC_next hs 0 (.inl rfl)]; exact hc0
  · rw [prevC_next hs 1 (.inr rfl)]; exact hc1

/-- **Steps chain.** Any step lemma — from a position `s` where `ok s` and
whose encoding `E` a ready state satisfies, the program's instruction at the
pc runs and `s.next` encodes the result — gives runs of any length. -/
theorem run_of_step (prog : BitVec 64 → Option zisk_inst) (E : Pos → BitVec 64 → BitVec 64 → Prop)
    (ok : Pos → Prop)
    (hstep : ∀ s σ pc zc, ok s → Ready σ pc zc → E s pc zc →
      ∃ inst σ' pc' zc', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
        Ready σ' pc' zc' ∧ E s.next pc' zc')
    (n : Nat) :
    ∀ (s : Pos) (σ : State) (pc zc : BitVec 64), Ready σ pc zc → E s pc zc →
      (∀ k, k < n → ok (s.after k)) →
      ∃ σ' pc' zc', Runs prog n σ σ' ∧ Ready σ' pc' zc' ∧ E (s.after n) pc' zc' := by
  induction n with
  | zero => exact fun s σ pc zc hσ henc _ => ⟨σ, pc, zc, .refl σ, hσ, henc⟩
  | succ n ih =>
    intro s σ pc zc hσ henc hseg
    obtain ⟨inst, σ₁, pc₁, zc₁, hinst, hrun, hσ₁, henc₁⟩ :=
      hstep s σ pc zc (hseg 0 (by omega)) hσ henc
    obtain ⟨σ', pc', zc', hruns, hσ', henc'⟩ :=
      ih s.next σ₁ pc₁ zc₁ hσ₁ henc₁ (fun k hk => hseg (k + 1) (by omega))
    exact ⟨σ', pc', zc', .step hσ.1 hinst hrun hruns, hσ', henc'⟩

/-- **A run.** If position `s` encodes a ready state, and the next `n`
positions are in one segment, the model runs `n` steps of the program from it,
and the position `n` steps on encodes the state it reaches.

What remains assumed is what one step assumes — the ROM lookup's soundness
(`LookupHolds`), the Rom's contents (`RomHolds`) and the pilot's scope — and
the start: that `s` encodes the state the run begins from. -/
theorem run_internal (prog : BitVec 64 → Option zisk_inst) (hprog : InternalProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) (hm : ∀ i, Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : ∀ l, l < 4 → LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr) (n : Nat) :
    ∀ (s : Pos) (σ : State) (pc zc : BitVec 64), Ready σ pc zc → Encodes xm tm s pc zc →
      (∀ k, k < n → InSegment tm (s.after k)) →
      ∃ σ' pc' zc', Runs prog n σ σ' ∧ Ready σ' pc' zc' ∧ Encodes xm tm (s.after n) pc' zc' :=
  run_of_step prog (Encodes xm tm) (InSegment tm)
    (fun s σ pc zc hs hσ henc =>
      step_internal prog hprog xm tm hm xr tr hr hbus hrom s hs σ pc zc hσ henc) n

/-! ### All six opcodes -/

/-- A jump offset of at most 31 bits, either way: what an indirect jump adds
to a 32-bit `c`. -/
def ShortJump (j : BitVec 64) : Prop := -2 ^ 31 ≤ j.toInt ∧ j.toInt < 2 ^ 31

/-- An instruction the pilot covers, of any of the six modeled opcodes. It may
set the pc — an indirect jump — if its op is internal and its `jmp_off1` short. -/
structure Pilot (inst : zisk_inst) : Prop where
  a : inst.a_src = .A_IMM ∨ inst.a_src = .A_C
  b : inst.b_src = .B_IMM ∨ inst.b_src = .B_C
  op : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG () ∨ inst.op = .ZOP_ADD () ∨
    inst.op = .ZOP_LTU () ∨ inst.op = .ZOP_EQ () ∨ inst.op = .ZOP_AND ()
  store : inst.store = .ST_NONE
  set_pc : inst.set_pc = true →
    (inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ()) ∧ ShortJump inst.jmp_off1
  is_end : inst.is_end = false

/-- Every instruction of `prog` is one the pilot covers, with both jumps
landing in the 32-bit pc range. -/
def PilotProg (prog : BitVec 64 → Option zisk_inst) : Prop :=
  ∀ pc inst, prog pc = some inst → Pilot inst ∧ Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2

/-- Prover `p` computes each modeled external op on the operation bus. -/
structure OpsOk (p : Prover F) : Prop where
  add : OpProves 10 addSem p
  ltu : OpProves 6 ltuSem p
  eq : OpProves 9 eqSem p
  and : OpProves 14 andSem p

/-- Every prover in `ps` does: what `Adders.lean`, `Binary.lean`,
`BinaryOps.lean`, `OpBus.lean`, `OpBusCmp.lean` and the checked FROPS tables
give, prover by prover, and `Provers.lean` collects. -/
def OpsProved (ps : List (Prover F)) : Prop :=
  ∀ p ∈ ps, OpsOk p

/-- **One lane, one indirect jump.** A lane holding an internal op with
`set_pc` jumps to `c + jmp_off1`. Main computes that target as
`c₀ + jmp_offset1`, with `c₁ = 0` (`main.pil:489`) and no flag
(`main.pil:478`); if the next pc column is a 32-bit `q` — the next step's ROM
lookup gives one — the target is `q` exactly, as the model's 64-bit addition
gives it: both sides are below the characteristic, so the field equation is an
equation between integers. -/
theorem lane_setpc {r : Main.Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = true) (hend : inst.is_end = false)
    (hslot : SlotHolds r l inst pc) (hprev : p0 = lo zc ∧ p1 = hi zc)
    (hj : ShortJump inst.jmp_off1) (q : BitVec 64) (hq : q.toNat < 2 ^ 32)
    (hnp : np = ((q.toNat : Nat) : F)) :
    ∃ σ' c, zisk_execute inst σ = .ok false σ' ∧ σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.PC = some q ∧
      σ'.regs.get? Register.Step = some (Sail.BitVec.addInt st 1) ∧
      r.c l 0 = lo c ∧ r.c l 1 = hi c := by
  obtain ⟨σ', hrun, hc, -, hp, hs⟩ :=
    execute_setpc inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend
  obtain ⟨c0, c1, cf⟩ := lane_cf hl inst pc zc ha hb hop hslot hprev
  refine ⟨σ', _, hrun, hc, ?_, hs, c0, c1⟩
  rw [hp]
  congr 1
  generalize hR : (zisk_exec inst.op (srcA inst zc) (srcB inst zc)) = R at c0 c1 cf ⊢
  -- The lane's set_pc is 1, so its flag is 0 and its `c₁` is 0.
  have s14 := hslot.set_pc
  have s15 := hslot.jmp_offset1
  rw [hset] at s14
  simp only [bit, if_true] at s14
  have hfs := hl.fs
  have hsc := hl.sc1
  have hn := hl.npc
  rw [s14] at hfs hsc hn
  have hf0 : r.flag l = 0 := by grind
  have hc1 : r.c l 1 = 0 := by grind
  -- `c` is below 2^32.
  have hhi : R.1.toNat / 2 ^ 32 = 0 := by
    have h' : ((R.1.toNat / 2 ^ 32 : Nat) : F) = ((0 : Nat) : F) := by
      rw [Lean.Grind.Semiring.natCast_zero, ← hc1, c1, hi]
    exact natCast_inj (F := F) (by have := R.1.isLt; unfold goldilocks; omega)
      (by unfold goldilocks; omega) h'
  -- The target, in the field: `q = c₀ + jmp_offset1`.
  have hfield : ((q.toNat : Nat) : F) = ((R.1.toNat % 2 ^ 32 : Nat) : F) + off inst.jmp_off1 := by
    rw [← hnp, hn, hf0, ← s15, c0, lo]; grind
  have hint : (((q.toNat : Nat) : Int) : F) =
      ((((R.1.toNat % 2 ^ 32 : Nat) : Int) + inst.jmp_off1.toInt : Int) : F) := by
    rw [Lean.Grind.Ring.intCast_natCast, Lean.Grind.Ring.intCast_add,
      Lean.Grind.Ring.intCast_natCast, hfield, off]
  have hmod := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp hint
  obtain ⟨hj0, hj1⟩ := hj
  have hR1 := R.1.isLt
  have heq : ((q.toNat : Nat) : Int) = ((R.1.toNat % 2 ^ 32 : Nat) : Int) + inst.jmp_off1.toInt := by
    unfold goldilocks at hmod; omega
  -- So the 64-bit sum is `q`.
  apply BitVec.eq_of_toNat_eq
  have hz : (((R.1 + inst.jmp_off1).toNat : Nat) : Int) = (R.1.toNat : Int) + inst.jmp_off1.toInt := by
    rw [BitVec.toNat_add]
    have := BitVec.toInt_eq_toNat_cond inst.jmp_off1
    have := inst.jmp_off1.isLt
    split at * <;> omega
  omega

/-- **One step of any of the six opcodes, at one lane.** Lane `l` of row `i`,
with its constraints `LaneHolds` for whatever neighbours it has, reads the
model's `c` and sits at the model's pc: the step there executes the program's
instruction at that pc — which the ROM lookup finds — leaving the lane's `c`
and the next pc `np` as `zisk_execute` does. An external op's result comes
over the operation bus, from whichever prover answers the lane's lookup. -/
theorem step_core (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) {i : Int} (hmi : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    {l : Nat} (hl4 : l < 4) (hbus : LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds (opArg l) xm tm ps) (hpr : OpsProved ps)
    {p0 p1 np : F} (hlane : LaneHolds (tm i) l p0 p1 np)
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ np = ((q.toNat : Nat) : F))
    (σ : State) (pc zc : BitVec 64) (hσ : Ready σ pc zc)
    (hcol : (tm i).pc l = ((pc.toNat : Nat) : F)) (hprev : p0 = lo zc ∧ p1 = hi zc) :
    ∃ inst σ' pc' c, prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' c ∧ np = ((pc'.toNat : Nat) : F) ∧
      (tm i).c l 0 = lo c ∧ (tm i).c l 1 = hi c := by
  obtain ⟨hpc, hzc, ⟨st, hst⟩, hpc32⟩ := hσ
  obtain ⟨inst, hinst, hslot⟩ :=
    rom_lane prog xm tm i hmi xr tr hr hl4 hbus hrom pc hpc32 hcol
  obtain ⟨⟨ha, hb, hop, hstore, hset', hend⟩, hl1, hl2⟩ := hprog pc inst hinst
  cases hset : inst.set_pc
  case true =>
    -- An indirect jump: the next pc is the 32-bit `q` the next pc column holds.
    obtain ⟨hint, hj⟩ := hset' hset
    obtain ⟨q, hq32, hnq⟩ := hq
    obtain ⟨σ', c, hrun, hc, hp, hst', hc0, hc1⟩ :=
      lane_setpc hlane inst σ pc zc st hpc hzc hst ha hb hint hstore hset hend hslot hprev hj
        q hq32 hnq
    exact ⟨inst, σ', q, c, hinst, hrun, ⟨hp, hc, ⟨_, hst'⟩, hq32⟩, hnq, hc0, hc1⟩
  have hw : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2 := ⟨hl1.noWrap, hl2.noWrap⟩
  have key : ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      σ'.regs.get? Register.Step = some (Sail.BitVec.addInt st 1) ∧
      (pc' = pc + inst.jmp_off1 ∨ pc' = pc + inst.jmp_off2) ∧
      (tm i).c l 0 = lo c ∧ (tm i).c l 1 = hi c ∧ (tm i).flag l = bit flag ∧
      np = ((pc'.toNat : Nat) : F) := by
    have hext := fun (code : Nat) sem h4 (hpc' : ∀ p ∈ ps, OpProves code sem p) hc hsm =>
      lane_external code sem hl4 hlane ps hops hpc' inst σ pc zc st hpc hzc hst
        hslot ha hb h4 hc hsm hstore hset hend hprev hw
    rcases hop with h | h | h | h | h | h
    · exact lane_internal hlane inst σ pc zc st hpc hzc hst ha hb (.inl h) hstore hset hend
        hslot hprev hw
    · exact lane_internal hlane inst σ pc zc st hpc hzc hst ha hb (.inr h) hstore hset hend
        hslot hprev hw
    · exact hext 10 addSem (.inl h) (fun p hp => (hpr p hp).add) (by rw [h]; rfl)
        (by rw [h]; exact exec_add)
    · exact hext 6 ltuSem (.inr (.inl h)) (fun p hp => (hpr p hp).ltu) (by rw [h]; rfl)
        (by rw [h]; exact exec_ltu)
    · exact hext 9 eqSem (.inr (.inr (.inl h))) (fun p hp => (hpr p hp).eq) (by rw [h]; rfl)
        (by rw [h]; exact exec_eq)
    · exact hext 14 andSem (.inr (.inr (.inr h))) (fun p hp => (hpr p hp).and)
        (by rw [h]; rfl) (by rw [h]; exact exec_and)
  obtain ⟨σ', c, f, pc', hrun, hc, -, hp, hst', hnext, hc0, hc1, -, hnp⟩ := key
  refine ⟨inst, σ', pc', c, hinst, hrun, ⟨hp, hc, ⟨_, hst'⟩, ?_⟩, hnp, hc0, hc1⟩
  rcases hnext with rfl | rfl
  · exact hl1.lt
  · exact hl2.lt

/-- **One step of any of the six opcodes, at any position** inside a segment:
`step_core` with the position's neighbours. -/
theorem step_pilot (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) (hm : ∀ i, Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : ∀ l, l < 4 → LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr)
    (ps : List (Prover F)) (hops : ∀ l, l < 4 → LookupHolds (opArg l) xm tm ps)
    (hpr : OpsProved ps)
    (s : Pos) (hs : InSegment tm s) (σ : State) (pc zc : BitVec 64) (hσ : Ready σ pc zc)
    (henc : Encodes xm tm s pc zc) :
    ∃ inst σ' pc' zc', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' zc' ∧ Encodes xm tm s.next pc' zc' := by
  obtain ⟨hcol, hp0, hp1⟩ := henc
  obtain ⟨inst, σ', pc', c, hinst, hrun, hσ', hnp, hc0, hc1⟩ :=
    step_core prog hprog xm tm (hm s.row) xr tr hr s.lt (hbus s.lane s.lt) hrom ps
      (hops s.lane s.lt) hpr (laneHolds_at hm hs)
      (rom_pc32 prog xm tm s.next.row xr tr hr s.next.lt (hbus s.next.lane s.next.lt) hrom)
      σ pc zc hσ hcol ⟨hp0, hp1⟩
  refine ⟨inst, σ', pc', c, hinst, hrun, hσ', hnp, ?_, ?_⟩
  · rw [prevC_next hs 0 (.inl rfl)]; exact hc0
  · rw [prevC_next hs 1 (.inr rfl)]; exact hc1

/-- **A run of the pilot's programs.** As `run_internal`, for programs of all
six modeled opcodes: from an encoded start, the trace forces `n` steps of the
model within a segment, each external op's result taken from the operation
bus.

What remains assumed: the soundness of the ROM lookup and the operation bus
(`LookupHolds`, at every lane), the Rom's contents (`RomHolds`), that every
prover on the operation bus computes each op (`OpsProved`, which the prover
files give one by one), the pilot's scope, and the start. -/
theorem run_pilot (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) (hm : ∀ i, Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : ∀ l, l < 4 → LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr)
    (ps : List (Prover F)) (hops : ∀ l, l < 4 → LookupHolds (opArg l) xm tm ps)
    (hpr : OpsProved ps) (n : Nat) :
    ∀ (s : Pos) (σ : State) (pc zc : BitVec 64), Ready σ pc zc → Encodes xm tm s pc zc →
      (∀ k, k < n → InSegment tm (s.after k)) →
      ∃ σ' pc' zc', Runs prog n σ σ' ∧ Ready σ' pc' zc' ∧ Encodes xm tm (s.after n) pc' zc' :=
  run_of_step prog (Encodes xm tm) (InSegment tm)
    (fun s σ pc zc hs hσ henc =>
      step_pilot prog hprog xm tm hm xr tr hr hbus hrom ps hops hpr s hs σ pc zc hσ henc) n

end Run

end Correspondence
