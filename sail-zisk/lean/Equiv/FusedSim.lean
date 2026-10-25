/-
  The simulation on auipc followed by a jalr through its rd, which the
  transpiler fuses (riscv2zisk_context.rs, auipc): two sail-riscv steps, one
  ZisK step. The ZisK instruction at the auipc's address writes rd — with
  auipc's value when the jalr links to x0, with the return address pc + 8
  when it links to rd — and jumps to the jalr's target, both computed at
  transpile time.
-/
import Equiv.Prog

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

set_option maxHeartbeats 4000000 in
/-- sail-riscv's step on auipc (rd ≠ x0): rd gets pc + uimm << 12. -/
theorem rv_auipc_step (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc) (rd : BitVec 5) (uimm : BitVec 20)
    (hw : WordAt s rs pc (byte (UInstr.encode ⟨.AUIPC, rd, uimm⟩) 0) (byte (UInstr.encode ⟨.AUIPC, rd, uimm⟩) 1)
      (byte (UInstr.encode ⟨.AUIPC, rd, uimm⟩) 2) (byte (UInstr.encode ⟨.AUIPC, rd, uimm⟩) 3)) (n : Nat) :
    ∃ s1, (interpretSailEffects (try_step n false)).run s = .ok false s1 ∧ s1.mem = s.mem ∧
      StepReady s1 rs ∧ s1.regs.get? Register.PC = some (pc + 4) ∧
      ∀ k, k < 32 → getX s1 k = getX (setX s rd.toNat (utypeVal .AUIPC pc uimm)) k := by
  obtain ⟨hop, himm, hrdx, -⟩ := UInstr.fields ⟨.AUIPC, rd, uimm⟩
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  obtain ⟨s1, hrun, hmem, hwf, hpc1, hxs, ho⟩ :=
    try_step_gen s rs hr pc hpc _ _ _ _ hw (.UTYPE (uimm, .Regidx rd, .AUIPC))
      (by intro t htp ⟨w, hw'⟩ hmt _
          have := decode_utype t _ .AUIPC hop htp w hw'
          rw [himm, hrdx] at this; exact this)
      rd.toNat (utypeVal .AUIPC pc uimm) (pc + 4)
      (by intro t _ _ _ _ _ hnext hother
          rw [execute_utype_eq, execute_utype t .AUIPC rd uimm pc (other_regs hpc mi hmi hother).1,
            insert_same t _ _ hnext]
          rfl) n
  exact ⟨s1, hrun, hmem, hr.keep hwf ho, hpc1, hxs⟩

set_option maxHeartbeats 4000000 in
/-- sail-riscv's step on jalr (any rs1) to a 4-aligned target: rd gets
    pc + 4, pc the target. -/
theorem rv_jalr_step (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc) (i : JrInstr)
    (hw : WordAt s rs pc (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3))
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (tal : (jalrTarget v1 i.imm)[0] = false ∧ (jalrTarget v1 i.imm)[1] = false) (n : Nat) :
    ∃ s2, (interpretSailEffects (try_step n false)).run s = .ok false s2 ∧ s2.mem = s.mem ∧
      StepReady s2 rs ∧ s2.regs.get? Register.PC = some (jalrTarget v1 i.imm) ∧
      ∀ k, k < 32 → getX s2 k = getX (setX s i.rd.toNat (pc + 4)) k := by
  obtain ⟨hop, hf3, himm, hr1x, hrdx, -⟩ := i.fields
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  obtain ⟨mi, hmi⟩ := hr.mem.misa
  obtain ⟨sv, hsv, -⟩ := hr.pmm
  obtain ⟨s2, hrun, hmem, hwf, hpc2, hxs, ho⟩ :=
    try_step_gen s rs hr pc hpc _ _ _ _ hw (.JALR (i.imm, .Regidx i.rs1, .Regidx i.rd))
      (by intro t htp ⟨w, hw'⟩ hmt _
          have := decode_jalr t _ hop hf3 htp w hw'
          rw [himm, hr1x, hrdx] at this; exact this)
      i.rd.toNat (pc + 4) (jalrTarget v1 i.imm)
      (by intro t he _ hmr hsec _ hnext hother
          obtain ⟨-, hmt⟩ := other_regs hpc mi hmi hother
          rw [execute_jalr_eq, execute_jalr t i.rs1 i.rd i.imm v1 (pc + 4) mi (getX_of_agree he l1 hv1) hnext hmt
            hmr.priv sv (hsec.trans hsv) tal]
          rfl) n
  exact ⟨s2, hrun, hmem, hr.keep hwf ho, hpc2, hxs⟩

