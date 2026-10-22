/-
  sail-zisk's semantics, for the instructions the transpiler emits: what
  `zisk_execute` does to a ZisK state, as equations on lean-sail's state
  monad (sail-zisk's Lean runs in EStateM directly, with no free monad in
  between). The ZisK side of the equivalence proofs, as Equiv/Riscv.lean is
  the RISC-V side.

  So far: `add rd, rs1, rs2` as the transpiler emits it for rs1, rs2 ≠ x0
  (create_register_op), one ZisK instruction; then any pure op emitted the
  same way, which covers the R-type instructions.
-/
import Zisk

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

abbrev ZState := Sail.ConcurrencyInterfaceV1.SequentialState Zisk.RegisterType
  Sail.ConcurrencyInterfaceV1.trivialChoiceSource

/-- The instruction the transpiler emits for `add rd, rs1, rs2`, rs1, rs2 ≠ x0
    (create_register_op): add(r[rs1], r[rs2]) -> r[rd], on to pc + 4. -/
def addInst (rd rs1 rs2 : Nat) : zisk_inst :=
  { a_src := .A_REG, a_imm := .ofNat 64 rs1, b_src := .B_REG, b_imm := .ofNat 64 rs2,
    ind_width := 8, op := .ZOP_ADD (), store := .ST_REG, store_off := .ofNat 64 rd,
    store_pc := false, set_pc := false, jmp_off1 := 4, jmp_off2 := 4, is_end := false }

/-- The register index an instruction field holds: its low 6 bits. -/
theorem reg_index (n : Nat) (h : n < 64) :
    BitVec.toNatInt (Sail.BitVec.extractLsb (BitVec.ofNat 64 n) 5 0) = n := by
  simp [Sail.BitVec.extractLsb, BitVec.toNatInt, BitVec.extractLsb, Nat.mod_eq_of_lt h]

@[simp] theorem run_bind' (x : EStateM ε σ α) (g : α → EStateM ε σ β) (s : σ) :
    (x >>= g).run s = match x.run s with
      | .ok a s' => (g a).run s'
      | .error e s' => .error e s' := rfl

@[simp] theorem run_pure' (a : α) (s : σ) : (pure a : EStateM ε σ α).run s = .ok a s := rfl

@[simp] theorem run_map' (f : α → β) (x : EStateM ε σ α) (s : σ) :
    (f <$> x).run s = match x.run s with
      | .ok a s' => .ok (f a) s'
      | .error e s' => .error e s' := by
  show EStateM.map f x s = _
  simp only [EStateM.map, EStateM.run]; split <;> simp_all

theorem readReg_run (s : ZState) (r : Register) (v : RegisterType r) (h : s.regs.get? r = some v) :
    (readReg r).run s = .ok v s := by
  simp [Zisk.readReg, Sail.ConcurrencyInterfaceV1.PreSail.readReg, h]

theorem readReg_run' (s : ZState) (r : Register) :
    (readReg r).run s = match s.regs.get? r with
      | some v => .ok v s
      | none => .error .Unreachable s := by
  simp only [Zisk.readReg, Sail.ConcurrencyInterfaceV1.PreSail.readReg]
  split <;> simp_all <;> rfl

theorem writeReg_run (s : ZState) (r : Register) (v : RegisterType r) :
    (writeReg r v).run s = .ok () { s with regs := s.regs.insert r v } := rfl

theorem assert_run (s : ZState) (b : Bool) (m : String) (h : b = true) :
    (Zisk.assert b m).run s = .ok () s := by
  subst h; rfl

theorem execute_add (s : ZState) (rd rs1 rs2 : Nat)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (h2 : 1 ≤ rs2 ∧ rs2 < 32) (hd : 1 ≤ rd ∧ rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    (zisk_execute (addInst rd rs1 rs2)).run s = .ok false
      { s with regs := (((((s.regs.insert Xs (vectorUpdate xs rd (xs[(rs1 : Int)]! + xs[(rs2 : Int)]!))).insert
          Zc (xs[(rs1 : Int)]! + xs[(rs2 : Int)]!)).insert Zflag false).insert Step (BitVec.addInt st 1)).insert
          PC (pc + 4)) } := by
  have n1 : rs1 ≠ 0 := by omega
  have n2 : rs2 ≠ 0 := by omega
  have nd : rd ≠ 0 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have l2 : (rs2 : Int) < 40 := by omega
  have ld : (rd : Int) < 40 := by omega
  simp [zisk_execute, addInst, source_a, source_b, rX, wX, store_c, value_to_store, next_pc,
    zisk_exec, zisk_exec_mem, zisk_op_is_precompiled, Functions.zisk_op_attrs,
    reg_index rs1 (by omega), reg_index rs2 (by omega), reg_index rd (by omega),
    Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, n2, nd, l1, l2, ld,
    writeReg_run]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]; try dsimp only
  all_goals rfl

/-- The instruction the transpiler emits for an R-type instruction whose
    operands are not x0 (create_register_op): op(r[rs1], r[rs2]) -> r[rd],
    on to pc + 4. -/
def regInst (op : zisk_op) (rd rs1 rs2 : Nat) : zisk_inst :=
  { a_src := .A_REG, a_imm := .ofNat 64 rs1, b_src := .B_REG, b_imm := .ofNat 64 rs2,
    ind_width := 8, op, store := .ST_REG, store_off := .ofNat 64 rd,
    store_pc := false, set_pc := false, jmp_off1 := 4, jmp_off2 := 4, is_end := false }

/-- An op that is a pure function of a and b: not one of zisk_exec_mem's,
    not a precompile, not halt. -/
structure PureOp (op : zisk_op) : Prop where
  mem : ∀ a b e, zisk_exec_mem op a b e = pure none
  notPre : zisk_op_is_precompiled op = false
  notHalt : ∀ u, op ≠ .ZOP_HALT u

/-- Any pure op, as the transpiler emits it for a register instruction: rd
    gets its c, and the step moves on to pc + 4 whatever its flag. -/
theorem execute_regop (s : ZState) (op : zisk_op) (hop : PureOp op) (rd rs1 rs2 : Nat)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (h2 : 1 ≤ rs2 ∧ rs2 < 32) (hd : 1 ≤ rd ∧ rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    let r := zisk_exec op xs[(rs1 : Int)]! xs[(rs2 : Int)]!
    (zisk_execute (regInst op rd rs1 rs2)).run s = .ok false
      { s with regs := (((((s.regs.insert Xs (vectorUpdate xs rd r.1)).insert
          Zc r.1).insert Zflag r.2).insert Step (BitVec.addInt st 1)).insert
          PC (pc + 4)) } := by
  intro r
  have n1 : rs1 ≠ 0 := by omega
  have n2 : rs2 ≠ 0 := by omega
  have nd : rd ≠ 0 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have l2 : (rs2 : Int) < 40 := by omega
  have ld : (rd : Int) < 40 := by omega
  have hm := hop.mem
  have hp := hop.notPre
  simp [zisk_execute, regInst, source_a, source_b, rX, wX, store_c, value_to_store, next_pc,
    hm, hp, reg_index rs1 (by omega), reg_index rs2 (by omega), reg_index rd (by omega),
    Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, n2, nd, l1, l2, ld, writeReg_run]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run s Xs xs hx]; try dsimp only
  split
  · exact absurd rfl (hop.notHalt ())
  · simp only [run_bind', run_map', writeReg_run]
    rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
    rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]; try dsimp only
    all_goals rfl

end Equiv.Zisk
