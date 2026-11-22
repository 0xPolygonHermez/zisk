/- sail-riscv's decoder on amomax.d. -/
import Equiv.RiscvDecodeAmoBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
theorem decode_amo_max_d (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x2F#7)
    (hf5 : w.extractLsb 31 27 = 20#5) (hf3 : w.extractLsb 14 12 = 3#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmA : _get_Misa_A mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.AMO (.AMOMAX, bool_bit_backwards (w.extractLsb 26 26), bool_bit_backwards (w.extractLsb 25 25),
          .Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), 8, .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  repeat skip_i
  try dsimp only
  rw [interp_bind, run_bind']
  try dsimp only
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, encdec_amoop_backwards_matches, width_enc_wide_backwards_matches,
      Functions.base_E_enabled, LeanZiskRV.not, hop, hf5, hf3, bit_matches]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, hf5, hf3, encdec_amoop_backwards, width_enc_wide_backwards, interp_pure,
    run_pure']
  try dsimp only
  simp (config := { decide := true }) only [amo_encoding_valid, amocas_odd_register_reserved_behavior,
    Functions.xlen_bytes, if_false, if_true, Int.reduceToNat, interp_bind, run_bind', interp_pure, run_pure']
  rw [ce_zaamo s mv hmv hmA]
  try dsimp only
  have hit : ∀ (c : Bool), (interpretSailEffects (if c then (pure false : SailM Bool) else pure true)).run s
      = .ok (!c) s := by intro c; cases c <;> rfl
  simp (config := { decide := true }) only [hit, Bool.true_and, Bool.true_or, Bool.or_true, Bool.false_and,
    if_true, interp_pure, run_pure']
  try dsimp only
  simp only [decide_true, Bool.true_or, if_true, interp_pure, run_pure']
  try dsimp only
  try simp only [interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.AMO.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_, ?_⟩ <;> first | trivial | (apply BitVec.eq_of_getLsbD_eq; intro i hi; simp)

end Equiv.Riscv
