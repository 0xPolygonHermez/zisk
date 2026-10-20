/-
  The public outputs: `pubout`'s provers.

  No AIR proves `pubout` (`0xA0`) on the operation bus: the global updates do,
  one per public input, `(0xA0, i, 0, inputs[2i], inputs[2i+1], inputs[2i],
  inputs[2i+1])` (`zisk.pil`) — `c = b`, and the flag, absent, 0. Each is a
  prover of its own (`pubGlobals`): it proves `pubout` correctly and none of
  the other modeled ops. With them and ZisK's provers (`System.ok`,
  `System.pub`), `run_to_end_memory`'s hypotheses on the operation bus are
  discharged (`run_to_end_memory_system`).
-/
import Proofs.Provers
import Proofs.MemEnd

open Out.Functions

namespace Correspondence

open Pil.Zisk MemSort

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The global update proving public output `i`, of halves `v0`, `v1`. -/
def pubArg (i : Nat) (v0 v1 : F) : BusArg F Unit Unit where
  opids := [5000]
  bus := fun _ _ _ => 5000
  piop := .direct
  side := .proves
  reps := fun _ _ _ => 1
  once := true
  tuple := [fun _ _ _ => 160, fun _ _ _ => ((i : Nat) : F), fun _ _ _ => 0, fun _ _ _ => v0,
    fun _ _ _ => v1, fun _ _ _ => v0, fun _ _ _ => v1]

/-- The global updates proving the `n` public outputs, from the public inputs. -/
def pubGlobals (inputs : Nat → F) (n : Nat) : List (Prover F) :=
  (List.range n).map fun i => ⟨pubArg i (inputs (2 * i)) (inputs (2 * i + 1)), (), ()⟩

theorem pub_ne (k : Nat) (hk : k < 160) : (160 : F) ≠ (((k : Int)) : F) := by
  rw [Lean.Grind.Ring.intCast_natCast, natCast_ofNat' 160]
  intro h
  have := natCast_inj33 (F := F) (by omega) (by omega) h
  omega

/-- **A public output's update proves `pubout` correctly**, and no other
modeled op. -/
theorem pubArg_ok (i : Nat) (v0 v1 : F) :
    OpsOk (⟨pubArg i v0 v1, (), ()⟩ : Prover F) ∧ OpProves 0xA0 pubSem (⟨pubArg i v0 v1, (), ()⟩ : Prover F) := by
  have nt : ∀ k, k < 160 → ∀ j, (pubArg (F := F) i v0 v1).reps () () j ≠ 0 →
      opOf (pubArg i v0 v1) () () j ≠ (((k : Nat) : Int) : F) := fun k hk j _ => by
    simp only [opOf, pubArg, List.headD]
    exact pub_ne k hk
  refine ⟨⟨opProves_of_not (nt 10 (by omega)), opProves_of_not (nt 6 (by omega)),
    opProves_of_not (nt 9 (by omega)), opProves_of_not (nt 14 (by omega))⟩, ?_⟩
  intro j _ _ _ A B _ _ hb0 hb1
  simp only [tupleAt, pubArg, List.getD_cons_zero, List.getD_cons_succ, List.getD_nil] at hb0 hb1 ⊢
  exact ⟨hb0, hb1, by simp [pubSem, bit]⟩

theorem pubGlobals_ok (inputs : Nat → F) (n : Nat) :
    OpsProved (pubGlobals inputs n) ∧ PubProved (pubGlobals inputs n) := by
  refine ⟨fun p hp => ?_, fun p hp => ?_⟩ <;>
  · obtain ⟨i, -, rfl⟩ := List.mem_map.mp hp
    first | exact (pubArg_ok i _ _).1 | exact (pubArg_ok i _ _).2

/-- **The run to the end with registers and memory, over ZisK's provers.**
`run_to_end_memory` with the operation bus answered by the system's provers
and the public outputs' global updates: `OpsProved` and `PubProved` are no
longer assumed, and in their place are the tables' contents, which
`check_tables.py` checks. -/
theorem run_to_end_memory_system (S : System F) (hT : Tables S.xv S.tv)
    (hF : ∀ p ∈ frops S.x0 S.t0, FropsOk p) (inputs : Nat → F)
    {prog : BitVec 64 → Option zisk_inst} (hprog : MemProgEnd prog)
    {xr : Rom.Ctx F} {tr : Rom.Trace F}
    (hr : ∀ j, Rom.everyRow xr tr j) (hrom : RomHolds prog tr)
    {M : MainSegs F} (hch : ChainOk M) (hK : M.K < 1048576)
    {D : Nat → Prop} {init : Nat → F × F} {ws : List (Emit (F × F))}
    (hA : MemAnswers M D init ws) (hD : MemDomain D)
    (boot fin : BitVec 64) (segs : List (Segment xr tr (S.provers ++ pubGlobals inputs 32)))
    (hc : ChainHolds boot segs) (hcl : ChainCloses fin segs) (hlen : 0 < segs.length)
    (hfin : fin.toNat < 2 ^ 32) (hfe : ∃ e, prog fin = some e ∧ e.is_end = true)
    (hM : SegsAre M segs) (σ₀ : State) (h₀ : Ready σ₀ boot 0)
    (hx₀ : σ₀.regs.get? Register.Xs = some (Vector.replicate 32 0)) {W₀ : Nat → BitVec 64}
    (hm₀ : MemOk σ₀ W₀ D) (hW₀ : ∀ A, D A → (lo (W₀ A), hi (W₀ A)) = init A) :
    ∃ m σ pc zc xs W e σe, m ≤ segs.length * segSteps ∧ Runs prog m σ₀ σ ∧
      Ready σ pc zc ∧ σ.regs.get? Register.Xs = some xs ∧ RegInv M xs (1 + 4 * m) ∧
      MemOk σ W D ∧ MemInv D init ws W (1 + 4 * m) ∧
      prog pc = some e ∧ e.is_end = true ∧ zisk_execute e σ = .ok true σe := by
  have hg := pubGlobals_ok inputs 32
  exact run_to_end_memory hprog hr hrom (opsProved_append (S.ok hT hF) hg.1)
    (pubProved_append (S.pub hT hF) hg.2) hch hK hA hD boot fin segs hc hcl hlen hfin hfe hM σ₀ h₀
    hx₀ hm₀ hW₀

end Correspondence
