/-
  Across segments: a run from the boot state.

  `Run.lean` proves runs inside one Main segment, from a position assumed to
  encode the model's state. A ZisK execution is a chain of segments, each a
  Main instance of `numRows` rows, four steps a row, linked by the
  continuation bus (1000): a segment *assumes* the state it starts from —
  `main_segment`, the step, `segment_initial_pc` and `segment_previous_c` —
  and *proves* the state it hands on — the next segment's id, the step,
  `segment_next_pc` and `segment_last_c` (`Main.bus714`, `Main.bus715`). Two
  global updates open the chain at the boot state, `(0, 0, 0, BOOT_ADDR, 0, 0)`,
  and close it at the end.

  Inside a segment, the airvals are tied to the trace by constraints on its
  first and last rows (`__L1__`, the fixed column that is 1 on row 0 only):

  * `segment_initial_pc` is row 0's pc (`main.pil:661`), and lane 0 of row 0
    reads `segment_previous_c` as the previous step's `c`
    (`previous_c_0_first`) — so a segment's entry airvals encode position
    `(0, 0)` (`entry_encodes`);
  * lane 3 of the last row hands its pc to `segment_next_pc`
    (`laneHolds_3_last`) and its `c` to `segment_last_c` (`main.pil:532`) — so
    the segment's last step leaves its exit airvals encoding the state it
    reaches (`exit_step`).

  `segment_run`: from its entry airvals, a segment forces `4·numRows` steps of
  the model and ends with its exit airvals encoding the state they reach.
  `run_from_boot`: along a chain of segments from the boot state, the trace
  forces every step of the model, and each step's position encodes the state
  before it — the run's start is no longer assumed.

  What the continuation bus guarantees is assumed, as the other buses' are,
  and stated over the generated arguments (`ChainHolds`): the segments, in
  order, each assume what the one before proved, the first the boot tuple. The
  global updates are not extracted, so the boot tuple is written here.
-/
import Proofs.Provers

open Out.Functions

namespace Correspondence

open Pil.Zisk

/-! ### Positions, counted -/

/-- Two positions with the same row and lane are the same. -/
theorem Pos.ext' {s s' : Pos} (hr : s.row = s'.row) (hl : s.lane = s'.lane) : s = s' := by
  cases s; cases s'; simp only at hr hl; subst hr hl; rfl

/-- `k` steps on from `s`: four lanes a row. -/
theorem Pos.after_eq (s : Pos) (k : Nat) :
    (s.after k).row = s.row + (((s.lane + k) / 4 : Nat) : Int) ∧
      (s.after k).lane = (s.lane + k) % 4 := by
  induction k generalizing s with
  | zero => have := s.lt; simp only [Pos.after, Nat.add_zero]; omega
  | succ k ih =>
    obtain ⟨h1, h2⟩ := ih s.next
    show (s.next.after k).row = _ ∧ (s.next.after k).lane = _
    rw [h1, h2]
    have := s.lt
    unfold Pos.next
    split <;> (simp only; omega)

/-- The first step of a segment. -/
def Pos.first : Pos := ⟨0, 0, by omega⟩

/-- The last step of a segment: lane 3 of its last row. -/
def Pos.last : Pos := ⟨(Main.numRows : Int) - 1, 3, by omega⟩

/-- The steps of one segment. -/
abbrev segSteps : Nat := 4 * Main.numRows

theorem first_after_row (k : Nat) : (Pos.first.after k).row = ((k / 4 : Nat) : Int) := by
  rw [(Pos.after_eq Pos.first k).1]
  show (0 : Int) + (((0 + k) / 4 : Nat) : Int) = _
  rw [Nat.zero_add, Int.zero_add]

theorem first_after_lane (k : Nat) : (Pos.first.after k).lane = k % 4 := by
  rw [(Pos.after_eq Pos.first k).2]
  show (0 + k) % 4 = _
  rw [Nat.zero_add]

theorem first_after_last : Pos.first.after (segSteps - 1) = Pos.last := by
  apply Pos.ext'
  · rw [first_after_row]; simp only [Pos.last, segSteps, Main.numRows]; omega
  · rw [first_after_lane]; simp only [Pos.last, segSteps, Main.numRows]