/-- auipc rd, uimm then jalr rd2, jimm(rd), rd2 rd or x0: the pair the
    transpiler fuses. -/
structure FInstr where
  rd : BitVec 5
  uimm : BitVec 20
  rd2 : BitVec 5
  jimm : BitVec 12

def FInstr.auipc (i : FInstr) : UInstr := ⟨.AUIPC, i.rd, i.uimm⟩
def FInstr.jalr (i : FInstr) : JrInstr := ⟨i.rd2, i.rd, i.jimm⟩
/-- auipc's value at pc. -/
def FInstr.A (i : FInstr) (pc : BitVec 64) : BitVec 64 := utypeVal .AUIPC pc i.uimm
/-- The pair's target. -/
def FInstr.T (i : FInstr) (pc : BitVec 64) : BitVec 64 := jalrTarget (i.A pc) i.jimm
/-- What rd holds after the pair: auipc's value when the jalr links to x0,
    the return address when it links to rd. -/
def FInstr.c (i : FInstr) (pc : BitVec 64) : BitVec 64 := if i.rd2 = 0 then i.A pc else pc + 8

/-- The fused pair at pc: both words in executable memory, rd ≠ x0, the jalr
    linking to rd or x0, a 4-aligned target, and the transpiler's fused
    instruction at pc. -/
structure FAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : FInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  x3 : ExecAt rs (pc + 4)
  x4 : ExecAt rs (pc + 4 + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.auipc.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.auipc.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.auipc.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.auipc.encode 3)
  g0 : s.mem.get? (pc + 4).toNat = some (byte i.jalr.encode 0)
  g1 : s.mem.get? ((pc + 4).toNat + 1) = some (byte i.jalr.encode 1)
  g2 : s.mem.get? ((pc + 4).toNat + 2) = some (byte i.jalr.encode 2)
  g3 : s.mem.get? ((pc + 4).toNat + 3) = some (byte i.jalr.encode 3)
  rd : i.rd ≠ 0
  link : i.rd2 = i.rd ∨ i.rd2 = 0
  tal : (i.T pc)[0] = false ∧ (i.T pc)[1] = false
  rom : zrom pc = Equiv.Zisk.constJInst i.rd.toNat (i.c pc) (i.T pc - pc)

set_option maxHeartbeats 4000000 in
/-- The fused pair: two steps of sail-riscv's main loop and one of ZisK keep
    the states related, both at the pair's target. -/
