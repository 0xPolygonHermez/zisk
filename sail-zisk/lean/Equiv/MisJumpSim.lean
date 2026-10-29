/-
  A jump whose target is 2 mod 4 — jal, jalr, a taken branch: sail-riscv
  traps on it (the compressed instructions off, an instruction address
  misaligned), writing nothing; ZisK jumps there, its target the same. The
  transpiler puts nothing at such an address (its RISC-V instructions on
  4-byte boundaries, its internal ones at odd addresses), and the model's
  rom_fetch asserts on an address with no instruction: ZisK's run fails at
  its next step.
-/
import Equiv.FaultSim
import Equiv.ProgSpec

namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 4000000 in
/-- A jump to a target 2 mod 4, the compressed instructions off: a trap
    (an instruction address misaligned), the state as it was. -/
theorem jump_to_mis (t : RVState) (h : WF t) (hp : t.regs.get? cur_privilege = some .Machine)
    (tgt : BitVec 64) (hmis : tgt[0] = false ∧ tgt[1] = true) :
    ∃ cp : ExceptionContext × BitVec 64,
      (interpretSailEffects (jump_to tgt)).run t = .ok (.Trap (.Machine, cp.1, cp.2)) t := by
  have h0 : (BitVec.access tgt 0 == 0#1) = true := by simp [BitVec.access, hmis.1]
  have h1 : bit_to_bool (BitVec.access tgt 1) = true := by simp [BitVec.access, bit_to_bool, bool_bit_backwards, hmis.2]
  simp only [jump_to, ext_control_check_pc, h0, h1, Bool.true_and, trap, exception_context]
  simp only [currentlyEnabled, hartSupports, privLevel_is_virtual, inVirtualPrivilege, LeanZiskRV.not]
  repeat rv_step h
  simp only [liftM, me_run_lift]
  repeat rv_step h
  exact ⟨(_, _), rfl⟩

/-- `jal` to a target 2 mod 4: a trap, nothing written. -/
theorem execute_jal_mis (t : RVState) (h : WF t) (hp : t.regs.get? cur_privilege = some .Machine)
    (rd : RegBits) (imm : BitVec 21) (pc : BitVec 64) (hpc : t.regs.get? PC = some pc)
    (hmis : (pc + imm.signExtend 64)[0] = false ∧ (pc + imm.signExtend 64)[1] = true) :
    ∃ cp : ExceptionContext × BitVec 64,
      (interpretSailEffects (execute_JAL imm (.Regidx rd))).run t = .ok (.Trap (.Machine, cp.1, cp.2)) t := by
  obtain ⟨cp, hj⟩ := jump_to_mis t h hp _ hmis
  refine ⟨cp, ?_⟩
  simp only [execute_JAL, get_next_pc, interp_bind, run_bind', sign_extend, Sail.BitVec.signExtend]
  rw [readReg_run t _ _ (WF.get? h nextPC)]; dsimp only
  rw [readReg_run t _ pc hpc]; dsimp only
  rw [hj]; rfl

/-- `jalr` to a target 2 mod 4, Zicfilp off: a trap, nothing written. -/
theorem execute_jalr_mis (t : RVState) (h : WF t) (hp : t.regs.get? cur_privilege = some .Machine)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (hmis : (jalrTarget v1 imm)[0] = false ∧ (jalrTarget v1 imm)[1] = true) :
    ∃ cp : ExceptionContext × BitVec 64,
      (interpretSailEffects (execute_JALR imm (.Regidx rs1) (.Regidx rd))).run t
        = .ok (.Trap (.Machine, cp.1, cp.2)) t := by
  obtain ⟨cp, hj⟩ := jump_to_mis t h hp _ hmis
  refine ⟨cp, ?_⟩
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  simp only [execute_JALR, update_elp_state, get_next_pc, rX_bits, toNatInt_toNat, interp_bind,
    run_bind', sign_extend, Sail.BitVec.signExtend]
  rw [ce_zicfilp t hp sec hsec]; dsimp only
  simp only [Bool.false_eq_true, if_false, interp_pure, run_pure']
  rw [readReg_run t _ _ (WF.get? h nextPC)]; dsimp only
  rw [r1]; dsimp only
  try (rw [run_pure']; dsimp only)
  have ej : BitVec.update (v1 + BitVec.signExtend 64 imm) 0 0#1 = jalrTarget v1 imm := rfl
  rw [ej, hj]; rfl

/-- A branch taken to a target 2 mod 4: a trap. -/
theorem execute_btype_mis (t : RVState) (h : WF t) (hp : t.regs.get? cur_privilege = some .Machine)
    (op : bop) (rs1 rs2 : RegBits) (imm : BitVec 13) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (pc : BitVec 64) (hpc : t.regs.get? PC = some pc) (htk : btypeTaken op v1 v2 = true)
    (hmis : (pc + imm.signExtend 64)[0] = false ∧ (pc + imm.signExtend 64)[1] = true) :
    ∃ cp : ExceptionContext × BitVec 64,
      (interpretSailEffects (execute_BTYPE imm (.Regidx rs2) (.Regidx rs1) op)).run t
        = .ok (.Trap (.Machine, cp.1, cp.2)) t := by
  obtain ⟨cp, hj⟩ := jump_to_mis t h hp _ hmis
  refine ⟨cp, ?_⟩
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  cases op <;>
  · simp only [execute_BTYPE, rX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
    rw [r1]; dsimp only; rw [r2]; dsimp only
    simp only [btypeTaken] at htk
    simp only [htk, if_true]
    rw [interp_bind, run_bind', readReg_run t _ pc hpc]; dsimp only
    exact hj

end Equiv.Riscv

namespace Equiv.Sim
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Riscv Equiv.Machine

/-- jal `i` at pc, its target any. -/
structure JAtM (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rom : zrom pc = Equiv.Zisk.jalInst i.rd.toNat (i.imm.signExtend 64)

/-- Branch `i` at pc, rs1, rs2 ≠ x0, its target any. -/
structure BAtM (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : BInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rs1 : i.rs1 ≠ 0
  rs2 : i.rs2 ≠ 0
  rom : zrom pc = Equiv.Zisk.branchInst (zbopOp i.op) i.rs1.toNat i.rs2.toNat i.zjumps.1 i.zjumps.2

/-- Branch \`i\` at pc, any registers (\`beqz\` and the like among them), its
    target any, the spec's translation in the ROM. -/
structure BtAtM (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : BInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

/-- A jump at pc to a target 2 mod 4, the ZisK instructions given: jal,
    jalr (the transpiler's one-instruction form, an even offset, or its
    two-instruction form), or a branch taken (rs1, rs2 ≠ x0, or any
    registers). -/
def MisJumpAtR (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (s : RVState) : Prop :=
  ∃ pc tgt, s.regs.get? Register.PC = some pc ∧ tgt[0] = false ∧ tgt[1] = true ∧
    ((∃ i : JInstr, JAtM s rs pc zrom i ∧ tgt = pc + i.imm.signExtend 64) ∨
     (∃ i : JrInstr, JrAt s rs pc zrom i ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧ tgt = jalrTarget v1 i.imm) ∨
     (∃ i : BInstr, BAtM s rs pc zrom i ∧ ∃ v1 v2, getX s i.rs1.toNat = some v1 ∧
        getX s i.rs2.toNat = some v2 ∧ btypeTaken i.op v1 v2 = true ∧ tgt = pc + i.imm.signExtend 64) ∨
     (∃ i : BInstr, ∃ next, BtAtM s rs pc zrom i next ∧ ∃ v1 v2, getX s i.rs1.toNat = some v1 ∧
        getX s i.rs2.toNat = some v2 ∧ btypeTaken i.op v1 v2 = true ∧ tgt = pc + i.imm.signExtend 64) ∨
     (∃ i : JrInstr, ∃ ia, JoAt s rs pc zrom i ia ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧
        tgt = jalrTarget v1 i.imm))

set_option maxHeartbeats 4000000 in
/-- The jump, one step of each: sail-riscv's to the trap vector, a plain
    word at an executable pc; sail-zisk's, one step or two, to the target. -/
theorem misjump_stepR (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (h : MisJumpAtR rs zrom s) (n : Nat) :
    ∃ pc tgt, s.regs.get? Register.PC = some pc ∧ WordOk s.mem rs pc ∧ tgt[0] = false ∧ tgt[1] = true ∧
      (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
        s'.regs.get? Register.PC = some 0x4#64) ∧
      ∃ k z', ZRun zrom k z z' ∧ z'.regs.get? _root_.Zisk.Register.PC = some tgt := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  obtain ⟨pc, tgt, hpc, ht0, ht1, ⟨i, hi, rfl⟩ | ⟨i, hi, v1, hv1, rfl⟩ | ⟨i, hi, v1, v2, hv1, hv2, htk, rfl⟩ |
    ⟨i, next, hi, v1, v2, hv1, hv2, htk, rfl⟩ | ⟨i, ia, hi, v1, hv1, rfl⟩⟩ := h
  all_goals have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  all_goals subst this
  · obtain ⟨hop, h20, h19, h11, h10, hrdx, hc⟩ := i.fields
    have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
      ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
    have ld : i.rd.toNat < 32 := i.rd.isLt
    refine ⟨pc', _, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_j i).plain⟩, ht0, ht1,
      try_step_trap s rs hr pc' hpc _ _ _ _ hw (.JAL (i.imm, .Regidx i.rd)) ?_ ?_ n, ?_⟩
    · intro t htp ⟨w, hw'⟩ _ _
      have := decode_jal t _ hop htp w hw'
      rw [h20, h19, h11, h10, hrdx] at this; exact this
    · intro t _ _ hmr _ hwf hpct
      rw [execute_jal_eq]
      exact execute_jal_mis t hwf hmr.priv i.rd i.imm pc' hpct ⟨ht0, ht1⟩
    · have hz := Equiv.Zisk.execute_jal z i.rd.toNat (i.imm.signExtend 64) ld xs pc' st hx hzpc hst
      rw [← hi.rom, ← zstep_run zrom z pc' hzpc] at hz
      exact ⟨1, _, .step hz (.done _), by simp⟩
  · obtain ⟨hop, hf3, himm, hr1x, hrdx, hc⟩ := i.fields
    have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
      ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
    have ld : i.rd.toNat < 32 := i.rd.isLt
    have l1 : i.rs1.toNat < 32 := i.rs1.isLt
    have hxv : Equiv.Zisk.regv xs i.rs1.toNat = v1 := by
      have a1 := getX_regv agree l1; rw [hv1] at a1; exact (Option.some.inj a1).symm
    obtain ⟨sv, hsv, -⟩ := hr.pmm
    refine ⟨pc', _, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_jr i).plain⟩, ht0, ht1,
      try_step_trap s rs hr pc' hpc _ _ _ _ hw (.JALR (i.imm, .Regidx i.rs1, .Regidx i.rd)) ?_ ?_ n, ?_⟩
    · intro t htp ⟨w, hw'⟩ _ _
      have := decode_jalr t _ hop hf3 htp w hw'
      rw [himm, hr1x, hrdx] at this; exact this
    · intro t he _ hmr hsec hwf _
      rw [execute_jalr_eq]
      exact execute_jalr_mis t hwf hmr.priv sv (hsec.trans hsv) i.rs1 i.rd i.imm v1 (getX_of_agree he l1 hv1)
        ⟨ht0, ht1⟩
    · have hz := Equiv.Zisk.execute_jalr z i.rd.toNat i.rs1.toNat (i.imm.signExtend 64) ld l1 xs pc' st hx hzpc hst
      rw [← hi.rom, ← zstep_run zrom z pc' hzpc] at hz
      exact ⟨1, _, .step hz (.done _), by simp [hxv, jalr_target v1 i.imm hi.even]⟩
  · obtain ⟨hop, hf3, h12, h11, h10, h4, hr2x, hr1x, hc⟩ := i.fields
    have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
      ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
    have l1 : i.rs1.toNat < 32 := i.rs1.isLt
    have l2 : i.rs2.toNat < 32 := i.rs2.isLt
    have e1 := ne_zero_toNat hi.rs1
    have e2 := ne_zero_toNat hi.rs2
    have a1 := agree i.rs1.toNat (by omega) l1
    have a2 := agree i.rs2.toNat (by omega) l2
    have hxv1 : xs[i.rs1.toNat]! = v1 := by rw [hv1] at a1; exact (Option.some.inj a1).symm
    have hxv2 : xs[i.rs2.toNat]! = v2 := by rw [hv2] at a2; exact (Option.some.inj a2).symm
    refine ⟨pc', _, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_br i).plain⟩, ht0, ht1,
      try_step_trap s rs hr pc' hpc _ _ _ _ hw (.BTYPE (i.imm, .Regidx i.rs2, .Regidx i.rs1, i.op)) ?_ ?_ n, ?_⟩
    · intro t htp ⟨w, hw'⟩ _ _
      have := decode_btype t _ i.op hop hf3 htp w hw'
      rw [h12, h11, h10, h4, hr2x, hr1x] at this; exact this
    · intro t he _ hmr _ hwf hpct
      rw [execute_btype_eq]
      exact execute_btype_mis t hwf hmr.priv i.op i.rs1 i.rs2 i.imm v1 v2 (getX_of_agree he l1 hv1)
        (getX_of_agree he l2 hv2) pc' hpct htk ⟨ht0, ht1⟩
    · have hxv1' : xs[(i.rs1.toNat : Int)]! = v1 := by rw [getElem!_int]; exact hxv1
      have hxv2' : xs[(i.rs2.toNat : Int)]! = v2 := by rw [getElem!_int]; exact hxv2
      have hz := Equiv.Zisk.execute_branch z (zbopOp i.op) (zbop_pure i.op) i.rs1.toNat i.rs2.toNat
        i.zjumps.1 i.zjumps.2 (by omega) (by omega) xs pc' st hx hzpc hst
      refine ⟨1, _, .step (by rw [zstep_run zrom z pc' hzpc, hi.rom]; exact hz) (.done _), ?_⟩
      simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
      have ht := zbop_flag i.op v1 v2
      rw [hxv1', hxv2']
      simp only [BInstr.zjumps]
      rw [htk] at ht
      cases hn : zbopNeg i.op <;> simp_all
  · obtain ⟨hop, hf3, h12, h11, h10, h4, hr2x, hr1x, hc⟩ := i.fields
    have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
      ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
    have l1 : i.rs1.toNat < 32 := i.rs1.isLt
    have l2 : i.rs2.toNat < 32 := i.rs2.isLt
    have hr1 : Equiv.Zisk.regv xs i.rs1.toNat = v1 := by
      have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
    have hr2 : Equiv.Zisk.regv xs i.rs2.toNat = v2 := by
      have := getX_regv agree l2; rw [hv2] at this; exact (Option.some.inj this).symm
    refine ⟨pc', _, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_br i).plain⟩, ht0, ht1,
      try_step_trap s rs hr pc' hpc _ _ _ _ hw (.BTYPE (i.imm, .Regidx i.rs2, .Regidx i.rs1, i.op)) ?_ ?_ n, ?_⟩
    · intro t htp ⟨w, hw'⟩ _ _
      have := decode_btype t _ i.op hop hf3 htp w hw'
      rw [h12, h11, h10, h4, hr2x, hr1x] at this; exact this
    · intro t he _ hmr _ hwf hpct
      rw [execute_btype_eq]
      exact execute_btype_mis t hwf hmr.priv i.op i.rs1 i.rs2 i.imm v1 v2 (getX_of_agree he l1 hv1)
        (getX_of_agree he l2 hv2) pc' hpct htk ⟨ht0, ht1⟩
    · have hz := Equiv.Zisk.execute_mkj z _ _ _ (zbop_pure i.op) 0 (by omega) i.zjumps.1 i.zjumps.2 xs pc' st
        hx hzpc hst _ _ (Equiv.Zisk.srcA_ASrc xs _ l1) (Equiv.Zisk.srcB_BSrc xs _ l2)
      refine ⟨1, _, .step (by rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec hi.tr]; exact hz) (.done _), ?_⟩
      simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
      have ht := zbop_flag i.op v1 v2
      rw [hr1, hr2]
      simp only [BInstr.zjumps]
      rw [htk] at ht
      cases hn : zbopNeg i.op <;> simp_all
  · obtain ⟨hop, hf3, himm, hr1x, hrdx, hc⟩ := i.fields
    have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
      ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
    have ld : i.rd.toNat < 32 := i.rd.isLt
    have l1 : i.rs1.toNat < 32 := i.rs1.isLt
    have hxv : Equiv.Zisk.regv xs i.rs1.toNat = v1 := by
      have a1 := getX_regv agree l1; rw [hv1] at a1; exact (Option.some.inj a1).symm
    obtain ⟨sv, hsv, -⟩ := hr.pmm
    refine ⟨pc', _, hpc, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_jr i).plain⟩, ht0, ht1,
      try_step_trap s rs hr pc' hpc _ _ _ _ hw (.JALR (i.imm, .Regidx i.rs1, .Regidx i.rd)) ?_ ?_ n, ?_⟩
    · intro t htp ⟨w, hw'⟩ _ _
      have := decode_jalr t _ hop hf3 htp w hw'
      rw [himm, hr1x, hrdx] at this; exact this
    · intro t he _ hmr hsec hwf _
      rw [execute_jalr_eq]
      exact execute_jalr_mis t hwf hmr.priv sv (hsec.trans hsv) i.rs1 i.rd i.imm v1 (getX_of_agree he l1 hv1)
        ⟨ht0, ht1⟩
    · -- ZisK, step 1: the add, on to ia; step 2: the and, from ia, to the target
      have hz1 := Equiv.Zisk.execute_jalr_add z i.rs1.toNat (i.imm.signExtend 64) (ia - pc') l1 xs pc' st
        hx hzpc hst
      rw [← hi.rom1, ← zstep_run zrom z pc' hzpc] at hz1
      have eia : pc' + (ia - pc') = ia := by rw [BitVec.add_comm]; exact BitVec.sub_add_cancel _ _
      have hz2 := Equiv.Zisk.execute_jalr_and { z with regs := (((z.regs.insert _root_.Zisk.Register.Zc
          (i.imm.signExtend 64 + Equiv.Zisk.regv xs i.rs1.toNat)).insert _root_.Zisk.Register.Zflag false).insert
          _root_.Zisk.Register.Step (BitVec.addInt st 1)).insert _root_.Zisk.Register.PC (pc' + (ia - pc')) }
        i.rd.toNat (pc' + 4 - ia) ld xs ia (BitVec.addInt st 1)
        (i.imm.signExtend 64 + Equiv.Zisk.regv xs i.rs1.toNat)
        (by simp [Std.ExtDHashMap.get?_insert, hx]) (by simp [eia])
        (by simp [Std.ExtDHashMap.get?_insert]) (by simp [Std.ExtDHashMap.get?_insert])
      rw [← hi.rom2, ← zstep_run zrom _ ia (by simp [eia])] at hz2
      refine ⟨2, _, .step hz1 (.step hz2 (.done _)), ?_⟩
      simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
      rw [hxv]
      simp only [Equiv.Zisk.JALR_MASK, jalrTarget, Sail.BitVec.update, Sail.BitVec.updateSubrange']
      bv_decide

/-- jal \`i\` at pc, its target any, the spec's translation in the ROM. -/
structure JtAtM (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : JInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

/-- A jump at pc to a target 2 mod 4, its ZisK instructions the spec's
    translation: jal, jalr (either offset), or a branch taken, any registers. -/
def MisJumpAt (rs : List PMA_Region) (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (s : RVState) : Prop :=
  ∃ pc tgt, s.regs.get? Register.PC = some pc ∧ tgt[0] = false ∧ tgt[1] = true ∧
    ((∃ i next, JtAtM s rs pc zrom i next ∧ tgt = pc + i.imm.signExtend 64) ∨
     (∃ i next, JrtAt s rs pc zrom i next ∧ ∃ v1, getX s i.rs1.toNat = some v1 ∧ tgt = jalrTarget v1 i.imm) ∨
     (∃ i next, BtAtM s rs pc zrom i next ∧ ∃ v1 v2, getX s i.rs1.toNat = some v1 ∧
        getX s i.rs2.toNat = some v2 ∧ btypeTaken i.op v1 v2 = true ∧ tgt = pc + i.imm.signExtend 64))

theorem MisJumpAt.toR {rs zrom s} (h : MisJumpAt rs zrom s) : MisJumpAtR rs zrom s := by
  obtain ⟨pc, tgt, hpc, ht0, ht1, ⟨i, next, hi, ht⟩ | ⟨i, next, hi, v1, hv1, ht⟩ | ⟨i, next, hi, hb⟩⟩ := h
  · exact ⟨pc, tgt, hpc, ht0, ht1, .inl ⟨i, ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3,
      i.rom_of_spec hi.tr⟩, ht⟩⟩
  · cases he : i.imm[0]
    · exact ⟨pc, tgt, hpc, ht0, ht1, .inr (.inl ⟨i, hi.toJrAt he, v1, hv1, ht⟩)⟩
    · obtain ⟨ia, hj⟩ := hi.toJoAt he
      exact ⟨pc, tgt, hpc, ht0, ht1, .inr (.inr (.inr (.inr ⟨i, ia, hj, v1, hv1, ht⟩)))⟩
  · exact ⟨pc, tgt, hpc, ht0, ht1, .inr (.inr (.inr (.inl ⟨i, next, hi, hb⟩)))⟩

/-- \`misjump_stepR\`, the ZisK instructions the spec's translation. -/
theorem misjump_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (h : MisJumpAt rs zrom s) (n : Nat) :
    ∃ pc tgt, s.regs.get? Register.PC = some pc ∧ WordOk s.mem rs pc ∧ tgt[0] = false ∧ tgt[1] = true ∧
      (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
        s'.regs.get? Register.PC = some 0x4#64) ∧
      ∃ k z', ZRun zrom k z z' ∧ z'.regs.get? _root_.Zisk.Register.PC = some tgt :=
  misjump_stepR s z rs hrel zrom h.toR n

/-- The fused pair at pc, its target any, its ZisK instruction from the spec. -/
structure FtAtM (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
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
  tr : Transpiled zrom pc i.auipc.encode (some i.jalr.encode)

set_option maxHeartbeats 4000000 in
/-- The fused pair to a target 2 mod 4: sail-riscv's auipc step, then its
    jalr step to the trap vector; sail-zisk's one step, to the target. -/
theorem fusedmis_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : FInstr) (hi : FtAtM s rs pc zrom i)
    (ht : (i.T pc)[0] = false ∧ (i.T pc)[1] = true) (n : Nat) :
    ∃ s1, (interpretSailEffects (try_step n false)).run s = .ok false s1 ∧ StepReady s1 rs ∧
      s1.regs.get? Register.PC = some (pc + 4) ∧ WordOk s.mem rs pc ∧ WordOk s1.mem rs (pc + 4) ∧
      (∃ s2, (interpretSailEffects (try_step (n + 1) false)).run s1 = .ok false s2 ∧
        s2.regs.get? Register.PC = some 0x4#64) ∧
      ∃ z', (zstep zrom).run z = .ok false z' ∧ z'.regs.get? _root_.Zisk.Register.PC = some (i.T pc) := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨-, -, -, hc1⟩ := UInstr.fields i.auipc
  obtain ⟨hop, hf3, himm, hr1x, hrdx, hc2⟩ := i.jalr.fields
  have hw1 : WordAt s rs pc' (byte i.auipc.encode 0) (byte i.auipc.encode 1) (byte i.auipc.encode 2)
      (byte i.auipc.encode 3) := ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc1⟩
  obtain ⟨s1, hrun1, hmem1, hr1, hpc1, hxs1⟩ := rv_auipc_step s rs hr pc' hpc i.rd i.uimm hw1 n
  have hal4 : (pc' + 4).toNat % 4 = 0 := by rw [BitVec.toNat_add]; simp; have := hi.al; omega
  have hw2 : WordAt s1 rs (pc' + 4) (byte i.jalr.encode 0) (byte i.jalr.encode 1) (byte i.jalr.encode 2)
      (byte i.jalr.encode 3) :=
    ⟨hal4, hi.x3, hi.x4, hmem1 ▸ hi.g0, hmem1 ▸ hi.g1, hmem1 ▸ hi.g2, hmem1 ▸ hi.g3, hc2⟩
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have ed := ne_zero_toNat hi.rd
  have hv1 : getX s1 i.jalr.rs1.toNat = some (i.A pc') := by
    show getX s1 i.rd.toNat = _
    rw [hxs1 _ ld, getX_setX _ _ _ _ ld ld, if_pos ⟨ed, rfl⟩]; rfl
  have l1 : i.jalr.rs1.toNat < 32 := i.jalr.rs1.isLt
  obtain ⟨sv, hsv, -⟩ := hr1.pmm
  have htrap := try_step_trap s1 rs hr1 (pc' + 4) hpc1 _ _ _ _ hw2 (.JALR (i.jalr.imm, .Regidx i.jalr.rs1,
      .Regidx i.jalr.rd)) (by
        intro t htp ⟨w, hw'⟩ _ _
        have := decode_jalr t _ hop hf3 htp w hw'
        rw [himm, hr1x, hrdx] at this; exact this)
      (by
        intro t he _ hmr hsec hwf _
        rw [execute_jalr_eq]
        exact execute_jalr_mis t hwf hmr.priv sv (hsec.trans hsv) i.jalr.rs1 i.jalr.rd i.jalr.imm (i.A pc')
          (getX_of_agree he l1 hv1) ht) (n + 1)
  have hz := Equiv.Zisk.execute_constj z i.rd.toNat (i.c pc') (i.T pc' - pc') (by omega) xs pc' st hx hzpc hst
  rw [← i.rom_of_spec hi.rd hi.link hi.tr, ← zstep_run zrom z pc' hzpc] at hz
  refine ⟨s1, hrun1, hr1, hpc1, ⟨hi.x1, _, readWord_of hi.h0 hi.h1 hi.h2 hi.h3, (notSys_u i.auipc).plain⟩,
    ⟨hi.x3, _, readWord_of (hmem1 ▸ hi.g0) (hmem1 ▸ hi.g1) (hmem1 ▸ hi.g2) (hmem1 ▸ hi.g3),
      (notSys_jr i.jalr).plain⟩, htrap, _, hz, ?_⟩
  simp only [Std.ExtDHashMap.get?_insert, beq_self_eq_true, dite_true, cast_eq, Option.some.injEq]
  generalize i.T pc' = y; bv_decide

end Equiv.Sim
