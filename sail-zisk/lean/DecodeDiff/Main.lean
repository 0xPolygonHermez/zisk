/-
  decode_diff: sail-riscv's decoder, in ZisK's profile, on every word of a
  set the transpiler's decoder can tell apart, for comparing the two
  (`sail-difftest decode-diff`, `make decoder-diff`).

    decode_diff <out.bin> <names.txt>

  The words: every opcode with its low bits 11, every funct3 and every
  value of bits 31:20, with rd and rs1 each 0, 1 or 5 — what
  riscv_decoder.rs looks at, and whether a register is x0. In that order:
  opcode, funct3, bits 31:20, rd, rs1, the last varying fastest. For each,
  out.bin gets two bytes, little-endian: 0 if sail-riscv's decoder rejects
  it (ILLEGAL), 0xffff if it cannot tell, else 1 + the index in names.txt
  of the instruction it decodes to.

  Cannot tell: the executable model has only some of sail-riscv's modules
  (ZisK's profile's among them, see `make riscv-run`), and its decoder,
  asked whether an extension of a module it lacks is enabled, fails
  (`currentlyEnabled` has no clause for it) rather than answer. Such an
  extension is outside ZisK's profile; the report says where these words
  are, for a reader to confirm they are nothing the profile decodes.
-/
import ZiskRiscvRun.Registers

open LeanZiskRVExecutable LeanZiskRVExecutable.Functions Sail Sail.ArchSem

abbrev RVState := Sail.ArchSem.SequentialState RegisterType

def REGS : List Nat := [0, 1, 5]

/-- The words, in the order out.bin lists them. -/
def words : Array (BitVec 32) := Id.run do
  let mut ws := #[]
  for o in [0:32] do
    for f3 in [0:8] do
      for hi in [0:4096] do
        for rd in REGS do
          for rs1 in REGS do
            ws := ws.push (BitVec.ofNat 32 ((hi <<< 20) ||| (rs1 <<< 15) ||| (f3 <<< 12) ||| (rd <<< 7) ||| ((o <<< 2) ||| 3)))
  return ws

/-- The constructor's name in `repr`. -/
def ctorName (i : instruction) : String :=
  let cs := ((repr i).pretty 1000).toList.dropWhile (· == '(')
  let name := String.ofList (cs.takeWhile (fun c => c != ' ' && c != '\n'))
  name.replace "LeanZiskRVExecutable.instruction." ""

def main (argv : List String) : IO UInt32 := do
  let [out, namesFile] := argv | IO.eprintln "usage: decode_diff <out.bin> <names.txt>"; return 2
  let setup : SailM Unit := do
    sail_model_init ()
    ZiskRiscvRun.initializeRegisters
    init_model ""
  let s0 : RVState := ⟨Std.ExtDHashMap.emptyWithCapacity, {}, (), 0, #[]⟩
  let s ← match (interpretSailEffects setup).run s0 with
    | .ok _ s => pure s
    | .error _ _ => IO.eprintln "decode_diff: sail-riscv's set-up fails"; return 2
  let mut names : Array String := #[]
  let mut index : Std.HashMap String Nat := {}
  let mut bytes := ByteArray.emptyWithCapacity (2 * words.size)
  for w in words do
    let code ← match (interpretSailEffects (ext_decode w)).run s with
      | .ok (.ILLEGAL _) _ => pure 0
      | .ok i _ =>
        let n := ctorName i
        match index.get? n with
        | some k => pure (k + 1)
        | none =>
          index := index.insert n names.size
          names := names.push n
          pure names.size
      | .error (.Assertion m) _ =>
        if m.startsWith "Pattern match failure at mops/Zimop/zimop_insts.sail" then pure 0xffff
        else IO.eprintln s!"decode_diff: the decoder fails on {w}: {m}"; return 2
      | .error e _ => IO.eprintln s!"decode_diff: the decoder fails on {w}: {e.print}"; return 2
    bytes := (bytes.push (code % 256).toUInt8).push (code / 256).toUInt8
  IO.FS.writeBinFile out bytes
  IO.FS.writeFile namesFile (String.intercalate "\n" names.toList ++ "\n")
  IO.println s!"{words.size} words, {names.size} instructions"
  return 0
