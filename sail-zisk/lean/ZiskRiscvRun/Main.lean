/-
  zisk_riscv_run: run a RISC-V program on ZisK's RISC-V machine
  (doc/riscv_contract.md), sail-riscv's executable Lean in ZisK's profile
  with sail-zisk's operations for the ZisK calls.

    zisk_riscv_run <elf> [-b <addr>,<file>]... [--dump <begin>,<end>] [--steps <n>]

  Memory starts with the ELF's segments, as ZisK loads them
  (ZiskRiscv/Elf.lean), then each `-b` file at its address:
  the same `-b` sections tools/sail-difftest writes for the C emulator
  (`mem.args`) — the input, the fcall tape — so both machines start from the
  same memory. It prints how the run ended, the output region and the dump
  range, in the format of sail-zisk's trace (model/main.sail):

    end <error> <code>
    out <k> <word>
    dump <address> <word>
-/
import ELFSage
import ZiskRiscvRun.Model
import ZiskRiscvRun.Registers
import ZiskRiscv.Elf

open LeanZiskRVExecutable
open LeanZiskRVExecutable.Functions
open Sail
open ZiskRiscv
open ZiskRiscvRun
open Register

/-- ZisK's start (contract § 3): the BIOS's work, done to the RISC-V state. -/
def ARCH_ID_CSR_ADDR : Nat := 0xA040_F890
def MTVEC_CELL : Nat := 0xA040_8000 + 0x305 * 8
def BIOS_HANDLER : BitVec 64 := 0x107C

def hex (v : BitVec n) (digits : Nat) : String :=
  let s := (Nat.toDigits 16 v.toNat).asString.toUpper
  "0x" ++ "".pushn '0' (digits - s.length) ++ s

def parseHex (s : String) : Option Nat :=
  let s := if s.startsWith "0x" || s.startsWith "0X" then s.drop 2 else s
  s.foldl (fun acc c => acc.bind fun n =>
    if c.isDigit then some (16 * n + (c.toNat - '0'.toNat))
    else if 'a' ≤ c.toLower && c.toLower ≤ 'f' then some (16 * n + (c.toLower.toNat - 'a'.toNat + 10))
    else none) (some 0)

def loadBytes (m : Mem) (addr : Nat) (bytes : ByteArray) : Mem := Id.run do
  let mut m := m
  for i in [0:bytes.size] do
    m := m.insert (addr + i) (bytes.get! i).toBitVec
  m

structure Args where
  elf : String := ""
  sections : List (Nat × String) := []
  dump : Option (Nat × Nat) := none
  steps : Nat := 10_000_000

partial def parseArgs : List String → Args → Except String Args
  | [], a => if a.elf == "" then .error "no ELF given" else .ok a
  | "-b" :: spec :: rest, a =>
    match spec.splitOn "," with
    | [addr, file] => match parseHex addr with
      | some n => parseArgs rest { a with sections := a.sections ++ [(n, file)] }
      | none => .error s!"bad address {addr}"
    | _ => .error s!"bad -b {spec}"
  | "--dump" :: spec :: rest, a =>
    match (spec.splitOn ",").map parseHex with
    | [some b, some e] => parseArgs rest { a with dump := some (b, e) }
    | _ => .error s!"bad --dump {spec}"
  | "--steps" :: n :: rest, a => parseArgs rest { a with steps := n.toNat! }
  | elf :: rest, a => parseArgs rest { a with elf }

def main (argv : List String) : IO UInt32 := do
  let args ← match parseArgs argv {} with
    | .ok a => pure a
    | .error e => IO.eprintln s!"zisk_riscv_run: {e}"; return 2
  let elf ← match mkRawELFFile? (← IO.FS.readBinFile args.elf) with
    | .ok (.elf64 elf) => pure elf
    | _ => IO.eprintln "zisk_riscv_run: not a 64-bit ELF"; return 2
  -- The ELF as ZisK loads it (ZiskRiscv/Elf.lean, what the theorem
  -- `zisk_elf` starts from): its PT_LOAD segments, checked as the Rust
  -- checks them, the .bss zero-filled; the input comes in the -b sections.
  let segs ← match ZiskRiscv.Elf.segments elf with
    | .ok segs => pure segs
    | .error e => IO.eprintln s!"zisk_riscv_run: {e}"; return 2
  let mut mem := ZiskRiscv.Elf.riscvMem segs ByteArray.empty
  for (addr, file) in args.sections do
    mem := loadBytes mem addr (← IO.FS.readBinFile file)
  -- The BIOS's CSR cells (§ 3), which loads reach too
  mem := writeDword mem ARCH_ID_CSR_ADDR 0x0FFF_EEEE
  mem := writeDword mem MTVEC_CELL BIOS_HANDLER
  let entry : BitVec 64 := (elf.file_header.e_entry : UInt64).toBitVec
  let setup : SailM Unit := do
    sail_model_init ()
    initializeRegisters
    init_model ""
    writeReg PC entry
    writeReg mtvec TRAP_VEC
    wX_bits (regidx 10) (BitVec.ofNat 64 INPUT_ADDR)
    wX_bits (regidx 11) (BitVec.ofNat 64 OUTPUT_ADDR)
    wX_bits (regidx 1) EXIT_ADDR
  let s0 : RVState := ⟨Std.ExtDHashMap.emptyWithCapacity, mem, (), 0, #[]⟩
  let rv ← match runM setup s0 with
    | .ok (_, rv) => pure rv
    | .error e => IO.eprintln s!"zisk_riscv_run: sail-riscv's set-up fails: {e.describe}"; return 2
  let s : State model := { rv, fcall := {} }
  match run args.steps s with
  | .error e => IO.eprintln s!"zisk_riscv_run: {e}"; return 2
  | .ok (s, status, n) =>
    IO.eprintln s!"{n} steps"
    match status with
    | .exit code => IO.println s!"end {if code == 0 then 0 else 1} {hex code 16}"
    | .failure why => IO.eprintln s!"failure: {why}"; IO.println s!"end 1 {hex (0 : BitVec 64) 16}"
    | .outside why => IO.eprintln s!"zisk_riscv_run: outside the contract: {why}"; return 3
    | .running => IO.eprintln s!"zisk_riscv_run: no end within {args.steps} steps"; return 2
    for (w, k) in (output s).zipIdx do
      IO.println s!"out {k} {hex w 16}"
    if let some (b, e) := args.dump then
      for a in List.range' b ((e - b) / 4) 4 do
        let w := (List.range 4).foldr (fun i acc =>
          acc ||| ((s.rv.mem.getD (a + i) 0).zeroExtend 32 <<< (8 * i))) (0 : BitVec 32)
        IO.println s!"dump {hex (BitVec.ofNat 64 a) 16} {hex w 8}"
    return 0
