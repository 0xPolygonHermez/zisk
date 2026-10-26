/-
  A ZisK call, one step of the machine (`ZiskRiscv.call`) and ZisK's steps
  on the spec's translation (`Transpiler.callInsts`), from related states
  to related states (`Equiv/CallSim.lean` for the operation).
-/
import Equiv.CallSim
import Equiv.Machine

set_option linter.unusedSimpArgs false

namespace Equiv.Machine

open Equiv.Riscv Equiv.Sim
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The machine's step on a ZisK call: not at the exit address, the call
    decoded at pc, its run. -/
theorem step_call {M : ZiskRiscv.RiscvModel} (S : ZiskRiscv.State M) (c : ZiskRiscv.Call) (S' : ZiskRiscv.State M)
    (hx : M.getPC S.rv ≠ M.exitAddr)
    (hd : ZiskRiscv.decode (ZiskRiscv.readWord (M.mem S.rv)) (M.getPC S.rv).toNat = .call c)
    (hc : ZiskRiscv.call S c = .ok S') :
    ZiskRiscv.step S = .ok (S', .running) := by
  have e : ZiskRiscv.tryStep S = .ok (S', .running) := by
    simp only [ZiskRiscv.tryStep, hd]
    simp [hx, bind, Except.bind, hc, pure, Except.pure]
  rw [ZiskRiscv.step, ZiskRiscv.step.go, e]

/-- The memory a call's operation starts from: its count first stored at
    EXTRA_PARAMS_ADDR, if it has one. -/
noncomputable def callMem (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) : Equiv.Mem :=
  match c.count with
  | some r => ZiskRiscv.writeDword (model.mem S.rv) ZiskRiscv.EXTRA_PARAMS_ADDR (model.getX S.rv r)
  | none => model.mem S.rv

/-- The machine's call, its operation's run given. -/
theorem call_run (S : ZiskRiscv.State model) (c : ZiskRiscv.Call) (cval : BitVec 64) (m' : Equiv.Mem)
    (f' : ZiskRiscv.Fcall)
    (hrun : ZiskRiscv.runOp c.op (S.argValue c.a) (S.argValue c.b) c.ext (callMem S c) S.fcall = .ok (cval, m', f')) :
    ZiskRiscv.call S c = .ok
      { rv := model.setPC (if c.rd != 0 then model.setX (model.setMem S.rv
                (ZiskRiscv.writeDword m' ZiskRiscv.INPUT_ADDR f'.freeInput)) c.rd cval
              else model.setMem S.rv (ZiskRiscv.writeDword m' ZiskRiscv.INPUT_ADDR f'.freeInput))
            (model.getPC S.rv + BitVec.ofNat 64 (4 * c.len)),
        fcall := f' } := by
  unfold ZiskRiscv.call
  simp only [bind, Except.bind]
  cases hc : c.count <;> simp only [callMem, hc] at hrun <;> simp only [hc, hrun] <;> rfl

end Equiv.Machine
