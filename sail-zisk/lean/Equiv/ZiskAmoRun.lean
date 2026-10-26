/-
  ZisK's runs of an AMO as the transpiler makes it (Transpiler/Spec.lean,
  `amo`), any width: the load at pc, to rd or, when rd is rs1 or rs2, to
  ZisK's register 32, on to the internal address ia; for all but the swap,
  the op of c (what was loaded) and r[rs2] at ia; the store of c (the swap:
  r[rs2]) at r[rs1]; for register 32, its copy to rd. On to pc + 4.
-/
import Equiv.ZiskAmo
import Equiv.UwSim

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Zisk (regv ASrc BSrc)
open _root_.Zisk _root_.Zisk.Functions Sail

theorem regv_xsUpd_ne (xs : Vector (BitVec 64) 40) (rd : Nat) (v : BitVec 64) (hd : rd < 40) {k : Nat}
    (hk : k < 40) (hne : k ≠ rd) : regv (Equiv.Zisk.xsUpd xs rd v) k = regv xs k := by
  unfold Equiv.Zisk.xsUpd
  split
  · rfl
  · unfold regv; split
    · rfl
    · rw [getElem!_int, getElem!_int]
      have s40 : k < xs.toArray.size := by simp; omega
      simp [vectorUpdate, Vector.set!, hk]
      rw [Array.getElem_setIfInBounds]; simp [Ne.symm hne]; rfl

