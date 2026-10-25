/-
  sail-riscv's decoder on loads and stores: as Equiv/RiscvDecode.lean, by
  peeling the decoder's clauses; LOAD comes after the R-type ones.
-/
import Equiv.RiscvDecodeBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A clause's guard is false for the word, also through the mappings of
    the I-type ops, the load width and the unsigned bit. -/
macro "guard_false_m" : tactic => `(tactic| (
  intro h
  simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq] at h
  first
  | bv_decide
  | (simp_all (config := { decide := true }) [encdec_uop_backwards_matches, encdec_iop_backwards_matches,
      width_enc_backwards_matches, bool_bit_backwards_matches]; done)))

/-- One step through the decoder's chain of clauses (`peel`, with `guard_false_m`). -/
macro "peel_m" : tactic => `(tactic| first
  | (rw [if_neg]; rotate_left; focus guard_false_m)
  | (rw [interp_pure, run_pure']; try dsimp only)
  | (rw [ce_run]; rotate_left; (focus (simp [currentlyEnabled, hartSupports]; done)); try dsimp only)
  | (rw [ce_zicfilp _ (by assumption) _ (by assumption)]; try dsimp only)
  | (rw [interp_bind, run_bind']; try dsimp only))

end Equiv.Riscv
