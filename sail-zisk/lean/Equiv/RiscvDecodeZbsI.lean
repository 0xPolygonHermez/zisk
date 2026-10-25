/-
  sail-riscv's decoder on Zbs's immediate forms: bclri, bexti, binvi, bseti
  (ZBS_IOP), a clause each, Zbs enabled through misa's B bit and, on RV64,
  any shift amount.
-/
import Equiv.RiscvDecodeZbs

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 0 in
/-- A Zbs immediate word decodes to ZBS_IOP with its fields. -/
theorem decode_zbs_i (s : RVState) (w : BitVec 32) (op : biop_zbs) (hop : w.extractLsb 6 0 = 0x13#7)
    (hf6 : w.extractLsb 31 26 = zbsiF6 op) (hf3 : w.extractLsb 14 12 = zbsiF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBS_IOP (w.extractLsb 25 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [zbsiF6, zbsiF3] at hf6 hf3 <;>
  · unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel_i
    rw [if_pos]; rotate_left
    · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf6, hf3]
    simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
      Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
    rw [ce_zbs s mv hmv hmB]; try dsimp only
    try simp only [Functions.xlen, Bool.true_and, Bool.and_true, beq_self_eq_true, Bool.true_or, if_true,
      interp_pure, run_pure']
    try dsimp only
    try simp only [interp_pure, run_pure']
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.ZBS_IOP.injEq, Prod.mk.injEq, regidx.Regidx.injEq,
      and_true]
    refine ⟨?_, ?_, ?_⟩ <;> first | trivial | (apply BitVec.eq_of_getLsbD_eq; intro i hi; simp)

end Equiv.Riscv
