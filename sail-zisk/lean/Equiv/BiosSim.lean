/-
  The BIOS's start on sail-zisk: from ZisK's initial state (pc at
  ROM_ENTRY), its first six instructions write marchid and mtvec to their
  CSR cells, set a0 and a1 to the input and output addresses, and call the
  program's entry with ra = OUTPUT_BLOCK.
-/
import Equiv.ZiskBios
import Equiv.Block
import Equiv.Sim
import Equiv.ZStepKeep

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Transpiler (bios)

/-- The ZisK program holds the BIOS, for the program's entry. -/
def BiosAt (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) : Prop :=
  ∀ p ∈ bios entry, zrom p.1 = p.2

theorem arch_writable : writable Transpiler.ARCH_ID_CSR_ADDR 8 = true ∧
    hits_registers Transpiler.ARCH_ID_CSR_ADDR 8 = false := by decide

theorem mtvec_writable : writable Transpiler.MTVEC_ADDR 8 = true ∧
    hits_registers Transpiler.MTVEC_ADDR 8 = false := by decide

theorem BiosAt.get {zrom : BitVec 64 → zisk_inst} {entry : BitVec 64} (h : BiosAt zrom entry)
    {a : BitVec 64} {i : zisk_inst} (hm : (a, i) ∈ bios entry) : zrom a = i := h _ hm

/-- What the BIOS's start leaves in ZisK's registers: a0, a1 and ra set. -/
def startXs (xs : Vector (BitVec 64) 40) : Vector (BitVec 64) 40 :=
  vectorUpdate (vectorUpdate (vectorUpdate xs 10 Transpiler.INPUT_ADDR) 11 Transpiler.OUTPUT_ADDR) 1
    Transpiler.OUTPUT_BLOCK

/-- And in memory: marchid and mtvec in their CSR cells. -/
def startMem (m : Equiv.Mem) : Equiv.Mem :=
  Equiv.memWrite (n := 8) (Equiv.memWrite (n := 8) m Transpiler.ARCH_ID_CSR_ADDR.toNat Transpiler.ARCH_ID_ZISK)
    Transpiler.MTVEC_ADDR.toNat 0x107C

