/-
  The BIOS's end on sail-zisk: from the output block (0x101C), where a
  program returns to and where the exit with code 0 goes, its loop reads
  the 32 output words and publishes them, its 15 reads touch the input, and
  it jumps to ROM_EXIT, which ends the run: no error, the memory unchanged.
  The exit system call reaches it through the ecall handler (0x107C), or,
  with a code c ≠ 0, halts with c.
-/
import Equiv.BiosSim
import Equiv.FusedSim

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Transpiler (bios)

theorem eq_val (a b : BitVec 64) : zisk_exec (.ZOP_EQ ()) a b = (if a = b then 1 else 0, decide (a = b)) := by
  simp [zisk_exec, bool_c]
  split <;> simp_all <;> rfl

theorem add_val (a b : BitVec 64) : zisk_exec (.ZOP_ADD ()) a b = (a + b, false) := by
  first | rfl | simp [zisk_exec]

theorem ofNat_add64 (a b : Nat) : BitVec.ofNat 64 a + BitVec.ofNat 64 b = BitVec.ofNat 64 (a + b) := by
  apply BitVec.eq_of_toNat_eq; simp [BitVec.toNat_add]

theorem out_region : ∀ j, j < 32 →
    readable (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)) 8 = true ∧
    hits_registers (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)) 8 = false ∧
    Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j) ≠ INPUT_ADDR := by
  decide

/-- The 32 output words are in memory, as the output block reads them. -/
def OutBytes (m : Equiv.Mem) : Prop :=
  ∀ j, j < 32 → ∃ v, Equiv.memRead m 8 (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)).toNat = some v

/-- The output block's loop head (0x1028), at word i: x11 = 32, x12 = i, x13
    the word's address. -/
structure LoopAt (z : Equiv.Zisk.ZState) (m : Equiv.Mem) (i : Nat) : Prop where
  pc : z.regs.get? .PC = some 0x1028
  xs : ∃ xs : Vector (BitVec 64) 40, z.regs.get? .Xs = some xs ∧ xs[11]! = 32 ∧ xs[12]! = BitVec.ofNat 64 i ∧
    xs[13]! = Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * i)
  step : ∃ st, z.regs.get? .Step = some st
  zerr : z.regs.get? .Zerror = some false
  mem : z.mem = m