/-! ### A segment -/

section Segment

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- One Main segment: its trace, its constraints at every row, `__L1__` as the
setup fixes it, and the soundness of the ROM and operation-bus lookups at
every lane, the latter answered by `ps`. -/
structure Segment (xr : Rom.Ctx F) (tr : Rom.Trace F) (ps : List (Prover F)) where
  x : Main.Ctx F
  t : Main.Trace F
  rows : ∀ i, Main.everyRow x t i
  cyclic : Main.cyclic t
  first : (t 0).__L1__ = 1
  inner : ∀ i : Int, 0 < i → i < Main.numRows → (t i).__L1__ = 0
  rom : ∀ l, l < 4 → LookupHolds (romArg l) x t [⟨Rom.bus0, xr, tr⟩]
  ops : ∀ l, l < 4 → LookupHolds (opArg l) x t ps

variable {xr : Rom.Ctx F} {tr : Rom.Trace F} {ps : List (Prover F)}

/-- The segment starts from pc `pc` and `c` `zc`: its entry airvals. -/
def Segment.Entry (G : Segment xr tr ps) (pc zc : BitVec 64) : Prop :=
  G.x.Main_segment_initial_pc = ((pc.toNat : Nat) : F) ∧
    G.x.Main_segment_previous_c 0 = lo zc ∧ G.x.Main_segment_previous_c 1 = hi zc

/-- The segment hands on pc `pc` and `c` `zc`: its exit airvals. -/
def Segment.Exit (G : Segment xr tr ps) (pc zc : BitVec 64) : Prop :=
  G.x.Main_segment_next_pc = ((pc.toNat : Nat) : F) ∧
    G.x.Main_segment_last_c 0 = lo zc ∧ G.x.Main_segment_last_c 1 = hi zc

omit [Lean.Grind.IsCharP F goldilocks] in
/-- Every position but the last is followed by one in the same segment. -/
theorem Segment.inSegment (G : Segment xr tr ps) {k : Nat} (hk : k < segSteps - 1) :
    InSegment G.t (Pos.first.after k) := by
  intro h3
  rw [first_after_lane] at h3
  rw [first_after_row]
  apply G.inner
  · omega
  · simp only [segSteps, Main.numRows] at hk ⊢; omega

/-- **The entry airvals encode the first position.** -/
theorem Segment.entry_encodes (G : Segment xr tr ps) {pc zc : BitVec 64} (h : G.Entry pc zc) :
    Encodes G.x G.t Pos.first pc zc := by
  obtain ⟨h1, h2, h3⟩ := h
  have e := (G.rows 0).c211
  simp only [Main.c211, G.first] at e
  obtain ⟨q0, q1⟩ := previous_c_0_first (x := G.x) (t := G.t) (i := 0) G.first
  refine ⟨?_, ?_, ?_⟩
  · show (G.t 0).pc 0 = _
    rw [← h1]; grind
  · show Main.Main_previous_c_0_0 G.x G.t 0 = lo zc
    rw [q0, h2]
  · show Main.Main_previous_c_0_1 G.x G.t 0 = hi zc
    rw [q1, h3]

