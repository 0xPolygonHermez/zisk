/-
  The program's end, with the register file.

  `run_to_end` (`End.lean`) runs the pilot's programs from boot to an end
  instruction. Here with register sources and stores, and the register file
  carried to the end: the model runs the program to its first end
  instruction, the registers it reaches it with are Main's register chain's,
  and it executes the end instruction — whatever its op, sources and store —
  and stops.

  As in `End.lean`, the trace runs the program as Main sees it (`mainView`).
  The model's own run follows it to the first end instruction
  (`runs_to_first_end`); runs being deterministic (`Runs.det`), the state it
  stops at is the one `run_from_boot_regs` describes.
-/
import Proofs.RegBoot

open Out.Functions PreSail Sail

namespace Correspondence

open Pil.Zisk MemCheck

/-! ### The model -/

/-- **An end instruction, in general.** As `execute_gen`, with `is_end` set:
the model executes it and stops, with the register file its store leaves. -/
theorem execute_gen_end (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (xs : Vector (BitVec 64) 32)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st) (hxs : σ.regs.get? Register.Xs = some xs)
    (ha : ASrcOk inst) (hb : BSrcOk inst) (hs : StoreOk inst)
    (hend : inst.is_end = true) :
    ∃ σ', zisk_execute inst σ = .ok true σ' ∧
      σ'.regs.get? Register.Xs = some (storeXs inst pc
        (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).1 xs) := by
  generalize hr : zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs) = r
  obtain ⟨c, flag⟩ := r
  obtain ⟨σ₁, hs₁, p₁, z₁, t₁, x₁⟩ := store_c_ok inst (aVal inst zc xs) c σ pc xs hpc hxs hs
  have ea := source_a_ok inst σ zc xs hzc hxs ha
  have eb := source_b_ok inst (aVal inst zc xs) σ zc xs hzc hxs hb
  simp only [zisk_execute, bind, EStateM.bind, ea, eb, hr, hs₁, pure, EStateM.pure]
  rw [hst] at t₁
  cases hop : inst.op <;>
  · simp [writeReg_run, readReg_eq, Std.ExtDHashMap.get?_insert, t₁, x₁, hend,
      bind, EStateM.bind, pure, EStateM.pure]

/-! ### Runs -/

/-- Runs are deterministic. -/
theorem Runs.det {prog : BitVec 64 → Option zisk_inst} {n : Nat} {σ σ₁ σ₂ : State}
    (h₁ : Runs prog n σ σ₁) (h₂ : Runs prog n σ σ₂) : σ₁ = σ₂ := by
  induction h₁ with
  | refl => cases h₂; rfl
  | step hpc hinst hrun _ ih =>
    cases h₂ with
    | step hpc' hinst' hrun' hr' =>
      rw [hpc] at hpc'; cases hpc'
      rw [hinst] at hinst'; cases hinst'
      rw [hrun] at hrun'; cases hrun'
      exact ih hr'

