/- sail-riscv's decoder: `decode_add_uw`, on its own so that it checks in parallel (RiscvDecodeZba.lean). -/
import Equiv.RiscvDecodeZbaBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- An add.uw word decodes to ZBA_RTYPEUW with shift 0 with its fields. -/
theorem decode_add_uw (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x3B#7)
    (hf7 : w.extractLsb 31 25 = 0x04#7) (hf3 : w.extractLsb 14 12 = 0#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBA_RTYPEUW (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          0#2)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf7, hf3]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  rw [ce_zba s mv hmv hmB]; try dsimp only
  try simp only [Functions.xlen, beq_self_eq_true, Bool.true_and, Bool.and_true, if_true, interp_pure, run_pure']
  try dsimp only
  try simp only [interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.ZBA_RTYPEUW.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true]
  refine ⟨?_, ?_, ?_⟩ <;> first | trivial | (apply BitVec.eq_of_getLsbD_eq; intro i hi; simp)

end Equiv.Riscv