theorem fused_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : FInstr) (hi : FAt s rs pc zrom i) (n : Nat) :
    ∃ s2 z', RVRun n 2 s s2 ∧ ZRun zrom 1 z z' ∧ Rel s2 z' rs ∧
      s2.regs.get? Register.PC = some (i.T pc) ∧
      StepReady (rvNext n s) rs ∧ (rvNext n s).regs.get? Register.PC = some (pc + 4) ∧
      (rvNext n s).mem = s.mem ∧ s2.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨-, -, -, hc1⟩ := UInstr.fields i.auipc
  obtain ⟨-, -, -, -, -, hc2⟩ := i.jalr.fields
  have hw1 : WordAt s rs pc' (byte i.auipc.encode 0) (byte i.auipc.encode 1) (byte i.auipc.encode 2)
      (byte i.auipc.encode 3) := ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc1⟩
  obtain ⟨s1, hrun1, hmem1, hr1, hpc1, hxs1⟩ := rv_auipc_step s rs hr pc' hpc i.rd i.uimm hw1 n
  have hal4 : (pc' + 4).toNat % 4 = 0 := by rw [BitVec.toNat_add]; simp; have := hi.al; omega
  have hw2 : WordAt s1 rs (pc' + 4) (byte i.jalr.encode 0) (byte i.jalr.encode 1) (byte i.jalr.encode 2)
      (byte i.jalr.encode 3) :=
    ⟨hal4, hi.x3, hi.x4, hmem1 ▸ hi.g0, hmem1 ▸ hi.g1, hmem1 ▸ hi.g2, hmem1 ▸ hi.g3, hc2⟩
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have l2 : i.rd2.toNat < 32 := i.rd2.isLt
  have ed := ne_zero_toNat hi.rd
  have hv1 : getX s1 i.jalr.rs1.toNat = some (i.A pc') := by
    show getX s1 i.rd.toNat = _
    rw [hxs1 _ ld, getX_setX _ _ _ _ ld ld, if_pos ⟨ed, rfl⟩]; rfl
  obtain ⟨s2, hrun2, hmem2, hr2, hpc2, hxs2⟩ := rv_jalr_step s1 rs hr1 (pc' + 4) hpc1 i.jalr hw2 (i.A pc') hv1
    hi.tal (n + 1)
  have hz := Equiv.Zisk.execute_constj z i.rd.toNat (i.c pc') (i.T pc' - pc') (by omega) xs pc' st hx hzpc hst
  have hzrun := hz
  rw [← hi.rom, ← zstep_run zrom z pc' hzpc] at hzrun
  refine ⟨s2, _, .step hrun1 (.step hrun2 (.done _ _)), .step hzrun (.done _),
    ⟨hr2, ⟨vectorUpdate xs i.rd.toNat (i.c pc'), by simp [Std.ExtDHashMap.get?_insert], ?_⟩,
      ⟨_, hpc2, ?_⟩,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, ?_⟩, hpc2,
    by rw [rvNext_of hrun1]; exact hr1, by rw [rvNext_of hrun1]; exact hpc1, by rw [rvNext_of hrun1]; exact hmem1,
    hmem2.trans hmem1⟩
  · intro k hk1 hk2
    have e8 : pc' + 4 + 4 = pc' + 8 := by rw [BitVec.add_assoc]; rfl
    rw [hxs2 k hk2]
    simp only [FInstr.jalr]
    rw [getX_setX _ _ _ _ l2 hk2, hxs1 k hk2, getX_setX _ _ _ _ ld hk2,
      getElem!_update xs _ k _ (by omega), e8]
    simp only [FInstr.c]
    rcases hi.link with h | h
    · have : i.rd2 ≠ 0 := h ▸ hi.rd
      rw [h] at this ⊢
      by_cases e : k = i.rd.toNat
      · simp [e, ed]; intro h0; exact absurd h0 hi.rd
      · simp [e, ed]; exact agree k hk1 hk2
    · have h0 : i.rd2.toNat = 0 := by simp [h]
      by_cases e : k = i.rd.toNat
      · simp [e, ed, h]; rfl
      · simp [e, ed, h]; exact agree k hk1 hk2
  · simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
    show pc' + (i.T pc' - pc') = i.T pc'
    generalize i.T pc' = y; bv_decide
  · show z.mem = s2.mem
    rw [hmem2, hmem1, hzm]

theorem RVRun.append {n a b : Nat} {s t u : RVState} (h1 : RVRun n a s t) (h2 : RVRun (n + a) b t u) :
    RVRun n (a + b) s u := by
  induction h1 with
  | done i s => simpa using h2
  | @step i m _ _ _ h r ih =>
    rw [show m + 1 + b = (m + b) + 1 by omega]
    exact .step h (ih (by rw [show i + 1 + m = i + (m + 1) by omega]; exact h2))

theorem ZRun.append {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {a b : Nat} {z y x : Equiv.Zisk.ZState}
    (h1 : ZRun zrom a z y) (h2 : ZRun zrom b y x) : ZRun zrom (a + b) z x := by
  induction h1 with
  | done z => simpa using h2
  | @step m _ _ _ h r ih =>
    rw [show m + 1 + b = (m + b) + 1 by omega]
    exact .step h (ih h2)

/-- jalr `i` with the transpiler's two-instruction form (for an odd offset;
    any offset, as far as the proof goes): its word in executable memory at
    pc, any rs1, the add at pc and the and at the internal address ia. -/
structure JoAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JrInstr) (ia : BitVec 64) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rom1 : zrom pc = Equiv.Zisk.jalrAddInst i.rs1.toNat (i.imm.signExtend 64) (ia - pc)
  rom2 : zrom ia = Equiv.Zisk.jalrAndInst i.rd.toNat (pc + 4 - ia)

set_option maxHeartbeats 4000000 in
/-- jalr in the transpiler's two-instruction form: one sail-riscv step and
    two ZisK steps keep the states related, both at the target, when it is
    4-aligned. -/
theorem jo_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JrInstr) (ia : BitVec 64) (hi : JoAt s rs pc zrom i ia)
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (tal : (jalrTarget v1 i.imm)[0] = false ∧ (jalrTarget v1 i.imm)[1] = false) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ ZRun zrom 2 z z' ∧
      Rel s' z' rs ∧ s'.regs.get? Register.PC = some (jalrTarget v1 i.imm) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨-, -, -, -, -, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  obtain ⟨s', hrun, hmem, hr', hpc'', hxs'⟩ := rv_jalr_step s rs hr pc' hpc i hw v1 hv1 tal n
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have hxv : Equiv.Zisk.regv xs i.rs1.toNat = v1 := by
    have a1 := getX_regv agree l1; rw [hv1] at a1; exact (Option.some.inj a1).symm
  -- ZisK, step 1: the add, on to ia
  have hz1 := Equiv.Zisk.execute_jalr_add z i.rs1.toNat (i.imm.signExtend 64) (ia - pc') l1 xs pc' st
    hx hzpc hst
  rw [← hi.rom1, ← zstep_run zrom z pc' hzpc] at hz1
  -- ZisK, step 2: the and, from ia
  have eia : pc' + (ia - pc') = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
  have hz2 := Equiv.Zisk.execute_jalr_and { z with regs := (((z.regs.insert _root_.Zisk.Register.Zc
      (i.imm.signExtend 64 + Equiv.Zisk.regv xs i.rs1.toNat)).insert _root_.Zisk.Register.Zflag false).insert
      _root_.Zisk.Register.Step (BitVec.addInt st 1)).insert _root_.Zisk.Register.PC (pc' + (ia - pc')) }
    i.rd.toNat (pc' + 4 - ia) ld xs ia (BitVec.addInt st 1) (i.imm.signExtend 64 + Equiv.Zisk.regv xs i.rs1.toNat)
    (by simp [Std.ExtDHashMap.get?_insert, hx]) (by simp [eia])
    (by simp [Std.ExtDHashMap.get?_insert]) (by simp [Std.ExtDHashMap.get?_insert])
  rw [← hi.rom2, ← zstep_run zrom _ ia (by simp [eia])] at hz2
  refine ⟨s', _, hrun, .step hz1 (.step hz2 (.done _)), ⟨hr', ⟨Equiv.Zisk.xsUpd xs i.rd.toNat (pc' + 4), ?_, ?_⟩,
    ⟨_, hpc'', ?_⟩, ⟨BitVec.addInt (BitVec.addInt st 1) 1, by simp [Std.ExtDHashMap.get?_insert],
      by simp [Std.ExtDHashMap.get?_insert, hze]⟩, ?_⟩, hpc'', hmem⟩
  · have e4 : ia + (pc' + 4 - ia) = pc' + 4 := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
    simp only [Std.ExtDHashMap.get?_insert, beq_iff_eq, reduceCtorEq, dite_false, e4]
    · exact linkRegs_xs _ xs i.rd.toNat (pc' + 4) (by simp [Std.ExtDHashMap.get?_insert, hx])
  · intro k hk1 hk2; rw [hxs' k hk2]; exact regs_after0 s xs i.rd.toNat (pc' + 4) ld agree k hk1 hk2
  · simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
    rw [hxv]
    simp only [Equiv.Zisk.JALR_MASK, jalrTarget, Sail.BitVec.update, Sail.BitVec.updateSubrange']
    bv_decide
  · show z.mem = s'.mem
    rw [hmem, hzm]

/-- One step of sail-riscv, from s at its pc: an instruction proved, or a
    jalr in the transpiler's two-instruction form; ZisK takes one step or
    two. -/
def OneOk (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (s : RVState) : Prop :=
  ∃ pc, s.regs.get? Register.PC = some pc ∧
    ((∃ o, StepOk2 s rs pc zrom o) ∨
     (∃ i ia, JoAt s rs pc zrom i ia ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧
        (jalrTarget v1 i.imm)[0] = false ∧ (jalrTarget v1 i.imm)[1] = false))

theorem one_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (h : OneOk rs zrom s) (n : Nat) :
    ∃ s' z' k', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ ZRun zrom k' z z' ∧
      Rel s' z' rs := by
  obtain ⟨pc, hpc, ho | ⟨i, ia, hi, v1, hv1, t0, t1⟩⟩ := h
  · obtain ⟨o, ho⟩ := ho
    obtain ⟨s', z', h1, h2, h3⟩ := step_rel2 s z rs hrel pc hpc zrom o ho n
    exact ⟨s', z', 1, h1, .step h2 (.done _), h3⟩
  · obtain ⟨s', z', h1, h2, h3, -⟩ := jo_rel s z rs hrel pc hpc zrom i ia hi v1 hv1 ⟨t0, t1⟩ n
    exact ⟨s', z', 2, h1, h2, h3⟩

/-- For the next k steps of sail-riscv, from the state it reaches: each one
    an instruction proved (or a jalr in two ZisK instructions), or, two at a
    time, a fused auipc-jalr pair. -/
def RunOk3 (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) :
    Nat → RVState → Nat → Prop
  | 0, _, _ => True
  | 1, s, _ => OneOk rs zrom s
  | k + 2, s, n =>
    (OneOk rs zrom s ∧ RunOk3 rs zrom (k + 1) (rvNext n s) (n + 1)) ∨
      ((∃ pc i, s.regs.get? Register.PC = some pc ∧ FAt s rs pc zrom i) ∧
        RunOk3 rs zrom k (rvNext (n + 1) (rvNext n s)) (n + 2))

/-- The simulation over whole runs with fused pairs: k steps of sail-riscv's
    main loop from related states, and some number of ZisK steps, end
    related. -/
theorem prog_sim3 (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (rs : List PMA_Region) :
    ∀ (k : Nat) (s : RVState) (z : Equiv.Zisk.ZState) (n : Nat),
      Rel s z rs → RunOk3 rs zrom k s n →
      ∃ s' z' k', RVRun n k s s' ∧ ZRun zrom k' z z' ∧ Rel s' z' rs
  | 0, s, z, n, hrel, _ => ⟨s, z, 0, .done n s, .done z, hrel⟩
  | 1, s, z, n, hrel, ho => by
    obtain ⟨s1, z1, k1, hrun, hzrun, hrel1⟩ := one_step s z rs hrel zrom ho n
    exact ⟨s1, z1, k1, .step hrun (.done _ _), hzrun, hrel1⟩
  | k + 2, s, z, n, hrel, .inl ⟨ho, hrest⟩ => by
    obtain ⟨s1, z1, k1, hrun, hzrun, hrel1⟩ := one_step s z rs hrel zrom ho n
    rw [rvNext_of hrun] at hrest
    obtain ⟨s2, z2, k2, hrun2, hzrun2, hrel2⟩ := prog_sim3 zrom rs (k + 1) s1 z1 (n + 1) hrel1 hrest
    exact ⟨s2, z2, k1 + k2, .step hrun hrun2, ZRun.append hzrun hzrun2, hrel2⟩
  | k + 2, s, z, n, hrel, .inr ⟨⟨pc, i, hpc, hi⟩, hrest⟩ => by
    obtain ⟨s2, z1, hrun, hzrun, hrel1, -, -, -, -, -⟩ := fused_rel s z rs hrel pc hpc zrom i hi n
    have hnext : rvNext (n + 1) (rvNext n s) = s2 := by
      cases hrun with
      | step h1 r => cases r with
        | step h2 r2 => cases r2 with
          | done => rw [rvNext_of h1, rvNext_of h2]
    rw [hnext] at hrest
    obtain ⟨s3, z3, k3, hrun3, hzrun3, hrel3⟩ := prog_sim3 zrom rs k s2 z1 (n + 2) hrel1 hrest
    exact ⟨s3, z3, 1 + k3, by simpa [Nat.add_comm] using RVRun.append hrun hrun3, ZRun.append hzrun hzrun3,
      hrel3⟩

end Equiv.Sim