set_option maxHeartbeats 8000000 in
/-- An AMO but the swap, rd neither rs1 nor rs2: three steps. -/
theorem zisk_amo_op (zrom : BitVec 64 → zisk_inst) (z : Equiv.Zisk.ZState) (lop op : zisk_op)
    (hl : Equiv.Zisk.PureOp lop) (ho : Equiv.Zisk.PureOp op) (w : Nat) (rd rs1 rs2 : Nat)
    (ld : rd < 32) (l1 : rs1 < 32) (l2 : rs2 < 32) (n1 : rd ≠ rs1) (n2 : rd ≠ rs2)
    (xs : Vector (BitVec 64) 40) (pc st ia : BitVec 64)
    (hx : z.regs.get? Register.Xs = some xs) (hpc : z.regs.get? Register.PC = some pc)
    (hst : z.regs.get? Register.Step = some st) (hze : z.regs.get? Register.Zerror = some false)
    (v : BitVec (8 * w))
    (hrb : readable (regv xs rs1 + 0) w = true) (hwr : writable (regv xs rs1 + 0) w = true)
    (hnr : hits_registers (regv xs rs1 + 0) w = false) (hin : (regv xs rs1 + 0 == INPUT_ADDR && w == 8) = false)
    (hm : Equiv.memRead z.mem w (regv xs rs1 + 0).toNat = some v)
    (r1 : zrom pc = Transpiler.mk (Transpiler.srcA rs1) (.B_IND, 0) lop (Transpiler.storeReg rd) (ia - pc) (ia - pc)
      (w := w))
    (r2 : zrom ia = Transpiler.mk (.A_C, 0) (Transpiler.srcB rs2) op (.ST_NONE, 0, false) 2 2)
    (r3 : zrom (ia + 2) = Transpiler.mk (Transpiler.srcA rs1) (.B_C, 0) (.ZOP_COPYB ()) (.ST_IND, 0, false)
      (pc + 4 - (ia + 2)) (pc + 4 - (ia + 2)) (w := w)) :
    let L := (zisk_exec lop (regv xs rs1) (v.setWidth 64)).1
    let c := (zisk_exec op L (regv xs rs2)).1
    ∃ z', ZRun zrom 3 z z' ∧ z'.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs rd L) ∧
      z'.regs.get? Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? Register.Step = some st ∧ z'.regs.get? Register.Zerror = some false) ∧
      z'.mem = Equiv.memWrite (n := w) z.mem (regv xs rs1 + 0).toNat (c.setWidth (8 * w)) := by
  intro L c
  have eia : pc + (ia - pc) = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have e4 : ia + 2 + (pc + 4 - (ia + 2)) = pc + 4 := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  -- step 1: the load, to rd
  have hz1 := Equiv.Zisk.execute_load_mkj z _ lop hl rd (by omega) (ia - pc) 0 w xs pc st hx hpc hst _
    (Equiv.Zisk.srcA_ASrc xs _ l1) v hrb hnr hin hm
  rw [← r1, ← zstep_run zrom z pc hpc] at hz1
  dsimp only at hz1
  generalize hz1s : ({ z with regs := _ } : Equiv.Zisk.ZState) = z1 at hz1
  have hx1 : z1.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs rd L) := by
    subst hz1s; simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
    exact linkRegs_xs _ _ _ _ hx
  have hpc1 : z1.regs.get? Register.PC = some ia := by subst hz1s; simp [Std.ExtDHashMap.get?_insert, eia]
  have hst1 : z1.regs.get? Register.Step = some (BitVec.addInt st 1) := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert]
  have hc1 : z1.regs.get? Register.Zc = some L := by subst hz1s; simp [Std.ExtDHashMap.get?_insert]; rfl
  have hze1 : z1.regs.get? Register.Zerror = some false := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, hze]
  have hm1 : z1.mem = z.mem := by subst hz1s; rfl
  have u1 : regv (Equiv.Zisk.xsUpd xs rd L) rs1 = regv xs rs1 := regv_xsUpd_ne xs rd L (by omega) (by omega) (Ne.symm n1)
  have u2 : regv (Equiv.Zisk.xsUpd xs rd L) rs2 = regv xs rs2 := regv_xsUpd_ne xs rd L (by omega) (by omega) (Ne.symm n2)
  -- step 2: the op on c
  have hz2 := Equiv.Zisk.execute_cop z1 _ op ho 2 _ ia _ L hx1 hpc1 hst1 hc1 _ (Equiv.Zisk.srcB_BSrc _ _ l2)
  rw [← r2, ← zstep_run zrom z1 ia hpc1, u2] at hz2
  dsimp only at hz2
  generalize hz2s : ({ z1 with regs := _ } : Equiv.Zisk.ZState) = z2 at hz2
  have hx2 : z2.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs rd L) := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert, hx1]
  have hpc2 : z2.regs.get? Register.PC = some (ia + 2) := by subst hz2s; simp [Std.ExtDHashMap.get?_insert]
  have hst2 : z2.regs.get? Register.Step = some (BitVec.addInt (BitVec.addInt st 1) 1) := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert]
  have hc2 : z2.regs.get? Register.Zc = some c := by subst hz2s; simp [Std.ExtDHashMap.get?_insert]; rfl
  have hze2 : z2.regs.get? Register.Zerror = some false := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert, hze1]
  have hm2 : z2.mem = z.mem := by subst hz2s; exact hm1
  -- step 3: the store of c
  have hz3 := Equiv.Zisk.execute_store_c z2 _ 0 w (pc + 4 - (ia + 2)) _ (ia + 2) _ hx2 hpc2 hst2 _ c
    (Equiv.Zisk.srcA_ASrc _ _ l1) hc2 (by rw [u1]; exact hwr) (by rw [u1]; exact hnr)
  rw [← r3, ← zstep_run zrom z2 (ia + 2) hpc2, u1] at hz3
  refine ⟨_, .step hz1 (.step hz2 (.step hz3 (.done _))), ?_, ?_,
    ⟨BitVec.addInt (BitVec.addInt (BitVec.addInt st 1) 1) 1, ?_, ?_⟩, ?_⟩
  · simp [Std.ExtDHashMap.get?_insert, hx2]
  · simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  · simp [Std.ExtDHashMap.get?_insert]
  · simp [Std.ExtDHashMap.get?_insert, hze2]
  · show Equiv.memWrite z2.mem _ _ = _
    rw [hm2]

