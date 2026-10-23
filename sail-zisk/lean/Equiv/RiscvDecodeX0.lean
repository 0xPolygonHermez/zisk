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
import Equiv.RiscvDecode
import Equiv.RiscvDecodeImm
import Equiv.RiscvDecodeLoad

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

set_option maxHeartbeats 0 in
/-- An R-type word, any registers, decodes to RTYPE with its fields. (`add x0,
    x0, rs2` matches Zihintntl's clause, which, Zihintntl off, yields nothing.) -/
theorem decode_rtype0 (s : RVState) (w : BitVec 32) (op : rop)
    (hop : w.extractLsb 6 0 = 0x33#7) (hf3 : w.extractLsb 14 12 = rtypeF3 op)
    (hf7 : w.extractLsb 31 25 = rtypeF7 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.RTYPE (.Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [rtypeF3, rtypeF7] at hf3 hf7
  case ADD =>
    unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel
    -- Zihintntl's ntl.*: add x0, x0, rs2 for rs2 in x2..x5; its clause,
    -- Zihintntl off, yields nothing.
    by_cases hg : (encdec_ntl_backwards_matches (Sail.BitVec.extractLsb w 24 20) &&
        (Sail.BitVec.extractLsb w 31 25 == 0#7 && Sail.BitVec.extractLsb w 19 0 == 51#20)) = true
    all_goals first
      | (rw [if_pos hg]
         have hm : encdec_ntl_backwards_matches (Sail.BitVec.extractLsb w 24 20) = true := by
           simp only [Bool.and_eq_true] at hg; exact hg.1
         obtain ⟨v, hv⟩ := ntl_run s _ hm
         rw [interp_bind, run_bind', hv]; dsimp only
         repeat (first | peel | peel_f))
      | (rw [if_neg hg]
         repeat (first | peel | peel_f))
    all_goals
      rw [if_pos]; rotate_left
      · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
        simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, hf7]
      simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false, Bool.true_or,
        if_true, interp_bind, run_bind', interp_pure, run_pure']
      simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
      simp only [EStateM.Result.ok.injEq, instruction.RTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true]
      refine ⟨?_, ?_, ?_⟩ <;>
      · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp
  all_goals
    unfold ext_decode
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

set_option maxHeartbeats 0 in
/-- An I-type word, any registers, decodes to ITYPE with its fields. (`ori x0, …`
    matches Zicbop's prefetch clause, which, Zicbop off, yields nothing.) -/
theorem decode_itype0 (s : RVState) (w : BitVec 32) (op : iop)
    (hop : w.extractLsb 6 0 = 0x13#7) (hf3 : w.extractLsb 14 12 = itypeF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.ITYPE (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  -- Zicbop's prefetch.{i,r,w}: ori x0, rs1, imm with imm[4:0] in {0, 1, 3}; its
  -- clause, Zicbop off, yields nothing.
  by_cases hg : (encdec_cbop_zicbop_backwards_matches (Sail.BitVec.extractLsb w 24 20) &&
      encdec_reg_backwards_matches (Sail.BitVec.extractLsb w 19 15) &&
      Sail.BitVec.extractLsb w 14 0 == 24595#15) = true
  all_goals first
    | (rw [if_pos hg]
       have hm : encdec_cbop_zicbop_backwards_matches (Sail.BitVec.extractLsb w 24 20) = true := by
         simp only [Bool.and_eq_true] at hg; exact hg.1.1
       obtain ⟨v, hv⟩ := cbop_run s _ hm
       obtain ⟨r, hr⟩ := reg_run s (Sail.BitVec.extractLsb w 19 15)
       rw [interp_bind, run_bind', hv]; dsimp only
       rw [interp_bind, run_bind', hr]; dsimp only
       repeat (first | peel_i | peel_f))
    | (rw [if_neg hg]
       repeat (first | peel_i | peel_f))
  all_goals
    have hf3' : Sail.BitVec.extractLsb w 14 12 = itypeF3 op := hf3
    rw [if_pos]; rotate_left
    · simp only [hf3', iop_matches, Bool.and_true, Bool.true_and]
      simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop]
    simp only [hf3', iop_dec, encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
      Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.ITYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
      true_and]
    refine ⟨?_, ?_⟩ <;>
    · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

set_option maxHeartbeats 0 in
/-- An immediate-shift word, any registers, decodes to SHIFTIOP with its fields. -/
theorem decode_shiftiop0 (s : RVState) (w : BitVec 32) (op : sop)
    (hop : w.extractLsb 6 0 = 0x13#7) (hf3 : w.extractLsb 14 12 = shiftF3 op)
    (hf6 : w.extractLsb 31 26 = shiftF6 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.SHIFTIOP (w.extractLsb 25 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7), op)) s := by
  cases op <;> simp only [shiftF3, shiftF6] at hf3 hf6 <;>
  · unfold ext_decode
    delta encdec_backwards
    rw [interp_bind, run_bind']
    dsimp only
    repeat peel_i
    rw [if_pos]; rotate_left
    · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
      simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3, hf6]
    simp (config := { decide := true }) only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not,
      Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure', Functions.xlen,
      beq_self_eq_true]
    simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
    simp only [EStateM.Result.ok.injEq, instruction.SHIFTIOP.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
      true_and]
    refine ⟨?_, ?_⟩ <;>
    · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

set_option maxHeartbeats 0 in
/-- A load word (any registers, a valid funct3) decodes to LOAD with its fields. -/
theorem decode_load0 (s : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x03#7)
    (hvalid : valid_load_encdec (width_enc_backwards (w.extractLsb 13 12)).toNat
      (bool_bit_backwards (w.extractLsb 14 14)) = true)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.LOAD (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7),
          bool_bit_backwards (w.extractLsb 14 14), width_enc_backwards (w.extractLsb 13 12))) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_m
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, width_enc_backwards_matches, bool_bit_backwards_matches,
      Functions.base_E_enabled, LeanZiskRV.not, hop]
    generalize BitVec.extractLsb 14 14 w = b; generalize BitVec.extractLsb 13 12 w = c
    revert b c; decide
  simp (config := { decide := true }) only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not,
    Bool.not_false, Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure', Sail.BitVec.extractLsb]
  rw [if_pos hvalid]
  simp only [interp_pure, run_pure', Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.LOAD.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