/-- **The last step leaves the exit airvals encoding its result.** -/
theorem Segment.exit_step (G : Segment xr tr ps) (prog : BitVec 64 → Option zisk_inst)
    (hprog : PilotProg prog) (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    (hpr : OpsProved ps) (σ : State) (pc zc : BitVec 64) (hσ : Ready σ pc zc)
    (henc : Encodes G.x G.t Pos.last pc zc) :
    ∃ inst σ' pc' zc', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' zc' ∧ G.Exit pc' zc' := by
  obtain ⟨hcol, hp0, hp1⟩ := henc
  have hwrap : G.t ((Main.numRows : Int) - 1 + 1) = G.t 0 := by
    have := G.cyclic 0
    rw [Int.zero_add] at this
    rw [Int.sub_add_cancel]; exact this
  have hl : (G.t ((Main.numRows : Int) - 1 + 1)).__L1__ = 1 := by rw [hwrap]; exact G.first
  have hlane := laneHolds_3_last (G.rows ((Main.numRows : Int) - 1)) hl
  obtain ⟨inst, σ', pc', c, hinst, hrun, hσ', hnp, hc0, hc1⟩ :=
    step_core prog hprog G.x G.t (G.rows _) xr tr hr (l := 3) (by omega) (G.rom 3 (by omega))
      hrom ps (G.ops 3 (by omega)) hpr hlane σ pc zc hσ hcol ⟨hp0, hp1⟩
  have e0 := (G.rows ((Main.numRows : Int) - 1)).c106
  have e1 := (G.rows ((Main.numRows : Int) - 1)).c107
  simp only [Main.c106, Main.c107, hl] at e0 e1
  refine ⟨inst, σ', pc', c, hinst, hrun, hσ', hnp, ?_, ?_⟩
  · have : G.x.Main_segment_last_c 0 = (G.t ((Main.numRows : Int) - 1)).c 3 0 := by grind
    rw [this]; exact hc0
  · have : G.x.Main_segment_last_c 1 = (G.t ((Main.numRows : Int) - 1)).c 3 1 := by grind
    rw [this]; exact hc1

/-- Runs compose. -/
theorem Runs.append {prog : BitVec 64 → Option zisk_inst} {a b : Nat} {σ σ₁ σ₂ : State}
    (h : Runs prog a σ σ₁) (h' : Runs prog b σ₁ σ₂) : Runs prog (a + b) σ σ₂ := by
  induction h with
  | refl => rw [Nat.zero_add]; exact h'
  | step hpc hinst hrun _ ih =>
    rw [Nat.add_right_comm]
    exact .step hpc hinst hrun (ih h')

/-- One step, as a run. -/
theorem Runs.one {prog : BitVec 64 → Option zisk_inst} {σ σ' : State} {pc : BitVec 64}
    {inst : zisk_inst} (hpc : σ.regs.get? Register.PC = some pc) (hinst : prog pc = some inst)
    (hrun : zisk_execute inst σ = .ok false σ') : Runs prog 1 σ σ' :=
  .step hpc hinst hrun (.refl σ')

/-- **Inside a segment.** From its entry airvals, the first `n` steps of a
segment, `n` below `4·numRows`, are a run of the model, and the position they
reach encodes the state it leaves. -/
theorem Segment.run_within (G : Segment xr tr ps) (prog : BitVec 64 → Option zisk_inst)
    (hprog : PilotProg prog) (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    (hpr : OpsProved ps) {n : Nat} (hn : n < segSteps) (σ : State) (pc zc : BitVec 64)
    (hσ : Ready σ pc zc) (hin : G.Entry pc zc) :
    ∃ σ' pc' zc', Runs prog n σ σ' ∧ Ready σ' pc' zc' ∧
      Encodes G.x G.t (Pos.first.after n) pc' zc' :=
  run_pilot prog hprog G.x G.t G.rows xr tr hr G.rom hrom ps G.ops hpr n Pos.first σ pc zc hσ
    (G.entry_encodes hin) (fun k hk => G.inSegment (by omega))

/-- **A whole segment.** From its entry airvals, a segment forces
`4·numRows` steps of the model, and its exit airvals encode the state they
reach. -/
theorem Segment.run (G : Segment xr tr ps) (prog : BitVec 64 → Option zisk_inst)
    (hprog : PilotProg prog) (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    (hpr : OpsProved ps) (σ : State) (pc zc : BitVec 64) (hσ : Ready σ pc zc)
    (hin : G.Entry pc zc) :
    ∃ σ' pc' zc', Runs prog segSteps σ σ' ∧ Ready σ' pc' zc' ∧ G.Exit pc' zc' := by
  obtain ⟨σ₁, pc₁, zc₁, hruns, hσ₁, henc⟩ :=
    G.run_within prog hprog hr hrom hpr (n := segSteps - 1) (by simp [segSteps, Main.numRows])
      σ pc zc hσ hin
  rw [first_after_last] at henc
  obtain ⟨inst, σ', pc', zc', hinst, hrun, hσ', hex⟩ :=
    G.exit_step prog hprog hr hrom hpr σ₁ pc₁ zc₁ hσ₁ henc
  refine ⟨σ', pc', zc', ?_, hσ', hex⟩
  have := hruns.append (Runs.one hσ₁.1 hinst hrun)
  rwa [Nat.sub_add_cancel (by simp [segSteps, Main.numRows])] at this

/-! ### The chain -/

/-- The tuple the global update opens the chain with: segment 0, not the
last, step 0, the boot pc, `c = 0`. -/
def bootTuple (boot : BitVec 64) : List F := [0, 0, 0, ((boot.toNat : Nat) : F), 0, 0]

/-- What a sound continuation bus guarantees, with the global update that
opens it: the segments, in order, each assume what the one before proved, and
the first assumes the boot tuple. Stated over the generated arguments,
`Main.bus714` (assumes) and `Main.bus715` (proves); they do not depend on the
row. -/
structure ChainHolds (boot : BitVec 64) (segs : List (Segment xr tr ps)) : Prop where
  first : ∀ (h : 0 < segs.length) (m : Nat),
    tupleAt Main.bus714.tuple segs[0].x segs[0].t 0 m = (bootTuple boot).getD m 0
  next : ∀ (k : Nat) (h : k + 1 < segs.length) (m : Nat),
    tupleAt Main.bus714.tuple segs[k + 1].x segs[k + 1].t 0 m =
      tupleAt Main.bus715.tuple segs[k].x segs[k].t 0 m

omit [Lean.Grind.IsCharP F goldilocks] in
theorem lo_zero : lo (0 : BitVec 64) = (0 : F) := by
  simp [lo, Lean.Grind.Semiring.natCast_zero]

omit [Lean.Grind.IsCharP F goldilocks] in
theorem hi_zero : hi (0 : BitVec 64) = (0 : F) := by
  simp [hi, Lean.Grind.Semiring.natCast_zero]

omit [Lean.Grind.IsCharP F goldilocks] in
/-- The boot tuple is the first segment's entry: the boot pc, `c = 0`. -/
theorem ChainHolds.entry_first {boot : BitVec 64} {segs : List (Segment xr tr ps)}
    (hc : ChainHolds boot segs) (h : 0 < segs.length) : segs[0].Entry boot 0 := by
  have e3 := hc.first h 3
  have e4 := hc.first h 4
  have e5 := hc.first h 5
  simp only [tupleAt, Main.bus714, bootTuple, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at e3 e4 e5
  exact ⟨e3, by rw [e4, lo_zero], by rw [e5, hi_zero]⟩

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A segment's exit is the next one's entry. -/
theorem ChainHolds.entry_next {boot : BitVec 64} {segs : List (Segment xr tr ps)}
    (hc : ChainHolds boot segs) {k : Nat} (h : k + 1 < segs.length) {pc zc : BitVec 64}
    (hex : segs[k].Exit pc zc) : segs[k + 1].Entry pc zc := by
  have e3 := hc.next k h 3
  have e4 := hc.next k h 4
  have e5 := hc.next k h 5
  simp only [tupleAt, Main.bus714, Main.bus715, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at e3 e4 e5
  obtain ⟨x3, x4, x5⟩ := hex
  exact ⟨e3.trans x3, e4.trans x4, e5.trans x5⟩

/-- **Whole segments from boot.** Along the chain, the first `k` segments
force `k·4·numRows` steps of the model from the boot state, and the state they
reach is the next segment's entry. -/
theorem run_segments (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    (boot : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (σ₀ : State) (h₀ : Ready σ₀ boot 0) :
    ∀ k, k ≤ segs.length → ∃ σ pc zc, Runs prog (k * segSteps) σ₀ σ ∧ Ready σ pc zc ∧
      ∀ (h : k < segs.length), segs[k].Entry pc zc := by
  intro k
  induction k with
  | zero => exact fun _ => ⟨σ₀, boot, 0, by rw [Nat.zero_mul]; exact .refl σ₀, h₀,
      fun h => hc.entry_first h⟩
  | succ k ih =>
    intro hk
    obtain ⟨σ, pc, zc, hruns, hσ, hin⟩ := ih (by omega)
    obtain ⟨σ', pc', zc', hrun, hσ', hex⟩ :=
      segs[k].run prog hprog hr hrom hpr σ pc zc hσ (hin (by omega))
    refine ⟨σ', pc', zc', ?_, hσ', fun h => hc.entry_next h hex⟩
    rw [Nat.succ_mul]
    exact hruns.append hrun

theorem div_lt_len {n len : Nat} (hn : n < len * segSteps) : n / segSteps < len :=
  (Nat.div_lt_iff_lt_mul (by simp [segSteps, Main.numRows])).mpr hn

/-- **A run from boot.** Along a chain of segments from the boot state, every
step the segments hold is a step of the model: for any `n` below their total,
the model runs `n` steps of the program from the boot state, and the position
of step `n` — segment `n / (4·numRows)`, step `n mod 4·numRows` within it —
encodes the state it reaches.

What remains assumed: the soundness of every bus involved (the continuation
bus as `ChainHolds`, the ROM and operation-bus lookups of every segment), the
contents of the Rom and of `__L1__`, that every prover on the operation bus
computes each op (`OpsProved`, which `System.ok` gives), and the pilot's
scope. The start is no longer assumed: it is the boot state the chain opens
with. -/
theorem run_from_boot (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    (boot : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (σ₀ : State) (h₀ : Ready σ₀ boot 0) (n : Nat) (hn : n < segs.length * segSteps) :
    ∃ σ pc zc, Runs prog n σ₀ σ ∧ Ready σ pc zc ∧
      Encodes (segs[n / segSteps]'(div_lt_len hn)).x (segs[n / segSteps]'(div_lt_len hn)).t
        (Pos.first.after (n % segSteps)) pc zc := by
  have hpos : 0 < segSteps := by simp [segSteps, Main.numRows]
  have hk : n / segSteps < segs.length := div_lt_len hn
  obtain ⟨σ, pc, zc, hruns, hσ, hin⟩ :=
    run_segments prog hprog hr hrom hpr boot segs hc σ₀ h₀ (n / segSteps) (by omega)
  obtain ⟨σ', pc', zc', hrun, hσ', henc⟩ :=
    segs[n / segSteps].run_within prog hprog hr hrom hpr (n := n % segSteps)
      (Nat.mod_lt _ hpos) σ pc zc hσ (hin hk)
  refine ⟨σ', pc', zc', ?_, hσ', henc⟩
  have := hruns.append hrun
  rwa [Nat.mul_comm, Nat.div_add_mod] at this

end Segment

/-! ### Over ZisK's provers -/

section System

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- **A run from boot, over ZisK's provers.** `run_from_boot` with the
operation bus answered by a system's provers (`Provers.lean`): `OpsProved` is
replaced by the tables' contents, which `check_tables.py` checks. -/
theorem run_from_boot_system (S : System F) (hT : Tables S.xv S.tv)
    (hF : ∀ p ∈ frops S.x0 S.t0, FropsOk p)
    (prog : BitVec 64 → Option zisk_inst) (hprog : PilotProg prog)
    {xr : Rom.Ctx F} {tr : Rom.Trace F}
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    (boot : BitVec 64) (segs : List (Segment xr tr S.provers)) (hc : ChainHolds boot segs)
    (σ₀ : State) (h₀ : Ready σ₀ boot 0) (n : Nat) (hn : n < segs.length * segSteps) :
    ∃ σ pc zc, Runs prog n σ₀ σ ∧ Ready σ pc zc :=
  let ⟨σ, pc, zc, h1, h2, _⟩ :=
    run_from_boot prog hprog hr hrom (S.ok hT hF) boot segs hc σ₀ h₀ n hn
  ⟨σ, pc, zc, h1, h2⟩

end System

end Correspondence
