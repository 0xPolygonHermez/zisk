/- sail-riscv's decoder on Zbb's rev8 (RV64's encoding). -/
import Equiv.RiscvDecodeZbkbBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
theorem decode_zbb_rev8 (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x13#7)
    (hf : w.extractLsb 31 20 = 0x6B8#12) (hf3 : w.extractLsb 14 12 = 5#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.REV8 (.Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  repeat skip_i
  try dsimp only
  rw [interp_bind, run_bind']
  try dsimp only
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf, hf3]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  try (rw [ce_zbb s mv hmv hmB]; try dsimp only)
  try (rw [ce_zbkb]; try dsimp only)
  try simp only [Functions.xlen, Bool.true_and, Bool.and_true, beq_self_eq_true, Bool.true_or, if_true,
    interp_pure, run_pure']
  try dsimp only
  try simp only [interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.REV8.injEq, Prod.mk.injEq, regidx.Regidx.injEq,
    and_true]
  refine ⟨?_, ?_⟩ <;> first | trivial | (apply BitVec.eq_of_getLsbD_eq; intro i hi; simp)

end Equiv.Riscv
