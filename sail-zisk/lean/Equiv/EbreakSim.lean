/-
  `ebreak` (the one word 0x00100073), a breakpoint, and `unimp`
  (`csrrw x0, cycle, x0`, 0xC0001073), a write to a read-only CSR:
  sail-riscv's step traps (Equiv/RiscvTrap.lean: the breakpoint's trap,
  the illegal instruction's), so the machine's run fails; the transpiler's
  translation of both is the halt (`halt_with_error`), which ends
  sail-zisk's run in error.
-/
import Equiv.FaultSim
import Equiv.ZiskBios
import Equiv.RiscvDecodeCsr
import Equiv.MNext
import Equiv.ZbkbSim

namespace Equiv.Zisk

open _root_.Zisk _root_.Zisk.Functions Sail Sail.ConcurrencyInterfaceV1
open Register

set_option maxHeartbeats 4000000 in
/-- The spec's halt (`halt_with_error`): the run's end, in error. -/
theorem execute_halt0 (s : ZState) (st : BitVec 64) (hst : s.regs.get? Step = some st) :
    ∃ s', (zisk_execute Transpiler.halt).run s = .ok true s' ∧ s'.regs.get? Zerror = some true := by
  have hm' := halt_mem
  have hp := halt_notPre
  simp [zisk_execute, Transpiler.halt, Transpiler.mk, source_a, source_b, store_c, value_to_store,
    hm', hp, writeReg_run]
  repeat (first
    | (rw [readReg_run']; (try simp only [Std.ExtDHashMap.get?_insert]); simp [hst]))
  all_goals simp [Std.ExtDHashMap.get?_insert]

end Equiv.Zisk

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxRecDepth 100000 in
theorem acc_c00 (p : Privilege) (at' : CSRAccessType) :
    is_CSR_accessible 0xC00#12 p at' =
      (do pure ((← currentlyEnabled .Ext_Zicntr) && (← counter_enabled 0 p))) := rfl

set_option maxRecDepth 100000 in
theorem stateen_c00 (p : Privilege) (at' : CSRAccessType) :
    stateen_allows_CSR_access 0xC00#12 p at' = pure true := rfl

set_option maxRecDepth 100000 in
theorem virt_c00 (p : Privilege) (at' : CSRAccessType) :
    is_CSR_exception_virtual 0xC00#12 p at' = check_CSR 0xC00#12 .Supervisor at' := rfl

theorem zreg0 : (regidx.Regidx 0#5 == zreg) = true := rfl

set_option maxHeartbeats 0 in
set_option maxRecDepth 100000 in
/-- `unimp` (`csrrw x0, cycle, x0`) in machine mode: a write to a read-only
    CSR, an illegal instruction, the state as it was. -/
theorem execute_unimp (t : RVState) (h : WF t) (hp : t.regs.get? cur_privilege = some .Machine) :
    (interpretSailEffects (execute (.CSRReg (0xC00#12, .Regidx 0#5, .Regidx 0#5, .CSRRW)))).run t
      = .ok (.Illegal_Instruction ()) t := by
  simp only [execute, execute_CSRReg, doCSR, check_CSR_result, check_CSR, check_CSR_priv, check_CSR_access,
    privLevel_to_CSR_privbits, csr_access_type, rX_bits]
  have r0 : (interpretSailEffects (rX (regno.Regno (BitVec.toNatInt 0#5).toNat))).run t = .ok 0 t :=
    rX_run t _ (by decide) 0 (by simp [getX, BitVec.toNatInt])
  simp only [zreg0, acc_c00, virt_c00, stateen_c00, check_CSR, check_CSR_priv, check_CSR_access,
    privLevel_to_CSR_privbits, currentlyEnabled, hartSupports, counter_enabled, feature_enabled_for_priv]
  cases hb : (BitVec.access (regVal h mcounteren) 0 == 1#1) <;>
  · repeat (first | (erw [r0]; try dsimp only) | rv_step h | (simp only [hb]; try dsimp only))


end Equiv.Riscv

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- `ebreak`'s word. -/
def EBREAK : BitVec 32 := 0x00100073

set_option maxHeartbeats 4000000 in
/-- `ebreak` in machine mode: a trap, the state as it was. -/
theorem execute_ebreak (t : RVState) (h : WF t) (hp : t.regs.get? Register.cur_privilege = some .Machine) :
    ∃ cp : ExceptionContext × BitVec 64,
      (interpretSailEffects (execute (.EBREAK ()))).run t = .ok (.Trap (.Machine, cp.1, cp.2)) t := by
  simp only [execute, execute_EBREAK, trap, exception_context, inVirtualPrivilege]
  repeat rv_step h
  exact ⟨(_, _), rfl⟩

/-- The machine leaves `ebreak` to sail-riscv. -/
theorem ebreak_plain : Equiv.Machine.Plain EBREAK := by
  intro fetch a hf
  simp only [ZiskRiscv.decode, hf]
  rfl

set_option maxHeartbeats 4000000 in
/-- `ebreak` at pc, one step of each: sail-riscv's to the trap vector;
    sail-zisk's on the halt, the run's end with Zerror set. -/
theorem ebreak_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom EBREAK next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  obtain ⟨hr, -, ⟨pc', hpc', hzpc⟩, ⟨st, hst, -⟩, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hw : WordAt s rs pc' (byte EBREAK 0) (byte EBREAK 1) (byte EBREAK 2) (byte EBREAK 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, by decide⟩
  refine ⟨try_step_trap s rs hr pc' hpc _ _ _ _ hw (.EBREAK ()) ?_ ?_ n, ?_⟩
  · intro t htp ⟨w, hw'⟩ _ _
    exact decode_ebreak t _ (by decide) htp w hw'
  · intro t _ _ hmr _ hwf
    exact execute_ebreak t hwf hmr.priv
  · have hrom : zrom pc' = Transpiler.halt := hi.tr.one fun _ => rfl
    rw [zstep_run zrom z pc' hzpc, hrom]
    exact Equiv.Zisk.execute_halt0 z st hst

/-- `unimp`'s word. -/
def UNIMP : BitVec 32 := 0xC0001073

/-- The machine leaves `unimp` to sail-riscv. -/
theorem unimp_plain : Equiv.Machine.Plain UNIMP := by
  intro fetch a hf
  simp only [ZiskRiscv.decode, hf]
  rfl

/-- The spec's translation of `unimp`: the halt. -/
theorem unimp_spec (pc ia : BitVec 64) (next next2 : Option (BitVec 32)) :
    Transpiler.convert pc UNIMP next ia next2 = some [(pc, Transpiler.halt)] := by
  have hd : ZiskRiscv.decode (Transpiler.fetch2 pc UNIMP next) pc.toNat = .riscv := by
    simp only [ZiskRiscv.decode, Transpiler.fetch2, if_true]; rfl
  cases next2 <;> simp only [Transpiler.convert, Transpiler.csr3, Transpiler.csr, hd] <;> rfl

set_option maxHeartbeats 4000000 in
/-- `unimp` at pc, one step of each: sail-riscv's to the trap vector;
    sail-zisk's on the halt, the run's end with Zerror set. -/
theorem unimp_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom UNIMP next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  obtain ⟨hr, -, ⟨pc', hpc', hzpc⟩, ⟨st, hst, -⟩, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hw : WordAt s rs pc' (byte UNIMP 0) (byte UNIMP 1) (byte UNIMP 2) (byte UNIMP 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, by decide⟩
  refine ⟨try_step_illegal s rs hr hr.mtvec pc' hpc _ _ _ _ hw (.CSRReg (0xC00#12, .Regidx 0#5, .Regidx 0#5, .CSRRW))
    ?_ ?_ n, ?_⟩
  · intro t htp ⟨w, hw'⟩ _ _
    exact decode_unimp t _ (by decide) htp w hw'
  · intro t _ _ hmr _ hwf
    exact execute_unimp t hwf hmr.priv
  · have hrom : zrom pc' = Transpiler.halt := hi.tr.one fun ia => unimp_spec pc' ia _ _
    rw [zstep_run zrom z pc' hzpc, hrom]
    exact Equiv.Zisk.execute_halt0 z st hst

/-- A word the transpiler halts on, and the simulation covers: `ebreak` or
    `unimp`. -/
def HaltWord (e : BitVec 32) : Prop := e = EBREAK ∨ e = UNIMP

theorem HaltWord.plain {e : BitVec 32} (h : HaltWord e) : Equiv.Machine.Plain e := by
  rcases h with rfl | rfl
  · exact ebreak_plain
  · exact unimp_plain

theorem halt_step (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (next : Option (BitVec 32)) (e : BitVec 32) (he : HaltWord e)
    (hi : WordsAt s rs pc zrom e next) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧
    ∃ z', (zstep zrom).run z = .ok true z' ∧ z'.regs.get? _root_.Zisk.Register.Zerror = some true := by
  rcases he with rfl | rfl
  · exact ebreak_step s z rs hrel pc hpc zrom next hi n
  · exact unimp_step s z rs hrel pc hpc zrom next hi n

end Equiv.Sim

