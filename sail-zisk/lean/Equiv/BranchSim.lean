/-
  The simulation, one step, on a conditional branch: sail-riscv's
  beq/bne/blt/bge/bltu/bgeu and the transpiler's create_branch_op (eq, lt or
  ltu sets the flag; a negated branch swaps the jump offsets). Both go to
  the same next pc: pc + imm when the branch is taken, pc + 4 when not.
-/
import Equiv.Run
import Equiv.RiscvStepGen
import Equiv.RiscvDecodeCtl
import Equiv.ZiskCtl

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The ZisK op the transpiler emits for each branch, and whether it negates
    it (swapping the jump offsets). -/
def zbopOp : bop → _root_.Zisk.zisk_op
  | .BEQ | .BNE => .ZOP_EQ () | .BLT | .BGE => .ZOP_LT () | .BLTU | .BGEU => .ZOP_LTU ()

def zbopNeg : bop → Bool
  | .BEQ | .BLT | .BLTU => false | .BNE | .BGE | .BGEU => true

theorem zbop_pure (op : bop) : Equiv.Zisk.PureOp (zbopOp op) := by
  cases op
  · exact Equiv.Zisk.eq_pure
  · exact Equiv.Zisk.eq_pure
  · exact pureOp .SLT
  · exact pureOp .SLT
  · exact pureOp .SLTU
  · exact pureOp .SLTU

/-- ZisK's flag, negated where the transpiler negates, is sail-riscv's
    "taken". -/
theorem zbop_flag (op : bop) (a b : BitVec 64) :
    (zbopNeg op != (_root_.Zisk.Functions.zisk_exec (zbopOp op) a b).2) = btypeTaken op a b := by
  cases op <;>
    simp [zbopNeg, zbopOp, btypeTaken, _root_.Zisk.Functions.zisk_exec, _root_.Zisk.Functions.bool_c,
      _root_.Zisk.Functions.zopz0zI_s, _root_.Zisk.Functions.zopz0zI_u, zopz0zI_s, zopz0zI_u,
      zopz0zKzJ_s, zopz0zKzJ_u]
  all_goals (split <;> simp_all <;> omega)

/-- A branch, by its fields; the immediate in the four parts the B format
    splits it into (taken whole as the decoder assembles it). -/
structure BInstr where
  op : bop
  rs1 : BitVec 5
  rs2 : BitVec 5
  b12 : BitVec 1
  b11 : BitVec 1
  b10_5 : BitVec 6
  b4_1 : BitVec 4

/-- The branch offset, as sail-riscv's decoder assembles it. -/
def BInstr.imm (i : BInstr) : BitVec 13 := i.b12 ++ i.b11 ++ i.b10_5 ++ i.b4_1 ++ 0#1

def BInstr.encode (i : BInstr) : BitVec 32 :=
  i.b12 ++ i.b10_5 ++ i.rs2 ++ i.rs1 ++ btypeF3 i.op ++ i.b4_1 ++ i.b11 ++ 0x63#7

theorem BInstr.fields (i : BInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x63#7 ∧ v.extractLsb 14 12 = btypeF3 i.op ∧
    v.extractLsb 31 31 = i.b12 ∧ v.extractLsb 7 7 = i.b11 ∧ v.extractLsb 30 25 = i.b10_5 ∧
    v.extractLsb 11 8 = i.b4_1 ∧ v.extractLsb 24 20 = i.rs2 ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, BInstr.encode, byte]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- The transpiler's jump offsets for branch i: the branch offset when the
    flag is set, 4 when not; swapped for a negated branch. -/
def BInstr.zjumps (i : BInstr) : BitVec 64 × BitVec 64 :=
  if zbopNeg i.op then (4, i.imm.signExtend 64) else (i.imm.signExtend 64, 4)

/-- Branch `i` at pc: its encoding in executable memory there, rs1, rs2 ≠
    x0, a 4-aligned target, and the transpiler's instruction for it at pc. -/
structure BAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : BInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rs1 : i.rs1 ≠ 0
  rs2 : i.rs2 ≠ 0
  tal : (pc + i.imm.signExtend 64)[0] = false ∧ (pc + i.imm.signExtend 64)[1] = false
  rom : zrom pc = Equiv.Zisk.branchInst (zbopOp i.op) i.rs1.toNat i.rs2.toNat i.zjumps.1 i.zjumps.2

