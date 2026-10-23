/-
  sail-zisk's semantics for the instructions the transpiler emits for lui
  and auipc (copyb of a constant to rd), jal (flag; rd gets pc + 4; the step
  goes to pc + off) and jalr with an even offset (and of the mask and r[rs1];
  set_pc: the step goes to that + off; rd gets pc + 4). A store to x0 is no
  store at all (ZiskInstBuilder::store returns early for register 0).
-/
import Equiv.ZiskCtl

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- The instruction the transpiler emits for lui and auipc: the constant c
    (computed at transpile time) to rd. -/
def constInst (rd : Nat) (c : BitVec 64) : zisk_inst :=
  { a_src := .A_IMM, a_imm := 0, b_src := .B_IMM, b_imm := c,
    ind_width := 8, op := .ZOP_COPYB (), store := .ST_REG, store_off := .ofNat 64 rd,
    store_pc := false, set_pc := false, jmp_off1 := 4, jmp_off2 := 4, is_end := false }

/-- The store of a link to rd: none for x0. -/
def linkStore (rd : Nat) : c_store := if rd = 0 then .ST_NONE else .ST_REG

/-- The instruction the transpiler emits for jal. -/
def jalInst (rd : Nat) (off : BitVec 64) : zisk_inst :=
  { a_src := .A_IMM, a_imm := 0, b_src := .B_IMM, b_imm := 0,
    ind_width := 8, op := .ZOP_FLAG (), store := linkStore rd, store_off := .ofNat 64 rd,
    store_pc := decide (rd ≠ 0), set_pc := false, jmp_off1 := off, jmp_off2 := 4, is_end := false }

/-- jalr's mask: bit 0 cleared. -/
def JALR_MASK : BitVec 64 := 0xfffffffffffffffe#64

/-- The instruction the transpiler emits for jalr with an even offset. -/
def jalrInst (rd rs1 : Nat) (off : BitVec 64) : zisk_inst :=
  { a_src := .A_IMM, a_imm := JALR_MASK, b_src := .B_REG, b_imm := .ofNat 64 rs1,
    ind_width := 8, op := .ZOP_AND (), store := linkStore rd, store_off := .ofNat 64 rd,
    store_pc := decide (rd ≠ 0), set_pc := true, jmp_off1 := off, jmp_off2 := 4, is_end := false }

/-- The register vector after writing v to rd: unchanged for x0. -/
def xsUpd (xs : Vector (BitVec 64) 40) (rd : Nat) (v : BitVec 64) : Vector (BitVec 64) 40 :=
  if rd = 0 then xs else vectorUpdate xs rd v

theorem flag_pure : PureOp (.ZOP_FLAG ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

theorem and_pure : PureOp (.ZOP_AND ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

set_option maxHeartbeats 4000000 in
theorem execute_const (s : ZState) (rd : Nat) (c : BitVec 64) (hd : 1 ≤ rd ∧ rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    (zisk_execute (constInst rd c)).run s = .ok false
      { s with regs := (((((s.regs.insert Xs (vectorUpdate xs rd c)).insert Zc c).insert Zflag false).insert
          Step (BitVec.addInt st 1)).insert PC (pc + 4)) } := by
  have nd : rd ≠ 0 := by omega
  have ld : (rd : Int) < 40 := by omega
  have hm := copyb_pure.mem
  have hp := copyb_pure.notPre
  simp [zisk_execute, constInst, source_a, source_b, wX, store_c, value_to_store, next_pc,
    hm, hp, copyb_val, reg_index rd (by omega), Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert,
    nd, ld, writeReg_run]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]

/-- The register map after a link to rd: unchanged for x0. -/
def linkRegs (m : Std.ExtDHashMap Register RegisterType) (xs : Vector (BitVec 64) 40) (rd : Nat)
    (v : BitVec 64) : Std.ExtDHashMap Register RegisterType :=
  if rd = 0 then m else m.insert Xs (vectorUpdate xs rd v)

set_option maxHeartbeats 4000000 in
/-- jal as the transpiler emits it: rd (if not x0) gets pc + 4, and the step
    goes to pc + off. -/
theorem execute_jal (s : ZState) (rd : Nat) (off : BitVec 64) (hd : rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    (zisk_execute (jalInst rd off)).run s = .ok false
      { s with regs := ((((linkRegs s.regs xs rd (pc + 4)).insert Zc 0).insert Zflag true).insert
          Step (BitVec.addInt st 1)).insert PC (pc + off) } := by
  have ld : (rd : Int) < 40 := by omega
  have hm := flag_pure.mem
  have hp := flag_pure.notPre
  have hv : zisk_exec (.ZOP_FLAG ()) 0#64 0#64 = (0#64, true) := rfl
  by_cases h0 : rd = 0
  · subst h0
    simp [zisk_execute, jalInst, linkStore, linkRegs, source_a, source_b, store_c, value_to_store, next_pc,
      hm, hp, hv, writeReg_run]
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc])
  · simp [zisk_execute, jalInst, linkStore, linkRegs, h0, source_a, source_b, wX, store_c, value_to_store,
      next_pc, hm, hp, hv, reg_index rd (by omega), Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert,
      ld, writeReg_run]
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx])

set_option maxHeartbeats 4000000 in
/-- jalr with an even offset as the transpiler emits it: rd (if not x0) gets
    pc + 4, and the step goes to (r[rs1] with bit 0 cleared) + off. -/
theorem execute_jalr (s : ZState) (rd rs1 : Nat) (off : BitVec 64) (hd : rd < 32) (h1 : 1 ≤ rs1 ∧ rs1 < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st) :
    (zisk_execute (jalrInst rd rs1 off)).run s = .ok false
      { s with regs := ((((linkRegs s.regs xs rd (pc + 4)).insert Zc (JALR_MASK &&& xs[(rs1 : Int)]!)).insert
          Zflag false).insert Step (BitVec.addInt st 1)).insert PC ((JALR_MASK &&& xs[(rs1 : Int)]!) + off) } := by
  have ld : (rd : Int) < 40 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have n1 : rs1 ≠ 0 := by omega
  have hm := and_pure.mem
  have hp := and_pure.notPre
  have hv : ∀ a b, zisk_exec (.ZOP_AND ()) a b = (a &&& b, false) := fun _ _ => rfl
  by_cases h0 : rd = 0
  · subst h0
    simp [zisk_execute, jalrInst, linkStore, linkRegs, source_a, source_b, rX, store_c, value_to_store,
      next_pc, hm, hp, hv, reg_index rs1 (by omega), Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert,
      n1, l1, writeReg_run]
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hx])
  · simp [zisk_execute, jalrInst, linkStore, linkRegs, h0, source_a, source_b, rX, wX, store_c, value_to_store,
      next_pc, hm, hp, hv, reg_index rd (by omega), reg_index rs1 (by omega), Zisk.assert,
      Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, l1, ld, writeReg_run]
    repeat (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx])

end Equiv.Zisk