set_option maxHeartbeats 8000000 in
/-- An AMO but the swap, rd rs1 or rs2: four steps, through register 32. -/
theorem zisk_amo_op32 (zrom : BitVec 64 → zisk_inst) (z : Equiv.Zisk.ZState) (lop op : zisk_op)
    (hl : Equiv.Zisk.PureOp lop) (ho : Equiv.Zisk.PureOp op) (w : Nat) (rd rs1 rs2 : Nat)
    (ld : rd < 32) (l1 : rs1 < 32) (l2 : rs2 < 32)
    (xs : Vector (BitVec 64) 40) (pc st ia : BitVec 64)
    (hx : z.regs.get? Register.Xs = some xs) (hpc : z.regs.get? Register.PC = some pc)
    (hst : z.regs.get? Register.Step = some st) (hze : z.regs.get? Register.Zerror = some false)
    (v : BitVec (8 * w))
    (hrb : readable (regv xs rs1 + 0) w = true) (hwr : writable (regv xs rs1 + 0) w = true)
    (hnr : hits_registers (regv xs rs1 + 0) w = false) (hin : (regv xs rs1 + 0 == INPUT_ADDR && w == 8) = false)
    (hm : Equiv.memRead z.mem w (regv xs rs1 + 0).toNat = some v)
    (r1 : zrom pc = Transpiler.mk (Transpiler.srcA rs1) (.B_IND, 0) lop (Transpiler.storeReg 32) (ia - pc) (ia - pc)
      (w := w))
    (r2 : zrom ia = Transpiler.mk (.A_C, 0) (Transpiler.srcB rs2) op (.ST_NONE, 0, false) 2 2)
    (r3 : zrom (ia + 2) = Transpiler.mk (Transpiler.srcA rs1) (.B_C, 0) (.ZOP_COPYB ()) (.ST_IND, 0, false)
      (ia + 4 - (ia + 2)) (ia + 4 - (ia + 2)) (w := w))
    (r4 : zrom (ia + 4) = Transpiler.mk (.A_IMM, 0) (Transpiler.srcB 32) (.ZOP_COPYB ()) (Transpiler.storeReg rd)
      (pc + 4 - (ia + 4)) (pc + 4 - (ia + 4))) :
    let L := (zisk_exec lop (regv xs rs1) (v.setWidth 64)).1
    let c := (zisk_exec op L (regv xs rs2)).1
    ∃ z', ZRun zrom 4 z z' ∧
      (∃ xs', z'.regs.get? Register.Xs = some xs' ∧ ∀ k, k < 32 → regv xs' k = regv (Equiv.Zisk.xsUpd xs rd L) k) ∧
      z'.regs.get? Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? Register.Step = some st ∧ z'.regs.get? Register.Zerror = some false) ∧
      z'.mem = Equiv.memWrite (n := w) z.mem (regv xs rs1 + 0).toNat (c.setWidth (8 * w)) := by
  intro L c
  have eia : pc + (ia - pc) = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have hz1 := Equiv.Zisk.execute_load_mkj z _ lop hl 32 (by omega) (ia - pc) 0 w xs pc st hx hpc hst _
    (Equiv.Zisk.srcA_ASrc xs _ l1) v hrb hnr hin hm
  rw [← r1, ← zstep_run zrom z pc hpc] at hz1
  dsimp only at hz1
  generalize hz1s : ({ z with regs := _ } : Equiv.Zisk.ZState) = z1 at hz1
  have hx1 : z1.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs 32 L) := by
    subst hz1s; simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
    exact linkRegs_xs _ _ _ _ hx
  have hpc1 : z1.regs.get? Register.PC = some ia := by subst hz1s; simp [Std.ExtDHashMap.get?_insert, eia]
  have hst1 : z1.regs.get? Register.Step = some (BitVec.addInt st 1) := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert]
  have hc1 : z1.regs.get? Register.Zc = some L := by subst hz1s; simp [Std.ExtDHashMap.get?_insert]; rfl
  have hze1 : z1.regs.get? Register.Zerror = some false := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, hze]
  have hm1 : z1.mem = z.mem := by subst hz1s; rfl
  have u1 : regv (Equiv.Zisk.xsUpd xs 32 L) rs1 = regv xs rs1 := regv_xsUpd_ne xs 32 L (by omega) (by omega) (by omega)
  have u2 : regv (Equiv.Zisk.xsUpd xs 32 L) rs2 = regv xs rs2 := regv_xsUpd_ne xs 32 L (by omega) (by omega) (by omega)
  -- step 2: the op on c
  have hz2 := Equiv.Zisk.execute_cop z1 _ op ho 2 _ ia _ L hx1 hpc1 hst1 hc1 _ (Equiv.Zisk.srcB_BSrc _ _ l2)
  rw [← r2, ← zstep_run zrom z1 ia hpc1, u2] at hz2
  dsimp only at hz2
  generalize hz2s : ({ z1 with regs := _ } : Equiv.Zisk.ZState) = z2 at hz2
  have hx2 : z2.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs 32 L) := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert, hx1]
  have hpc2 : z2.regs.get? Register.PC = some (ia + 2) := by subst hz2s; simp [Std.ExtDHashMap.get?_insert]
  have hst2 : z2.regs.get? Register.Step = some (BitVec.addInt (BitVec.addInt st 1) 1) := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert]
  have hc2 : z2.regs.get? Register.Zc = some c := by subst hz2s; simp [Std.ExtDHashMap.get?_insert]; rfl
  have hze2 : z2.regs.get? Register.Zerror = some false := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert, hze1]
  have hm2 : z2.mem = z.mem := by subst hz2s; exact hm1
  -- step 3: the store of c, on to ia + 4
  have hz3 := Equiv.Zisk.execute_store_c z2 _ 0 w (ia + 4 - (ia + 2)) _ (ia + 2) _ hx2 hpc2 hst2 _ c
    (Equiv.Zisk.srcA_ASrc _ _ l1) hc2 (by rw [u1]; exact hwr) (by rw [u1]; exact hnr)
  rw [← r3, ← zstep_run zrom z2 (ia + 2) hpc2, u1] at hz3
  generalize hz3s : ({ z2 with regs := _, mem := _ } : Equiv.Zisk.ZState) = z3 at hz3
  have hx3 : z3.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs 32 L) := by
    subst hz3s; simp [Std.ExtDHashMap.get?_insert, hx2]
  have hpc3 : z3.regs.get? Register.PC = some (ia + 4) := by
    subst hz3s; simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have hst3 : z3.regs.get? Register.Step = some (BitVec.addInt (BitVec.addInt (BitVec.addInt st 1) 1) 1) := by
    subst hz3s; simp [Std.ExtDHashMap.get?_insert]
  have hze3 : z3.regs.get? Register.Zerror = some false := by
    subst hz3s; simp [Std.ExtDHashMap.get?_insert, hze2]
  have hm3 : z3.mem = Equiv.memWrite (n := w) z.mem (regv xs rs1 + 0).toNat (c.setWidth (8 * w)) := by
    subst hz3s; show Equiv.memWrite z2.mem _ _ = _; rw [hm2]
  -- step 4: register 32 to rd
  have hz4 := Equiv.Zisk.execute_mkj z3 (.A_IMM, 0) (Transpiler.srcB 32) (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure rd
    (by omega) (pc + 4 - (ia + 4)) (pc + 4 - (ia + 4)) _ (ia + 4) _ hx3 hpc3 hst3 0 _ (Or.inl rfl) (srcB32 _)
  rw [← r4, ← zstep_run zrom z3 (ia + 4) hpc3] at hz4
  have e32 : regv (Equiv.Zisk.xsUpd xs 32 L) 32 = L := upd32_self xs L
  rw [e32] at hz4
  dsimp only at hz4
  refine ⟨_, .step hz1 (.step hz2 (.step hz3 (.step hz4 (.done _)))),
    ⟨Equiv.Zisk.xsUpd (Equiv.Zisk.xsUpd xs 32 L) rd (zisk_exec (.ZOP_COPYB ()) 0 L).1, ?_, fun k hk => ?_⟩, ?_,
    ⟨BitVec.addInt (BitVec.addInt (BitVec.addInt (BitVec.addInt st 1) 1) 1) 1, ?_, ?_⟩, ?_⟩
  · simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
    exact linkRegs_xs _ _ _ _ hx3
  · have ex : Equiv.Zisk.xsUpd xs 32 L = vectorUpdate xs 32 L := by simp [Equiv.Zisk.xsUpd]
    rw [Equiv.Zisk.copyb_val, ex]; exact regv_xsUpd32 xs L rd L ld hk
  · simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  · simp [Std.ExtDHashMap.get?_insert]
  · simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs]; split <;> simp [Std.ExtDHashMap.get?_insert, hze3]
  · exact hm3

