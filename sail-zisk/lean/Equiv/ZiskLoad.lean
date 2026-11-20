/-
  sail-zisk's semantics for the instruction the transpiler emits for a load
  (load_op): op(r[rs1], mem[r[rs1] + off]) -> r[rd], on to pc + 4, the
  memory read `ind_width` bytes wide (B_IND).
-/
import Equiv.ZiskImm
import Equiv.MemRead

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- The instruction the transpiler emits for a load from rs1 + off, w bytes
    wide, rs1 not x0 (load_op). -/
def loadInst (op : zisk_op) (rd rs1 : Nat) (off : BitVec 64) (w : Nat) : zisk_inst :=
  { a_src := .A_REG, a_imm := .ofNat 64 rs1, b_src := .B_IND, b_imm := off,
    ind_width := w, op, store := .ST_REG, store_off := .ofNat 64 rd,
    store_pc := false, set_pc := false, jmp_off1 := 4, jmp_off2 := 4, is_end := false }

/-- An 8-byte read inside ZisK's memory map, off the register file and not
    of the free input: the bytes there. -/
theorem read_mem8_run (s : ZState) (a : BitVec 64) (v : BitVec 64)
    (hrb : readable a 8 = true) (hnr : hits_registers a 8 = false) (hin : a ≠ INPUT_ADDR)
    (hm : Equiv.memRead s.mem 8 a.toNat = some v) :
    (read_mem a 8).run s = .ok v s := by
  have hr := Equiv.zisk_readBytes s 8 _ v hm
  simp only [read_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, hrb, hnr, if_true, run_bind',
    run_pure', Bool.not_false]
  try dsimp only
  have : (a == INPUT_ADDR && (8 == 8)) = false := by simp [hin]
  rw [this]
  simp only [Bool.false_eq_true, if_false, Sail.ConcurrencyInterfaceV1.PreSail.read_ram, run_bind']
  rw [hr]; rfl

theorem copyb_pure : PureOp (.ZOP_COPYB ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

theorem copyb_val (a b : BitVec 64) : (zisk_exec (.ZOP_COPYB ()) a b) = (b, false) := rfl

set_option maxHeartbeats 4000000 in
/-- `ld` as the transpiler emits it (copyb of the 8 bytes at r[rs1] + off):
    rd gets them, and the step moves on to pc + 4. The address is in ZisK's
    memory map, off the register file, and not the free input's. -/
theorem execute_ld (s : ZState) (rd rs1 : Nat) (off : BitVec 64)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (hd : 1 ≤ rd ∧ rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (v : BitVec 64)
    (hrb : readable (xs[(rs1 : Int)]! + off) 8 = true) (hnr : hits_registers (xs[(rs1 : Int)]! + off) 8 = false)
    (hin : xs[(rs1 : Int)]! + off ≠ INPUT_ADDR)
    (hm : Equiv.memRead s.mem 8 (xs[(rs1 : Int)]! + off).toNat = some v) :
    (zisk_execute (loadInst (.ZOP_COPYB ()) rd rs1 off 8)).run s = .ok false
      { s with regs := (((((s.regs.insert Xs (vectorUpdate xs rd v)).insert
          Zc v).insert Zflag false).insert Step (BitVec.addInt st 1)).insert
          PC (pc + 4)) } := by
  have n1 : rs1 ≠ 0 := by omega
  have nd : rd ≠ 0 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have ld : (rd : Int) < 40 := by omega
  have hr := Equiv.zisk_readBytes s 8 _ v hm
  have hm' := copyb_pure.mem
  have hp' := copyb_pure.notPre
  simp [zisk_execute, loadInst, source_a, source_b, rX, wX, store_c, value_to_store, next_pc,
    hm', hp', copyb_val, reg_index rs1 (by omega), reg_index rd (by omega),
    Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, nd, l1, ld, writeReg_run]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [read_mem8_run s _ v hrb hnr hin hm]; dsimp only
  rw [readReg_run s Xs xs hx]; dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
  rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]; try dsimp only
  all_goals simp [_root_.Zisk.Functions.zero_extend, Sail.BitVec.zeroExtend]

end Equiv.Zisk
