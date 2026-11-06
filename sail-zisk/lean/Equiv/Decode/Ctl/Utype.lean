/- sail-riscv's decoder: `decode_utype`, on its own so that it checks in parallel (RiscvDecodeCtl.lean). -/
import Equiv.RiscvDecodeCtlBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A lui or auipc word decodes to UTYPE with its fields. -/
theorem decode_utype (s : RVState) (w : BitVec 32) (op : uop) (hop : w.extractLsb 6 0 = uopEnc op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.UTYPE (w.extractLsb 31 12, .Regidx (w.extractLsb 11 7), op)) s := by
  have hop' : Sail.BitVec.extractLsb w 6 0 = uopEnc op := hop
  cases op <;>
  · have hlit := hop
    simp only [uopEnc] at hlit
    unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel_i
    rw [if_pos]; rotate_left
    · simp only [hop', uop_matches, Bool.and_true]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not]
    simp only [hop', uop_dec, encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
      Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.UTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
      true_and]
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