theorem BAt.of_mem {s s' : RVState} {rs pc zrom i} (h : BAt s rs pc zrom i) (hm : s'.mem = s.mem) :
    BAt s' rs pc zrom i := by
  obtain ⟨al, x1, x2, h0, h1, h2, h3, r1, r2, tal, rom⟩ := h
  exact ⟨al, x1, x2, hm ▸ h0, hm ▸ h1, hm ▸ h2, hm ▸ h3, r1, r2, tal, rom⟩

theorem execute_btype_eq (imm : BitVec 13) (a b : regidx) (op : bop) :
    execute (.BTYPE (imm, a, b, op)) = execute_BTYPE imm a b op := by
  simp [execute]

/-- Where a branch goes: pc + imm when taken, pc + 4 when not. -/
def BInstr.target (i : BInstr) (pc v1 v2 : BitVec 64) : BitVec 64 :=
  if btypeTaken i.op v1 v2 then pc + i.imm.signExtend 64 else pc + 4

set_option maxHeartbeats 4000000 in
/-- One step of each on a branch keeps the states related, both at the
    branch's target. -/
theorem brstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : BInstr) (hi : BAt s rs pc zrom i)
    (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (hv2 : getX s i.rs2.toNat = some v2) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (i.target pc v1 v2) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, h12, h11, h10, h4, hr2x, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have e1 := ne_zero_toNat hi.rs1
  have e2 := ne_zero_toNat hi.rs2
  have a1 := agree i.rs1.toNat (by omega) l1
  have a2 := agree i.rs2.toNat (by omega) l2
  have hxv1 : xs[i.rs1.toNat]! = v1 := by rw [hv1] at a1; exact (Option.some.inj a1).symm
  have hxv2 : xs[i.rs2.toNat]! = v2 := by rw [hv2] at a2; exact (Option.some.inj a2).symm
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_gen s rs hr pc' hpc _ _ _ _ hw (.BTYPE (i.imm, .Regidx i.rs2, .Regidx i.rs1, i.op))
      (by intro t htp ⟨w, hw'⟩ hmt
          have := decode_btype t _ i.op hop hf3 htp w hw'
          rw [h12, h11, h10, h4, hr2x, hr1x] at this; exact this)
      0 0 (i.target pc' v1 v2)
      (by intro t he _ _ _ _ hnext hother
          rw [execute_btype_eq]
          have hpct : t.regs.get? Register.PC = some pc' := by
            rw [hother _ (by decide) (by decide) (by decide)]; exact hpc
          have hmt : t.regs.get? Register.misa = some mi := by
            rw [hother _ (by decide) (by decide) (by decide)]; exact hmi
          rw [execute_btype t i.op i.rs1 i.rs2 i.imm v1 v2 pc' mi (getX_of_agree he l1 hv1)
            (getX_of_agree he l2 hv2) hpct hmt hi.tal]
          simp only [setX, if_true, BInstr.target]
          split
          · rfl
          · rw [insert_same t _ _ hnext]) n
  have hxv1' : xs[(i.rs1.toNat : Int)]! = v1 := by rw [getElem!_int]; exact hxv1
  have hxv2' : xs[(i.rs2.toNat : Int)]! = v2 := by rw [getElem!_int]; exact hxv2
  have hz := Equiv.Zisk.execute_branch z (zbopOp i.op) (zbop_pure i.op) i.rs1.toNat i.rs2.toNat
    i.zjumps.1 i.zjumps.2 (by omega) (by omega) xs pc' st hx hzpc hst
  refine ⟨s', _, hrun, by rw [zstep_run zrom z pc' hzpc, hi.rom]; exact hz, ⟨hr.keep hwf ho, ⟨xs, ?_, ?_⟩,
    ⟨_, hpc'', ?_⟩, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert]⟩, ?_⟩, hpc'', hmem⟩
  · simp [Std.ExtDHashMap.get?_insert, hx]
  · intro k hk1 hk2; rw [hxs' k hk2]; simp only [setX, if_true]; exact agree k hk1 hk2
  · simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
    rw [hxv1', hxv2', BInstr.target, ← zbop_flag, BInstr.zjumps]
    cases zbopNeg i.op <;> cases (_root_.Zisk.Functions.zisk_exec (zbopOp i.op) v1 v2).2 <;> simp
  · show (z.mem) = s'.mem
    rw [hmem, hzm]

end Equiv.Sim
