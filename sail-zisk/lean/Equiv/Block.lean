/-
  Chaining steps: the simulation over a straight-line block of R-type
  instructions.

  A ZisK program is a function from pc to instruction (sail-zisk's
  `zisk_step` fetches `rom_fetch(PC)`; here the ROM is a parameter), and a
  ZisK step executes the instruction at PC. `Rel` relates a RISC-V state and
  a ZisK state: sail-riscv's state ready for a step, the registers agreeing,
  the same pc. A block is a list of R-type instructions at pc, pc + 4, …:
  each encoded in memory, and the ZisK program holding the transpiler's
  instruction at its address. From related states, running the block on
  both sides — sail-riscv's main loop's steps, ZisK's steps — ends in related
  states, both at the block's end.
-/
import Equiv.StepSim

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- A ZisK step on the program `zrom`: execute the instruction at PC. -/
def zstep (zrom : BitVec 64 → _root_.Zisk.zisk_inst) : _root_.Zisk.SailM Bool := do
  let pc ← _root_.Zisk.readReg _root_.Zisk.Register.PC
  _root_.Zisk.Functions.zisk_execute (zrom pc)

theorem zstep_run (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (z : Equiv.Zisk.ZState) (pc : BitVec 64)
    (h : z.regs.get? _root_.Zisk.Register.PC = some pc) :
    (zstep zrom).run z = (_root_.Zisk.Functions.zisk_execute (zrom pc)).run z := by
  simp only [zstep, Equiv.Zisk.run_bind', Equiv.Zisk.readReg_run z _root_.Zisk.Register.PC pc h]

/-- sail-riscv's main loop: `n` steps from step number `i`, none ending the run. -/
inductive RVRun : Nat → Nat → RVState → RVState → Prop
  | done (i : Nat) (s : RVState) : RVRun i 0 s s
  | step {i n : Nat} {s s' s'' : RVState} :
      (interpretSailEffects (try_step i false)).run s = .ok false s' → RVRun (i + 1) n s' s'' →
      RVRun i (n + 1) s s''

/-- `n` ZisK steps on the program `zrom`, none ending the run. -/
inductive ZRun (zrom : BitVec 64 → _root_.Zisk.zisk_inst) : Nat → Equiv.Zisk.ZState → Equiv.Zisk.ZState → Prop
  | done (z : Equiv.Zisk.ZState) : ZRun zrom 0 z z
  | step {n : Nat} {z z' z'' : Equiv.Zisk.ZState} :
      (zstep zrom).run z = .ok false z' → ZRun zrom n z' z'' → ZRun zrom (n + 1) z z''

/-- A RISC-V state and a ZisK state in step: sail-riscv's ready for a step,
    the registers agree, the same pc, ZisK has a step count and no error. -/
structure Rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) : Prop where
  ready : StepReady s rs
  regs : ∃ xs, z.regs.get? _root_.Zisk.Register.Xs = some xs ∧ RegsAgree s xs
  pc : ∃ pc, s.regs.get? Register.PC = some pc ∧ z.regs.get? _root_.Zisk.Register.PC = some pc
  /-- ZisK's step counter, and no halt so far: Zerror false, as a run starts
      it, until the BIOS halts on an exit code (only a halt sets it). -/
  step : ∃ st, z.regs.get? _root_.Zisk.Register.Step = some st ∧
    z.regs.get? _root_.Zisk.Register.Zerror = some false
  /-- The same memory: both models keep it as a map of bytes. -/
  mem : z.mem = s.mem

/-- An R-type instruction, by its fields. -/
structure RInstr where
  op : rop
  rd : BitVec 5
  rs1 : BitVec 5
  rs2 : BitVec 5

/-- Its encoding (RISC-V's base R-type format). -/
def RInstr.encode (i : RInstr) : BitVec 32 :=
  rtypeF7 i.op ++ i.rs2 ++ i.rs1 ++ rtypeF3 i.op ++ i.rd ++ 0x33#7

/-- Byte k of a word, little-endian. -/
def byte (w : BitVec 32) (k : Nat) : BitVec 8 := w.extractLsb' (8 * k) 8

/-- The fields of an R-type instruction, read from its four bytes as
    sail-riscv reassembles them. (Stated slice by slice: an equation between
    the reassembled word, of width 8 + 8 + (8 + 8), and a word of width 32
    would have Lean insert a coercion between the widths.) -/
theorem RInstr.fields (i : RInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x33#7 ∧ v.extractLsb 14 12 = rtypeF3 i.op ∧ v.extractLsb 31 25 = rtypeF7 i.op ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 24 20 = i.rs2 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, RInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- R-type instruction `i` at pc: its encoding in executable memory there,
    rd, rs1, rs2 ≠ x0, and the transpiler's instruction for it in the ZisK
    program at pc. -/
structure RAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rd : i.rd ≠ 0
  rs1 : i.rs1 ≠ 0
  rs2 : i.rs2 ≠ 0
  rom : zrom pc = Equiv.Zisk.regInst (ziskOp i.op) i.rd.toNat i.rs1.toNat i.rs2.toNat

theorem RAt.of_mem {s s' : RVState} {rs pc zrom i} (h : RAt s rs pc zrom i) (hm : s'.mem = s.mem) :
    RAt s' rs pc zrom i := by
  obtain ⟨al, x1, x2, h0, h1, h2, h3, rd, rs1, rs2, rom⟩ := h
  exact ⟨al, x1, x2, hm ▸ h0, hm ▸ h1, hm ▸ h2, hm ▸ h3, rd, rs1, rs2, rom⟩

theorem ne_zero_toNat {x : BitVec 5} (h : x ≠ 0) : x.toNat ≠ 0 := by
  intro e; apply h; apply BitVec.eq_of_toNat_eq; simpa using e

set_option maxHeartbeats 4000000 in
/-- One step of each on an R-type instruction keeps the states related. -/
theorem rstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : RInstr) (hi : RAt s rs pc zrom i) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf7, hrdx, hr1x, hr2x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have := rtype_step_sim s rs hr pc' hpc _ _ _ _ hw i.op hop hf3 hf7
    (by rw [hrdx]; exact ne_zero_toNat hi.rd) (by rw [hr1x]; exact ne_zero_toNat hi.rs1)
    (by rw [hr2x]; exact ne_zero_toNat hi.rs2) n z xs st hx hzpc hst agree
  simp only [hrdx, hr1x, hr2x] at this
  obtain ⟨s', z', xs', hrun, hzrun, hx', hag, hpc'', hzpc', hst', hze', hready, hmem, hzmem⟩ := this
  refine ⟨s', z', hrun, ?_,
    ⟨hready, ⟨xs', hx', hag⟩, ⟨_, hpc'', hzpc'⟩, ⟨_, hst', hze'.trans hze⟩, hzmem.trans (hzm.trans hmem.symm)⟩,
    hpc'', hmem⟩
  rw [zstep_run zrom z pc' hzpc, hi.rom]; exact hzrun

/-- A block of R-type instructions at pc, pc + 4, …. -/
def RBlock (s : RVState) (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) :
    BitVec 64 → List RInstr → Prop
  | _, [] => True
  | pc, i :: is => RAt s rs pc zrom i ∧ RBlock s rs zrom (pc + 4) is

theorem RBlock.of_mem {s s' : RVState} {rs zrom} (hm : s'.mem = s.mem) :
    ∀ {pc l}, RBlock s rs zrom pc l → RBlock s' rs zrom pc l
  | _, [], _ => trivial
  | _, _ :: _, ⟨hi, hb⟩ => ⟨hi.of_mem hm, RBlock.of_mem hm hb⟩

/-- The simulation over a block: from related states at its start, sail-riscv's
    main loop and ZisK both run through it, and end related, at its end. -/
theorem block_sim (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) :
    ∀ (l : List RInstr) (s : RVState) (z : Equiv.Zisk.ZState) (pc : BitVec 64) (n : Nat),
      Rel s z rs → s.regs.get? Register.PC = some pc → RBlock s rs zrom pc l →
      ∃ s' z', RVRun n l.length s s' ∧ ZRun zrom l.length z z' ∧ Rel s' z' rs ∧
        s'.regs.get? Register.PC = some (pc + BitVec.ofNat 64 (4 * l.length)) ∧ s'.mem = s.mem
  | [], s, z, pc, n, hrel, hpc, _ => ⟨s, z, .done n s, .done z, hrel, by simpa using hpc, rfl⟩
  | i :: is, s, z, pc, n, hrel, hpc, ⟨hi, hb⟩ => by
    obtain ⟨s1, z1, hrun, hzrun, hrel1, hpc1, hmem1⟩ := rstep_rel s z rs hrel pc hpc zrom i hi n
    obtain ⟨s2, z2, hrun2, hzrun2, hrel2, hpc2, hmem2⟩ :=
      block_sim zrom rs is s1 z1 (pc + 4) (n + 1) hrel1 hpc1 (hb.of_mem hmem1)
    refine ⟨s2, z2, .step hrun hrun2, .step hzrun hzrun2, hrel2, ?_, hmem2.trans hmem1⟩
    rw [hpc2]; congr 1
    simp only [List.length_cons]
    rw [BitVec.add_assoc]; congr 1
    apply BitVec.eq_of_toNat_eq; simp; omega

end Equiv.Sim
