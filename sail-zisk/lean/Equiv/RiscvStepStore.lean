/-
  One step of sail-riscv on an instruction that writes memory and no
  register — a store: as Equiv/RiscvStep.lean's `try_step_alu`, with the
  memory the instruction leaves in place of the register it writes.
-/
import Equiv.RiscvStep
import Equiv.MemWrite

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 4000000 in
/-- One step of sail-riscv, in ZisK's profile, on an instruction that
    writes memory only: the word at pc decodes to `ins` (in machine mode),
    and `ins` leaves memory `M` and changes nothing else (from any state
    whose x registers and memory are s's). Then pc moves to pc + 4, memory
    is M, and no register but PC, nextPC, minstret and minstret_increment
    changes. -/
theorem try_step_store (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? cur_privilege = some .Machine →
      (∃ v, t.regs.get? mseccfg = some v) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_M v = 1#1) →
      (∃ v, t.regs.get? misa = some v ∧ _get_Misa_B v = 1#1) →
      (interpretSailEffects (ext_decode (b3 ++ b2 ++ (b1 ++ b0)))).run t = .ok ins t)
    (M : Equiv.Mem)
    (hexec : ∀ t : RVState, (∀ k, k < 32 → getX t k = getX s k) → t.mem = s.mem → MemReady t rs →
      t.regs.get? mseccfg = s.regs.get? mseccfg → WF t →
      (interpretSailEffects (execute ins)).run t = .ok RETIRE_SUCCESS { t with mem := M }) (n : Nat) :
    ∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.mem = M ∧ WF s' ∧ s'.regs.get? PC = some (pc + 4) ∧
      (∀ k, k < 32 → getX s' k = getX s k) ∧
      (∀ r, isXReg r = false → r ≠ PC → r ≠ nextPC → r ≠ minstret → r ≠ minstret_increment →
        s'.regs.get? r = s.regs.get? r) := by
  obtain ⟨h, hmip, hmie, hhs, help, hs, -, ⟨mv, hmv, hmM⟩, ⟨bv, hbv, hbB⟩, -⟩ := hr
  obtain ⟨hal, ⟨r1, hm1, hx1⟩, ⟨r2, hm2, hx2⟩, h0, h1, h2, h3, hc⟩ := hw
  have hp := hs.priv
  simp only [try_step, run_hart_active, should_inc_minstret, counter_priv_filter_bit, dispatchInterrupt,
    getPendingSet, read_mip, is_landing_pad_expected, hart_is_active, hart_is_waiting, tick_pc,
    get_config_rvfi, get_config_print_instr, currentlyEnabled, hartSupports, external_interrupts_pending,
    supervisor_software_interrupt_pending, plat_have_sig]
  rw [interp_bind, run_bind', update_mip_run s h hmip]; dsimp only
  repeat rv_step h
  rw [fetch_word _ rs ?hs1 pc ?hpc1 hal r1 r2 hm1 hm2 hx1 hx2 b0 b1 b2 b3 ?h0 ?h1 ?h2 ?h3 hc]
  rotate_left
  · exact hs.congr (by
      intro r hr
      simp only [memRegs, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [Std.ExtDHashMap.get?_insert])
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hpc
  · exact h0
  · exact h1
  · exact h2
  · exact h3
  dsimp only [ext_fetch_hook]
  repeat rv_step h
  rw [hdec _ ?hp1 ?hsec1 ?hm1 ?hb1]
  rotate_left
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hp
  · exact ⟨regVal h mseccfg, by simp only [Std.ExtDHashMap.get?_insert]; simpa using h.get? mseccfg⟩
  · exact ⟨mv, by simp only [Std.ExtDHashMap.get?_insert]; simpa using hmv, hmM⟩
  · exact ⟨bv, by simp only [Std.ExtDHashMap.get?_insert]; simpa using hbv, hbB⟩
  repeat rv_step h
  simp only [no_lp, Bool.false_and, Bool.false_eq_true, if_false]
  repeat rv_step h
  rw [hexec _ ?gx ?gm ?gr ?gs ?gw]
  rotate_left
  · intro k _
    simp (disch := decide) only [getX, Std.ExtDHashMap.get?_insert, beq_xreg_false, Bool.false_eq_true,
      dite_false]
  · rfl
  · exact hs.congr (by
      intro r hr
      simp only [memRegs, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [Std.ExtDHashMap.get?_insert])
  · simp [Std.ExtDHashMap.get?_insert]
  · exact ⟨isSome_insert _ _ _ (isSome_insert _ _ _ h.all)⟩
  simp only [RETIRE_SUCCESS]
  repeat rv_step h
  split
  all_goals (repeat rv_step h)
  all_goals refine ⟨_, rfl, rfl, ⟨?_⟩, ?_, ?_, ?_⟩
  all_goals try ((repeat apply isSome_insert); exact h.all)
  all_goals try (simp [Std.ExtDHashMap.get?_insert, BitVec.addInt]; done)
  all_goals try (intro r _ hpc' hn hm hmi
                 simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, Ne.symm hpc',
                   Ne.symm hn, Ne.symm hm, Ne.symm hmi, dite_false, WF.get? h]
                 done)
  all_goals intro k hk32
  all_goals simp only [getX]
  all_goals split
  all_goals try rfl
  all_goals simp (disch := decide) only [Std.ExtDHashMap.get?_insert, beq_xreg_false, Bool.false_eq_true,
                     dite_false]
  all_goals rfl

end Equiv.Riscv
