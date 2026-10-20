/-
  A run from boot with the register file and memory.

  `RegBoot.lean` carries the model's registers along the run; here memory
  too, against the memory bus's guarantee for Main's accesses (`MemAnswers`,
  a hypothesis). At every step, the model's registers are Main's register
  chain's, and its memory words are `memVal` of Main's stores.

  * `step_mem_at`: one step at a lane — register and memory sources read off
    the chain and the memory function, and after it both invariants hold four
    memory steps on.
-/
import Proofs.StepMem
import Proofs.RegBoot

open Out.Functions

namespace Correspondence

open Pil.Zisk MemCheck MemSort

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- **One step with the register file and memory.** At lane `l` of row `m` of
segment `k`, from a ready state whose registers are the chain's and whose
memory words are `memVal` at the lane's first memory step, the program's
instruction runs, and both hold again four memory steps on. -/
theorem step_mem_at (prog : BitVec 64 → Option zisk_inst) (hprog : MemProg prog)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    (ps : List (Prover F)) (hpr : OpsProved ps)
    (M : MainSegs F) (hch : ChainOk M) (hK : M.K < 1048576)
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hA : MemAnswers M D init ws) (hD : MemDomain D) {k : Nat} (hk : k < M.K)
    (hmi : ∀ i, Main.everyRow (M.seg k).1 (M.seg k).2 i)
    (hbus : ∀ l, l < 4 → LookupHolds (romArg l) (M.seg k).1 (M.seg k).2 [⟨Rom.bus0, xr, tr⟩])
    (hops : ∀ l, l < 4 → LookupHolds (opArg l) (M.seg k).1 (M.seg k).2 ps)
    {m l : Nat} (hm : m < Main.numRows) (hl : l < 4)
    {p0 p1 np : F} (hlane : LaneHolds ((M.seg k).2 m) l p0 p1 np)
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ np = ((q.toNat : Nat) : F))
    (σ : State) (pc zc : BitVec 64) (xs : Vector (BitVec 64) 32) (W : Nat → BitVec 64)
    (hσ : Ready σ pc zc) (hxs : σ.regs.get? Register.Xs = some xs) (hmo : MemOk σ W D)
    (hcol : ((M.seg k).2 m).pc l = ((pc.toNat : Nat) : F)) (hprev : p0 = lo zc ∧ p1 = hi zc)
    (hinv : RegInv M xs (memBase k m l)) (hminv : MemInv D init ws W (memBase k m l)) :
    ∃ inst σ' pc' c xs' W', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' c ∧ σ'.regs.get? Register.Xs = some xs' ∧
      RegInv M xs' (memBase k m l + 4) ∧ MemOk σ' W' D ∧ MemInv D init ws W' (memBase k m l + 4) ∧
      np = ((pc'.toNat : Nat) : F) ∧ ((M.seg k).2 m).c l 0 = lo c ∧
      ((M.seg k).2 m).c l 1 = hi c := by
  have hsrc : ∀ inst, prog pc = some inst →
      MemPilot inst ∧ SlotHolds ((M.seg k).2 m) l inst pc ∧ StoreHolds ((M.seg k).2 m) l inst := by
    intro inst hinst
    obtain ⟨inst', hinst', hslot, hsto⟩ :=
      rom_lane' prog _ _ _ (hmi m) xr tr hr hl (hbus l hl) hrom pc hσ.2.2.2 hcol
    rw [hinst] at hinst'
    cases hinst'
    exact ⟨(hprog pc inst hinst).1, hslot, hsto⟩
  -- Source `a`, in the lane's columns, whatever it is.
  have hAc : ∀ inst, prog pc = some inst → ACols ((M.seg k).2 m) l (aValM inst zc xs W) := by
    intro inst hinst
    obtain ⟨hpil, hslot, -⟩ := hsrc inst hinst
    rcases hpil.a with ((h | h | ⟨h, -⟩) | ⟨h, h32⟩)
    · have := a_cols_imm_c hlane hslot hprev (.inl h) xs
      simp only [aValM, h]; exact this
    · have := a_cols_imm_c hlane hslot hprev (.inr h) xs
      simp only [aValM, h]; exact this
    · have := (reg_srcs hch hk hm hl hslot hpil.idx hinv).1 h
      simp only [aValM, aVal, h]; exact this
    · have := (mem_a_cols hA hD hk hm hl hslot h h32 hminv).2
      simp only [aValM, h]; exact this
  obtain ⟨inst, σ', pc', c, hinst, hslot, hsto, hpil, hrun, hσ', hx, hmem, -, hnp, hc0, hc1, haV⟩ :=
    step_genM prog hprog _ _ (hmi m) xr tr hr hl (hbus l hl) hrom ps (hops l hl) hpr hlane hq
      σ pc zc xs hσ hxs hmo hD.aligned hcol hprev
      (fun inst hinst ha => ⟨hAc inst hinst, fun h => by
        obtain ⟨hpil, hslot, -⟩ := hsrc inst hinst
        rcases hpil.a with ((h' | h' | ⟨h', -⟩) | ⟨-, h32⟩)
        all_goals first
          | (rw [h] at h'; cases h')
          | exact hD.memAt (mem_a_cols hA hD hk hm hl hslot h h32 hminv).1⟩)
      (fun inst hinst hb => by
        obtain ⟨hpil, hslot, hsto⟩ := hsrc inst hinst
        rcases hb with h | h | h
        · refine ⟨?_, fun h' => (by rw [h] at h'; cases h'), fun h' => (by rw [h] at h'; cases h')⟩
          have := (reg_srcs hch hk hm hl hslot hpil.idx hinv).2 h
          simp only [bValM, bVal, h]; exact this
        · have h32 : inst.b_imm.toNat < 2 ^ 32 := by
            rcases hpil.b with ((h' | h' | ⟨h', -⟩) | ⟨-, h32⟩)
            all_goals first | (rw [h] at h'; cases h') | exact h32
          obtain ⟨hDb, hBc⟩ := mem_b_cols hA hD hk hm hl (hmi m) hslot hsto (.inl h) h32
            (hAc inst hinst) hminv
          simp only [bAddr, h] at hDb hBc
          refine ⟨?_, fun _ => hD.memAt hDb, fun h' => (by rw [h] at h'; cases h')⟩
          simp only [bValM, h]; exact hBc
        · have h32 : inst.b_imm.toNat < 2 ^ 32 := by
            rcases hpil.b with ((h' | h' | ⟨h', -⟩) | ⟨-, h32⟩)
            all_goals first | (rw [h] at h'; cases h') | exact h32
          obtain ⟨hDb, hBc⟩ := mem_b_cols hA hD hk hm hl (hmi m) hslot hsto (.inr h) h32
            (hAc inst hinst) hminv
          simp only [bAddr, h] at hDb hBc
          refine ⟨?_, fun h' => (by rw [h] at h'; cases h'), fun _ => hD.memAt hDb⟩
          simp only [bValM, h]; exact hBc)
      (fun inst hinst => by
        obtain ⟨hpil, hslot, hsto⟩ := hsrc inst hinst
        have hsa := fun hs => mem_store_addr hA hD hk hm hl (hmi m) hslot hsto hs (hAc inst hinst)
        refine ⟨fun h => ?_, fun h => ?_⟩
        · rcases hpil.store with h' | h' | ⟨-, h31⟩
          all_goals first
            | (rw [h] at h'; cases h')
            | exact hD.memAt (hsa ⟨.inl h, h31⟩ _ (by simp [storeAddr, h]))
        · rcases hpil.store with h' | h' | ⟨-, h31⟩
          all_goals first
            | (rw [h] at h'; cases h')
            | exact hD.memAt (hsa ⟨.inr h, h31⟩ _ (by simp [storeAddr, h])))
  have hl2 := (hprog pc inst hinst).2.2
  exact ⟨inst, σ', pc', c, _, _, hinst, hrun, hσ', hx,
    reg_update hch hK hk hm hl hslot hsto hpil.idx hl2 hc0 hc1 hinv, hmem,
    (mem_update hA hD hk hm hl (hmi m) hslot hsto hl2 hc0 hc1 hpil.store haV hminv).1, hnp, hc0, hc1⟩


section Segment

variable {xr : Rom.Ctx F} {tr : Rom.Trace F} {ps : List (Prover F)}

/-- What a run with registers and memory needs: the program, the Rom, the
provers, the register chain, and the memory bus's guarantee. -/
structure MemSetup (prog : BitVec 64 → Option zisk_inst) (xr : Rom.Ctx F) (tr : Rom.Trace F)
    (ps : List (Prover F)) (M : MainSegs F) (D : Nat → Prop) (init : Nat → F × F)
    (ws : List (Emit (F × F))) : Prop where
  prog_ok : MemProg prog
  rom_rows : ∀ j, Rom.everyRow xr tr j
  rom : RomHolds prog tr
  ops : OpsProved ps
  chain : ChainOk M
  small : M.K < 1048576
  ans : MemAnswers M D init ws
  dom : MemDomain D

/-- Step `j` of segment `k` is about to run from `σ`: the state is ready, the
position encodes it, its registers are the chain's and its memory words
`memVal` at the step's first memory step. -/
def RunAt (M : MainSegs F) (D : Nat → Prop) (init : Nat → F × F) (ws : List (Emit (F × F)))
    (G : Segment xr tr ps) (k j : Nat) (σ : State) (pc zc : BitVec 64) : Prop :=
  Ready σ pc zc ∧ Encodes G.x G.t (Pos.first.after j) pc zc ∧
    ∃ xs W, σ.regs.get? Register.Xs = some xs ∧ RegInv M xs (1 + 4 * (k * 16777216 + j)) ∧
      MemOk σ W D ∧ MemInv D init ws W (1 + 4 * (k * 16777216 + j))

/-- **A step inside a segment, with registers and memory.** -/
theorem Segment.step_mem {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F} {D : Nat → Prop}
    {init : Nat → F × F} {ws : List (Emit (F × F))} (hS : MemSetup prog xr tr ps M D init ws)
    (G : Segment xr tr ps) {k : Nat} (hk : k < M.K)
    (hMk : M.seg k = (G.x, G.t)) {j : Nat} (hj : j < segSteps - 1) (σ : State) (pc zc : BitVec 64)
    (h : RunAt M D init ws G k j σ pc zc) :
    ∃ inst σ' pc' zc', prog pc = some inst ∧ zisk_execute inst σ = .ok false σ' ∧
      RunAt M D init ws G k (j + 1) σ' pc' zc' := by
  obtain ⟨hσ, ⟨hcol, hp0, hp1⟩, xs, W, hxs, hinv, hmo, hminv⟩ := h
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
  have hb : memBase k (j / 4) (j % 4) = 1 + 4 * (k * 16777216 + j) := by
    simp only [memBase]; omega
  obtain ⟨inst, σ', pc', c, xs', W', hinst, hrun, hσ', hx, hinv', hmo', hminv', hnp, hc0, hc1⟩ :=
    step_mem_at prog hS.prog_ok xr tr hS.rom_rows hS.rom ps hS.ops M hS.chain hS.small hS.ans hS.dom
      hk (by rw [hMk]; exact G.rows) (by rw [hMk]; exact G.rom) (by rw [hMk]; exact G.ops)
      hm hl4 (by rw [hMk]; exact hlane) ⟨q, hq32, hq⟩ σ pc zc xs W hσ hxs hmo
      (by rw [hMk]; exact hcol) ⟨hp0, hp1⟩ (by rw [hb]; exact hinv) (by rw [hb]; exact hminv)
  rw [hMk] at hc0 hc1
  rw [hb] at hinv' hminv'
  refine ⟨inst, σ', pc', c, hinst, hrun, hσ', ⟨hnp, ?_, ?_⟩, xs', W', hx, ?_, hmo', ?_⟩
  · rw [← first_after_next, prevC_next hs 0 (.inl rfl), hrow, hl]; exact hc0
  · rw [← first_after_next, prevC_next hs 1 (.inr rfl), hrow, hl]; exact hc1
  · rwa [show 1 + 4 * (k * 16777216 + (j + 1)) = 1 + 4 * (k * 16777216 + j) + 4 by omega]
  · rwa [show 1 + 4 * (k * 16777216 + (j + 1)) = 1 + 4 * (k * 16777216 + j) + 4 by omega]

/-- **Inside a segment, with registers and memory.** -/
theorem Segment.run_mem_within {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hS : MemSetup prog xr tr ps M D init ws) (G : Segment xr tr ps) {k : Nat} (hk : k < M.K)
    (hMk : M.seg k = (G.x, G.t)) :
    ∀ n, n < segSteps → ∀ σ pc zc, RunAt M D init ws G k 0 σ pc zc →
      ∃ σ' pc' zc', Runs prog n σ σ' ∧ RunAt M D init ws G k n σ' pc' zc' := by
  intro n
  induction n with
  | zero => exact fun _ σ pc zc h => ⟨σ, pc, zc, .refl σ, h⟩
  | succ n ih =>
    intro hn σ pc zc h
    obtain ⟨σ₁, pc₁, zc₁, hruns, h₁⟩ := ih (by omega) σ pc zc h
    obtain ⟨inst, σ₂, pc₂, zc₂, hinst, hrun, h₂⟩ :=
      G.step_mem hS hk hMk (j := n) (by omega) σ₁ pc₁ zc₁ h₁
    exact ⟨σ₂, pc₂, zc₂, hruns.append (Runs.one h₁.1.1 hinst hrun), h₂⟩

/-- **A whole segment, with registers and memory.** -/
theorem Segment.run_mem {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hS : MemSetup prog xr tr ps M D init ws) (G : Segment xr tr ps) {k : Nat} (hk : k < M.K)
    (hMk : M.seg k = (G.x, G.t))
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ G.x.Main_segment_next_pc = ((q.toNat : Nat) : F))
    (σ : State) (pc zc : BitVec 64) (h : RunAt M D init ws G k 0 σ pc zc) :
    ∃ σ' pc' zc' xs' W', Runs prog segSteps σ σ' ∧ Ready σ' pc' zc' ∧ G.Exit pc' zc' ∧
      σ'.regs.get? Register.Xs = some xs' ∧ RegInv M xs' (1 + 4 * ((k + 1) * 16777216)) ∧
      MemOk σ' W' D ∧ MemInv D init ws W' (1 + 4 * ((k + 1) * 16777216)) := by
  have hS1 := segSteps_eq
  obtain ⟨σ₁, pc₁, zc₁, hruns, hσ₁, henc, xs, W, hxs, hinv, hmo, hminv⟩ :=
    G.run_mem_within hS hk hMk (segSteps - 1) (by omega) σ pc zc h
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
  have h2 : segSteps = 4 * Main.numRows := rfl
  have hb : memBase k (Main.numRows - 1) 3 = 1 + 4 * (k * 16777216 + (segSteps - 1)) := by
    simp only [memBase]; omega
  obtain ⟨inst, σ', pc', c, xs', W', hinst, hrun, hσ', hx, hinv', hmo', hminv', hnp, hc0, hc1⟩ :=
    step_mem_at prog hS.prog_ok xr tr hS.rom_rows hS.rom ps hS.ops M hS.chain hS.small hS.ans hS.dom
      hk (by rw [hMk]; exact G.rows) (by rw [hMk]; exact G.rom) (by rw [hMk]; exact G.ops)
      hm (l := 3) (by omega) (by rw [hMk]; exact hlane) hq σ₁ pc₁ zc₁ xs W hσ₁ hxs hmo
      (by rw [hMk]; simp only [Pos.last] at hcol; rw [hrow] at hcol; exact hcol)
      ⟨hp0, hp1⟩ (by rw [hb]; exact hinv) (by rw [hb]; exact hminv)
  rw [hMk] at hc0 hc1
  rw [hb] at hinv' hminv'
  have e0 := (G.rows ((Main.numRows : Int) - 1)).c106
  have e1 := (G.rows ((Main.numRows : Int) - 1)).c107
  rw [hrow] at e0 e1
  simp only [Main.c106, Main.c107, hl1] at e0 e1
  have he : 1 + 4 * ((k + 1) * 16777216) = 1 + 4 * (k * 16777216 + (segSteps - 1)) + 4 := by omega
  refine ⟨σ', pc', c, xs', W', ?_, hσ', ⟨hnp, ?_, ?_⟩, hx, by rw [he]; exact hinv', hmo',
    by rw [he]; exact hminv'⟩
  · have := hruns.append (Runs.one hσ₁.1 hinst hrun)
    rwa [Nat.sub_add_cancel (by omega)] at this
  · have : G.x.Main_segment_last_c 0 = (G.t ((Main.numRows - 1 : Nat) : Int)).c 3 0 := by grind
    rw [this]; exact hc0
  · have : G.x.Main_segment_last_c 1 = (G.t ((Main.numRows - 1 : Nat) : Int)).c 3 1 := by grind
    rw [this]; exact hc1

/-- Memory starts at its initial words: before every store, `memVal` is
`init`. -/
theorem memInv_init {M : MainSegs F} {D : Nat → Prop} {init : Nat → F × F}
    {ws : List (Emit (F × F))} (hA : MemAnswers M D init ws) {W : Nat → BitVec 64}
    (hW : ∀ A, D A → (lo (W A), hi (W A)) = init A) : MemInv D init ws W 1 := by
  intro A hDA
  rw [memVal_init (fun w hw => by have := (hA.only w hw).1; omega), hW A hDA]

/-- **Whole segments from boot, with registers and memory.** -/
theorem run_segments_mem {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hS : MemSetup prog xr tr ps M D init ws) (boot : BitVec 64) (segs : List (Segment xr tr ps))
    (hc : ChainHolds boot segs) (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) {W₀ : Nat → BitVec 64}
    (hm₀ : MemOk σ₀ W₀ D) (hW₀ : ∀ A, D A → (lo (W₀ A), hi (W₀ A)) = init A) :
    ∀ k (hk : k < segs.length), ∃ σ pc zc, Runs prog (k * segSteps) σ₀ σ ∧
      RunAt M D init ws segs[k] k 0 σ pc zc := by
  intro k
  induction k with
  | zero =>
    intro h
    refine ⟨σ₀, boot, 0, by rw [Nat.zero_mul]; exact .refl σ₀, h₀,
      segs[0].entry_encodes (hc.entry_first h), _, W₀, hx₀, regInv_zero hS.chain, hm₀,
      memInv_init hS.ans hW₀⟩
  | succ k ih =>
    intro hk
    obtain ⟨σ, pc, zc, hruns, hat⟩ := ih (by omega)
    obtain ⟨σ', pc', zc', xs', W', hrun, hσ', hex, hx, hinv, hmo, hminv⟩ :=
      segs[k].run_mem hS (by rw [hM.1]; omega) (hM.2 k (by omega))
        (hc.next_pc32 prog hS.rom_rows hS.rom hk) σ pc zc hat
    refine ⟨σ', pc', zc', ?_, hσ', segs[k + 1].entry_encodes (hc.entry_next hk hex), xs', W', hx,
      by rwa [Nat.add_zero], hmo, by rwa [Nat.add_zero]⟩
    rw [Nat.succ_mul]; exact hruns.append hrun

/-- **A run from boot, with the register file and memory.** Along a chain of
segments from the boot state — registers 0, memory the words `W₀`, whose
halves are `init` — every step the segments hold is a step of the model, now
with memory sources and stores too: for any `n` below their total, the model
runs `n` steps of the program from the boot state, and the state it reaches
has the chain's registers and memory words `memVal init ws` at memory step
`1 + 4n`.

What remains assumed, besides `run_from_boot_regs`'s: the memory bus's
guarantee for Main's accesses (`MemAnswers`), over the word addresses `D`
(`MemDomain`), and the pilot's scope with memory (`MemProg`). -/
theorem run_from_boot_memory {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hS : MemSetup prog xr tr ps M D init ws) (boot : BitVec 64) (segs : List (Segment xr tr ps))
    (hc : ChainHolds boot segs) (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) {W₀ : Nat → BitVec 64}
    (hm₀ : MemOk σ₀ W₀ D) (hW₀ : ∀ A, D A → (lo (W₀ A), hi (W₀ A)) = init A)
    (n : Nat) (hn : n < segs.length * segSteps) :
    ∃ σ pc zc xs W, Runs prog n σ₀ σ ∧ Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧
      RegInv M xs (1 + 4 * n) ∧ MemOk σ W D ∧ MemInv D init ws W (1 + 4 * n) := by
  have hS1 := segSteps_eq
  have hk : n / segSteps < segs.length := div_lt_len hn
  obtain ⟨σ, pc, zc, hruns, hat⟩ :=
    run_segments_mem hS boot segs hc hM σ₀ h₀ hx₀ hm₀ hW₀ (n / segSteps) hk
  obtain ⟨σ', pc', zc', hrun, hσ', -, xs, W, hx, hinv, hmo, hminv⟩ :=
    segs[n / segSteps].run_mem_within hS (by rw [hM.1]; omega) (hM.2 _ hk) (n % segSteps)
      (Nat.mod_lt _ (by omega)) σ pc zc hat
  have e : 1 + 4 * (n / segSteps * 16777216 + n % segSteps) = 1 + 4 * n := by
    rw [hS1]; omega
  refine ⟨σ', pc', zc', xs, W, ?_, hσ', hx, by rw [← e]; exact hinv, hmo, by rw [← e]; exact hminv⟩
  have := hruns.append hrun
  rwa [Nat.mul_comm, Nat.div_add_mod] at this

end Segment

end Correspondence
