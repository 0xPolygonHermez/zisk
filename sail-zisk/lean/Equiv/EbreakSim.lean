/-
  `ebreak` (the one word 0x00100073): a breakpoint. sail-riscv's step traps
  (Equiv/RiscvTrap.lean), so the machine's run fails; the transpiler's
  translation is the halt (`halt_with_error`), which ends sail-zisk's run
  in error.
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

end Equiv.Sim

