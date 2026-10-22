/-
  ZisK's RISC-V as a machine (doc/riscv_contract.md): sail-riscv's step, but
  at a ZisK call — the CSR sequences of §§ 7–8 — sail-zisk's operation, and
  the end of § 4.

  The machine is generic in the RISC-V model, `RiscvModel`: what it needs of
  sail-riscv is its state, its step, and access to the registers, the pc and
  the memory. Two instances (ZiskRiscv/Model*.lean): sail-riscv's Lean, for
  the proofs, and its executable Lean, to run programs.

  A ZisK call runs sail-zisk's own operation — `zisk_exec_mem`, or
  `zisk_exec` for a pure one — on a ZisK state whose memory is the RISC-V
  memory: both models keep memory as the same map of bytes, so the operation
  sees and writes the RISC-V program's memory itself. The fcall state (the
  tape, the current result, the free input) is ZisK's alone, and the machine
  keeps it beside the RISC-V state. The free input is also mirrored into the
  8 bytes at INPUT_ADDR, the input's read-only header, so that a RISC-V load
  there reads it, as on ZisK (contract § 2).
-/
import ZiskRiscv.Decode
import Zisk

namespace ZiskRiscv

open Sail

/-- Addresses the machine uses (core/src/mem.rs). -/
def INPUT_ADDR : Nat := 0x1000_0000
def OUTPUT_ADDR : Nat := 0xA041_0000
def EXTRA_PARAMS_ADDR : Nat := 0xA040_0F00

abbrev Mem := Std.ExtHashMap Nat (BitVec 8)

/-- ZisK's memory regions (contract § 2): the input, the ROM, the RAM. -/
def inRegions (addr : Nat) : Bool :=
  (0x1000_0000 ≤ addr && addr < 0x9000_0000) ||
  (0x9000_0000 ≤ addr && addr < 0x9800_0000) ||
  (0xA000_0000 ≤ addr && addr < 0xC000_0000)