set_option maxHeartbeats 8000000 in
/-- One round of the loop: word i read and published, on to word i + 1. -/
theorem loop_iter (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (m : Equiv.Mem) (hm : OutBytes m) (i : Nat) (hi : i < 32)
    (z : Equiv.Zisk.ZState) (h : LoopAt z m i) : ∃ z', ZRun zrom 5 z z' ∧ LoopAt z' m (i + 1) := by
  obtain ⟨hpc, ⟨xs, hx, h11, h12, h13⟩, ⟨st, hst⟩, hze, hzm⟩ := h
  have hne : decide ((32 : BitVec 64) = BitVec.ofNat 64 i) = false := by
    simp only [decide_eq_false_iff_not]; intro e
    have := congrArg BitVec.toNat e; simp at this; omega
  -- 0x1028: x11 = x12? Not yet.
  have e1 := Equiv.Zisk.execute_mkj z (.A_REG, 11) (.B_REG, 12) (.ZOP_EQ ()) Equiv.Zisk.eq_pure 0 (by omega)
    0x14 4 xs _ st hx hpc hst 32 (BitVec.ofNat 64 i)
    (Or.inr ⟨11, by omega, by omega, rfl, by rw [getElem!_int]; exact h11⟩)
    (Or.inr ⟨12, by omega, by omega, rfl, by rw [getElem!_int]; exact h12⟩)
  simp only [Equiv.Zisk.linkRegs, eq_val, hne, if_true, Bool.false_eq_true, if_false] at e1
  obtain ⟨z1, r1, p1, x1, ⟨s1, t1⟩, f1, -, m1⟩ := zstep_facts hpc
    (hb.get (a := 0x1028) (i := Transpiler.mk (.A_REG, 11) (.B_REG, 12) (.ZOP_EQ ()) (Transpiler.storeReg 0) 0x14 4)
      (by simp [bios, Transpiler.storeReg])) e1
  have p1' : z1.regs.get? .PC = some 0x102C := by rw [p1]; rfl
  rw [hx] at x1
  have q1 : z1.regs.get? .Zerror = some false := by simp [f1, Std.ExtDHashMap.get?_insert, hze]
  -- 0x102C: the word at x13
  have add0 : ∀ a : BitVec 64, a + 0 = a := fun a => BitVec.add_zero a
  obtain ⟨v, hv⟩ := hm i hi
  have hr := out_region i hi
  have e2 := Equiv.Zisk.execute_read_none z1 (.A_REG, 13) 0 0 4 xs _ s1 x1 p1' t1
    (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * i))
    (Or.inr ⟨13, by omega, by omega, rfl, by rw [getElem!_int]; exact h13⟩) v
    (by rw [add0]; exact hr.1) (by rw [add0]; exact hr.2.1)
    (by rw [add0]; exact hr.2.2) (by rw [m1, hzm, add0]; exact hv)
  obtain ⟨z2, r2, p2, x2, ⟨s2, t2⟩, f2, c2, m2⟩ := zstep_facts p1'
    (hb.get (a := 0x102C) (i := Transpiler.mk (.A_REG, 13) (.B_IND, 0) (.ZOP_COPYB ()) (.ST_NONE, 0, false) 0 4)
      (by simp [bios])) e2
  have p2' : z2.regs.get? .PC = some 0x1030 := by rw [p2]; rfl
  have x2' : z2.regs.get? .Xs = some xs := by rw [x2]; simpa [Std.ExtDHashMap.get?_insert] using x1
  have q2 : z2.regs.get? .Zerror = some false := by simp [f2, Std.ExtDHashMap.get?_insert, q1]
  -- 0x1030: publish it
  have e3 := Equiv.Zisk.execute_pubout z2 12 (by omega) xs _ s2 v x2' p2' t2 c2
  obtain ⟨z3, r3, p3, x3, ⟨s3, t3⟩, f3, -, m3⟩ := zstep_facts p2'
    (hb.get (a := 0x1030) (i := Transpiler.mk (.A_REG, BitVec.ofNat 64 12) (.B_C, 0) (.ZOP_PUBOUT ())
      (.ST_NONE, 0, false) 0 4) (by simp [bios])) e3
  have p3' : z3.regs.get? .PC = some 0x1034 := by rw [p3]; rfl
  have x3' : z3.regs.get? .Xs = some xs := by rw [x3]; simpa [Std.ExtDHashMap.get?_insert] using x2'
  have q3 : z3.regs.get? .Zerror = some false := by simp [f3, Std.ExtDHashMap.get?_insert, q2]
  -- 0x1034: x13 += 8
  have e4 := Equiv.Zisk.execute_mkj z3 (.A_REG, 13) (.B_IMM, 8) (.ZOP_ADD ()) Equiv.Zisk.add_pure 13 (by omega)
    0 4 xs _ s3 x3' p3' t3 (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * i)) 8
    (Or.inr ⟨13, by omega, by omega, rfl, by rw [getElem!_int]; exact h13⟩) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, add_val, Bool.false_eq_true, if_false, Nat.reduceEqDiff, ↓reduceIte] at e4
  obtain ⟨z4, r4, p4, x4, ⟨s4, t4⟩, f4, -, m4⟩ := zstep_facts p3'
    (hb.get (a := 0x1034) (i := Transpiler.mk (.A_REG, 13) (.B_IMM, 8) (.ZOP_ADD ()) (Transpiler.storeReg 13) 0 4)
      (by simp [bios, Transpiler.storeReg])) e4
  have p4' : z4.regs.get? .PC = some 0x1038 := by rw [p4]; rfl
  have q4 : z4.regs.get? .Zerror = some false := by simp [f4, Std.ExtDHashMap.get?_insert, q3]
  have x4' : z4.regs.get? .Xs =
      some (vectorUpdate xs 13 (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * i) + 8)) := by
    rw [x4]; simp [Std.ExtDHashMap.get?_insert]
  -- 0x1038: x12 += 1, back to the head
  have e5 := Equiv.Zisk.execute_mkj z4 (.A_REG, 12) (.B_IMM, 1) (.ZOP_ADD ()) Equiv.Zisk.add_pure 12 (by omega)
    4 (-16) _ _ s4 x4' p4' t4 (BitVec.ofNat 64 i) 1
    (Or.inr ⟨12, by omega, by omega, rfl,
      by rw [getElem!_int, getElem!_update _ _ _ _ (by omega), if_neg (by omega)]; exact h12⟩) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, add_val, Bool.false_eq_true, if_false, Nat.reduceEqDiff, ↓reduceIte] at e5
  obtain ⟨z5, r5, p5, x5, ⟨s5, t5⟩, f5, -, m5⟩ := zstep_facts p4'
    (hb.get (a := 0x1038) (i := Transpiler.mk (.A_REG, 12) (.B_IMM, 1) (.ZOP_ADD ()) (Transpiler.storeReg 12) 4 (-16))
      (by simp [bios, Transpiler.storeReg])) e5
  refine ⟨z5, .step r1 (.step r2 (.step r3 (.step r4 (.step r5 (.done _))))), ⟨by rw [p5]; rfl,
    ⟨vectorUpdate (vectorUpdate xs 13 (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * i) + 8)) 12
      (BitVec.ofNat 64 i + 1), by rw [x5]; simp [Std.ExtDHashMap.get?_insert], ?_, ?_, ?_⟩, ⟨s5, t5⟩,
    by simp [f5, Std.ExtDHashMap.get?_insert, q4], by rw [m5, m4, m3, m2, m1, hzm]⟩⟩
  · rw [getElem!_update _ _ _ _ (by omega), if_neg (by omega), getElem!_update _ _ _ _ (by omega), if_neg (by omega)]
    exact h11
  · rw [getElem!_update _ _ _ _ (by omega), if_pos rfl, show (1 : BitVec 64) = BitVec.ofNat 64 1 from rfl,
      ofNat_add64]
  · rw [getElem!_update _ _ _ _ (by omega), if_neg (by omega), getElem!_update _ _ _ _ (by omega), if_pos rfl,
      BitVec.add_assoc, show (8 : BitVec 64) = BitVec.ofNat 64 8 from rfl, ofNat_add64]
    congr 2

