/-
  sail-riscv's decoder on the x0 forms: the decode lemmas of RiscvDecode*.lean
  without their rd ≠ x0 hypothesis, for the steps of Equiv/X0Sim.lean.

  With rd (and rs1) x0, two clauses before the instruction's own match the
  word: Zihintntl's hints (`add x0, x0, rs2`, rs2 in x2..x5) and Zicbop's
  prefetches (`ori x0, rs1, imm`, imm[4:0] in {0, 1, 3}). Each checks that
  its extension is enabled before it decodes, and neither is in ZisK's
  profile, so each yields nothing and the decoder goes on. (Slow to check,
  as the other decode lemmas.)
-/
import Equiv.RiscvDecodeBase
import Equiv.RiscvDecodeImmBase
import Equiv.RiscvDecodeLoadBase

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- Zicbop's operand: its value, when the field matches. -/
theorem cbop_run (s : RVState) (x : BitVec 5) (h : encdec_cbop_zicbop_backwards_matches x = true) :
    ∃ v, (interpretSailEffects (encdec_cbop_zicbop_backwards x)).run s = .ok v s := by
  unfold encdec_cbop_zicbop_backwards_matches at h
  unfold encdec_cbop_zicbop_backwards
  split at h <;> simp_all <;> exact ⟨_, rfl⟩
theorem reg_run (s : RVState) (x : BitVec 5) :
    ∃ v, (interpretSailEffects (encdec_reg_backwards x)).run s = .ok v s := by
  simp [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not]
/-- A clause skipped: `if false = true`. -/
macro "peel_f" : tactic => `(tactic| (rw [if_neg Bool.false_ne_true]; try dsimp only))

/-- Zihintntl's operand: its value, when the field matches. -/
theorem ntl_run (s : RVState) (x : BitVec 5) (h : encdec_ntl_backwards_matches x = true) :
    ∃ v, (interpretSailEffects (encdec_ntl_backwards x)).run s = .ok v s := by
  unfold encdec_ntl_backwards_matches at h
  unfold encdec_ntl_backwards
  split at h <;> simp_all <;> exact ⟨_, rfl⟩

end Equiv.Riscv
