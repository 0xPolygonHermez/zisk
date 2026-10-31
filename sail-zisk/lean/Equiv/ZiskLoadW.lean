/-
  sail-zisk's semantics for every load the transpiler emits (load_op): any
  pure op on r[rs1] and the w bytes at r[rs1] + off, w ∈ {1, 2, 4, 8}, as
  Equiv/ZiskLoad.lean has it for ld.
-/
import Equiv.ZiskLoad

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

/-- A w-byte read inside ZisK's memory map, off the register file and not
    the free input's (which only an 8-byte read at INPUT_ADDR is): the bytes
    there. -/
theorem read_mem_run (s : ZState) (a : BitVec 64) (n : Nat) (v : BitVec (8 * n))
    (hrb : readable a n = true) (hnr : hits_registers a n = false)
    (hin : (a == INPUT_ADDR && n == 8) = false)
    (hm : Equiv.memRead s.mem n a.toNat = some v) :
    (read_mem a n).run s = .ok v s := by
  have hr := Equiv.zisk_readBytes s n _ v hm
  simp only [read_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, hrb, hnr, if_true, run_bind',
    run_pure', Bool.not_false]
  try dsimp only
  rw [hin]
  simp only [Bool.false_eq_true, if_false, Sail.ConcurrencyInterfaceV1.PreSail.read_ram, run_bind']
  rw [hr]; rfl

set_option maxHeartbeats 4000000 in
/-- A load as the transpiler emits it (op of r[rs1] and the w bytes at
    r[rs1] + off, zero-extended): rd gets op's c, and the step moves on to
    pc + 4. -/
theorem execute_load (s : ZState) (op : zisk_op) (hop : PureOp op) (rd rs1 : Nat) (off : BitVec 64) (w : Nat)
    (h1 : 1 ≤ rs1 ∧ rs1 < 32) (hd : 1 ≤ rd ∧ rd < 32)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (v : BitVec (8 * w))
    (hrb : readable (xs[(rs1 : Int)]! + off) w = true) (hnr : hits_registers (xs[(rs1 : Int)]! + off) w = false)
    (hin : (xs[(rs1 : Int)]! + off == INPUT_ADDR && w == 8) = false)
    (hm : Equiv.memRead s.mem w (xs[(rs1 : Int)]! + off).toNat = some v) :
    let r := zisk_exec op xs[(rs1 : Int)]! (v.setWidth 64)
    (zisk_execute (loadInst op rd rs1 off w)).run s = .ok false
      { s with regs := (((((s.regs.insert Xs (vectorUpdate xs rd r.1)).insert
          Zc r.1).insert Zflag r.2).insert Step (BitVec.addInt st 1)).insert
          PC (pc + 4)) } := by
  intro r
  have n1 : rs1 ≠ 0 := by omega
  have nd : rd ≠ 0 := by omega
  have l1 : (rs1 : Int) < 40 := by omega
  have ld : (rd : Int) < 40 := by omega
  have hm' := hop.mem
  have hp' := hop.notPre
  simp [zisk_execute, loadInst, source_a, source_b, rX, wX, store_c, value_to_store, next_pc,
    hm', hp', reg_index rs1 (by omega), reg_index rd (by omega),
    Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, n1, nd, l1, ld, writeReg_run]
  rw [readReg_run s Xs xs hx]; try dsimp only
  rw [read_mem_run s _ w v hrb hnr hin hm]; dsimp only
  rw [readReg_run s Xs xs hx]; dsimp only
  split
  · exact absurd rfl (hop.notHalt ())
  · simp only [run_bind', run_map', writeReg_run]
    rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hst]; try dsimp only
    rw [readReg_run']; simp only [Std.ExtDHashMap.get?_insert]; simp [hpc]; try dsimp only
    all_goals simp [r, _root_.Zisk.Functions.zero_extend, Sail.BitVec.zeroExtend]

theorem sextb_pure : PureOp (.ZOP_SIGNEXTEND_B ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide
theorem sexth_pure : PureOp (.ZOP_SIGNEXTEND_H ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide
theorem sextw_pure : PureOp (.ZOP_SIGNEXTEND_W ()) := by
  refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;> first | rfl | simp | decide

end Equiv.Zisk
