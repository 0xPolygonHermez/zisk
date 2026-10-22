/-
  sail-riscv's decoder on the instructions the simulation needs: an R-type
  word decodes to RTYPE with its fields.

  sail-riscv's `encdec_backwards` is one definition of some 28,000 lines, a
  chain of clauses, one per encoding: each a guard on the word's bits, and
  some reading registers (`currentlyEnabled`). Its equation lemma is too big
  to generate, so the proofs unfold it with `delta` and peel the clauses off
  the front one at a time (`peel`): a guard refuted by `bv_decide` from the
  word's fields, an extension the profile leaves out read as false, a `pure`
  stepped over. At the instruction's own clause the guard holds, its fields
  are read back, and the rest of the decoder is discarded. Each decoding
  takes a minute or two to check: the ten R-type ops about twenty minutes.
-/
import Equiv.RiscvMem

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- An extension the profile leaves out, which reads no register. -/
theorem ce_run (s : RVState) (e : extension) (h : currentlyEnabled e = pure false) :
    (interpretSailEffects (currentlyEnabled e)).run s = .ok false s := by rw [h]; rfl

/-- Zicfilp, which reads the landing-pad enable: off in machine mode's mseccfg. -/
theorem ce_zicfilp (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (v : RegisterType mseccfg) (hv : s.regs.get? mseccfg = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zicfilp)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, get_xLPE, readReg_run _ _ _ hp, readReg_run _ _ _ hv]

/-- A clause's guard is false for the word. -/
macro "guard_false" : tactic => `(tactic| (
  intro h
  simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq] at h
  first
  | bv_decide
  | (simp_all (config := { decide := true }) [encdec_uop_backwards_matches]; done)))

/-- One step through the decoder's chain of clauses. -/
macro "peel" : tactic => `(tactic| first
  | (rw [if_neg]; rotate_left; focus guard_false)
  | (rw [interp_pure, run_pure']; try dsimp only)
  | (rw [ce_run]; rotate_left; (focus (simp [currentlyEnabled, hartSupports]; done)); try dsimp only)
  | (rw [ce_zicfilp _ (by assumption) _ (by assumption)]; try dsimp only)
  | (rw [interp_bind, run_bind']; try dsimp only))

/-- funct3 and funct7 of each R-type op (RISC-V's base encoding). -/
def rtypeF3 : rop → BitVec 3
  | .ADD => 0 | .SUB => 0 | .SLL => 1 | .SLT => 2 | .SLTU => 3
  | .XOR => 4 | .SRL => 5 | .SRA => 5 | .OR => 6 | .AND => 7

def rtypeF7 : rop → BitVec 7
  | .SUB => 0x20 | .SRA => 0x20 | _ => 0

set_option maxHeartbeats 0 in
/-- An R-type word, rd ≠ x0, decodes to RTYPE with its fields. -/
theorem decode_rtype (s : RVState) (w : BitVec 32) (op : rop)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = rtypeF3 op)
    (hf7 : w.extractLsb 31 25 = rtypeF7 op) (hrd : w.extractLsb 11 7 ≠ 0#5)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [rtypeF3, rtypeF7] at hf3 hf7 <;>
  · unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel
    rw [if_pos]; rotate_left
    · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, hf7]
    simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false, Bool.true_or,
      if_true, interp_bind, run_bind', interp_pure, run_pure']
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.RTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true]
    refine ⟨?_, ?_, ?_⟩ <;>
    · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