/-- A ZisK step whose result has the usual shape: what the chain needs of it. -/
theorem zstep_facts {zrom : BitVec 64 → zisk_inst} {z : Equiv.Zisk.ZState} {pc : BitVec 64}
    (hpc : z.regs.get? .PC = some pc) {inst : zisk_inst} (hi : zrom pc = inst)
    {R : Std.ExtDHashMap _root_.Zisk.Register _root_.Zisk.RegisterType} {c st' p : BitVec 64} {f : Bool}
    {m : Equiv.Mem}
    (he : (zisk_execute inst).run z = .ok false
      { z with regs := (((R.insert .Zc c).insert .Zflag f).insert .Step st').insert .PC p, mem := m }) :
    ∃ z', (zstep zrom).run z = .ok false z' ∧ z'.regs.get? .PC = some p ∧ z'.regs.get? .Xs = R.get? .Xs ∧
      (∃ st, z'.regs.get? .Step = some st) ∧ z'.regs.get? .Zerror = R.get? .Zerror ∧
      z'.regs.get? .Zc = some c ∧ z'.mem = m := by
  refine ⟨_, by rw [zstep_run zrom z pc hpc, hi]; exact he, ?_, ?_, ⟨st', ?_⟩, ?_, ?_, rfl⟩ <;>
    simp [Std.ExtDHashMap.get?_insert]

theorem fkeep_at {zrom : BitVec 64 → zisk_inst} {z z' : Equiv.Zisk.ZState} {pc : BitVec 64}
    (hpc : z.regs.get? .PC = some pc) {inst : zisk_inst} (hi : zrom pc = inst)
    (hw : Equiv.Zisk.writesF inst.op = false) {b : Bool} (hr : (zstep zrom).run z = .ok b z') : FKeep z z' :=
  zstep_fkeep hpc (hi ▸ hw) hr

set_option maxHeartbeats 8000000 in
/-- The BIOS's start: six ZisK steps from ROM_ENTRY reach the program's
    entry, with a0, a1, ra and the two CSR cells set. -/
theorem bios_start (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (z : Equiv.Zisk.ZState) (xs : Vector (BitVec 64) 40) (st : BitVec 64)
    (hx : z.regs.get? .Xs = some xs) (hpc : z.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hst : z.regs.get? .Step = some st) (hze : z.regs.get? .Zerror = some false) :
    ∃ z', ZRun zrom 6 z z' ∧ z'.regs.get? .PC = some entry ∧ z'.regs.get? .Xs = some (startXs xs) ∧
      (∃ st', z'.regs.get? .Step = some st') ∧ z'.regs.get? .Zerror = some false ∧
      z'.mem = startMem z.mem ∧ FKeep z z' := by
  -- 0x1000: jump over the end instruction
  have e0 := Equiv.Zisk.execute_mkj z (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure 0 (by omega)
    8 8 xs _ st hx hpc hst 0 0 (Or.inl rfl) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, if_true, Equiv.Zisk.copyb_val, ite_self] at e0
  obtain ⟨z1, r1, p1, x1, ⟨s1, t1⟩, f1, -, m1⟩ := zstep_facts hpc
    (hb.get (a := 0x1000) (i := Transpiler.copyImm 0 (.ST_NONE, 0, false) 8 8) (by simp [bios])) e0
  have q1 : z1.regs.get? .Zerror = some false := by
    rw [f1]; simp [Std.ExtDHashMap.get?_insert, hze]
  rw [hx] at x1
  -- 0x1008: marchid to its cell
  obtain ⟨z2, r2, p2, x2, ⟨s2, t2⟩, f2, -, m2⟩ := zstep_facts p1
    (hb.get (a := 0x1000 + 8) (i := Transpiler.copyImm Transpiler.ARCH_ID_ZISK
      (.ST_MEM, Transpiler.ARCH_ID_CSR_ADDR, false) 4 4) (by simp [bios]))
    (Equiv.Zisk.execute_copy_mem z1 _ _ 4 _ s1 p1 t1 arch_writable.1 arch_writable.2)
  have q2 : z2.regs.get? .Zerror = some false := by
    rw [f2]; simp [Std.ExtDHashMap.get?_insert, q1]
  rw [x1] at x2
  -- 0x100C: mtvec to its cell
  obtain ⟨z3, r3, p3, x3, ⟨s3, t3⟩, f3, -, m3⟩ := zstep_facts p2
    (hb.get (a := 0x1000 + 8 + 4) (i := Transpiler.copyImm 0x107C (.ST_MEM, Transpiler.MTVEC_ADDR, false) 4 4)
      (by simp [bios]))
    (Equiv.Zisk.execute_copy_mem z2 _ _ 4 _ s2 p2 t2 mtvec_writable.1 mtvec_writable.2)
  have q3 : z3.regs.get? .Zerror = some false := by
    rw [f3]; simp [Std.ExtDHashMap.get?_insert, q2]
  rw [x2] at x3
  -- 0x1010: a0 = the input's address
  have xx3 : z3.regs.get? .Xs = some xs := by simpa [Std.ExtDHashMap.get?_insert] using x3
  have e3 := Equiv.Zisk.execute_mkj z3 (.A_IMM, 0) (.B_IMM, Transpiler.INPUT_ADDR) (.ZOP_COPYB ())
    Equiv.Zisk.copyb_pure 10 (by omega) 0 4 xs _ s3 xx3 p3 t3 0 _ (Or.inl rfl) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, Equiv.Zisk.copyb_val, Bool.false_eq_true, if_false] at e3
  obtain ⟨z4, r4, p4, x4, ⟨s4, t4⟩, f4, -, m4⟩ := zstep_facts p3
    (hb.get (a := 0x1000 + 8 + 4 + 4) (i := Transpiler.mk (.A_IMM, 0) (.B_IMM, Transpiler.INPUT_ADDR)
      (.ZOP_COPYB ()) (Transpiler.storeReg 10) 0 4) (by simp [bios, Transpiler.storeReg, Transpiler.copyImm]))
    (by simpa using e3)
  have q4 : z4.regs.get? .Zerror = some false := by
    rw [f4]; simp [Std.ExtDHashMap.get?_insert, q3]
  -- 0x1014: a1 = the output's address
  have xx4 : z4.regs.get? .Xs = some (vectorUpdate xs 10 Transpiler.INPUT_ADDR) := by
    simpa [Std.ExtDHashMap.get?_insert] using x4
  have e4 := Equiv.Zisk.execute_mkj z4 (.A_IMM, 0) (.B_IMM, Transpiler.OUTPUT_ADDR) (.ZOP_COPYB ())
    Equiv.Zisk.copyb_pure 11 (by omega) 0 4 _ _ s4 xx4 p4 t4 0 _ (Or.inl rfl) (Or.inl rfl)
  simp only [Equiv.Zisk.linkRegs, Equiv.Zisk.copyb_val, Bool.false_eq_true, if_false] at e4
  obtain ⟨z5, r5, p5, x5, ⟨s5, t5⟩, f5, -, m5⟩ := zstep_facts p4
    (hb.get (a := 0x1000 + 8 + 4 + 4 + 4) (i := Transpiler.mk (.A_IMM, 0) (.B_IMM, Transpiler.OUTPUT_ADDR)
      (.ZOP_COPYB ()) (Transpiler.storeReg 11) 0 4) (by simp [bios, Transpiler.storeReg, Transpiler.copyImm]))
    (by simpa using e4)
  have q5 : z5.regs.get? .Zerror = some false := by
    rw [f5]; simp [Std.ExtDHashMap.get?_insert, q4]
  -- 0x1018: call the entry, ra = 0x101C
  have xx5 : z5.regs.get? .Xs = some (vectorUpdate (vectorUpdate xs 10 Transpiler.INPUT_ADDR) 11
      Transpiler.OUTPUT_ADDR) := by simpa [Std.ExtDHashMap.get?_insert] using x5
  have e5 := Equiv.Zisk.execute_call z5 entry _ _ s5 xx5 p5 t5
  obtain ⟨z6, r6, p6, x6, ⟨s6, t6⟩, f6, -, m6⟩ := zstep_facts p5
    (hb.get (a := 0x1000 + 8 + 4 + 4 + 4 + 4) (i := Transpiler.mk (.A_IMM, 0) (.B_IMM, entry) (.ZOP_COPYB ())
      (.ST_REG, 1, true) 0 4 (setPc := true)) (by simp [bios])) (by simpa using e5)
  have q6 : z6.regs.get? .Zerror = some false := by
    rw [f6]; simp [Std.ExtDHashMap.get?_insert, q5]
  have k1 := fkeep_at hpc (hb.get (a := 0x1000) (i := Transpiler.copyImm 0 (.ST_NONE, 0, false) 8 8)
    (by simp [bios])) rfl r1
  have k2 := fkeep_at p1 (hb.get (a := 0x1000 + 8) (i := Transpiler.copyImm Transpiler.ARCH_ID_ZISK
    (.ST_MEM, Transpiler.ARCH_ID_CSR_ADDR, false) 4 4) (by simp [bios])) rfl r2
  have k3 := fkeep_at p2 (hb.get (a := 0x1000 + 8 + 4) (i := Transpiler.copyImm 0x107C
    (.ST_MEM, Transpiler.MTVEC_ADDR, false) 4 4) (by simp [bios])) rfl r3
  have k4 := fkeep_at p3 (hb.get (a := 0x1000 + 8 + 4 + 4) (i := Transpiler.mk (.A_IMM, 0)
    (.B_IMM, Transpiler.INPUT_ADDR) (.ZOP_COPYB ()) (Transpiler.storeReg 10) 0 4)
    (by simp [bios, Transpiler.storeReg, Transpiler.copyImm])) rfl r4
  have k5 := fkeep_at p4 (hb.get (a := 0x1000 + 8 + 4 + 4 + 4) (i := Transpiler.mk (.A_IMM, 0)
    (.B_IMM, Transpiler.OUTPUT_ADDR) (.ZOP_COPYB ()) (Transpiler.storeReg 11) 0 4)
    (by simp [bios, Transpiler.storeReg, Transpiler.copyImm])) rfl r5
  have k6 := fkeep_at p5 (hb.get (a := 0x1000 + 8 + 4 + 4 + 4 + 4) (i := Transpiler.mk (.A_IMM, 0)
    (.B_IMM, entry) (.ZOP_COPYB ()) (.ST_REG, 1, true) 0 4 (setPc := true)) (by simp [bios])) rfl r6
  refine ⟨z6, .step r1 (.step r2 (.step r3 (.step r4 (.step r5 (.step r6 (.done _)))))), by simpa using p6,
    ?_, ⟨s6, t6⟩, q6, ?_, ((((k1.trans k2).trans k3).trans k4).trans k5).trans k6⟩
  · rw [x6]; simp [Std.ExtDHashMap.get?_insert, startXs, Transpiler.ROM_ENTRY, Transpiler.OUTPUT_BLOCK]
  · rw [m6, m5, m4, m3, m2, m1]; rfl

/-- The contract's start (doc/riscv_contract.md § 3), of a RISC-V state, for
    a program with entry `entry` and initial memory `m0`: ready to step, at
    the entry, a0 and a1 the input and output addresses, ra the BIOS's
    return address, every other register 0, and the memory m0 with marchid
    and mtvec in their CSR cells. -/
structure Start (s : Equiv.Riscv.RVState) (rs : List LeanZiskRV.PMA_Region) (entry : BitVec 64)
    (m0 : Equiv.Mem) : Prop where
  ready : Equiv.Riscv.StepReady s rs
  pc : s.regs.get? LeanZiskRV.Register.PC = some entry
  a0 : Equiv.Riscv.getX s 10 = some Transpiler.INPUT_ADDR
  a1 : Equiv.Riscv.getX s 11 = some Transpiler.OUTPUT_ADDR
  ra : Equiv.Riscv.getX s 1 = some Transpiler.OUTPUT_BLOCK
  zero : ∀ k, 1 ≤ k → k < 32 → k ≠ 1 → k ≠ 10 → k ≠ 11 → Equiv.Riscv.getX s k = some 0
  mem : s.mem = startMem m0

theorem startXs_get (k : Nat) (hk : k < 32) :
    (startXs (Vector.replicate 40 0))[k]! =
      if k = 1 then Transpiler.OUTPUT_BLOCK else if k = 11 then Transpiler.OUTPUT_ADDR
      else if k = 10 then Transpiler.INPUT_ADDR else 0 := by
  simp only [startXs]
  rw [getElem!_update _ _ _ _ (by omega), getElem!_update _ _ _ _ (by omega), getElem!_update _ _ _ _ (by omega)]
  have : k < 40 := by omega
  by_cases h1 : k = 1 <;> by_cases h11 : k = 11 <;> by_cases h10 : k = 10 <;> simp_all

/-- The start, related: from ZisK's initial state (registers 0, at
    ROM_ENTRY, memory m0, the BIOS in its program), the BIOS's six steps
    reach a state related to any RISC-V state at the contract's start. -/
theorem start_rel (zrom : BitVec 64 → zisk_inst) (entry : BitVec 64) (hb : BiosAt zrom entry)
    (m0 : Equiv.Mem) (z : Equiv.Zisk.ZState) (st : BitVec 64)
    (hx : z.regs.get? .Xs = some (Vector.replicate 40 0)) (hpc : z.regs.get? .PC = some Transpiler.ROM_ENTRY)
    (hst : z.regs.get? .Step = some st) (hze : z.regs.get? .Zerror = some false) (hzm : z.mem = m0)
    (s : Equiv.Riscv.RVState) (rs : List LeanZiskRV.PMA_Region) (hs : Start s rs entry m0) :
    ∃ z', ZRun zrom 6 z z' ∧ Rel s z' rs ∧ FKeep z z' := by
  obtain ⟨z', hrun, hp, hx', ⟨st', hst'⟩, hze', hm, hk⟩ := bios_start zrom entry hb z _ st hx hpc hst hze
  refine ⟨z', hrun, ⟨hs.ready, ⟨_, hx', ?_⟩, ⟨entry, hs.pc, hp⟩, ⟨st', hst', hze'⟩, by rw [hm, hzm, hs.mem]⟩, hk⟩
  intro k h1 h2
  rw [startXs_get k h2]
  by_cases e1 : k = 1
  · subst e1; simpa using hs.ra
  by_cases e11 : k = 11
  · subst e11; simpa using hs.a1
  by_cases e10 : k = 10
  · subst e10; simpa using hs.a0
  simp only [e1, e11, e10, if_false]; exact hs.zero k h1 h2 e1 e10 e11

end Equiv.Sim
