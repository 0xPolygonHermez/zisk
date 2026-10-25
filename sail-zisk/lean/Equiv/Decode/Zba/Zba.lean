/- sail-riscv's decoder: `decode_zba`, on its own so that it checks in parallel (RiscvDecodeZba.lean). -/
import Equiv.RiscvDecodeZbaBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A sh1add, sh2add or sh3add word decodes to ZBA_RTYPE with its fields. -/
theorem decode_zba (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x33#7)
    (hf7 : w.extractLsb 31 25 = 0x10#7) (h12 : w.extractLsb 12 12 = 0#1) (hsh : w.extractLsb 14 13 ≠ 0#2)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBA_RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          w.extractLsb 14 13)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf7, h12]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  rw [ce_zba s mv hmv hmB]; try dsimp only
  have hsh' : (Sail.BitVec.extractLsb w 14 13 != 0#2) = true := by
    simp only [Sail.BitVec.extractLsb, bne_iff_ne, ne_eq]; exact hsh
  try simp only [hsh', Bool.true_and, if_true, interp_pure, run_pure']
  try dsimp only
  try simp only [interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.ZBA_RTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true]
  refine ⟨?_, ?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
