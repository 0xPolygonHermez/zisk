/-
  The program's end, with the register file and memory.

  `RegEnd.lean` with memory: the trace runs the program as Main sees it
  (`mainView`), the model's own run follows it to the first end instruction,
  and runs being deterministic, the state it stops at is the one
  `run_from_boot_memory` describes — registers the chain's, memory words
  `memVal` of Main's stores. The end instruction itself makes no memory
  access, as ZisK's `ROM_EXIT` does not.
-/
import Proofs.MemBoot
import Proofs.RegEnd

open Out.Functions

namespace Correspondence

open Pil.Zisk MemCheck MemSort

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

/-- Every instruction of `prog` is in the scope of the pilot with memory,
except that it may end the program, with no memory access. -/
def MemProgEnd (prog : BitVec 64 → Option zisk_inst) : Prop :=
  ∀ pc inst, prog pc = some inst →
    MemPilot (asMain inst) ∧ Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2 ∧
      (inst.is_end = true → ASrcOk inst ∧ BSrcOk inst ∧ StoreOk inst)

theorem MemProgEnd.mainView {prog : BitVec 64 → Option zisk_inst} (h : MemProgEnd prog) :
    MemProg (mainView prog) := by
  intro pc inst' hp
  simp only [Correspondence.mainView, Option.map_eq_some_iff] at hp
  obtain ⟨inst, hinst, rfl⟩ := hp
  obtain ⟨h1, h2, h3, -⟩ := h pc inst hinst
  exact ⟨h1, h2, h3⟩

section Chain

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]
variable {xr : Rom.Ctx F} {tr : Rom.Trace F} {ps : List (Prover F)}

