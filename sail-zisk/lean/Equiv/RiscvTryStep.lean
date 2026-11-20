/-
  sail-riscv's step, whatever its arguments: `try_step step_no exit_wait`
  reads `step_no` only to print it, and `exit_wait` only for a waiting hart.
  So in a state ready for a step (the hart active), every `try_step n b`
  runs as `try_step 0 false`, the one the step lemmas are about — and the
  machine's step, `try_step 0 true` (ZiskRiscvRun/Model.lean), among them.
-/
import Equiv.RiscvStep

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

theorem run_hart_active_any (n : Nat) : run_hart_active n = run_hart_active 0 := by
  unfold run_hart_active; rfl

set_option maxHeartbeats 4000000 in
theorem try_step_any (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs) (n : Nat) (b : Bool) :
    (interpretSailEffects (try_step n b)).run s = (interpretSailEffects (try_step 0 false)).run s := by
  obtain ⟨h, hmip, -, hhs, -, hs, -, -, -⟩ := hr
  have hp := hs.priv
  simp only [try_step, run_hart_active_any n, should_inc_minstret, counter_priv_filter_bit]
  -- each side up to its hart's state, active: then the two are the same
  rw [interp_bind, run_bind', update_mip_run s h hmip]; dsimp only
  repeat rv_step h
  symm
  rw [update_mip_run s h hmip]; dsimp only
  repeat rv_step h

end Equiv.Riscv