/-- **The model follows Main's run to the first end instruction.** If Main's
view of the program runs to a state about to execute an end instruction, the
model's own run reaches the first one within those steps — the same run, as
far as it goes. -/
theorem runs_to_first_end {prog : BitVec 64 → Option zisk_inst} {n : Nat} {σ σ' : State}
    (hrun : Runs (mainView prog) n σ σ')
    (hend : ∃ pc e, σ'.regs.get? Register.PC = some pc ∧ prog pc = some e ∧ e.is_end = true) :
    ∃ m σm pce e, m ≤ n ∧ Runs prog m σ σm ∧ Runs (mainView prog) m σ σm ∧
      σm.regs.get? Register.PC = some pce ∧ prog pce = some e ∧ e.is_end = true := by
  induction hrun with
  | refl σ =>
    obtain ⟨pc, e, hpc, he, hie⟩ := hend
    exact ⟨0, σ, pc, e, Nat.le_refl 0, .refl σ, .refl σ, hpc, he, hie⟩
  | @step n σ₀ σ₁ σ₂ pc inst' hpc hinst hrun' hr ih =>
    have hinst₀ := hinst
    simp only [mainView, Option.map_eq_some_iff] at hinst
    obtain ⟨inst, hp, rfl⟩ := hinst
    cases hie : inst.is_end
    · rw [asMain_of_not_end hie] at hrun' hinst₀
      obtain ⟨m, σm, pce, e, hm, h1, h2, h3, h4, h5⟩ := ih hend
      exact ⟨m + 1, σm, pce, e, by omega, .step hpc hp hrun' h1, .step hpc hinst₀ hrun' h2,
        h3, h4, h5⟩
    · exact ⟨0, σ₀, pc, inst, Nat.zero_le _, .refl σ₀, .refl σ₀, hpc, hp, hie⟩

/-! ### The register-aware pilot, with an end -/

/-- Every instruction of `prog` is in the register-aware pilot's scope, except
that it may end the program: any op, source and store the pilot has. -/
def RegProgEnd (prog : BitVec 64 → Option zisk_inst) : Prop :=
  ∀ pc inst, prog pc = some inst →
    RegPilot (asMain inst) ∧ Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2

theorem RegProgEnd.mainView {prog : BitVec 64 → Option zisk_inst} (h : RegProgEnd prog) :
    RegProg (mainView prog) := by
  intro pc inst' hp
  simp only [Correspondence.mainView, Option.map_eq_some_iff] at hp
  obtain ⟨inst, hinst, rfl⟩ := hp
  exact h pc inst hinst

section Chain

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]
variable {xr : Rom.Ctx F} {tr : Rom.Trace F} {ps : List (Prover F)}

/-- **Every segment, with the register file.** Along a chain that closes at
`fin`, all its segments force `len·4·numRows` steps of the model from the boot
state; the last exit airvals encode the state they reach, whose registers are
the chain's at memory step `1 + 4·len·4·numRows`. -/
theorem run_all_segments_regs {prog : BitVec 64 → Option zisk_inst} {M : MainSegs F}
    (hS : RegSetup prog xr tr ps M) (boot fin : BitVec 64) (segs : List (Segment xr tr ps))
    (hc : ChainHolds boot segs) (hcl : ChainCloses fin segs) (hfin : fin.toNat < 2 ^ 32)
    (hlen : 0 < segs.length) (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) :
    ∃ σ pc zc xs, Runs prog (segs.length * segSteps) σ₀ σ ∧ Ready σ pc zc ∧
      (segs[segs.length - 1]'(by omega)).Exit pc zc ∧ σ.regs.get? Register.Xs = some xs ∧
      RegInv M xs (1 + 4 * (segs.length * segSteps)) := by
  obtain ⟨σ, pc, zc, hruns, hat⟩ :=
    run_segments_reg hS boot segs hc hM σ₀ h₀ hx₀ (segs.length - 1) (by omega)
  have hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧
      (segs[segs.length - 1]'(by omega)).x.Main_segment_next_pc = ((q.toNat : Nat) : F) := by
    have e3 := hcl hlen 3
    simp only [tupleAt, Main.bus715, endTuple, List.getD_cons_zero, List.getD_cons_succ,
      Lean.Grind.Semiring.add_zero] at e3
    exact ⟨fin, hfin, e3⟩
  obtain ⟨σ', pc', zc', xs', hrun, hσ', hex, hx, hinv⟩ :=
    (segs[segs.length - 1]'(by omega)).run_reg hS (by rw [hM.1]; omega)
      (hM.2 _ (by omega)) hq σ pc zc hat
  refine ⟨σ', pc', zc', xs', ?_, hσ', hex, hx, ?_⟩
  · have := hruns.append hrun
    rwa [← Nat.succ_mul, Nat.succ_eq_add_one, Nat.sub_add_cancel hlen] at this
  · rwa [segSteps_eq, Nat.sub_add_cancel hlen] at *

/-- **A run from boot to the end, with the register file.** Along a chain of
segments from the boot state, every register 0, that the global update closes
at `fin`, where the program has its end instruction: the model runs the
program to an end instruction, within the steps the segments hold; the
registers `x1`–`x31` it reaches it with, after `m` steps, are what Main's
register chain holds at memory step `1 + 4m`; and it executes the end
instruction, with the register file its store leaves, and stops.

What remains assumed: `run_from_boot_regs`'s, with the chain closed
(`ChainCloses`) and the end instruction in the register-aware pilot's scope. -/
theorem run_to_end_regs {prog : BitVec 64 → Option zisk_inst} (hprog : RegProgEnd prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    {M : MainSegs F} (hch : ChainOk M) (hK : M.K < 1048576)
    (boot fin : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (hcl : ChainCloses fin segs) (hlen : 0 < segs.length) (hfin : fin.toNat < 2 ^ 32)
    (hfe : ∃ e, prog fin = some e ∧ e.is_end = true) (hM : SegsAre M segs)
    (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) :
    ∃ m σ pc zc xs e σe, m ≤ segs.length * segSteps ∧ Runs prog m σ₀ σ ∧
      Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧ RegInv M xs (1 + 4 * m) ∧
      prog pc = some e ∧ e.is_end = true ∧ zisk_execute e σ = .ok true σe ∧
      σe.regs.get? Register.Xs = some (storeXs e pc
        (zisk_exec e.op (aVal e zc xs) (bVal e zc xs)).1 xs) := by
  have hS : RegSetup (mainView prog) xr tr ps M :=
    ⟨hprog.mainView, hr, hrom.mainView, hpr, hch, hK⟩
  obtain ⟨σ, pc, zc, xs, hruns, hσ, hex, hx, hinv⟩ :=
    run_all_segments_regs hS boot fin segs hc hcl hfin hlen hM σ₀ h₀ hx₀
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
  -- The state the model stops at, with its registers.
  have hstate : ∃ zc xs, Ready σm pce zc ∧ σm.regs.get? Register.Xs = some xs ∧
      RegInv M xs (1 + 4 * m) := by
    rcases Nat.lt_or_ge m (segs.length * segSteps) with hlt | hge
    · obtain ⟨σ', pc', zc', xs', hr', hσ', hx', hinv', -⟩ :=
        run_from_boot_regs hS boot segs hc hM σ₀ h₀ hx₀ m hlt
      rw [← Runs.det hrm hr'] at hσ' hx'
      rw [hσ'.1] at hpce; cases hpce
      exact ⟨zc', xs', hσ', hx', hinv'⟩
    · have hmeq : m = segs.length * segSteps := by omega
      subst hmeq
      rw [← Runs.det hrm hruns] at hσ hx
      rw [hσ.1] at hpce; cases hpce
      exact ⟨zc, xs, hσ, hx, hinv⟩
  obtain ⟨zcm, xsm, hσm, hxm, hinvm⟩ := hstate
  obtain ⟨hpil, -, -⟩ := hprog pce e' he'
  obtain ⟨st, hst⟩ := hσm.2.2.1
  obtain ⟨σe, hexe, hxe⟩ := execute_gen_end e' σm pce zcm st xsm hσm.1 hσm.2.1 hst hxm
    hpil.a hpil.b hpil.store hie'
  exact ⟨m, σm, pce, zcm, xsm, e', σe, hm, hrp, hσm, hxm, hinvm, he', hie', hexe, hxe⟩

/-- **A run from boot to the end with the register file, over the memory
bus.** `run_to_end_regs`, with the register chain's facts from a sound memory
bus, as `run_from_boot_mem` derives them. -/
theorem run_to_end_mem {prog : BitVec 64 → Option zisk_inst} (hprog : RegProgEnd prog)
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr) (hpr : OpsProved ps)
    (boot fin : BitVec 64) (segs : List (Segment xr tr ps)) (hc : ChainHolds boot segs)
    (hcl : ChainCloses fin segs) (hlen : 0 < segs.length) (hfin : fin.toNat < 2 ^ 32)
    (hfe : ∃ e, prog fin = some e ∧ e.is_end = true)
    (hK : segs.length < 1048576)
    (hstep : ∀ k (h : k < segs.length), ∀ m : Nat, m < Main.numRows →
      (segs[k].t m).Main_SEGMENT_STEP = ((m : Nat) : F))
    (hrng : ∀ k (h : k < segs.length), ∀ g : PairTag,
      RangeHolds Main.ranges g.rangeArg segs[k].x segs[k].t)
    (M : MainSegs F) (hM : SegsAre M segs) (S : MemOthers F) {gp ga : List (List F)}
    (hgl : GlobalRegs gp ga)
    (hbus : SumHolds 10 (M.list.flatMap (fun m => mainMemSenders m.1 m.2) ++ S.senders) gp ga)
    (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) :
    ∃ m σ pc zc xs e σe, m ≤ segs.length * segSteps ∧ Runs prog m σ₀ σ ∧
      Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧ RegInv M xs (1 + 4 * m) ∧
      prog pc = some e ∧ e.is_end = true ∧ zisk_execute e σ = .ok true σe ∧
      σe.regs.get? Register.Xs = some (storeXs e pc
        (zisk_exec e.op (aVal e zc xs) (bVal e zc xs)).1 xs) := by
  have hok := hM.ok hc hK hstep hrng
  exact run_to_end_regs hprog hr hrom hpr (fun i hi => M.reg_chain hok S hgl hbus (by omega))
    hok.small boot fin segs hc hcl hlen hfin hfe hM σ₀ h₀ hx₀

end Chain

end Correspondence
