/-
  sail-riscv's decoder on the fences: `fence` (and `fence.tso`, its own
  clause) and `fence.i`. With Zihintpause out of the profile, `pause` is a
  plain fence.
-/
import Equiv.RiscvDecodeImm

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A fence word, not fence.tso's, decodes to FENCE with its fields. -/
theorem decode_fence (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x0F#7)
    (hf3 : w.extractLsb 14 12 = 0#3) (htso : w ≠ 0x8330000F#32)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.FENCE (w.extractLsb 31 28, w.extractLsb 27 24, w.extractLsb 23 20,
          .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.FENCE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

set_option maxHeartbeats 0 in
/-- fence.tso's word decodes to FENCE_TSO. -/
theorem decode_fence_tso (s : RVState)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode 0x8330000F#32)).run s = .ok (.FENCE_TSO ()) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos (by decide)]
  rfl

/-- Zifencei, in the profile, which reads no register. -/
theorem ce_zifencei (s : RVState) :
    (interpretSailEffects (currentlyEnabled .Ext_Zifencei)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports]

set_option maxHeartbeats 0 in
/-- A fence.i word decodes to FENCEI with its fields. -/
theorem decode_fencei (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x0F#7)
    (hf3 : w.extractLsb 14 12 = 1#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.FENCEI (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, currentlyEnabled,
      hartSupports]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  rw [ce_zifencei]; try dsimp only
  try simp only [if_true, interp_pure, run_pure']
  try dsimp only
  try simp only [interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.FENCEI.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