set_option maxHeartbeats 8000000 in
/-- The loop's last test: x12 = 32, out to the input reads (0x103C). -/
theorem loop_exit (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (m : Equiv.Mem) (z : Equiv.Zisk.ZState) (h : LoopAt z m 32) :
    ∃ z', ZRun zrom 1 z z' ∧ z'.regs.get? .PC = some 0x103C ∧ (∃ xs, z'.regs.get? .Xs = some xs) ∧
      (∃ st, z'.regs.get? .Step = some st) ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = m := by
  obtain ⟨hpc, ⟨xs, hx, h11, h12, -⟩, ⟨st, hst⟩, hze, hzm⟩ := h
  have e1 := Equiv.Zisk.execute_mkj z (.A_REG, 11) (.B_REG, 12) (.ZOP_EQ ()) Equiv.Zisk.eq_pure 0 (by omega)
    0x14 4 xs _ st hx hpc hst 32 32
    (Or.inr ⟨11, by omega, by omega, rfl, by rw [getElem!_int]; exact h11⟩)
    (Or.inr ⟨12, by omega, by omega, rfl, by rw [getElem!_int]; exact h12⟩)
  simp only [Equiv.Zisk.linkRegs, eq_val, decide_true, if_true] at e1
  obtain ⟨z1, r1, p1, x1, ⟨s1, t1⟩, f1, -, m1⟩ := zstep_facts hpc
    (hb.get (a := 0x1028) (i := Transpiler.mk (.A_REG, 11) (.B_REG, 12) (.ZOP_EQ ()) (Transpiler.storeReg 0) 0x14 4)
      (by simp [bios, Transpiler.storeReg])) e1
  exact ⟨z1, .step r1 (.done _), by rw [p1]; rfl, ⟨xs, by rw [x1, hx]⟩, ⟨s1, t1⟩,
    by simp [f1, Std.ExtDHashMap.get?_insert, hze], by rw [m1, hzm]⟩

/-- The whole loop, from word i. -/
theorem out_loop (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (m : Equiv.Mem) (hm : OutBytes m) :
    ∀ k i, i + k = 32 → ∀ z, LoopAt z m i →
    ∃ n z', ZRun zrom n z z' ∧ z'.regs.get? .PC = some 0x103C ∧ (∃ xs, z'.regs.get? .Xs = some xs) ∧
      (∃ st, z'.regs.get? .Step = some st) ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = m := by
  intro k
  induction k with
  | zero =>
    intro i hi z h
    obtain ⟨z', r, rest⟩ := loop_exit zrom entry hb m z (by simpa [← hi] using h)
    exact ⟨1, z', r, rest⟩
  | succ k ih =>
    intro i hi z h
    obtain ⟨z1, r1, h1⟩ := loop_iter zrom entry hb m hm i (by omega) z h
    obtain ⟨n, z', r, rest⟩ := ih (i + 1) (by omega) z1 h1
    exact ⟨5 + n, z', ZRun.append r1 r, rest⟩

theorem ofNat_mul64 (a b : Nat) : BitVec.ofNat 64 a * BitVec.ofNat 64 b = BitVec.ofNat 64 (a * b) := by
  apply BitVec.eq_of_toNat_eq; simp [BitVec.toNat_mul, Nat.mul_mod]

/-- The address of the BIOS's k-th input read: past the input's start by
    k + 1 steps of 128 MB. -/
def inAddr (k : Nat) : BitVec 64 := Transpiler.INPUT_ADDR + BitVec.ofNat 64 (k + 1) * Transpiler.INPUT_ANCHOR_STEP

theorem in_region : ∀ k, k < 15 →
    readable (inAddr k) 8 = true ∧ hits_registers (inAddr k) 8 = false ∧ inAddr k ≠ INPUT_ADDR := by
  decide

/-- The 15 words the BIOS reads in the input are in memory. -/
def InBytes (m : Equiv.Mem) : Prop :=
  ∀ k, k < 15 → ∃ v, Equiv.memRead m 8 (inAddr k).toNat = some v

set_option maxHeartbeats 8000000 in
/-- The input reads, from the k-th on: to the jump to ROM_EXIT (0x1078). -/
theorem in_reads (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (m : Equiv.Mem) (hm : InBytes m) :
    ∀ j k, k + j = 15 → ∀ z, z.regs.get? .PC = some (0x103C + BitVec.ofNat 64 (4 * k)) →
      (∃ xs, z.regs.get? .Xs = some xs) → (∃ st, z.regs.get? .Step = some st) →
      z.regs.get? .Zerror = some false → z.mem = m →
    ∃ z', ZRun zrom j z z' ∧ z'.regs.get? .PC = some 0x1078 ∧
      (∃ st, z'.regs.get? .Step = some st) ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = m := by
  intro j
  induction j with
  | zero =>
    intro k hk z hpc _ hst hze hzm
    have : k = 15 := by omega
    subst this
    exact ⟨z, .done _, by rw [hpc]; rfl, hst, hze, hzm⟩
  | succ j ih =>
    intro k hk z hpc ⟨xs, hx⟩ ⟨st, hst⟩ hze hzm
    have add0 : ∀ a : BitVec 64, a + 0 = a := fun a => BitVec.add_zero a
    obtain ⟨v, hv⟩ := hm k (by omega)
    have hr := in_region k (by omega)
    have e := Equiv.Zisk.execute_read_none z (.A_IMM, inAddr k) 0 4 4 xs _ st hx hpc hst (inAddr k) (Or.inl rfl) v
      (by rw [add0]; exact hr.1) (by rw [add0]; exact hr.2.1) (by rw [add0]; exact hr.2.2)
      (by rw [hzm, add0]; exact hv)
    have hmem : (0x103C + BitVec.ofNat 64 (4 * k), Transpiler.mk (.A_IMM, inAddr k) (.B_IND, 0) (.ZOP_COPYB ())
        (.ST_NONE, 0, false) 4 4) ∈ bios entry := by
      simp only [bios, List.mem_append, List.mem_map, List.mem_range]
      refine Or.inl (Or.inr ⟨k, by omega, ?_⟩)
      rw [show (4 : BitVec 64) * BitVec.ofNat 64 k = BitVec.ofNat 64 (4 * k) from ofNat_mul64 4 k]; rfl
    obtain ⟨z1, r1, p1, x1, ⟨s1, t1⟩, f1, -, m1⟩ := zstep_facts hpc (hb.get hmem) e
    obtain ⟨z', r, rest⟩ := ih (k + 1) (by omega) z1
      (by rw [p1, BitVec.add_assoc, show (4 : BitVec 64) = BitVec.ofNat 64 4 from rfl, ofNat_add64]; congr 3)
      ⟨xs, by rw [x1]; simpa [Std.ExtDHashMap.get?_insert] using hx⟩ ⟨s1, t1⟩
      (by simp [f1, Std.ExtDHashMap.get?_insert, hze]) (by rw [m1, hzm])
    exact ⟨z', .step r1 r, rest⟩

/-- The run ends from z, in z': some steps, then one that ends it. -/
def ZEnds (zrom : BitVec 64 → zisk_inst) (z z' : Equiv.Zisk.ZState) : Prop :=
  ∃ n z1, ZRun zrom n z z1 ∧ (zstep zrom).run z1 = .ok true z'

theorem ZEnds.of_run {zrom : BitVec 64 → zisk_inst} {n : Nat} {z z1 z' : Equiv.Zisk.ZState}
    (h1 : ZRun zrom n z z1) (h2 : ZEnds zrom z1 z') : ZEnds zrom z z' := by
  obtain ⟨k, z2, r, e⟩ := h2
  exact ⟨n + k, z2, ZRun.append h1 r, e⟩

set_option maxHeartbeats 8000000 in
/-- The output block, from its start (0x101C): the loop, the input reads,
    the jump to ROM_EXIT and the end there. No error, the memory as it was. -/
theorem output_end (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (m : Equiv.Mem) (ho : OutBytes m) (hi : InBytes m) (z : Equiv.Zisk.ZState) (xs : Vector (BitVec 64) 40)
    (hpc : z.regs.get? .PC = some Transpiler.OUTPUT_BLOCK) (hx : z.regs.get? .Xs = some xs)
    (hst : ∃ st, z.regs.get? .Step = some st) (hze : z.regs.get? .Zerror = some false) (hzm : z.mem = m) :
    ∃ z', ZEnds zrom z z' ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = m := by
  obtain ⟨st, hst⟩ := hst
  -- 0x101C: x11 = 32
  have e1 := Equiv.Zisk.execute_mkj z (.A_IMM, 0) (.B_IMM, 32) (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure 11 (by omega)
    0 4 xs _ st hx hpc hst 0 32 (Or.inl rfl) (Or.inl rfl) (w := 4)
  simp only [Equiv.Zisk.linkRegs, Equiv.Zisk.copyb_val, Bool.false_eq_true, if_false, Nat.reduceEqDiff,
    ↓reduceIte] at e1
  obtain ⟨z1, r1, p1, x1, ⟨s1, t1⟩, f1, -, m1⟩ := zstep_facts hpc
    (hb.get (a := 0x101C) (i := Transpiler.mk (.A_IMM, 0) (.B_IMM, 32) (.ZOP_COPYB ()) (Transpiler.storeReg 11) 0 4
      (w := 4)) (by simp [bios, Transpiler.storeReg])) e1
  have p1' : z1.regs.get? .PC = some 0x1020 := by rw [p1]; rfl
  have x1' : z1.regs.get? .Xs = some (vectorUpdate xs 11 32) := by rw [x1]; simp [Std.ExtDHashMap.get?_insert]
  -- 0x1020: x12 = 0
  have e2 := Equiv.Zisk.execute_mkj z1 (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure 12 (by omega)
    0 4 _ _ s1 x1' p1' t1 0 0 (Or.inl rfl) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, Equiv.Zisk.copyb_val, Bool.false_eq_true, if_false, Nat.reduceEqDiff,
    ↓reduceIte] at e2
  obtain ⟨z2, r2, p2, x2, ⟨s2, t2⟩, f2, -, m2⟩ := zstep_facts p1'
    (hb.get (a := 0x1020) (i := Transpiler.mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) (Transpiler.storeReg 12) 0 4)
      (by simp [bios, Transpiler.storeReg, Transpiler.copyImm])) e2
  have p2' : z2.regs.get? .PC = some 0x1024 := by rw [p2]; rfl
  have x2' : z2.regs.get? .Xs = some (vectorUpdate (vectorUpdate xs 11 32) 12 0) := by
    rw [x2]; simp [Std.ExtDHashMap.get?_insert]
  -- 0x1024: x13 = OUTPUT_ADDR
  have e3 := Equiv.Zisk.execute_mkj z2 (.A_IMM, 0) (.B_IMM, Transpiler.OUTPUT_ADDR) (.ZOP_COPYB ())
    Equiv.Zisk.copyb_pure 13 (by omega) 0 4 _ _ s2 x2' p2' t2 0 _ (Or.inl rfl) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, Equiv.Zisk.copyb_val, Bool.false_eq_true, if_false, Nat.reduceEqDiff,
    ↓reduceIte] at e3
  obtain ⟨z3, r3, p3, x3, ⟨s3, t3⟩, f3, -, m3⟩ := zstep_facts p2'
    (hb.get (a := 0x1024) (i := Transpiler.mk (.A_IMM, 0) (.B_IMM, Transpiler.OUTPUT_ADDR) (.ZOP_COPYB ())
      (Transpiler.storeReg 13) 0 4) (by simp [bios, Transpiler.storeReg, Transpiler.copyImm])) e3
  have hl : LoopAt z3 m 0 := by
    refine ⟨by rw [p3]; rfl, ⟨vectorUpdate (vectorUpdate (vectorUpdate xs 11 32) 12 0) 13 Transpiler.OUTPUT_ADDR,
      by rw [x3]; simp [Std.ExtDHashMap.get?_insert], ?_, ?_, ?_⟩, ⟨s3, t3⟩,
      by simp [f3, f2, f1, Std.ExtDHashMap.get?_insert, hze], by rw [m3, m2, m1, hzm]⟩
    · rw [getElem!_update _ _ _ _ (by omega), if_neg (by omega), getElem!_update _ _ _ _ (by omega),
        if_neg (by omega), getElem!_update _ _ _ _ (by omega), if_pos rfl]
    · rw [getElem!_update _ _ _ _ (by omega), if_neg (by omega), getElem!_update _ _ _ _ (by omega), if_pos rfl]; rfl
    · rw [getElem!_update _ _ _ _ (by omega), if_pos rfl]; rfl
  obtain ⟨n4, z4, r4, p4, x4, ⟨s4, t4⟩, q4, m4⟩ := out_loop zrom entry hb m ho 32 0 rfl z3 hl
  obtain ⟨z5, r5, p5, ⟨s5, t5⟩, q5, m5⟩ := in_reads zrom entry hb m hi 15 0 rfl z4 (by rw [p4]; rfl)
    x4 ⟨s4, t4⟩ q4 m4
  -- 0x1078: to ROM_EXIT
  obtain ⟨z6, r6, p6, -, ⟨s6, t6⟩, f6, -, m6⟩ := zstep_facts p5
    (hb.get (a := 0x1078) (i := Transpiler.mk (.A_IMM, 0) (.B_IMM, Transpiler.ROM_EXIT) (.ZOP_COPYB ())
      (.ST_NONE, 0, false) 0 0 (setPc := true)) (by simp [bios])) (Equiv.Zisk.execute_goto z5 _ s5 t5)
  -- 0x1004: the end
  have e7 : (zstep zrom).run z6 = .ok true
      { z6 with regs := ((z6.regs.insert .Zc 0).insert .Zflag false).insert .Step (BitVec.addInt s6 1) } := by
    rw [zstep_run zrom z6 _ p6, hb.get (a := Transpiler.ROM_EXIT + 0)
      (i := { Transpiler.copyImm 0 (.ST_NONE, 0, false) 0 0 with is_end := true }) (by simp [bios, Transpiler.ROM_EXIT])]
    exact Equiv.Zisk.execute_end z6 s6 t6
  refine ⟨_, ⟨3 + n4 + 15 + 1, z6,
    ZRun.append (ZRun.append (ZRun.append (.step r1 (.step r2 (.step r3 (.done _)))) r4) r5) (.step r6 (.done _)),
    e7⟩, ?_, by rw [← m5]; exact m6⟩
  simp [Std.ExtDHashMap.get?_insert, f6, q5]

set_option maxHeartbeats 8000000 in
/-- The exit system call, up to the handler's test of the code: the
    transpiled `ecall` (ra := pc + 4, to 0x107C), and a7 = 93 found there. -/
theorem ecall_exit_test (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (z : Equiv.Zisk.ZState) (pc : BitVec 64) (hpc : z.regs.get? .PC = some pc) (hi : zrom pc = Transpiler.ecall)
    (xs : Vector (BitVec 64) 40) (hx : z.regs.get? .Xs = some xs) (h17 : xs[17]! = 93)
    (hst : ∃ st, z.regs.get? .Step = some st) (hze : z.regs.get? .Zerror = some false) :
    ∃ z', ZRun zrom 2 z z' ∧ z'.regs.get? .PC = some 0x1084 ∧
      z'.regs.get? .Xs = some (vectorUpdate xs 1 (pc + 4)) ∧
      (∃ st, z'.regs.get? .Step = some st) ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = z.mem := by
  obtain ⟨st, hst⟩ := hst
  -- the ecall: to the handler
  obtain ⟨z1, r1, p1, x1, ⟨s1, t1⟩, f1, -, m1⟩ := zstep_facts (zrom := zrom) hpc (by rw [hi]; rfl)
    (Equiv.Zisk.execute_call z Transpiler.ECALL_HANDLER_ADDR xs pc st hx hpc hst)
  have p1' : z1.regs.get? .PC = some 0x107C := by rw [p1]; rfl
  have x1' : z1.regs.get? .Xs = some (vectorUpdate xs 1 (pc + 4)) := by
    rw [x1]; simp [Std.ExtDHashMap.get?_insert]
  -- 0x107C: a7 = 93
  have e2 := Equiv.Zisk.execute_mkj z1 (.A_REG, 17) (.B_IMM, 93) (.ZOP_EQ ()) Equiv.Zisk.eq_pure 0 (by omega)
    8 4 _ _ s1 x1' p1' t1 93 93
    (Or.inr ⟨17, by omega, by omega, rfl,
      by rw [getElem!_int, getElem!_update _ _ _ _ (by omega), if_neg (by omega)]; exact h17⟩) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, eq_val, decide_true, if_true] at e2
  obtain ⟨z2, r2, p2, x2, ⟨s2, t2⟩, f2, -, m2⟩ := zstep_facts p1'
    (hb.get (a := 0x107C) (i := Transpiler.mk (.A_REG, 17) (.B_IMM, 93) (.ZOP_EQ ()) (Transpiler.storeReg 0) 8 4)
      (by simp [bios, Transpiler.storeReg])) e2
  exact ⟨z2, .step r1 (.step r2 (.done _)), by rw [p2]; rfl, by rw [x2]; exact x1', ⟨s2, t2⟩,
    by simp [f2, f1, Std.ExtDHashMap.get?_insert, hze], by rw [m2, m1]⟩

set_option maxHeartbeats 8000000 in
/-- The exit system call with code 0: through the handler to the output
    block, and on to the end, in success. -/
theorem ecall_exit_ok (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (m : Equiv.Mem) (ho : OutBytes m) (hin : InBytes m)
    (z : Equiv.Zisk.ZState) (pc : BitVec 64) (hpc : z.regs.get? .PC = some pc) (hi : zrom pc = Transpiler.ecall)
    (xs : Vector (BitVec 64) 40) (hx : z.regs.get? .Xs = some xs) (h17 : xs[17]! = 93) (h10 : xs[10]! = 0)
    (hst : ∃ st, z.regs.get? .Step = some st) (hze : z.regs.get? .Zerror = some false) (hzm : z.mem = m) :
    ∃ z', ZEnds zrom z z' ∧ z'.regs.get? .Zerror = some false ∧ z'.mem = m := by
  obtain ⟨z2, r2, p2, x2, ⟨s2, t2⟩, q2, m2⟩ := ecall_exit_test zrom entry hb z pc hpc hi xs hx h17 hst hze
  -- 0x1084: a0 = 0, back to the output block
  have e3 := Equiv.Zisk.execute_mkj z2 (.A_REG, 10) (.B_IMM, 0) (.ZOP_EQ ()) Equiv.Zisk.eq_pure 0 (by omega)
    (-0x68) 4 _ _ s2 x2 p2 t2 0 0
    (Or.inr ⟨10, by omega, by omega, rfl,
      by rw [getElem!_int, getElem!_update _ _ _ _ (by omega), if_neg (by omega)]; exact h10⟩) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, eq_val, decide_true, if_true] at e3
  obtain ⟨z3, r3, p3, x3, ⟨s3, t3⟩, f3, -, m3⟩ := zstep_facts p2
    (hb.get (a := 0x1084) (i := Transpiler.mk (.A_REG, 10) (.B_IMM, 0) (.ZOP_EQ ()) (Transpiler.storeReg 0) (-0x68) 4)
      (by simp [bios, Transpiler.storeReg])) e3
  obtain ⟨z', he, rest⟩ := output_end zrom entry hb m ho hin z3 _ (by rw [p3]; rfl) (by rw [x3]; exact x2)
    ⟨s3, t3⟩ (by simp [f3, Std.ExtDHashMap.get?_insert, q2]) (by rw [m3, m2, hzm])
  exact ⟨z', ZEnds.of_run (ZRun.append r2 (.step r3 (.done _))) he, rest⟩

set_option maxHeartbeats 8000000 in
/-- The exit system call with code c ≠ 0: the handler halts, the run ends
    in error with c as its code. -/
theorem ecall_exit_fail (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (z : Equiv.Zisk.ZState) (pc : BitVec 64) (hpc : z.regs.get? .PC = some pc) (hi : zrom pc = Transpiler.ecall)
    (xs : Vector (BitVec 64) 40) (hx : z.regs.get? .Xs = some xs) (h17 : xs[17]! = 93) (h10 : xs[10]! ≠ 0)
    (hst : ∃ st, z.regs.get? .Step = some st) (hze : z.regs.get? .Zerror = some false) :
    ∃ z', ZEnds zrom z z' ∧ z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some xs[10]! ∧
      z'.mem = z.mem := by
  obtain ⟨z2, r2, p2, x2, ⟨s2, t2⟩, q2, m2⟩ := ecall_exit_test zrom entry hb z pc hpc hi xs hx h17 hst hze
  have ha : (vectorUpdate xs 1 (pc + 4))[((10 : Nat) : Int)]! = xs[10]! := by
    rw [getElem!_int, getElem!_update _ _ _ _ (by omega), if_neg (by omega)]
  have hne : decide (xs[10]! = 0) = false := decide_eq_false h10
  -- 0x1084: a0 ≠ 0, on to the halt
  have e3 := Equiv.Zisk.execute_mkj z2 (.A_REG, 10) (.B_IMM, 0) (.ZOP_EQ ()) Equiv.Zisk.eq_pure 0 (by omega)
    (-0x68) 4 _ _ s2 x2 p2 t2 xs[10]! 0 (Or.inr ⟨10, by omega, by omega, rfl, ha⟩) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, eq_val, hne, if_true, Bool.false_eq_true, if_false] at e3
  obtain ⟨z3, r3, p3, x3, ⟨s3, t3⟩, f3, -, m3⟩ := zstep_facts p2
    (hb.get (a := 0x1084) (i := Transpiler.mk (.A_REG, 10) (.B_IMM, 0) (.ZOP_EQ ()) (Transpiler.storeReg 0) (-0x68) 4)
      (by simp [bios, Transpiler.storeReg])) e3
  have p3' : z3.regs.get? .PC = some 0x1088 := by rw [p3]; rfl
  have x3' : z3.regs.get? .Xs = some (vectorUpdate xs 1 (pc + 4)) := by rw [x3]; exact x2
  -- 0x1088: the halt
  have e4 := Equiv.Zisk.execute_halt z3 10 (by omega) _ s3 x3' t3
  rw [← hb.get (a := 0x1088)
      (i := { Transpiler.mk (.A_REG, BitVec.ofNat 64 10) (.B_IMM, 0) (.ZOP_HALT ()) (.ST_NONE, 0, false) 0 0
        with is_end := true }) (by simp [bios]), ← zstep_run zrom z3 _ p3'] at e4
  refine ⟨_, ⟨2 + 1, z3, ZRun.append r2 (.step r3 (.done _)), e4⟩, ?_, ?_, by rw [m3, m2]⟩
  · simp [Std.ExtDHashMap.get?_insert]
  · simp only [Std.ExtDHashMap.get?_insert]; simp; exact ha

/-! ## From the relation -/

/-- The output words are off the input's first 8 bytes: two memories equal
    without them have the same. -/
theorem OutBytes.of_normIn {m m' : Equiv.Mem} (h : Equiv.normIn m' = Equiv.normIn m) (ho : OutBytes m) :
    OutBytes m' := fun j hj => by
  obtain ⟨v, hv⟩ := ho j hj
  have : (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)).toNat = 0xA041_0000 + 8 * j := by
    simp [Transpiler.OUTPUT_ADDR, BitVec.toNat_add]; omega
  have hoff : (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)).toNat + 8 ≤ Equiv.IN_LO ∨
      Equiv.IN_LO + 8 ≤ (Transpiler.OUTPUT_ADDR + BitVec.ofNat 64 (8 * j)).toNat := by
    right; rw [this]; simp only [Equiv.IN_LO]; omega
  exact ⟨v, by rw [Equiv.memRead_normIn h hoff]; exact hv⟩

/-- So are the input words the BIOS reads. -/
theorem InBytes.of_normIn {m m' : Equiv.Mem} (h : Equiv.normIn m' = Equiv.normIn m) (hi : InBytes m) :
    InBytes m' := fun k hk => by
  obtain ⟨v, hv⟩ := hi k hk
  have hoff : (inAddr k).toNat + 8 ≤ Equiv.IN_LO ∨ Equiv.IN_LO + 8 ≤ (inAddr k).toNat := by
    right
    have hk1 : (BitVec.ofNat 64 (k + 1)).toNat = k + 1 := by rw [BitVec.toNat_ofNat]; omega
    simp only [inAddr]
    generalize BitVec.ofNat 64 (k + 1) = K at hk1
    have b1 : 1#64 ≤ K := by rw [BitVec.le_def]; simp; omega
    have b2 : K ≤ 15#64 := by rw [BitVec.le_def]; simp; omega
    have : 0x10000008#64 ≤ Transpiler.INPUT_ADDR + K * Transpiler.INPUT_ANCHOR_STEP := by
      simp only [Transpiler.INPUT_ADDR, Transpiler.INPUT_ANCHOR_STEP]; bv_decide
    rw [BitVec.le_def] at this
    have e : (0x10000008#64).toNat = Equiv.IN_LO + 8 := rfl
    omega
  exact ⟨v, by rw [Equiv.memRead_normIn h hoff]; exact hv⟩


/-- The return to the initial ra, related: the run on sail-zisk ends in
    success, its memory sail-riscv's. -/
theorem rel_return_end (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (s : Equiv.Riscv.RVState) (z : Equiv.Zisk.ZState) (rs : List LeanZiskRV.PMA_Region) (hrel : Rel s z rs)
    (hpc : s.regs.get? LeanZiskRV.Register.PC = some Transpiler.OUTPUT_BLOCK)
    (ho : OutBytes s.mem) (hi : InBytes s.mem) :
    ∃ z', ZEnds zrom z z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn s.mem := by
  obtain ⟨-, ⟨xs, hx, -⟩, ⟨pc, hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  rw [hpc] at hpc'; cases hpc'
  obtain ⟨z', h1, h2, h3⟩ := output_end zrom entry hb z.mem (ho.of_normIn hzm) (hi.of_normIn hzm) z xs hzpc hx
    ⟨st, hst⟩ hze rfl
  exact ⟨z', h1, h2, by rw [h3]; exact hzm⟩

/-- The exit system call with code 0, related: success, sail-riscv's memory. -/
theorem rel_exit_ok (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (s : Equiv.Riscv.RVState) (z : Equiv.Zisk.ZState) (rs : List LeanZiskRV.PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? LeanZiskRV.Register.PC = some pc) (hi : zrom pc = Transpiler.ecall)
    (h17 : Equiv.Riscv.getX s 17 = some 93) (h10 : Equiv.Riscv.getX s 10 = some 0)
    (ho : OutBytes s.mem) (hin : InBytes s.mem) :
    ∃ z', ZEnds zrom z z' ∧ z'.regs.get? .Zerror = some false ∧ Equiv.normIn z'.mem = Equiv.normIn s.mem := by
  obtain ⟨-, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  rw [hpc] at hpc'; cases hpc'
  have a17 := agree 17 (by omega) (by omega)
  have a10 := agree 10 (by omega) (by omega)
  rw [h17] at a17; rw [h10] at a10
  obtain ⟨z', h1, h2, h3⟩ := ecall_exit_ok zrom entry hb z.mem (ho.of_normIn hzm) (hin.of_normIn hzm) z pc hzpc
    hi xs hx (Option.some.inj a17).symm (Option.some.inj a10).symm ⟨st, hst⟩ hze rfl
  exact ⟨z', h1, h2, by rw [h3]; exact hzm⟩

/-- The exit system call with code c ≠ 0, related: the run on sail-zisk
    ends in error, with code c. -/
theorem rel_exit_fail (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (s : Equiv.Riscv.RVState) (z : Equiv.Zisk.ZState) (rs : List LeanZiskRV.PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? LeanZiskRV.Register.PC = some pc) (hi : zrom pc = Transpiler.ecall)
    (h17 : Equiv.Riscv.getX s 17 = some 93) (c : BitVec 64) (h10 : Equiv.Riscv.getX s 10 = some c) (hc : c ≠ 0) :
    ∃ z', ZEnds zrom z z' ∧ z'.regs.get? .Zerror = some true ∧ z'.regs.get? .Zerror_code = some c := by
  obtain ⟨-, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, -⟩ := hrel
  rw [hpc] at hpc'; cases hpc'
  have a17 := agree 17 (by omega) (by omega)
  have a10 := agree 10 (by omega) (by omega)
  rw [h17] at a17; rw [h10] at a10
  have e10 : xs[10]! = c := (Option.some.inj a10).symm
  obtain ⟨z', he, h1, h2, -⟩ := ecall_exit_fail zrom entry hb z pc hzpc hi xs hx (Option.some.inj a17).symm
    (by rw [e10]; exact hc) ⟨st, hst⟩ hze
  exact ⟨z', he, h1, by rw [h2, e10]⟩

end Equiv.Sim