/-- Why a model could not take a step. -/
inductive StepError where
  /-- It read a byte nobody wrote: memory a ZisK run starts as 0. -/
  | missing (addr : Nat)
  /-- An assertion failed: what ZisK refuses (sail-zisk's model asserts). -/
  | assertion (msg : String)
  | other (msg : String)

def StepError.describe : StepError → String
  | .missing a => s!"a read of address {a}, which nobody wrote"
  | .assertion m => s!"an assertion: {m}"
  | .other m => m

def StepError.ofSail : Sail.Error ue → StepError
  | .OutOfMemoryRange a => .missing a
  | .Assertion m => .assertion m
  | e => .other e.print

/-- What the machine needs of a RISC-V model. -/
structure RiscvModel where
  State : Type
  mem : State → Mem
  setMem : State → Mem → State
  /-- Register `r`, x0 reading 0. -/
  getX : State → Nat → BitVec 64
  setX : State → Nat → BitVec 64 → State
  getPC : State → BitVec 64
  setPC : State → BitVec 64 → State
  /-- One step of the RISC-V model, or why it could not take one. -/
  step : State → Except StepError State
  /-- Whether the step just taken trapped: a fault, an illegal instruction.
      On ZisK, each is a failure (§ 4). -/
  trapped : State → Bool
  /-- The address returning to which ends the run in success: the initial
      `ra` (§ 3). -/
  exitAddr : BitVec 64

/-- The fcall state: sail-zisk's FreeInput and Fcall* registers. -/
structure Fcall where
  freeInput : BitVec 64 := 0
  next : BitVec 64 := 0x0002_0000_0000_0000  -- FCALL_TAPE
  base : BitVec 64 := 0
  size : BitVec 64 := 0
  got : BitVec 64 := 0
deriving Repr

structure State (M : RiscvModel) where
  rv : M.State
  fcall : Fcall

/-- How a run stands. -/
inductive Status where
  | running
  /-- The exit system call, or a return to the initial ra (code 0): success
      when the code is 0, a failure with that code otherwise (§ 4). -/
  | exit (code : BitVec 64)
  /-- Any other failure (§ 4), with what failed. -/
  | failure (why : String)
  /-- The program left the contract: a precondition broken. -/
  | outside (why : String)
deriving Repr

def readWord (m : Mem) (addr : Nat) : Option (BitVec 32) := do
  let b0 ← m.get? addr; let b1 ← m.get? (addr + 1)
  let b2 ← m.get? (addr + 2); let b3 ← m.get? (addr + 3)
  pure (b3 ++ b2 ++ b1 ++ b0)

def writeDword (m : Mem) (addr : Nat) (v : BitVec 64) : Mem :=
  (List.range 8).foldl (fun m i => m.insert (addr + i) (v.extractLsb' (8 * i) 8)) m

def readDword (m : Mem) (addr : Nat) : BitVec 64 :=
  (List.range 8).foldr (fun i acc => acc ||| ((m.getD (addr + i) 0).zeroExtend 64 <<< (8 * i))) 0

/-- A ZisK state over memory `mem` with the fcall state `f`; every other
    register as the C emulator starts it, 0. -/
def ziskState (mem : Mem) (f : Fcall) :
    ConcurrencyInterfaceV1.SequentialState Zisk.RegisterType ConcurrencyInterfaceV1.trivialChoiceSource :=
  let regs : Std.ExtDHashMap Zisk.Register Zisk.RegisterType :=
    (((((((((((default : Std.ExtDHashMap Zisk.Register Zisk.RegisterType)
      |>.insert .FreeInput f.freeInput)
      |>.insert .FcallNext f.next)
      |>.insert .FcallBase f.base)
      |>.insert .FcallSize f.size)
      |>.insert .FcallGot f.got)
      |>.insert .Xs (Vector.replicate 40 0))
      |>.insert .Zerror_code 0)
      |>.insert .Zerror false)
      |>.insert .Step 0)
      |>.insert .Zflag false)
      |>.insert .Zc 0
  { regs, choiceState := (), mem, tags := (), cycleCount := 0, sailOutput := #[] }

/-- Run sail-zisk's operation `op` on `a`, `b` and `ext` over the memory and
    fcall state given: its `c`, and the memory and fcall state after it. The
    extended argument reaches the op as the emulator passes it: only to an op
    that reads memory as a precompile. -/
def runOp (op : Zisk.zisk_op) (a b ext : BitVec 64) (mem : Mem) (f : Fcall) :
    Except StepError (BitVec 64 × Mem × Fcall) :=
  let ext := if Zisk.Functions.zisk_op_is_precompiled op then ext else 0
  let prog : Zisk.SailM (BitVec 64) := do
    match ← Zisk.Functions.zisk_exec_mem op a b ext with
    | some (c, _) => pure c
    | none => pure (Zisk.Functions.zisk_exec op a b).1
  match prog.run (ziskState mem f) with
  | .ok c s =>
    let get (r : Zisk.Register) (h : Zisk.RegisterType r = BitVec 64) : BitVec 64 :=
      match s.regs.get? r with | some v => h ▸ v | none => 0
    .ok (c, s.mem, { freeInput := get .FreeInput rfl, next := get .FcallNext rfl,
                     base := get .FcallBase rfl, size := get .FcallSize rfl, got := get .FcallGot rfl })
  | .error e _ => .error (.ofSail e)

variable {M : RiscvModel}

def State.argValue (s : State M) : Arg → BitVec 64
  | .reg r => M.getX s.rv r
  | .imm v => v
  | .freeInput => s.fcall.freeInput

/-- A ZisK call (§§ 7–8): its count to EXTRA_PARAMS_ADDR first, if it has
    one; then the operation; `rd` gets its `c`; the free input is mirrored at
    INPUT_ADDR; and the program continues after the sequence. -/
def call (s : State M) (c : Call) : Except StepError (State M) := do
  let pc := M.getPC s.rv
  let a := s.argValue c.a
  let b := s.argValue c.b
  let mem := match c.count with
    | some r => writeDword (M.mem s.rv) EXTRA_PARAMS_ADDR (M.getX s.rv r)
    | none => M.mem s.rv
  let (cval, mem, fcall) ← runOp c.op a b c.ext mem s.fcall
  let mem := writeDword mem INPUT_ADDR fcall.freeInput
  let rv := M.setMem s.rv mem
  let rv := if c.rd != 0 then M.setX rv c.rd cval else rv
  let rv := M.setPC rv (pc + BitVec.ofNat 64 (4 * c.len))
  pure { rv, fcall }

/-- One step, on the memory as it stands: a byte it reads that nobody wrote
    is an error here, which `step` turns into a 0. -/
def tryStep (s : State M) : Except StepError (State M × Status) := do
  let pc := M.getPC s.rv
  if pc == M.exitAddr then
    return (s, .exit 0)  -- returned to the initial ra (§ 4)
  match decode (readWord (M.mem s.rv)) pc.toNat with
  | .call c => return (← call s c, .running)
  | .nop => return ({ s with rv := M.setPC s.rv (pc + 4) }, .running)
  | .malformed why => return (s, .failure s!"a ZisK call the transpiler refuses: {why}")
  | .ecall =>
    if M.getX s.rv 17 == 93 then return (s, .exit (M.getX s.rv 10))
    else return (s, .outside "an ecall other than exit (precondition E)")
  | .riscv =>
    let rv ← M.step s.rv
    if M.trapped rv then return ({ s with rv }, .failure "a trap: a fault or an illegal instruction")
    return ({ s with rv }, .running)

/-- One step of ZisK's RISC-V. Memory inside ZisK's regions that nobody
    wrote reads as 0, as a ZisK run starts it: a step that reads such a byte
    is retried with the byte 0. Each step is a function of the state, so the
    retry is that step on that memory. A read outside the regions, or an
    assertion of sail-zisk's operation, is a failure. -/
def step (s : State M) : Except String (State M × Status) :=
  go 4096 s
where
  go : Nat → State M → Except String (State M × Status)
    | 0, _ => .error "a step reads more than 4096 bytes nobody wrote"
    | fuel + 1, s =>
      match tryStep s with
      | .ok r => .ok r
      | .error (.missing a) =>
        if inRegions a then go fuel { s with rv := M.setMem s.rv ((M.mem s.rv).insert a 0) }
        else .ok (s, .failure s!"a read outside ZisK's memory map, at {a}")
      | .error (.assertion m) => .ok (s, .failure m)
      | .error (.other m) => .error m

/-- Run up to `fuel` steps: the state reached and how the run stands, with
    the number of steps taken. -/
def run (fuel : Nat) (s : State M) : Except String (State M × Status × Nat) :=
  go fuel 0 s
where
  go : Nat → Nat → State M → Except String (State M × Status × Nat)
    | 0, n, s => pure (s, .running, n)
    | fuel + 1, n, s => do
      let (s', st) ← step s
      match st with
      | .running => go fuel (n + 1) s'
      | st => pure (s', st, n)

/-- The output a run that exits with code 0 publishes: the 32 64-bit words at OUTPUT_ADDR (§ 4). -/
def output (s : State M) : List (BitVec 64) :=
  (List.range 32).map fun k => readDword (M.mem s.rv) (OUTPUT_ADDR + 8 * k)

end ZiskRiscv