set_option maxHeartbeats 8000000 in
/-- A swap, rd neither rs1 nor rs2: two steps. -/
theorem zisk_amo_swap (zrom : BitVec 64 → zisk_inst) (z : Equiv.Zisk.ZState) (lop : zisk_op)
    (hl : Equiv.Zisk.PureOp lop) (w : Nat) (rd rs1 rs2 : Nat)
    (ld : rd < 32) (l1 : rs1 < 32) (l2 : rs2 < 32) (n1 : rd ≠ rs1) (n2 : rd ≠ rs2)
    (xs : Vector (BitVec 64) 40) (pc st ia : BitVec 64)
    (hx : z.regs.get? Register.Xs = some xs) (hpc : z.regs.get? Register.PC = some pc)
    (hst : z.regs.get? Register.Step = some st) (hze : z.regs.get? Register.Zerror = some false)
    (v : BitVec (8 * w))
    (hrb : readable (regv xs rs1 + 0) w = true) (hwr : writable (regv xs rs1 + 0) w = true)
    (hnr : hits_registers (regv xs rs1 + 0) w = false) (hin : (regv xs rs1 + 0 == INPUT_ADDR && w == 8) = false)
    (hm : Equiv.memRead z.mem w (regv xs rs1 + 0).toNat = some v)
    (r1 : zrom pc = Transpiler.mk (Transpiler.srcA rs1) (.B_IND, 0) lop (Transpiler.storeReg rd) (ia - pc) (ia - pc)
      (w := w))
    (r2 : zrom ia = Transpiler.mk (Transpiler.srcA rs1) (Transpiler.srcB rs2) (.ZOP_COPYB ()) (.ST_IND, 0, false)
      (pc + 4 - ia) (pc + 4 - ia) (w := w)) :
    let L := (zisk_exec lop (regv xs rs1) (v.setWidth 64)).1
    ∃ z', ZRun zrom 2 z z' ∧
      (∃ xs', z'.regs.get? Register.Xs = some xs' ∧ ∀ k, k < 32 → regv xs' k = regv (Equiv.Zisk.xsUpd xs rd L) k) ∧
      z'.regs.get? Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? Register.Step = some st ∧ z'.regs.get? Register.Zerror = some false) ∧
      z'.mem = Equiv.memWrite (n := w) z.mem (regv xs rs1 + 0).toNat ((regv xs rs2).setWidth (8 * w)) := by
  intro L
  have eia : pc + (ia - pc) = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have hz1 := Equiv.Zisk.execute_load_mkj z _ lop hl rd (by omega) (ia - pc) 0 w xs pc st hx hpc hst _
    (Equiv.Zisk.srcA_ASrc xs _ l1) v hrb hnr hin hm
  rw [← r1, ← zstep_run zrom z pc hpc] at hz1
  dsimp only at hz1
  generalize hz1s : ({ z with regs := _ } : Equiv.Zisk.ZState) = z1 at hz1
  have hx1 : z1.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs rd L) := by
    subst hz1s; simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
    exact linkRegs_xs _ _ _ _ hx
  have hpc1 : z1.regs.get? Register.PC = some ia := by subst hz1s; simp [Std.ExtDHashMap.get?_insert, eia]
  have hst1 : z1.regs.get? Register.Step = some (BitVec.addInt st 1) := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert]
  have hze1 : z1.regs.get? Register.Zerror = some false := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, hze]
  have hm1 : z1.mem = z.mem := by subst hz1s; rfl
  have u1 : regv (Equiv.Zisk.xsUpd xs rd L) rs1 = regv xs rs1 := regv_xsUpd_ne xs rd L (by omega) (by omega) (Ne.symm n1)
  have u2 : regv (Equiv.Zisk.xsUpd xs rd L) rs2 = regv xs rs2 := regv_xsUpd_ne xs rd L (by omega) (by omega) (Ne.symm n2)
  have hz2 := Equiv.Zisk.execute_store_mkj z1 _ _ 0 w (pc + 4 - ia) _ ia _ hx1 hpc1 hst1 _ _
    (Equiv.Zisk.srcA_ASrc _ _ l1) (Equiv.Zisk.srcB_BSrc _ _ l2) (by rw [u1]; exact hwr) (by rw [u1]; exact hnr)
  rw [← r2, ← zstep_run zrom z1 ia hpc1, u1, u2] at hz2
  refine ⟨_, .step hz1 (.step hz2 (.done _)), ⟨_, ?_, fun k _ => rfl⟩, ?_,
    ⟨BitVec.addInt (BitVec.addInt st 1) 1, ?_, ?_⟩, ?_⟩
  · simp [Std.ExtDHashMap.get?_insert, hx1]
  · simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  · simp [Std.ExtDHashMap.get?_insert]
  · simp [Std.ExtDHashMap.get?_insert, hze1]
  · show Equiv.memWrite z1.mem _ _ = _
    rw [hm1]