/-- **Every segment, with registers and memory.** -/
theorem run_all_segments_mem {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hS : MemSetup prog xr tr ps M D init ws) (boot fin : BitVec 64) (segs : List (Segment xr tr ps))
    (hc : ChainHolds boot segs) (hcl : ChainCloses fin segs) (hfin : fin.toNat < 2 ^ 32)
    (hlen : 0 < segs.length) (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) {W₀ : Nat → BitVec 64}
    (hm₀ : MemOk σ₀ W₀ D) (hW₀ : ∀ A, D A → (lo (W₀ A), hi (W₀ A)) = init A) :
    ∃ σ pc zc xs W, Runs prog (segs.length * segSteps) σ₀ σ ∧ Ready σ pc zc ∧
      (segs[segs.length - 1]'(by omega)).Exit pc zc ∧ σ.regs.get? Register.Xs = some xs ∧
      RegInv M xs (1 + 4 * (segs.length * segSteps)) ∧ MemOk σ W D ∧
      MemInv D init ws W (1 + 4 * (segs.length * segSteps)) := by
  obtain ⟨σ, pc, zc, hruns, hat⟩ :=
    run_segments_mem hS boot segs hc hM σ₀ h₀ hx₀ hm₀ hW₀ (segs.length - 1) (by omega)
  have hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧
      (segs[segs.length - 1]'(by omega)).x.Main_segment_next_pc = ((q.toNat : Nat) : F) := by
    have e3 := hcl hlen 3
    simp only [tupleAt, Main.bus715, endTuple, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at e3
    exact ⟨fin, hfin, e3⟩
  obtain ⟨σ', pc', zc', xs', W', hrun, hσ', hex, hx, hinv, hmo, hminv⟩ :=
    (segs[segs.length - 1]'(by omega)).run_mem hS (by rw [hM.1]; omega)
      (hM.2 _ (by omega)) hq σ pc zc hat
  have e : 1 + 4 * ((segs.length - 1 + 1) * 16777216) = 1 + 4 * (segs.length * segSteps) := by
    rw [segSteps_eq, Nat.sub_add_cancel hlen]
  refine ⟨σ', pc', zc', xs', W', ?_, hσ', hex, hx, by rw [← e]; exact hinv, hmo,
    by rw [← e]; exact hminv⟩
  have := hruns.append hrun
  rwa [← Nat.succ_mul, Nat.succ_eq_add_one, Nat.sub_add_cancel hlen] at this

/-- **A run from boot to the end, with registers and memory.** Along a chain
of segments from the boot state — registers 0, memory the words `W₀` — that
the global update closes at `fin`, where the program has its end instruction:
the model runs the program to an end instruction within the steps the
segments hold; the state it reaches it with, after `m` steps, has the chain's
registers and memory words `memVal init ws` at memory step `1 + 4m`; and it
executes the end instruction and stops.

What remains assumed: `run_to_end_regs`'s, and the memory bus's guarantee for
Main's accesses (`MemAnswers`) over the word addresses `D`, with the pilot's
scope with memory (`MemProgEnd`). -/
theorem run_to_end_memory {prog : BitVec 64 → Option zisk_inst} (hprog : MemProgEnd prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    {M : MainSegs F} (hch : ChainOk M) (hK : M.K < 1048576)
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hA : MemAnswers M D init ws) (hD : MemDomain D)
    (boot fin : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (hcl : ChainCloses fin segs) (hlen : 0 < segs.length) (hfin : fin.toNat < 2 ^ 32)
    (hfe : ∃ e, prog fin = some e ∧ e.is_end = true) (hM : SegsAre M segs)
    (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) {W₀ : Nat → BitVec 64}
    (hm₀ : MemOk σ₀ W₀ D) (hW₀ : ∀ A, D A → (lo (W₀ A), hi (W₀ A)) = init A) :
    ∃ m σ pc zc xs W e σe, m ≤ segs.length * segSteps ∧ Runs prog m σ₀ σ ∧
      Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧ RegInv M xs (1 + 4 * m) ∧
      MemOk σ W D ∧ MemInv D init ws W (1 + 4 * m) ∧
      prog pc = some e ∧ e.is_end = true ∧ zisk_execute e σ = .ok true σe := by
  have hS : MemSetup (mainView prog) xr tr ps M D init ws :=
    ⟨hprog.mainView, hr, hrom.mainView, hpr, hch, hK, hA, hD⟩
  obtain ⟨σ, pc, zc, xs, W, hruns, hσ, hex, hx, hinv, hmo, hminv⟩ :=
    run_all_segments_mem hS boot fin segs hc hcl hfin hlen hM σ₀ h₀ hx₀ hm₀ hW₀
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
  obtain ⟨m, σm, pce, e', hm, hrp, hrm, hpce, he', hie'⟩ :=
    runs_to_first_end hruns ⟨pc, e, hσ.1, hpcfin ▸ he, hie⟩
  -- The state the model stops at, with its registers and memory.
  have hstate : ∃ zc xs W, Ready σm pce zc ∧ σm.regs.get? Register.Xs = some xs ∧
      RegInv M xs (1 + 4 * m) ∧ MemOk σm W D ∧ MemInv D init ws W (1 + 4 * m) := by
    rcases Nat.lt_or_ge m (segs.length * segSteps) with hlt | hge
    · obtain ⟨σ', pc', zc', xs', W', hr', hσ', hx', hinv', hmo', hminv'⟩ :=
        run_from_boot_memory hS boot segs hc hM σ₀ h₀ hx₀ hm₀ hW₀ m hlt
      rw [← Runs.det hrm hr'] at hσ' hx' hmo'
      rw [hσ'.1] at hpce; cases hpce
      exact ⟨zc', xs', W', hσ', hx', hinv', hmo', hminv'⟩
    · have hmeq : m = segs.length * segSteps := by omega
      subst hmeq
      rw [← Runs.det hrm hruns] at hσ hx hmo
      rw [hσ.1] at hpce; cases hpce
      exact ⟨zc, xs, W, hσ, hx, hinv, hmo, hminv⟩
  obtain ⟨zcm, xsm, Wm, hσm, hxm, hinvm, hmom, hminvm⟩ := hstate
  obtain ⟨-, -, -, hend⟩ := hprog pce e' he'
  obtain ⟨ha, hb, hs⟩ := hend hie'
  obtain ⟨st, hst⟩ := hσm.2.2.1
  obtain ⟨σe, hexe, -⟩ := execute_gen_end e' σm pce zcm st xsm hσm.1 hσm.2.1 hst hxm ha hb hs hie'
  exact ⟨m, σm, pce, zcm, xsm, Wm, e', σe, hm, hrp, hσm, hxm, hinvm, hmom, hminvm, he', hie', hexe⟩

end Chain

end Correspondence
