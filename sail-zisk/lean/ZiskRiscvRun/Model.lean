/-
  ZisK's RISC-V machine on sail-riscv's executable Lean, in ZisK's profile:
  the `RiscvModel` instance that runs programs.

  The machine (ZiskRiscv/Machine.lean) asks the model for a step and for its
  registers, pc and memory; each is a computation of sail-riscv's own run on
  its state, through lean-sail's sequential interpreter, as sail-riscv's
  Lean emulator runs it.
-/
import LeanZiskRVExecutable
import ZiskRiscv.Machine

namespace ZiskRiscvRun

open LeanZiskRVExecutable
open LeanZiskRVExecutable.Functions
open Sail
open Sail.ArchSem
open ZiskRiscv
open Register

abbrev RVState := Sail.ArchSem.SequentialState RegisterType

/-- Where `mtvec` points at the start: outside ZisK's memory, so that a trap
    is seen as one. -/
def TRAP_VEC : BitVec 64 := 0x4

/-- The initial `ra`: the BIOS's return address (contract § 3). -/
def EXIT_ADDR : BitVec 64 := 0x101C

def runM (m : SailM α) (s : RVState) : Except StepError (α × RVState) :=
  match (Sail.ArchSem.interpretSailEffects m).run s with
  | .ok a s => .ok (a, s)
  | .error e _ => .error (.ofSail e)

def regidx (r : Nat) : regidx := .Regidx (BitVec.ofNat 5 r)

def model : RiscvModel where
  State := RVState
  mem s := s.mem
  setMem s m := { s with mem := m }
  getX s r :=
    if r == 0 then 0 else
    match runM (rX_bits (regidx r)) s with | .ok (v, _) => v | .error _ => 0
  setX s r v := match runM (wX_bits (regidx r) v) s with | .ok (_, s) => s | .error _ => s
  getPC s := match runM (readReg PC) s with | .ok (v, _) => v | .error _ => 0
  setPC s v := match runM (writeReg PC v) s with | .ok (_, s) => s | .error _ => s
  step s := do let (_, s) ← runM (try_step 0 true) s; pure s
  trapped s := match runM (readReg PC) s with | .ok (v, _) => v == TRAP_VEC | .error _ => false
  exitAddr := EXIT_ADDR

end ZiskRiscvRun
