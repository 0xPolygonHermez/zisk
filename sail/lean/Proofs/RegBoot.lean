/-
  A run from boot with the register file.

  `run_from_boot` (`Segments.lean`) runs the pilot's programs — immediate or
  `c` sources, no store — along a chain of segments. Here sources and the
  store may be registers: the model's register file is carried along the run,
  and at every step it is the one Main's register chain holds (`RegInv`).

  * `step_reg_at`: one step at a lane, the register sources' values read off
    the chain (`reg_srcs`), and the register file after it the chain's again
    (`reg_update`);
  * `run_from_boot_regs`: along a chain of segments from the boot state, with
    every register 0, the trace forces every step of the model, and after `n`
    steps the model's registers are the chain's at memory step `1 + 4n`.
-/
import Proofs.RegStep
import Proofs.Execution

open Out.Functions

namespace Correspondence

open Pil.Zisk MemCheck

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- **One step with the register file.** At lane `l` of row `m` of segment
`k`, from a ready state whose registers are the chain's at the lane's first
memory step, the program's instruction runs, and the registers it leaves are
the chain's four memory steps on. -/
theorem step_reg_at (prog : BitVec 64 → Option zisk_inst) (hprog : RegProg prog)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    (ps : List (Prover F)) (hpr : OpsProved ps)
    (M : MainSegs F) (hch : ChainOk M) (hK : M.K < 1048576) {k : Nat} (hk : k < M.K)
    (hmi : ∀ i, Main.everyRow (M.seg k).1 (M.seg k).2 i)
    (hbus : ∀ l, l < 4 → LookupHolds (romArg l) (M.seg k).1 (M.seg k).2 [⟨Rom.bus0, xr, tr⟩])
    (hops : ∀ l, l < 4 → LookupHolds (opArg l) (M.seg k).1 (M.seg k).2 ps)
    {m l : Nat} (hm : m < Main.numRows) (hl : l < 4)
    {p0 p1 np : F} (hlane : LaneHolds ((M.seg k).2 m) l p0 p1 np)
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ np = ((q.toNat : Nat) : F))
    (σ : State) (pc zc : BitVec 64) (xs : Vector (BitVec 64) 32) (hσ : Ready σ pc zc)
    (hxs : σ.regs.get? Register.Xs = some xs)
    (hcol : ((M.seg k).2 m).pc l = ((pc.toNat : Nat) : F)) (hprev : p0 = lo zc ∧ p1 = hi zc)
    (hinv : RegInv M xs (1 + 4 * (k * 16777216 + 4 * m + l))) :
    ∃ inst σ' pc' c xs', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' c ∧ σ'.regs.get? Register.Xs = some xs' ∧
      RegInv M xs' (1 + 4 * (k * 16777216 + 4 * m + l) + 4) ∧
      np = ((pc'.toNat : Nat) : F) ∧ ((M.seg k).2 m).c l 0 = lo c ∧
      ((M.seg k).2 m).c l 1 = hi c := by
  have hsrc : ∀ inst, prog pc = some inst → RegPilot inst ∧ SlotHolds ((M.seg k).2 m) l inst pc := by
    intro inst hinst
    obtain ⟨inst', hinst', hslot, -⟩ :=
      rom_lane' prog _ _ _ (hmi m) xr tr hr hl (hbus l hl) hrom pc hσ.2.2.2 hcol
    rw [hinst] at hinst'
    cases hinst'
    exact ⟨(hprog pc inst hinst).1, hslot⟩
  obtain ⟨inst, σ', pc', c, hinst, hslot, hsto, hpil, hrun, hσ', hx, -, hnp, hc0, hc1⟩ :=
    step_gen prog hprog _ _ (hmi m) xr tr hr hl (hbus l hl) hrom ps (hops l hl) hpr hlane hq
      σ pc zc xs hσ hxs hcol hprev
      (fun inst hinst ha => (reg_srcs hch hk hm hl (hsrc inst hinst).2 (hsrc inst hinst).1.idx hinv).1 ha)
      (fun inst hinst hb => (reg_srcs hch hk hm hl (hsrc inst hinst).2 (hsrc inst hinst).1.idx hinv).2 hb)
  exact ⟨inst, σ', pc', c, _, hinst, hrun, hσ', hx,
    reg_update hch hK hk hm hl hslot hsto hpil.idx (hprog pc inst hinst).2.2 hc0 hc1 hinv, hnp, hc0, hc1⟩

theorem first_after_next (j : Nat) : (Pos.first.after j).next = Pos.first.after (j + 1) := by
  have hr := first_after_row j
  have hl := first_after_lane j
  apply Pos.ext'
  · rw [first_after_row]
    unfold Pos.next
    split
    · simp only; rw [hr]; rename_i h; rw [hl] at h; omega
    · simp only; rw [hr]; rename_i h; rw [hl] at h; omega
  · rw [first_after_lane]
    unfold Pos.next
    split
    · simp only; rename_i h; rw [hl] at h; omega
    · simp only; rw [hl]; rename_i h; rw [hl] at h; omega

theorem segSteps_eq : segSteps = 16777216 := by simp [segSteps, Main.numRows]

section Segment

variable {xr : Rom.Ctx F} {tr : Rom.Trace F} {ps : List (Prover F)}

/-- Step `j` of segment `k` is about to run from `σ`, with pc `pc` and `c`
`zc`: the state is ready, the position encodes it, and its registers are the
chain's at the step's first memory step, `1 + 4·(k·2^24 + j)`. -/
def RegAt (M : MainSegs F) (G : Segment xr tr ps) (k j : Nat) (σ : State) (pc zc : BitVec 64) :
    Prop :=
  Ready σ pc zc ∧ Encodes G.x G.t (Pos.first.after j) pc zc ∧
    ∃ xs, σ.regs.get? Register.Xs = some xs ∧ RegInv M xs (1 + 4 * (k * 16777216 + j))

/-- What a register-aware run needs of the program, the Rom, the provers and
the register chain. -/
structure RegSetup (prog : BitVec 64 → Option zisk_inst) (xr : Rom.Ctx F) (tr : Rom.Trace F)
    (ps : List (Prover F)) (M : MainSegs F) : Prop where
  prog_ok : RegProg prog
  rom_rows : ∀ j, Rom.everyRow xr tr j
  rom : RomHolds prog tr
  ops : OpsProved ps
  chain : ChainOk M
  small : M.K < 1048576

/-- **A step inside a segment, with the register file.** -/
theorem Segment.step_reg {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    (hS : RegSetup prog xr tr ps M) (G : Segment xr tr ps) {k : Nat} (hk : k < M.K)
    (hMk : M.seg k = (G.x, G.t)) {j : Nat} (hj : j < segSteps - 1) (σ : State) (pc zc : BitVec 64)
    (h : RegAt M G k j σ pc zc) :
    ∃ inst σ' pc' zc', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      RegAt M G k (j + 1) σ' pc' zc' := by
  obtain ⟨hσ, ⟨hcol, hp0, hp1⟩, xs, hxs, hinv⟩ := h
  have hs := G.inSegment hj
  have hlane := laneHolds_at G.rows hs
  rw [first_after_next] at hlane
  have hrow := first_after_row j
  have hl := first_after_lane j
  have hl4 : j % 4 < 4 := Nat.mod_lt _ (by omega)
  have hm : j / 4 < Main.numRows := by
    have h1 := segSteps_eq; have h2 : segSteps = 4 * Main.numRows := rfl; omega
  obtain ⟨q, hq32, hq⟩ := rom_pc32 prog G.x G.t (Pos.first.after (j + 1)).row xr tr hS.rom_rows
    (Pos.first.after (j + 1)).lt (G.rom _ (Pos.first.after (j + 1)).lt) hS.rom
  rw [hrow, hl] at hlane hcol
  obtain ⟨inst, σ', pc', c, xs', hinst, hrun, hσ', hx, hinv', hnp, hc0, hc1⟩ :=
    step_reg_at prog hS.prog_ok xr tr hS.rom_rows hS.rom ps hS.ops M hS.chain hS.small hk
      (by rw [hMk]; exact G.rows) (by rw [hMk]; exact G.rom) (by rw [hMk]; exact G.ops)
      hm hl4 (by rw [hMk]; exact hlane) ⟨q, hq32, hq⟩ σ pc zc xs hσ hxs (by rw [hMk]; exact hcol)
      ⟨hp0, hp1⟩ (by rwa [show k * 16777216 + 4 * (j / 4) + j % 4 = k * 16777216 + j by omega])
  rw [hMk] at hc0 hc1
  refine ⟨inst, σ', pc', c, hinst, hrun, hσ', ⟨hnp, ?_, ?_⟩, xs', hx, ?_⟩
  · rw [← first_after_next, prevC_next hs 0 (.inl rfl), hrow, hl]; exact hc0
  · rw [← first_after_next, prevC_next hs 1 (.inr rfl), hrow, hl]; exact hc1
  · rwa [show 1 + 4 * (k * 16777216 + (j + 1)) =
      1 + 4 * (k * 16777216 + 4 * (j / 4) + j % 4) + 4 by omega]

/-- **Inside a segment, with the register file.** -/
theorem Segment.run_reg_within {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    (hS : RegSetup prog xr tr ps M) (G : Segment xr tr ps) {k : Nat} (hk : k < M.K)
    (hMk : M.seg k = (G.x, G.t)) :
    ∀ n, n < segSteps → ∀ σ pc zc, RegAt M G k 0 σ pc zc →
      ∃ σ' pc' zc', Runs prog n σ σ' ∧ RegAt M G k n σ' pc' zc' := by
  intro n
  induction n with
  | zero => exact fun _ σ pc zc h => ⟨σ, pc, zc, .refl σ, h⟩
  | succ n ih =>
    intro hn σ pc zc h
    obtain ⟨σ₁, pc₁, zc₁, hruns, h₁⟩ := ih (by omega) σ pc zc h
    obtain ⟨inst, σ₂, pc₂, zc₂, hinst, hrun, h₂⟩ :=
      G.step_reg hS hk hMk (j := n) (by omega) σ₁ pc₁ zc₁ h₁
    exact ⟨σ₂, pc₂, zc₂, hruns.append (Runs.one h₁.1.1 hinst hrun), h₂⟩

/-- **A whole segment, with the register file.** From step 0, a segment forces
`4·numRows` steps of the model; its exit airvals encode the state they reach,
whose registers are the chain's at the next segment's first memory step. -/
theorem Segment.run_reg {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    (hS : RegSetup prog xr tr ps M) (G : Segment xr tr ps) {k : Nat} (hk : k < M.K)
    (hMk : M.seg k = (G.x, G.t))
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ G.x.Main_segment_next_pc = ((q.toNat : Nat) : F))
    (σ : State) (pc zc : BitVec 64) (h : RegAt M G k 0 σ pc zc) :
    ∃ σ' pc' zc' xs', Runs prog segSteps σ σ' ∧ Ready σ' pc' zc' ∧ G.Exit pc' zc' ∧
      σ'.regs.get? Register.Xs = some xs' ∧ RegInv M xs' (1 + 4 * ((k + 1) * 16777216)) := by
  have hS1 := segSteps_eq
  obtain ⟨σ₁, pc₁, zc₁, hruns, hσ₁, henc, xs, hxs, hinv⟩ :=
    G.run_reg_within hS hk hMk (segSteps - 1) (by omega) σ pc zc h
  rw [first_after_last] at henc
  obtain ⟨hcol, hp0, hp1⟩ := henc
  have hwrap : G.t ((Main.numRows : Int) - 1 + 1) = G.t 0 := by
    have := G.cyclic 0
    rw [Int.zero_add] at this
    rw [Int.sub_add_cancel]; exact this
  have hl1 : (G.t ((Main.numRows : Int) - 1 + 1)).__L1__ = 1 := by rw [hwrap]; exact G.first
  have hlane := laneHolds_3_last (G.rows ((Main.numRows : Int) - 1)) hl1
  have hrow : ((Main.numRows : Int) - 1) = ((Main.numRows - 1 : Nat) : Int) := by
    simp [Main.numRows]
  rw [hrow] at hlane hl1
  have hm : Main.numRows - 1 < Main.numRows := by simp [Main.numRows]
  obtain ⟨inst, σ', pc', c, xs', hinst, hrun, hσ', hx, hinv', hnp, hc0, hc1⟩ :=
    step_reg_at prog hS.prog_ok xr tr hS.rom_rows hS.rom ps hS.ops M hS.chain hS.small hk
      (by rw [hMk]; exact G.rows) (by rw [hMk]; exact G.rom) (by rw [hMk]; exact G.ops)
      hm (l := 3) (by omega) (by rw [hMk]; exact hlane) hq σ₁ pc₁ zc₁ xs hσ₁ hxs
      (by rw [hMk]; simp only [Pos.last] at hcol; rw [hrow] at hcol; exact hcol)
      ⟨hp0, hp1⟩
      (by
        have h2 : segSteps = 4 * Main.numRows := rfl
        rwa [show k * 16777216 + 4 * (Main.numRows - 1) + 3 = k * 16777216 + (segSteps - 1) by
          omega])
  rw [hMk] at hc0 hc1
  have e0 := (G.rows ((Main.numRows : Int) - 1)).c106
  have e1 := (G.rows ((Main.numRows : Int) - 1)).c107
  rw [hrow] at e0 e1
  simp only [Main.c106, Main.c107, hl1] at e0 e1
  refine ⟨σ', pc', c, xs', ?_, hσ', ⟨hnp, ?_, ?_⟩, hx, ?_⟩
  · have := hruns.append (Runs.one hσ₁.1 hinst hrun)
    rwa [Nat.sub_add_cancel (by omega)] at this
  · have : G.x.Main_segment_last_c 0 = (G.t ((Main.numRows - 1 : Nat) : Int)).c 3 0 := by grind
    rw [this]; exact hc0
  · have : G.x.Main_segment_last_c 1 = (G.t ((Main.numRows - 1 : Nat) : Int)).c 3 1 := by grind
    rw [this]; exact hc1
  · have h2 : segSteps = 4 * Main.numRows := rfl
    rwa [show 1 + 4 * ((k + 1) * 16777216) =
      1 + 4 * (k * 16777216 + 4 * (Main.numRows - 1) + 3) + 4 by omega]

/-! ### The chain -/

/-- Registers start at 0: before every access, the chain holds 0. -/
theorem regInv_zero {M : MainSegs F} (hch : ChainOk M) :
    RegInv M (Vector.replicate 32 (0 : BitVec 64)) 1 := by
  intro i hi
  obtain ⟨-, hlt, -⟩ := hch i hi
  rw [regVal_of_none (fun e he _ hs => by have := hlt e he; omega)]
  have : (Vector.replicate 32 (0 : BitVec 64))[i + 1]! = 0 := by
    rw [getElem!_pos (Vector.replicate 32 (0 : BitVec 64)) (i + 1) (by omega)]; simp
  rw [this, lo_zero, hi_zero]

/-- The segments the run goes through are the memory bus's Main instances, in
order. -/
def SegsAre (M : MainSegs F) (segs : List (Segment xr tr ps)) : Prop :=
  M.K = segs.length ∧ ∀ k (h : k < segs.length), M.seg k = (segs[k].x, segs[k].t)

/-- **Whole segments from boot, with the register file.** -/
theorem run_segments_reg {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    (hS : RegSetup prog xr tr ps M) (boot : BitVec 64) (segs : List (Segment xr tr ps))
    (hc : ChainHolds boot segs) (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) :
    ∀ k (hk : k < segs.length), ∃ σ pc zc, Runs prog (k * segSteps) σ₀ σ ∧
      RegAt M segs[k] k 0 σ pc zc := by
  intro k
  induction k with
  | zero =>
    intro h
    refine ⟨σ₀, boot, 0, by rw [Nat.zero_mul]; exact .refl σ₀, h₀,
      segs[0].entry_encodes (hc.entry_first h), _, hx₀, ?_⟩
    exact regInv_zero hS.chain
  | succ k ih =>
    intro hk
    obtain ⟨σ, pc, zc, hruns, hat⟩ := ih (by omega)
    obtain ⟨σ', pc', zc', xs', hrun, hσ', hex, hx, hinv⟩ :=
      segs[k].run_reg hS (by rw [hM.1]; omega) (hM.2 k (by omega))
        (hc.next_pc32 prog hS.rom_rows hS.rom hk) σ pc zc hat
    refine ⟨σ', pc', zc', ?_, hσ', segs[k + 1].entry_encodes (hc.entry_next hk hex), xs', hx, ?_⟩
    · rw [Nat.succ_mul]; exact hruns.append hrun
    · rwa [Nat.add_zero]

/-- **A run from boot, with the register file.** Along a chain of segments
from the boot state, with every register 0, every step the segments hold is a
step of the model, now with register sources and register stores: for any `n`
below their total, the model runs `n` steps of the program from the boot
state, the position of step `n` encodes the state it reaches, and that
state's registers `x1`–`x31` are what Main's register chain holds at memory
step `1 + 4n` — the first memory step of step `n`.

What remains assumed, besides `run_from_boot`'s: the register chain's facts
(`ChainOk`, which `MainSegs.reg_chain` gives on a sound memory bus), and that
the segments are the memory bus's Main instances (`SegsAre`). -/
theorem run_from_boot_regs {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    (hS : RegSetup prog xr tr ps M) (boot : BitVec 64) (segs : List (Segment xr tr ps))
    (hc : ChainHolds boot segs) (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0))
    (n : Nat) (hn : n < segs.length * segSteps) :
    ∃ σ pc zc xs, Runs prog n σ₀ σ ∧ Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧
      RegInv M xs (1 + 4 * n) ∧
      Encodes (segs[n / segSteps]'(div_lt_len hn)).x (segs[n / segSteps]'(div_lt_len hn)).t
        (Pos.first.after (n % segSteps)) pc zc := by
  have hS1 := segSteps_eq
  have hk : n / segSteps < segs.length := div_lt_len hn
  obtain ⟨σ, pc, zc, hruns, hat⟩ := run_segments_reg hS boot segs hc hM σ₀ h₀ hx₀ (n / segSteps) hk
  obtain ⟨σ', pc', zc', hrun, hσ', henc, xs, hx, hinv⟩ :=
    segs[n / segSteps].run_reg_within hS (by rw [hM.1]; omega) (hM.2 _ hk) (n % segSteps)
      (Nat.mod_lt _ (by omega)) σ pc zc hat
  refine ⟨σ', pc', zc', xs, ?_, hσ', hx, ?_, henc⟩
  · have := hruns.append hrun
    rwa [Nat.mul_comm, Nat.div_add_mod] at this
  · rw [hS1] at hinv
    rwa [show n / 16777216 * 16777216 + n % 16777216 = n by omega] at hinv

/-! ### Over the memory bus -/

/-- The segments as the memory bus needs them: few enough, each one's first
step the continuation chain's, `SEGMENT_STEP` as the setup fixes it, and the
gaps' range checks sound. -/
theorem SegsAre.ok {M : MainSegs F} {boot : BitVec 64} {segs : List (Segment xr tr ps)}
    (hc : ChainHolds boot segs) (hM : SegsAre M segs) (hK : segs.length < 1048576)
    (hstep : ∀ k (h : k < segs.length), ∀ m : Nat, m < Main.numRows →
      (segs[k].t m).Main_SEGMENT_STEP = ((m : Nat) : F))
    (hrng : ∀ k (h : k < segs.length), ∀ g : PairTag,
      RangeHolds Main.ranges g.rangeArg segs[k].x segs[k].t) : M.Ok where
  small := by rw [hM.1]; exact hK
  seg k hk := by
    have hk' : k < segs.length := by rw [← hM.1]; exact hk
    rw [hM.2 k hk']
    exact ⟨segs[k].rows, hc.initial_step k hk', hstep k hk'⟩
  ranges k hk g := by
    have hk' : k < segs.length := by rw [← hM.1]; exact hk
    rw [hM.2 k hk']
    exact hrng k hk' g

/-- **A run from boot with the register file, over the memory bus.**
`run_from_boot_regs` with the register chain's facts from a sound memory bus
(`MainSegs.reg_chain`) over the segments and every other AIR, with the
registers' global updates: the model's registers after `n` steps are what
Main's register chain holds at memory step `1 + 4n`.

What remains assumed: the soundness of every bus involved (the continuation
bus as `ChainHolds`; the ROM and operation-bus lookups of every segment; the
memory bus as `SumHolds`, with the global updates `GlobalRegs` writes; the
range checks of the gaps), the contents of the Rom, of `__L1__` and of
`SEGMENT_STEP`, that every prover on the operation bus computes each op, at
most `2^20` segments, and the register-aware pilot's scope (`RegProg`). -/
theorem run_from_boot_mem {prog : BitVec 64 → Option zisk_inst} (hprog : RegProg prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    (boot : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (hK : segs.length < 1048576)
    (hstep : ∀ k (h : k < segs.length), ∀ m : Nat, m < Main.numRows →
      (segs[k].t m).Main_SEGMENT_STEP = ((m : Nat) : F))
    (hrng : ∀ k (h : k < segs.length), ∀ g : PairTag,
      RangeHolds Main.ranges g.rangeArg segs[k].x segs[k].t)
    (M : MainSegs F) (hM : SegsAre M segs) (S : MemOthers F) {gp ga : List (List F)}
    (hgl : GlobalRegs gp ga)
    (hbus : SumHolds 10 (M.list.flatMap (fun m => mainMemSenders m.1 m.2) ++ S.senders) gp ga)
    (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0))
    (n : Nat) (hn : n < segs.length * segSteps) :
    ∃ σ pc zc xs, Runs prog n σ₀ σ ∧ Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧
      RegInv M xs (1 + 4 * n) := by
  have hok := hM.ok hc hK hstep hrng
  have hS : RegSetup prog xr tr ps M :=
    ⟨hprog, hr, hrom, hpr, fun i hi => M.reg_chain hok S hgl hbus (by omega), hok.small⟩
  obtain ⟨σ, pc, zc, xs, h1, h2, h3, h4, -⟩ :=
    run_from_boot_regs hS boot segs hc hM σ₀ h₀ hx₀ n hn
  exact ⟨σ, pc, zc, xs, h1, h2, h3, h4⟩

end Segment

end Correspondence
