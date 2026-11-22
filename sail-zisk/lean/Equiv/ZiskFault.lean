/-
  sail-zisk on an access outside its memory map: a load the spec emits
  (B_IND) of bytes ZisK cannot read, or a store (ST_IND) to bytes it cannot
  write, fails — `read_mem` and `write_mem` assert.
-/
import Equiv.ZiskAmo

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

theorem read_mem_fault (s : ZState) (a : BitVec 64) (n : Nat) (hr : readable a n = false) :
    ∃ e, (read_mem a n).run s = .error e s := by
  simp only [read_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, hr, run_bind']
  exact ⟨_, rfl⟩

theorem write_mem_fault (s : ZState) (a : BitVec 64) (n : Nat) (hw : writable a n = false) :
    ∃ e, ∀ v : BitVec (8 * n), (write_mem a n v).run s = .error e s := by
  simp only [write_mem, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, hw, run_bind']
  exact ⟨_, fun _ => rfl⟩

set_option maxHeartbeats 4000000 in
/-- A load as the spec builds it, of bytes ZisK cannot read: the step fails. -/
theorem execute_load_fault_z (s : ZState) (ra : a_src × BitVec 64) (op : zisk_op) (rd : Nat)
    (j : BitVec 64) (off : BitVec 64) (w : Nat) (xs : Vector (BitVec 64) 40)
    (hx : s.regs.get? Xs = some xs) (va : BitVec 64) (ha : ASrc ra xs va)
    (hrb : readable (va + off) w = false) :
    ∃ e s', (zisk_execute (Transpiler.mk ra (.B_IND, off) op (Transpiler.storeReg rd) j j (w := w))).run s
      = .error e s' := by
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩
  · obtain ⟨e, he⟩ := read_mem_fault s _ w hrb
    simp only [zisk_execute, Transpiler.mk, source_a, source_b, run_bind', run_pure', Int.toNat_natCast]
    simp only [he]
    exact ⟨_, _, rfl⟩
  · subst hv1
    have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩
    obtain ⟨e, he⟩ := read_mem_fault s _ w hrb
    simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, Zisk.assert,
      Sail.ConcurrencyInterfaceV1.PreSail.assert, reg_index, *]
    rw [readReg_run s Xs xs hx]; try dsimp only
    have he' := he
    simp (disch := omega) only [getElem!_pos] at he'
    simp only [he']
    exact ⟨_, _, rfl⟩

set_option maxHeartbeats 4000000 in
/-- A store as the spec builds it, to bytes ZisK cannot write: the step fails. -/
theorem execute_store_fault_z (s : ZState) (ra : a_src × BitVec 64) (rb : b_src × BitVec 64)
    (off : BitVec 64) (w : Nat) (j : BitVec 64) (xs : Vector (BitVec 64) 40)
    (hx : s.regs.get? Xs = some xs) (va vb : BitVec 64) (ha : ASrc ra xs va) (hb : BSrc rb xs vb)
    (hwr : writable (va + off) w = false) :
    ∃ e s', (zisk_execute (Transpiler.mk ra rb (.ZOP_COPYB ()) (.ST_IND, off, false) j j (w := w))).run s
      = .error e s' := by
  have hm' := copyb_pure.mem
  have hp := copyb_pure.notPre
  rcases ha with rfl | ⟨r1, h1a, h1b, rfl, hv1⟩ <;> rcases hb with rfl | ⟨r2, h2a, h2b, rfl, hv2⟩ <;>
  · (try subst hv1)
    (try subst hv2)
    (try have e1 : r1 ≠ 0 ∧ (r1 : Int) < 40 := ⟨by omega, by omega⟩)
    (try have e2 : r2 ≠ 0 ∧ (r2 : Int) < 40 := ⟨by omega, by omega⟩)
    obtain ⟨e, he⟩ := write_mem_fault s _ w hwr
    have he' := he
    simp (disch := omega) only [getElem!_pos] at he'
    simp (disch := omega) [zisk_execute, Transpiler.mk, source_a, source_b, rX, store_c, value_to_store,
      next_pc, hm', hp, copyb_val, Zisk.assert, Sail.ConcurrencyInterfaceV1.PreSail.assert, writeReg_run,
      reg_index, *]
    repeat (first
      | (rw [readReg_run s Xs xs hx]; try dsimp only))
    all_goals (simp only [he', Int.toNat_natCast]; try exact ⟨_, _, rfl⟩)

end Equiv.Zisk
