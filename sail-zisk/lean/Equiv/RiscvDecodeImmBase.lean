/-
  sail-riscv's decoder on the immediate instructions: an I-type word
  (addi, slti, sltiu, xori, ori, andi) decodes to ITYPE, an immediate shift
  (slli, srli, srai) to SHIFTIOP, with their fields; as Equiv/RiscvDecode.lean
  does for the R-type, by peeling the decoder's clauses.
-/
import Equiv.RiscvDecodeBase
import Equiv.RiscvImm

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- A clause's guard is false for the word: as `guard_false`, also through
    the I-type op mapping. -/
macro "guard_false_i" : tactic => `(tactic| (
  intro h
  simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq] at h
  first
  | bv_decide
  | (simp_all (config := { decide := true }) [encdec_uop_backwards_matches, encdec_iop_backwards_matches]; done)))

/-- One step through the decoder's chain of clauses (`peel`, with `guard_false_i`). -/
macro "peel_i" : tactic => `(tactic| first
  | (rw [if_neg]; rotate_left; focus guard_false_i)
  | (rw [interp_pure, run_pure']; try dsimp only)
  | (rw [ce_run]; rotate_left; (focus (simp [currentlyEnabled, hartSupports]; done)); try dsimp only)
  | (rw [ce_zicfilp _ (by assumption) _ (by assumption)]; try dsimp only)
  | (rw [interp_bind, run_bind']; try dsimp only))

/-- funct3 of each I-type op. -/
def itypeF3 : iop → BitVec 3
  | .ADDI => 0#3 | .SLTI => 2#3 | .SLTIU => 3#3 | .XORI => 4#3 | .ORI => 6#3 | .ANDI => 7#3

theorem iop_dec (op : iop) : encdec_iop_backwards (itypeF3 op) = pure op := by cases op <;> rfl

theorem iop_matches (op : iop) : encdec_iop_backwards_matches (itypeF3 op) = true := by cases op <;> rfl

/-- funct3 and funct6 of each immediate shift. -/
def shiftF3 : sop → BitVec 3
  | .SLLI => 1 | .SRLI => 5 | .SRAI => 5

def shiftF6 : sop → BitVec 6
  | .SRAI => 0x10 | _ => 0

/-- `peel_i`'s steps, a clause at a time by its head: a clause whose guard
    `guard_false_i` refutes, or whose body `peel_i` runs to none. -/
macro "skip_i" : tactic => `(tactic| first
  | (refine (skip_ite _ _ _ _ ?h).trans ?_
     case h => intro hc; exfalso; revert hc; guard_false_i)
  | (refine (skip_none _ _ _ ?h).trans ?_
     case h => (try dsimp only); (repeat peel_i); first | done | rfl))

end Equiv.Riscv
