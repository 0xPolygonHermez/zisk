/-
  sail-riscv's decoder on Zbs: bclr, bext, binv, bset (ZBS_RTYPE) and their
  immediate forms (ZBS_IOP), a clause each. Each clause, past its guard,
  checks that Zbs is enabled: as Zba, through misa's B bit.
-/
import Equiv.RiscvDecodeZba

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zbs, enabled by misa's B bit. -/
theorem ce_zbs (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) (hb : _get_Misa_B v = 1#1) :
    (interpretSailEffects (currentlyEnabled .Ext_Zbs)).run s = .ok true s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv, hb]

def zbsF7 : brop_zbs → BitVec 7
  | .BCLR => 0x24 | .BEXT => 0x24 | .BINV => 0x34 | .BSET => 0x14

def zbsF3 : brop_zbs → BitVec 3
  | .BCLR => 1 | .BEXT => 5 | .BINV => 1 | .BSET => 1

def zbsiF6 : biop_zbs → BitVec 6
  | .BCLRI => 0x12 | .BEXTI => 0x12 | .BINVI => 0x1A | .BSETI => 0x0A

def zbsiF3 : biop_zbs → BitVec 3
  | .BCLRI => 1 | .BEXTI => 5 | .BINVI => 1 | .BSETI => 1

set_option maxHeartbeats 0 in
/-- A Zbs register word decodes to ZBS_RTYPE with its fields. -/
theorem decode_zbs_r (s : RVState) (w : BitVec 32) (op : brop_zbs) (hop : w.extractLsb 6 0 = 0x33#7)
    (hf7 : w.extractLsb 31 25 = zbsF7 op) (hf3 : w.extractLsb 14 12 = zbsF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) (hmB : _get_Misa_B mv = 1#1) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ZBS_RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          op)) s := by
  cases op <;> simp only [zbsF7, zbsF3] at hf7 hf3 <;>
  · unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel_i
    rw [if_pos]; rotate_left
    · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf7, hf3]
    simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
      Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
    rw [ce_zbs s mv hmv hmB]; try dsimp only
    try simp only [if_true, interp_pure, run_pure']
    try dsimp only
    try simp only [interp_pure, run_pure']
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.ZBS_RTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq,
      and_true]
    refine ⟨?_, ?_, ?_⟩ <;>
    · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
