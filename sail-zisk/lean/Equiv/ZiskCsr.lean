/-
  sail-zisk's semantics for the copy the transpiler makes of `csrr rd,
  marchid`: copyb of the 8 bytes at a fixed address (a b source of
  memory), into rd; on to pc + j.
-/
import Equiv.ZiskSpec

set_option linter.unusedSimpArgs false

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

set_option maxHeartbeats 4000000 in
/-- copyb of the 8 bytes at addr into rd (none for x0; any of the 40); on
    to pc + j. -/
theorem execute_bmem (s : ZState) (addr : BitVec 64) (rd : Nat) (hd : rd < 40) (j : BitVec 64)
    (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : s.regs.get? Xs = some xs) (hpc : s.regs.get? PC = some pc) (hst : s.regs.get? Step = some st)
    (v : BitVec (8 * 8))
    (hrb : readable addr 8 = true) (hnr : hits_registers addr 8 = false)
    (hin : (addr == INPUT_ADDR && (8 : Nat) == 8) = false)
    (hm : Equiv.memRead s.mem 8 addr.toNat = some v) :
    (zisk_execute (Transpiler.mk (.A_IMM, 0) (.B_MEM, addr) (.ZOP_COPYB ()) (Transpiler.storeReg rd) j j)).run s =
      .ok false
      { s with regs := ((((linkRegs s.regs xs rd v).insert Zc v).insert Zflag false).insert
          Step (BitVec.addInt st 1)).insert PC (pc + j) } := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  have hh := copyb_pure.notHalt ()
  have ld : (rd : Int) < 40 := by omega
  by_cases h0 : rd = 0 <;>
  · simp (disch := omega) [zisk_execute, Transpiler.mk, Transpiler.storeReg, h0, source_a, source_b, rX, wX,
      store_c, value_to_store, next_pc, hm', hp, linkRegs, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert,
      writeReg_run, ld, reg_index, copyb_val, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only)
      | (rw [read_mem_run s _ 8 v (by simpa using hrb) (by simpa using hnr) (by simpa using hin)
            (by simpa using hm)]; try dsimp only)
      | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst, hpc, hx, *]))
    all_goals simp (disch := omega) [copyb_val, *]

end Equiv.Zisk