set_option maxHeartbeats 8000000 in
/-- A swap, rd rs1 or rs2: three steps, through register 32. -/
theorem zisk_amo_swap32 (zrom : BitVec 64 → zisk_inst) (z : Equiv.Zisk.ZState) (lop : zisk_op)
    (hl : Equiv.Zisk.PureOp lop) (w : Nat) (rd rs1 rs2 : Nat)
    (ld : rd < 32) (l1 : rs1 < 32) (l2 : rs2 < 32)
    (xs : Vector (BitVec 64) 40) (pc st ia : BitVec 64)
    (hx : z.regs.get? Register.Xs = some xs) (hpc : z.regs.get? Register.PC = some pc)
    (hst : z.regs.get? Register.Step = some st) (hze : z.regs.get? Register.Zerror = some false)
    (v : BitVec (8 * w))
    (hrb : readable (regv xs rs1 + 0) w = true) (hwr : writable (regv xs rs1 + 0) w = true)
    (hnr : hits_registers (regv xs rs1 + 0) w = false) (hin : (regv xs rs1 + 0 == INPUT_ADDR && w == 8) = false)
    (hm : Equiv.memRead z.mem w (regv xs rs1 + 0).toNat = some v)
    (r1 : zrom pc = Transpiler.mk (Transpiler.srcA rs1) (.B_IND, 0) lop (Transpiler.storeReg 32) (ia - pc) (ia - pc)
      (w := w))
    (r2 : zrom ia = Transpiler.mk (Transpiler.srcA rs1) (Transpiler.srcB rs2) (.ZOP_COPYB ()) (.ST_IND, 0, false)
      (ia + 2 - ia) (ia + 2 - ia) (w := w))
    (r3 : zrom (ia + 2) = Transpiler.mk (.A_IMM, 0) (Transpiler.srcB 32) (.ZOP_COPYB ()) (Transpiler.storeReg rd)
      (pc + 4 - (ia + 2)) (pc + 4 - (ia + 2))) :
    let L := (zisk_exec lop (regv xs rs1) (v.setWidth 64)).1
    ∃ z', ZRun zrom 3 z z' ∧
      (∃ xs', z'.regs.get? Register.Xs = some xs' ∧ ∀ k, k < 32 → regv xs' k = regv (Equiv.Zisk.xsUpd xs rd L) k) ∧
      z'.regs.get? Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? Register.Step = some st ∧ z'.regs.get? Register.Zerror = some false) ∧
      z'.mem = Equiv.memWrite (n := w) z.mem (regv xs rs1 + 0).toNat ((regv xs rs2).setWidth (8 * w)) := by
  intro L
  have eia : pc + (ia - pc) = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have hz1 := Equiv.Zisk.execute_load_mkj z _ lop hl 32 (by omega) (ia - pc) 0 w xs pc st hx hpc hst _
    (Equiv.Zisk.srcA_ASrc xs _ l1) v hrb hnr hin hm
  rw [← r1, ← zstep_run zrom z pc hpc] at hz1
  dsimp only at hz1
  generalize hz1s : ({ z with regs := _ } : Equiv.Zisk.ZState) = z1 at hz1
  have hx1 : z1.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs 32 L) := by
    subst hz1s; simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
    exact linkRegs_xs _ _ _ _ hx
  have hpc1 : z1.regs.get? Register.PC = some ia := by subst hz1s; simp [Std.ExtDHashMap.get?_insert, eia]
  have hst1 : z1.regs.get? Register.Step = some (BitVec.addInt st 1) := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert]
  have hze1 : z1.regs.get? Register.Zerror = some false := by
    subst hz1s; simp [Std.ExtDHashMap.get?_insert, hze]
  have hm1 : z1.mem = z.mem := by subst hz1s; rfl
  have u1 : regv (Equiv.Zisk.xsUpd xs 32 L) rs1 = regv xs rs1 := regv_xsUpd_ne xs 32 L (by omega) (by omega) (by omega)
  have u2 : regv (Equiv.Zisk.xsUpd xs 32 L) rs2 = regv xs rs2 := regv_xsUpd_ne xs 32 L (by omega) (by omega) (by omega)
  have hz2 := Equiv.Zisk.execute_store_mkj z1 _ _ 0 w (ia + 2 - ia) _ ia _ hx1 hpc1 hst1 _ _
    (Equiv.Zisk.srcA_ASrc _ _ l1) (Equiv.Zisk.srcB_BSrc _ _ l2) (by rw [u1]; exact hwr) (by rw [u1]; exact hnr)
  rw [← r2, ← zstep_run zrom z1 ia hpc1, u1, u2] at hz2
  generalize hz2s : ({ z1 with regs := _, mem := _ } : Equiv.Zisk.ZState) = z2 at hz2
  have hx2 : z2.regs.get? Register.Xs = some (Equiv.Zisk.xsUpd xs 32 L) := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert, hx1]
  have hpc2 : z2.regs.get? Register.PC = some (ia + 2) := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert]; bv_decide
  have hst2 : z2.regs.get? Register.Step = some (BitVec.addInt (BitVec.addInt st 1) 1) := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert]
  have hze2 : z2.regs.get? Register.Zerror = some false := by
    subst hz2s; simp [Std.ExtDHashMap.get?_insert, hze1]
  have hm2 : z2.mem = Equiv.memWrite (n := w) z.mem (regv xs rs1 + 0).toNat ((regv xs rs2).setWidth (8 * w)) := by
    subst hz2s; show Equiv.memWrite z1.mem _ _ = _; rw [hm1]
  have hz3 := Equiv.Zisk.execute_mkj z2 (.A_IMM, 0) (Transpiler.srcB 32) (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure rd
    (by omega) (pc + 4 - (ia + 2)) (pc + 4 - (ia + 2)) _ (ia + 2) _ hx2 hpc2 hst2 0 _ (Or.inl rfl) (srcB32 _)
  rw [← r3, ← zstep_run zrom z2 (ia + 2) hpc2] at hz3
  have e32 : regv (Equiv.Zisk.xsUpd xs 32 L) 32 = L := upd32_self xs L
  rw [e32] at hz3
  dsimp only at hz3
  refine ⟨_, .step hz1 (.step hz2 (.step hz3 (.done _))),
    ⟨Equiv.Zisk.xsUpd (Equiv.Zisk.xsUpd xs 32 L) rd (zisk_exec (.ZOP_COPYB ()) 0 L).1, ?_, fun k hk => ?_⟩, ?_,
    ⟨BitVec.addInt (BitVec.addInt (BitVec.addInt st 1) 1) 1, ?_, ?_⟩, ?_⟩
  · simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false]
    exact linkRegs_xs _ _ _ _ hx2
  · have ex : Equiv.Zisk.xsUpd xs 32 L = vectorUpdate xs 32 L := by simp [Equiv.Zisk.xsUpd]
    rw [Equiv.Zisk.copyb_val, ex]; exact regv_xsUpd32 xs L rd L ld hk
  · simp [Std.ExtDHashMap.get?_insert]; rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  · simp [Std.ExtDHashMap.get?_insert]
  · simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs]; split <;> simp [Std.ExtDHashMap.get?_insert, hze2]
  · exact hm2

end Equiv.Sim
