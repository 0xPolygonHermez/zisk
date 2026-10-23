/-
  sail-riscv's decoder on control flow: a branch word decodes to BTYPE with
  its fields (by peeling the decoder's clauses; BTYPE comes early).
-/
import Equiv.RiscvDecodeImm

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- funct3 of each branch. -/
def btypeF3 : bop → BitVec 3
  | .BEQ => 0#3 | .BNE => 1#3 | .BLT => 4#3 | .BGE => 5#3 | .BLTU => 6#3 | .BGEU => 7#3

theorem bop_dec (op : bop) : encdec_bop_backwards (btypeF3 op) = pure op := by cases op <;> rfl

theorem bop_matches (op : bop) : encdec_bop_backwards_matches (btypeF3 op) = true := by cases op <;> rfl

set_option maxHeartbeats 0 in
/-- A branch word decodes to BTYPE with its fields: the immediate as the
    decoder assembles it, imm[12] ++ imm[11] ++ imm[10:5] ++ imm[4:1] ++ 0. -/
theorem decode_btype (s : RVState) (w : BitVec 32) (op : bop)
    (hop : w.extractLsb 6 0 = 0x63#7) (hf3 : w.extractLsb 14 12 = btypeF3 op)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.BTYPE (w.extractLsb 31 31 ++ w.extractLsb 7 7 ++ w.extractLsb 30 25 ++ w.extractLsb 11 8 ++ 0#1,
          .Regidx (w.extractLsb 24 20), .Regidx (w.extractLsb 19 15), op)) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  have hf3' : Sail.BitVec.extractLsb w 14 12 = btypeF3 op := hf3
  rw [if_pos]; rotate_left
  · simp only [hf3', bop_matches, Bool.and_true]
    simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop]
  simp only [hf3', bop_dec, encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.BTYPE.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

/-- The opcode of lui and auipc. -/
def uopEnc : uop → BitVec 7
  | .LUI => 0x37#7 | .AUIPC => 0x17#7

theorem uop_dec (op : uop) : encdec_uop_backwards (uopEnc op) = pure op := by cases op <;> rfl

theorem uop_matches (op : uop) : encdec_uop_backwards_matches (uopEnc op) = true := by cases op <;> rfl

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

set_option maxHeartbeats 0 in
/-- A jal word decodes to JAL with its fields: the offset as the decoder
    assembles it, imm[20] ++ imm[19:12] ++ imm[11] ++ imm[10:1] ++ 0. -/
theorem decode_jal (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x6f#7)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.JAL (w.extractLsb 31 31 ++ w.extractLsb 19 12 ++ w.extractLsb 20 20 ++ w.extractLsb 30 21 ++ 0#1,
          .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.JAL.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

set_option maxHeartbeats 0 in
/-- A jalr word decodes to JALR with its fields. -/
theorem decode_jalr (s : RVState) (w : BitVec 32) (hop : w.extractLsb 6 0 = 0x67#7)
    (hf3 : w.extractLsb 14 12 = 0#3)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : s.regs.get? mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run s
      = .ok (.JALR (w.extractLsb 31 20, .Regidx (w.extractLsb 19 15), .Regidx (w.extractLsb 11 7))) s := by
  unfold ext_decode
  delta encdec_backwards
  rw [interp_bind, run_bind']
  dsimp only
  repeat peel_i
  rw [if_pos]; rotate_left
  · simp only [Sail.BitVec.extractLsb, Bool.and_eq_true, beq_iff_eq]
    simp [encdec_reg_backwards_matches, Functions.base_E_enabled, LeanZiskRV.not, hop, hf3]
  simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
    Bool.true_or, if_true, interp_bind, run_bind', interp_pure, run_pure']
  simp only [Sail.BitVec.extractLsb, Functions.regidx_bit_width]
  simp only [EStateM.Result.ok.injEq, instruction.JALR.injEq, Prod.mk.injEq, regidx.Regidx.injEq, and_true,
    true_and]
  refine ⟨?_, ?_⟩ <;>
  · apply BitVec.eq_of_getLsbD_eq; intro i hi; simp

end Equiv.Riscv
