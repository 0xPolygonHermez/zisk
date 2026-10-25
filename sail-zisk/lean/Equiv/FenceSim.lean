/-
  The fences, `fence` (`fence.tso` and `pause` among them) and `fence.i`: on
  sail-riscv, in machine mode, a barrier, which the sequential interpreter
  makes nothing, and on to pc + 4; on sail-zisk the transpiler's nop. So one
  step of each keeps the states related.
-/
import Equiv.X0Sim
import Equiv.RiscvDecodeFence

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- A fence — `fence.i` when `fi` — by its fields. -/
structure FenceInstr where
  imm : BitVec 12
  rs1 : BitVec 5
  fi : Bool
  rd : BitVec 5

def FenceInstr.f3 (i : FenceInstr) : BitVec 3 := if i.fi then 1#3 else 0#3

def FenceInstr.encode (i : FenceInstr) : BitVec 32 := i.imm ++ i.rs1 ++ i.f3 ++ i.rd ++ 0x0F#7

theorem FenceInstr.fields (i : FenceInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x0F#7 ∧ v.extractLsb 14 12 = i.f3 ∧ v = i.encode ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, FenceInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_⟩ <;> bv_decide

/-! ## sail-riscv: a barrier, nothing -/

/-- A barrier, to the sequential interpreter: nothing. -/
theorem barrier_run (t : RVState) (b : Arch.barrier) :
    (interpretSailEffects (Sail.ArchSem.PreSail.sail_barrier b : SailM Unit)).run t = .ok () t := rfl

theorem execute_fence_run (t : RVState) (hp : t.regs.get? Register.cur_privilege = some .Machine)
    (fm pred succ : BitVec 4) (rs rd : regidx) :
    (interpretSailEffects (execute (.FENCE (fm, pred, succ, rs, rd)))).run t = .ok RETIRE_SUCCESS t := by
  have e : execute (.FENCE (fm, pred, succ, rs, rd)) = execute_FENCE fm pred succ rs rd := by simp [execute]
  rw [e]
  simp only [execute_FENCE, is_fiom_active, interp_bind, run_bind', readReg_run t _ _ hp]
  try dsimp only
  simp only [interp_pure, run_pure']
  try dsimp only
  split <;> simp only [interp_bind, run_bind', barrier_run, interp_pure, run_pure', LeanZiskRV.ArchSem.sail_barrier]

theorem execute_fence_tso_run (t : RVState) :
    (interpretSailEffects (execute (.FENCE_TSO ()))).run t = .ok RETIRE_SUCCESS t := by
  have e : execute (.FENCE_TSO ()) = execute_FENCE_TSO () := by simp [execute]
  rw [e]
  simp only [execute_FENCE_TSO, interp_bind, run_bind', barrier_run, interp_pure, run_pure', LeanZiskRV.ArchSem.sail_barrier]

theorem execute_fencei_run (t : RVState) (f : BitVec 12) (rs rd : regidx) :
    (interpretSailEffects (execute (.FENCEI (f, rs, rd)))).run t = .ok RETIRE_SUCCESS t := by
  have e : execute (.FENCEI (f, rs, rd)) = execute_FENCEI f rs rd := by simp [execute]
  rw [e]
  simp only [execute_FENCEI, interp_bind, run_bind', barrier_run, interp_pure, run_pure', LeanZiskRV.ArchSem.sail_barrier]

/-! ## The step -/

/-- A fence at pc: its word in executable memory, and the spec's translation,
    the nop, in the ZisK program. -/
structure FenceAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : FenceInstr) (next : Option (BitVec 32)) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  tr : Transpiled zrom pc i.encode next

theorem FenceInstr.rom_of_spec (i : FenceInstr) {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64}
    {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) : zrom pc = Transpiler.nop := by
  have ho : Transpiler.opcode i.encode = 0x0F#7 := by simp only [Transpiler.opcode, FenceInstr.encode]; bv_decide
  have hf : Transpiler.funct3 i.encode = i.f3 := by simp only [Transpiler.funct3, FenceInstr.encode]; bv_decide
  have hd : Transpiler.decode i.encode = some .fence := by
    cases hfi : i.fi <;> simp [Transpiler.decode, ho, hf, FenceInstr.f3, hfi]
  refine h.one fun ia => ?_
  simp [Transpiler.convert, hd]

set_option maxHeartbeats 4000000 in
/-- One step of each on a fence keeps the states related. -/
theorem fstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : FenceInstr) (next : Option (BitVec 32))
    (hi : FenceAt s rs pc zrom i next) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧ s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, -, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have hrom := i.rom_of_spec hi.tr
  -- ZisK: the nop, nothing but pc + 4
  have hz : ∃ z', (zstep zrom).run z = .ok false z' ∧
      z'.regs.get? _root_.Zisk.Register.Xs = some (Equiv.Zisk.xsUpd xs 0 0) ∧
      z'.regs.get? _root_.Zisk.Register.PC = some (pc' + 4) ∧
      (∃ st, z'.regs.get? _root_.Zisk.Register.Step = some st ∧
        z'.regs.get? _root_.Zisk.Register.Zerror = some false) ∧ z'.mem = z.mem := by
    have hn : Transpiler.nop =
        Transpiler.mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_FLAG ()) (Transpiler.storeReg 0) 4 4 := by
      simp [Transpiler.nop, storeReg_zero]
    rw [zstep_run zrom z pc' hzpc, hrom, hn]
    refine ⟨_, Equiv.Zisk.execute_mk z (.A_IMM, 0) (.B_IMM, 0) (.ZOP_FLAG ()) Equiv.Zisk.flag_pure 0 (by omega)
      xs pc' st hx hzpc hst 0 0 (Or.inl rfl) (Or.inl rfl), ?_, by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩
    simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs, Equiv.Zisk.xsUpd, hx]
  have hx0 : ∀ t : RVState, t.regs.get? Register.nextPC = some (pc' + 4) →
      setX { t with regs := t.regs.insert Register.nextPC (pc' + 4) } 0 0 = t := by
    intro t hnext; simp only [setX, if_true]; exact insert_same t _ _ hnext
  cases hfi : i.fi
  · -- a fence: fence.tso's word, or any other
    have hf0 : (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 14 12 = 0#3 := by
      rw [hf3]; simp [FenceInstr.f3, hfi]
    by_cases htso :
        @Eq (BitVec 32) (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)) 0x8330000F#32
    · refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw (.FENCE_TSO ()) ?_ 0 (by omega) 0 (pc' + 4)
        ?_ zrom hz n
      · intro t htp ⟨w, hw'⟩ _ _; rw [htso]; exact decode_fence_tso t htp w hw'
      · intro t _ _ _ _ _ hnext _; rw [execute_fence_tso_run, hx0 t hnext]
    · refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
        (.FENCE ((byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 31 28, (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 27 24,
          (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 23 20, .Regidx ((byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 19 15),
          .Regidx ((byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 11 7)))
        ?_ 0 (by omega) 0 (pc' + 4) ?_ zrom hz n
      · intro t htp ⟨w, hw'⟩ _ _; exact decode_fence t _ hop hf0 htso htp w hw'
      · intro t _ _ hmr _ _ hnext _; rw [execute_fence_run t hmr.priv, hx0 t hnext]
  · -- fence.i
    have hf1 : (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 14 12 = 1#3 := by
      rw [hf3]; simp [FenceInstr.f3, hfi]
    refine jump_step_rel s z rs hr xs agree hzm pc' hpc _ _ _ _ hw
      (.FENCEI ((byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 31 20, .Regidx ((byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 19 15),
        .Regidx ((byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 11 7)))
      ?_ 0 (by omega) 0 (pc' + 4) ?_ zrom hz n
    · intro t htp ⟨w, hw'⟩ _ _; exact decode_fencei t _ hop hf1 htp w hw'
    · intro t _ _ _ _ _ hnext _; rw [execute_fencei_run, hx0 t hnext]

end Equiv.Sim
