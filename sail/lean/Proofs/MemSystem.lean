/-
  The run to the end, with the memory bus.

  `run_to_end_memory_system` takes `MemAnswers` as a hypothesis;
  `MemBusWorld.answers` derives it from the memory bus. Together: the run to
  the end with registers and memory, over ZisK's provers on the operation bus
  and the RAM, InputData, the Rom and RomData on the memory bus and the
  continuation buses, its initial memory the input memory and the program's
  data.
-/
import Proofs.PubGlobal
import Proofs.MemCont

open Out.Functions

namespace Correspondence

open Pil.Zisk MemSort

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- **The boot instruction loads nothing from memory**: a property of the
program, which `check_scope.py` checks. -/
def BootNoLoad (prog : BitVec 64 → Option zisk_inst) (boot : BitVec 64) : Prop :=
  ∀ e, prog boot = some e → e.a_src ≠ .A_MEM ∧ e.b_src ≠ .B_MEM ∧ e.b_src ≠ .B_IND

/-- **Main's first lane runs the boot instruction**, so it loads nothing:
`FirstNoLoad` from `BootNoLoad`. -/
theorem first_no_load {prog : BitVec 64 → Option zisk_inst} {xr : Rom.Ctx F} {tr : Rom.Trace F}
    {ps : List (Prover F)} (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    {M : MainSegs F} {boot : BitVec 64} {segs : List (Segment xr tr ps)} (hc : ChainHolds boot segs)
    (hM : SegsAre M segs) (hlen : 0 < segs.length) (hb32 : boot.toNat < 2 ^ 32)
    (hboot : BootNoLoad prog boot) : FirstNoLoad M := by
  have henc := segs[0].entry_encodes (hc.entry_first hlen)
  obtain ⟨inst, hinst, hslot, -⟩ := rom_lane' prog segs[0].x segs[0].t 0 (segs[0].rows 0) xr tr hr
    (l := 0) (by omega) (segs[0].rom 0 (by omega)) hrom boot hb32 henc.1
  obtain ⟨ha, hbm, hbi⟩ := hboot inst hinst
  have hseg := hM.2 0 hlen
  have ea : isAMem inst.a_src = false := by cases h : inst.a_src <;> simp_all [isAMem]
  have eb : isBMem inst.b_src = false := by cases h : inst.b_src <;> simp_all [isBMem]
  have ei : isBInd inst.b_src = false := by cases h : inst.b_src <;> simp_all [isBInd]
  unfold FirstNoLoad
  rw [hseg]
  show (segs[0].t 0).a_src_mem 0 ≠ 1 ∧ (segs[0].t 0).b_src_mem 0 + (segs[0].t 0).b_src_ind 0 ≠ 1
  rw [hslot.a_src_mem, hslot.b_src_mem, hslot.b_src_ind, ea, eb, ei]
  simp only [bit, Bool.false_eq_true, if_false, Lean.Grind.Semiring.add_zero]
  exact ⟨Lean.Grind.Field.zero_ne_one, Lean.Grind.Field.zero_ne_one⟩

/-- **The run to the end, over the operation and memory buses.**
`run_to_end_memory_system` with `MemAnswers` discharged: the memory bus is
sound, with no instance of the AIRs that load and store on another's behalf
(`MemBusWorld`); the continuation buses are sound over the RAM's, InputData's
and RomData's instances (`ContWorld`), which chains each area's segments; the
Rom's data is ROM words (`RomOk`); Main's accesses avoid the register file
and the free-input base (`MainAvoids`); the boot instruction loads nothing
(`BootNoLoad`); and the model's initial memory is the bus's (`busInit`):
the input memory, the Rom's data. The memory invariant is then over `busD`,
`busInit` and Main's stores. -/
theorem run_to_end_memory_bus (S : System F) (hT : Tables S.xv S.tv)
    (hF : ∀ p ∈ frops S.x0 S.t0, FropsOk p) (inputs : Nat → F)
    {prog : BitVec 64 → Option zisk_inst} (hprog : MemProgEnd prog)
    {xr : Rom.Ctx F} {tr : Rom.Trace F}
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    {M : MainSegs F} (hch : ChainOk M) (hK : M.K < 1048576)
    {SM : MemOthers F} {gp ga : List (List F)} (W : MemBusWorld M SM gp ga)
    (CW : ContWorld SM) (hromd : RomOk SM) (hav : MainAvoids M)
    (boot fin : BitVec 64) (hboot : BootNoLoad prog boot)
    (segs : List (Segment xr tr (S.provers ++ pubGlobals inputs 32))) (hcs : ChainHolds boot segs) (hcl : ChainCloses fin segs) (hlen : 0 < segs.length)
    (hfin : fin.toNat < 2 ^ 32) (hfe : ∃ e, prog fin = some e ∧ e.is_end = true)
    (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) {W₀ : Nat → BitVec 64}
    (hm₀ : MemOk σ₀ W₀ busD) (hW₀ : ∀ A, busD A → (lo (W₀ A), hi (W₀ A)) = busInit SM A) :
    ∃ m σ pc zc xs Wm e σe, m ≤ segs.length * segSteps ∧ Runs prog m σ₀ σ ∧
      Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧ RegInv M xs (1 + 4 * m) ∧
      MemOk σ Wm busD ∧ MemInv busD (busInit SM) (mainStores M) Wm (1 + 4 * m) ∧
      prog pc = some e ∧ e.is_end = true ∧ zisk_execute e σ = .ok true σe :=
  run_to_end_memory_system S hT hF inputs hprog hr hrom hch hK
    (W.answers_cont CW hromd hav (first_no_load hr hrom hcs hM hlen h₀.2.2.2 hboot)) busD_domain
    boot fin segs hcs hcl hlen hfin hfe hM σ₀ h₀ hx₀ hm₀ hW₀

end Correspondence
