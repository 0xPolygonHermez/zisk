/-
  The simulation, one step, on lui and auipc (the transpiler's copyb of the
  constant it computes at the instruction's address), jal and jalr with an
  even offset. A jump's link goes to rd unless rd is x0, on both sides.
-/
import Equiv.BranchSim
import Equiv.RiscvJump
import Equiv.ZiskJump

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- Writing v to rd on both sides keeps them agreeing, x0 included (no
    write on either side). -/
theorem regs_after0 (r : RVState) (xs : Vector (BitVec 64) 40) (rd : Nat) (v : BitVec 64)
    (ld : rd < 32) (agree : RegsAgree r xs) :
    RegsAgree (setX r rd v) (Equiv.Zisk.xsUpd xs rd v) := by
  by_cases h0 : rd = 0
  · subst h0; simp only [setX, Equiv.Zisk.xsUpd, if_true]; exact agree
  · simp only [Equiv.Zisk.xsUpd, h0, if_false]; exact regs_after r xs rd v h0 ld agree

theorem linkRegs_xs (m : Std.ExtDHashMap _root_.Zisk.Register _root_.Zisk.RegisterType)
    (xs : Vector (BitVec 64) 40) (rd : Nat) (v : BitVec 64) (hx : m.get? _root_.Zisk.Register.Xs = some xs) :
    (Equiv.Zisk.linkRegs m xs rd v).get? _root_.Zisk.Register.Xs = some (Equiv.Zisk.xsUpd xs rd v) := by
  unfold Equiv.Zisk.linkRegs Equiv.Zisk.xsUpd
  split
  · exact hx
  · simp

theorem jalr_target (v1 : BitVec 64) (imm : BitVec 12) (h : imm[0] = false) :
    (Equiv.Zisk.JALR_MASK &&& v1) + imm.signExtend 64 = jalrTarget v1 imm := by
  simp only [Equiv.Zisk.JALR_MASK, jalrTarget, Sail.BitVec.update, Sail.BitVec.updateSubrange']
  bv_decide

theorem execute_utype_eq (imm : BitVec 20) (rd : regidx) (op : uop) :
    execute (.UTYPE (imm, rd, op)) = execute_UTYPE imm rd op := by simp [execute]

theorem execute_jal_eq (imm : BitVec 21) (rd : regidx) :
    execute (.JAL (imm, rd)) = execute_JAL imm rd := by simp [execute]

theorem execute_jalr_eq (imm : BitVec 12) (rs1 rd : regidx) :
    execute (.JALR (imm, rs1, rd)) = execute_JALR imm rs1 rd := by simp [execute]

/-- The common shape of the one-step proofs here: the generic step on
    sail-riscv, a ZisK step that writes v to rd and goes to tgt, the same on
    both sides. -/
theorem jump_step_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hr : StepReady s rs)
    (xs : Vector (BitVec 64) 40) (agree : RegsAgree s xs) (hzm : z.mem = s.mem)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (b0 b1 b2 b3 : BitVec 8) (hw : WordAt s rs pc b0 b1 b2 b3) (ins : instruction)
    (hdec : ∀ t : RVState, t.regs.get? Register.cur_privilege = some .Machine →
      (∃ v, t.regs.get? Register.mseccfg = some v) →
      (∃ v, t.regs.get? Register.misa = some v ∧ _get_Misa_M v = 1#1) →
      (interpretSailEffects (ext_decode (b3 ++ b2 ++ (b1 ++ b0)))).run t = .ok ins t)
    (rd : Nat) (ld : rd < 32) (v tgt : BitVec 64)
    (hexec : ∀ t : RVState, (∀ k, k < 32 → getX t k = getX s k) → t.mem = s.mem → MemReady t rs →
      t.regs.get? Register.mseccfg = s.regs.get? Register.mseccfg → WF t →
      t.regs.get? Register.nextPC = some (pc + 4) →
      (∀ r, isXReg r = false → r ≠ Register.nextPC → r ≠ Register.minstret_increment →
        t.regs.get? r = s.regs.get? r) →
      (interpretSailEffects (execute ins)).run t
        = .ok RETIRE_SUCCESS (setX { t with regs := t.regs.insert Register.nextPC tgt } rd v))
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst)
    (hz : ∃ z', (zstep zrom).run z = .ok false z' ∧
      z'.regs.get? _root_.Zisk.Register.Xs = some (Equiv.Zisk.xsUpd xs rd v) ∧
      z'.regs.get? _root_.Zisk.Register.PC = some tgt ∧
      (∃ st, z'.regs.get? _root_.Zisk.Register.Step = some st ∧
        z'.regs.get? _root_.Zisk.Register.Zerror = some false) ∧ z'.mem = z.mem) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some tgt := by
  obtain ⟨s', hrun, hmem, hwf, hpc', hxs', ho⟩ := try_step_gen s rs hr pc hpc b0 b1 b2 b3 hw ins hdec rd v tgt hexec n
  obtain ⟨z', hzrun, hx', hzpc', hst', hzmem⟩ := hz
  refine ⟨s', z', hrun, hzrun,
    ⟨hr.keep hwf ho, ⟨_, hx', ?_⟩, ⟨_, hpc', hzpc'⟩, hst', hzmem.trans (hzm.trans hmem.symm)⟩, hpc'⟩
  intro k hk1 hk2
  rw [hxs' k hk2]
  exact regs_after0 s xs rd v ld agree k hk1 hk2

/-- The x registers a ZisK state's vector holds after a step that writes
    only the listed non-Xs registers. -/
theorem other_regs {s : RVState} (hpc : s.regs.get? Register.PC = some pc)
    (mi : RegisterType Register.misa) (hmi : s.regs.get? Register.misa = some mi) {t : RVState}
    (hother : ∀ r, isXReg r = false → r ≠ Register.nextPC → r ≠ Register.minstret_increment →
      t.regs.get? r = s.regs.get? r) :
    t.regs.get? Register.PC = some pc ∧ t.regs.get? Register.misa = some mi := by
  refine ⟨?_, ?_⟩
  · rw [hother _ (by decide) (by decide) (by decide)]; exact hpc
  · rw [hother _ (by decide) (by decide) (by decide)]; exact hmi

/-! ### lui, auipc -/

structure UInstr where
  op : uop
  rd : BitVec 5
  imm : BitVec 20

def UInstr.encode (i : UInstr) : BitVec 32 := i.imm ++ i.rd ++ uopEnc i.op

theorem UInstr.fields (i : UInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = uopEnc i.op ∧ v.extractLsb 31 12 = i.imm ∧ v.extractLsb 11 7 = i.rd ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; cases h : i.op <;> simp only [v, UInstr.encode, byte, h, uopEnc] <;>
    refine ⟨?_, ?_, ?_, ?_⟩ <;> bv_decide

structure UAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : UInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rd : i.rd ≠ 0
  rom : zrom pc = Equiv.Zisk.constInst i.rd.toNat (utypeVal i.op pc i.imm)

set_option maxHeartbeats 4000000 in
theorem ustep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : UInstr) (hi : UAt s rs pc zrom i) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, himm, hrdx, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have ed := ne_zero_toNat hi.rd
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw (.UTYPE (i.imm, .Regidx i.rd, i.op))
    ?_ i.rd.toNat ld (utypeVal i.op pc' i.imm) (pc' + 4) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_utype t _ i.op hop htp w hw'
    rw [himm, hrdx] at this; exact this
  · intro t _ _ _ _ _ hnext hother
    rw [execute_utype_eq, execute_utype t i.op i.rd i.imm pc' (other_regs hpc mi hmi hother).1,
      insert_same t _ _ hnext]
    rfl
  · rw [zstep_run zrom z pc' hzpc, hi.rom]
    refine ⟨_, Equiv.Zisk.execute_const z i.rd.toNat _ (by omega) xs pc' st hx hzpc hst, ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.xsUpd, ed]

/-! ### jal -/

structure JInstr where
  rd : BitVec 5
  j20 : BitVec 1
  j19_12 : BitVec 8
  j11 : BitVec 1
  j10_1 : BitVec 10

/-- The jump offset, as sail-riscv's decoder assembles it. -/
def JInstr.imm (i : JInstr) : BitVec 21 := i.j20 ++ i.j19_12 ++ i.j11 ++ i.j10_1 ++ 0#1

def JInstr.encode (i : JInstr) : BitVec 32 := i.j20 ++ i.j10_1 ++ i.j11 ++ i.j19_12 ++ i.rd ++ 0x6f#7

theorem JInstr.fields (i : JInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x6f#7 ∧ v.extractLsb 31 31 = i.j20 ∧ v.extractLsb 19 12 = i.j19_12 ∧
    v.extractLsb 20 20 = i.j11 ∧ v.extractLsb 30 21 = i.j10_1 ∧ v.extractLsb 11 7 = i.rd ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, JInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

structure JAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tal : (pc + i.imm.signExtend 64)[0] = false ∧ (pc + i.imm.signExtend 64)[1] = false
  rom : zrom pc = Equiv.Zisk.jalInst i.rd.toNat (i.imm.signExtend 64)

set_option maxHeartbeats 4000000 in
theorem jstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JInstr) (hi : JAt s rs pc zrom i) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + i.imm.signExtend 64) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, h20, h19, h11, h10, hrdx, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have ld : i.rd.toNat < 32 := i.rd.isLt
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw (.JAL (i.imm, .Regidx i.rd))
    ?_ i.rd.toNat ld (pc' + 4) (pc' + i.imm.signExtend 64) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_jal t _ hop htp w hw'
    rw [h20, h19, h11, h10, hrdx] at this; exact this
  · intro t _ _ _ _ _ hnext hother
    obtain ⟨hpct, hmt⟩ := other_regs hpc mi hmi hother
    rw [execute_jal_eq, execute_jal t i.rd i.imm pc' (pc' + 4) mi hpct hnext hmt hi.tal]
    rfl
  · rw [zstep_run zrom z pc' hzpc, hi.rom]
    refine ⟨_, Equiv.Zisk.execute_jal z i.rd.toNat _ ld xs pc' st hx hzpc hst, ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp only [Std.ExtDHashMap.get?_insert]
    simpa using linkRegs_xs z.regs xs i.rd.toNat (pc' + 4) hx

/-! ### jalr -/

structure JrInstr where
  rd : BitVec 5
  rs1 : BitVec 5
  imm : BitVec 12

def JrInstr.encode (i : JrInstr) : BitVec 32 := i.imm ++ i.rs1 ++ 0#3 ++ i.rd ++ 0x67#7

theorem JrInstr.fields (i : JrInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x67#7 ∧ v.extractLsb 14 12 = 0#3 ∧ v.extractLsb 31 20 = i.imm ∧
    v.extractLsb 19 15 = i.rs1 ∧ v.extractLsb 11 7 = i.rd ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, JrInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- jalr `i` at pc, with an even offset (the transpiler's one-instruction
    form) and rs1 ≠ x0. -/
structure JrAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JrInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rs1 : i.rs1 ≠ 0
  even : i.imm[0] = false
  rom : zrom pc = Equiv.Zisk.jalrInst i.rd.toNat i.rs1.toNat (i.imm.signExtend 64)

set_option maxHeartbeats 4000000 in
/-- One step of each on a jalr keeps the states related, when its target
    (rs1 + imm, bit 0 cleared) is 4-aligned. -/
theorem jrstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JrInstr) (hi : JrAt s rs pc zrom i)
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (tal : (jalrTarget v1 i.imm)[0] = false ∧ (jalrTarget v1 i.imm)[1] = false) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (jalrTarget v1 i.imm) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hr1x, hrdx, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have e1 := ne_zero_toNat hi.rs1
  have a1 := agree i.rs1.toNat (by omega) l1
  have hxv : xs[i.rs1.toNat]! = v1 := by rw [hv1] at a1; exact (Option.some.inj a1).symm
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  obtain ⟨sv, hsv, -⟩ := hr.pmm
  refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw (.JALR (i.imm, .Regidx i.rs1, .Regidx i.rd))
    ?_ i.rd.toNat ld (pc' + 4) (jalrTarget v1 i.imm) ?_ zrom ?_ n
  · intro t htp ⟨w, hw'⟩ hmt
    have := decode_jalr t _ hop hf3 htp w hw'
    rw [himm, hr1x, hrdx] at this; exact this
  · intro t he _ hmr hsec _ hnext hother
    obtain ⟨-, hmt⟩ := other_regs hpc mi hmi hother
    rw [execute_jalr_eq, execute_jalr t i.rs1 i.rd i.imm v1 (pc' + 4) mi (getX_of_agree he l1 hv1) hnext hmt
      hmr.priv sv (hsec.trans hsv) tal]
    rfl
  · rw [zstep_run zrom z pc' hzpc, hi.rom]
    have hxv' : xs[(i.rs1.toNat : Int)]! = v1 := by rw [getElem!_int]; exact hxv
    refine ⟨_, Equiv.Zisk.execute_jalr z i.rd.toNat i.rs1.toNat _ ld (by omega) xs pc' st hx hzpc hst, ?_,
      ?_, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    · simp only [Std.ExtDHashMap.get?_insert]
      simpa using linkRegs_xs z.regs xs i.rd.toNat (pc' + 4) hx
    · simp [hxv', jalr_target v1 i.imm hi.even]

end Equiv.Sim
