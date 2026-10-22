/-
  The simulation over straight-line code mixing the instructions proved so
  far: R-type (Equiv/Block.lean), I-type and immediate shifts. Each kind is
  an instance of one generic step (`alu_step_rel`): an instruction the word
  at pc decodes to, that writes one register, with the same value on both
  sides.
-/
import Equiv.Block
import Equiv.SimImm
import Equiv.RiscvDecodeImm

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- Writing the same value to register rd on both sides keeps them agreeing. -/
theorem regs_after (r : RVState) (xs : Vector (BitVec 64) 40) (rd : Nat) (val : BitVec 64)
    (hd : rd ≠ 0) (ld : rd < 32) (agree : RegsAgree r xs) :
    RegsAgree (setX r rd val) (vectorUpdate xs rd val) := by
  intro k hk1 hk2
  rw [getX_setX r rd k val ld hk2, getElem!_update xs rd k val (by omega)]
  by_cases e : k = rd
  · rw [if_pos ⟨hd, e⟩, if_pos e]
  · rw [if_neg (fun h => e h.2), if_neg e]; exact agree k hk1 hk2

set_option maxHeartbeats 4000000 in
/-- One step of each on an instruction that writes `val` to rd on both
    sides keeps the states related. -/
theorem alu_step_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hr : StepReady s rs)
    (xs : Vector (BitVec 64) 40) (_hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (agree : RegsAgree s xs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (_hzpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? Register.cur_privilege = some .Machine →
      (∃ v, t.regs.get? Register.mseccfg = some v) →
      (interpretSailEffects (ext_decode (b3 ++ b2 ++ (b1 ++ b0)))).run t = .ok ins t)
    (rd : Nat) (hrd : rd ≠ 0) (ld : rd < 32) (val : BitVec 64)
    (hexec : ∀ t : RVState, (∀ k, k < 32 → getX t k = getX s k) → t.mem = s.mem → MemReady t rs →
      t.regs.get? Register.mseccfg = s.regs.get? Register.mseccfg → WF t →
      (interpretSailEffects (execute ins)).run t = .ok RETIRE_SUCCESS (setX t rd val))
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (hzm : z.mem = s.mem)
    (hz : ∃ z', (zstep zrom).run z = .ok false z' ∧
      z'.regs.get? _root_.Zisk.Register.Xs = some (vectorUpdate xs rd val) ∧
      z'.regs.get? _root_.Zisk.Register.PC = some (pc + 4) ∧
      (∃ st, z'.regs.get? _root_.Zisk.Register.Step = some st) ∧ z'.mem = z.mem) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨s', hrun, hmem, hwf, hpc', hxs', ho⟩ :=
    try_step_alu s rs hr pc hpc b0 b1 b2 b3 hw ins hdec rd hrd val hexec n
  obtain ⟨z', hzrun, hx', hzpc', hst', hzmem⟩ := hz
  refine ⟨s', z', hrun, hzrun,
    ⟨hr.keep hwf ho, ⟨_, hx', ?_⟩, ⟨_, hpc', hzpc'⟩, hst', hzmem.trans (hzm.trans hmem.symm)⟩, hpc', hmem⟩
  intro k hk1 hk2
  rw [hxs' k hk2]
  exact regs_after s xs rd val hrd ld agree k hk1 hk2

theorem execute_itype_eq (imm : BitVec 12) (a b : regidx) (op : iop) :
    execute (.ITYPE (imm, a, b, op)) = execute_ITYPE imm a b op := by
  simp [execute]

theorem execute_shiftiop_eq (sh : BitVec 6) (a b : regidx) (op : sop) :
    execute (.SHIFTIOP (sh, a, b, op)) = execute_SHIFTIOP sh a b op := by
  simp [execute]

/-! ### I-type -/

/-- An I-type instruction, by its fields. -/
structure IInstr where
  op : iop
  rd : BitVec 5
  rs1 : BitVec 5
  imm : BitVec 12

def IInstr.encode (i : IInstr) : BitVec 32 :=
  i.imm ++ i.rs1 ++ itypeF3 i.op ++ i.rd ++ 0x13#7

theorem IInstr.fields (i : IInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x13#7 ∧ v.extractLsb 14 12 = itypeF3 i.op ∧ v.extractLsb 31 20 = i.imm ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, IInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- I-type instruction `i` at pc: its encoding in executable memory there,
    rd, rs1 ≠ x0, and the transpiler's instruction for it at pc. -/
structure IAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : IInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rd : i.rd ≠ 0
  rs1 : i.rs1 ≠ 0
  rom : zrom pc = Equiv.Zisk.immInst (ziskOp (iopR i.op)) i.rd.toNat i.rs1.toNat (i.imm.signExtend 64)

theorem IAt.of_mem {s s' : RVState} {rs pc zrom i} (h : IAt s rs pc zrom i) (hm : s'.mem = s.mem) :
    IAt s' rs pc zrom i := by
  obtain ⟨al, x1, x2, h0, h1, h2, h3, rd, rs1, rom⟩ := h
  exact ⟨al, x1, x2, hm ▸ h0, hm ▸ h1, hm ▸ h2, hm ▸ h3, rd, rs1, rom⟩

set_option maxHeartbeats 4000000 in
theorem istep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : IInstr) (hi : IAt s rs pc zrom i) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have e1 := ne_zero_toNat hi.rs1
  have ed := ne_zero_toNat hi.rd
  have a1 := agree i.rs1.toNat (by omega) l1
  have hrd5 : (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 11 7 ≠ 0#5 := by
    rw [hrdx]; exact hi.rd
  refine alu_step_rel s z rs hr xs hx agree pc' hpc hzpc _ _ _ _ hw
    (.ITYPE (i.imm, .Regidx i.rs1, .Regidx i.rd, i.op)) ?_ i.rd.toNat ed ld
    (itypeVal i.op xs[i.rs1.toNat]! i.imm) ?_ zrom hzm ?_ n
  · intro t htp ⟨v, hv⟩
    have := decode_itype t _ i.op hop hf3 hrd5 htp v hv
    rw [himm, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _
    rw [execute_itype_eq]
    exact execute_itype t i.op i.rs1 i.rd i.imm _ (getX_of_agree he l1 a1)
  · rw [zstep_run zrom z pc' hzpc, hi.rom]
    refine ⟨_, Equiv.Zisk.execute_immop z _ (pureOp _) i.rd.toNat i.rs1.toNat _ (by omega) (by omega)
      xs pc' st hx hzpc hst, ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert]⟩, rfl⟩
    simp [Std.ExtDHashMap.get?_insert, ziskIVal, getElem!_int]

/-! ### Immediate shifts -/

/-- An immediate shift, by its fields. -/
structure SInstr where
  op : sop
  rd : BitVec 5
  rs1 : BitVec 5
  sh : BitVec 6

def SInstr.encode (i : SInstr) : BitVec 32 :=
  shiftF6 i.op ++ i.sh ++ i.rs1 ++ shiftF3 i.op ++ i.rd ++ 0x13#7

theorem SInstr.fields (i : SInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x13#7 ∧ v.extractLsb 14 12 = shiftF3 i.op ∧ v.extractLsb 31 26 = shiftF6 i.op ∧
    v.extractLsb 25 20 = i.sh ∧ v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, SInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

structure SAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rd : i.rd ≠ 0
  rs1 : i.rs1 ≠ 0
  rom : zrom pc = Equiv.Zisk.immInst (ziskOp (sopR i.op)) i.rd.toNat i.rs1.toNat (i.sh.zeroExtend 64)

theorem SAt.of_mem {s s' : RVState} {rs pc zrom i} (h : SAt s rs pc zrom i) (hm : s'.mem = s.mem) :
    SAt s' rs pc zrom i := by
  obtain ⟨al, x1, x2, h0, h1, h2, h3, rd, rs1, rom⟩ := h
  exact ⟨al, x1, x2, hm ▸ h0, hm ▸ h1, hm ▸ h2, hm ▸ h3, rd, rs1, rom⟩

set_option maxHeartbeats 4000000 in
theorem sstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SInstr) (hi : SAt s rs pc zrom i) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hf6, hsh, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have e1 := ne_zero_toNat hi.rs1
  have ed := ne_zero_toNat hi.rd
  have a1 := agree i.rs1.toNat (by omega) l1
  have hrd5 : (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 11 7 ≠ 0#5 := by
    rw [hrdx]; exact hi.rd
  refine alu_step_rel s z rs hr xs hx agree pc' hpc hzpc _ _ _ _ hw
    (.SHIFTIOP (i.sh, .Regidx i.rs1, .Regidx i.rd, i.op)) ?_ i.rd.toNat ed ld
    (shiftVal i.op xs[i.rs1.toNat]! i.sh) ?_ zrom hzm ?_ n
  · intro t htp ⟨v, hv⟩
    have := decode_shiftiop t _ i.op hop hf3 hf6 hrd5 htp v hv
    rw [hsh, hr1x, hrdx] at this; exact this
  · intro t he _ _ _ _
    rw [execute_shiftiop_eq]
    exact execute_shiftiop t i.op i.rs1 i.rd i.sh _ (getX_of_agree he l1 a1)
  · rw [zstep_run zrom z pc' hzpc, hi.rom]
    refine ⟨_, Equiv.Zisk.execute_immop z _ (pureOp _) i.rd.toNat i.rs1.toNat _ (by omega) (by omega)
      xs pc' st hx hzpc hst, ?_, by simp, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert]⟩, rfl⟩
    simp [Std.ExtDHashMap.get?_insert, ziskSVal, getElem!_int]

/-! ### Programs -/

/-- An instruction of the straight-line code proved so far. -/
inductive Instr where
  | R (i : RInstr)
  | I (i : IInstr)
  | S (i : SInstr)

/-- Instruction `i` at pc, in memory and in the ZisK program. -/
def InstrAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) : Instr → Prop
  | .R i => RAt s rs pc zrom i
  | .I i => IAt s rs pc zrom i
  | .S i => SAt s rs pc zrom i

theorem InstrAt.of_mem {s s' : RVState} {rs pc zrom} (hm : s'.mem = s.mem) :
    ∀ {i}, InstrAt s rs pc zrom i → InstrAt s' rs pc zrom i
  | .R _, h => RAt.of_mem h hm
  | .I _, h => IAt.of_mem h hm
  | .S _, h => SAt.of_mem h hm

theorem step_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : Instr) (hi : InstrAt s rs pc zrom i) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem :=
  match i, hi with
  | .R i, hi => rstep_rel s z rs hrel pc hpc zrom i hi n
  | .I i, hi => istep_rel s z rs hrel pc hpc zrom i hi n
  | .S i, hi => sstep_rel s z rs hrel pc hpc zrom i hi n

/-- Straight-line code at pc, pc + 4, …. -/
def CodeAt (s : RVState) (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) :
    BitVec 64 → List Instr → Prop
  | _, [] => True
  | pc, i :: is => InstrAt s rs pc zrom i ∧ CodeAt s rs zrom (pc + 4) is

theorem CodeAt.of_mem {s s' : RVState} {rs zrom} (hm : s'.mem = s.mem) :
    ∀ {pc l}, CodeAt s rs zrom pc l → CodeAt s' rs zrom pc l
  | _, [], _ => trivial
  | _, _ :: _, ⟨hi, hb⟩ => ⟨InstrAt.of_mem hm hi, CodeAt.of_mem hm hb⟩

/-- The simulation over straight-line code of R-type, I-type and immediate
    shift instructions: from related states at its start, sail-riscv's main
    loop and ZisK both run through it, and end related, at its end. -/
theorem code_sim (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) :
    ∀ (l : List Instr) (s : RVState) (z : Equiv.Zisk.ZState) (pc : BitVec 64) (n : Nat),
      Rel s z rs → s.regs.get? Register.PC = some pc → CodeAt s rs zrom pc l →
      ∃ s' z', RVRun n l.length s s' ∧ ZRun zrom l.length z z' ∧ Rel s' z' rs ∧
        s'.regs.get? Register.PC = some (pc + BitVec.ofNat 64 (4 * l.length)) ∧ s'.mem = s.mem
  | [], s, z, pc, n, hrel, hpc, _ => ⟨s, z, .done n s, .done z, hrel, by simpa using hpc, rfl⟩
  | i :: is, s, z, pc, n, hrel, hpc, ⟨hi, hb⟩ => by
    obtain ⟨s1, z1, hrun, hzrun, hrel1, hpc1, hmem1⟩ := step_rel s z rs hrel pc hpc zrom i hi n
    obtain ⟨s2, z2, hrun2, hzrun2, hrel2, hpc2, hmem2⟩ :=
      code_sim zrom rs is s1 z1 (pc + 4) (n + 1) hrel1 hpc1 (hb.of_mem hmem1)
    refine ⟨s2, z2, .step hrun hrun2, .step hzrun hzrun2, hrel2, ?_, hmem2.trans hmem1⟩
    rw [hpc2]; congr 1
    simp only [List.length_cons]
    rw [BitVec.add_assoc]; congr 1
    apply BitVec.eq_of_toNat_eq; simp; omega

end Equiv.Sim
