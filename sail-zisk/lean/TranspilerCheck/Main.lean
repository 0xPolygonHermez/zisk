/-
  transpiler_check: the transpiler's spec (Transpiler/Spec.lean) against the
  transpiler itself, on real programs.

    transpiler_check <listing>...

  Each listing is what `sail-difftest rom -e <elf>` writes: the ELF's code
  sections, and the ROM the Rust transpiler built from it, as ROM table
  entries (sail-zisk/model/zisk_rom_table.sail). The spec builds the
  program's ROM from the code (Transpiler/Rom.lean): every word at a 4-byte
  boundary translated, with the two words after it, internal addresses from
  the ROM's counter. That ROM and the Rust's must be the same, entry for
  entry, field for field, but where a word is one the spec does not
  describe; the BIOS too. And no instruction is at an address 2 mod 4,
  where a misaligned jump lands; and no word at a segment's end has a
  translation that reads past it (`tailOk`, what the theorems ask of the
  program; a program with such a word is reported, as outside the
  theorems, but is no difference). And, given the listing's `elf <path>`
  line (which `make transpiler-check` appends), the Lean loader
  (ZiskRiscv/Elf.lean, what `zisk_elf` starts from) reads that ELF to the
  same code segments as the Rust's. Exits 1 on any difference.
-/
import Transpiler.Spec
import Transpiler.Rom
import Transpiler.Bios
import ZiskRiscv.Elf

open _root_.Zisk Transpiler

def hexNat (s : String) : Option Nat :=
  s.foldl (fun acc c => acc.bind fun n =>
    if c.isDigit then some (16 * n + (c.toNat - '0'.toNat))
    else if 'a' ≤ c && c ≤ 'f' then some (16 * n + (c.toNat - 'a'.toNat + 10))
    else none) (some 0)

/-- A ROM table entry's first six words, as the model's rom_fetch reads them. -/
def decodeEntry (ws : Array Nat) : Option zisk_inst := do
  let w0 := ws[0]!
  let byte (k : Nat) := (w0 >>> (8 * k)) % 256
  let a ← [a_src.A_C, .A_MEM, .A_IMM, .A_STEP, .A_REG][byte 1]?
  let b ← [b_src.B_C, .B_MEM, .B_IMM, .B_IND, .B_REG][byte 2]?
  let st ← [c_store.ST_NONE, .ST_MEM, .ST_IND, .ST_REG][byte 3]?
  let op ← Functions.zisk_op_of_code (BitVec.ofNat 8 (byte 6))
  let bit (k : Nat) := (w0 >>> k) % 2 == 1
  pure { a_src := a, a_imm := .ofNat 64 ws[1]!, b_src := b, b_imm := .ofNat 64 ws[2]!,
         ind_width := (byte 4 : Int), op, store := st, store_off := .ofNat 64 ws[3]!,
         store_pc := bit 40, set_pc := bit 41, jmp_off1 := .ofNat 64 ws[4]!,
         jmp_off2 := .ofNat 64 ws[5]!, is_end := bit 42 }

structure Listing where
  code : Array (Nat × ByteArray) := #[]
  rom : Std.HashMap Nat (Option zisk_inst) := {}
  /-- The ELF the listing is of, when given: its code as the Lean loader
      reads it (ZiskRiscv/Elf.lean) checked against the Rust's. -/
  elf : Option String := none

def parseListing (text : String) : Except String Listing := do
  let mut l : Listing := {}
  for line in text.splitOn "\n" do
    match line.splitOn " " with
    | ["code", addr, bytes] =>
      let some a := hexNat addr | throw s!"bad line: {line}"
      let cs := bytes.toList.toArray
      let mut data := ByteArray.empty
      for k in [0:cs.size / 2] do
        let some b := hexNat (String.ofList [cs[2 * k]!, cs[2 * k + 1]!]) | throw s!"bad bytes at {addr}"
        data := data.push b.toUInt8
      l := { l with code := l.code.push (a, data) }
    | "inst" :: pc :: ws =>
      let some p := hexNat pc | throw s!"bad line: {line}"
      let ns := (ws.map hexNat).filterMap id
      if ns.length != 6 then throw s!"bad line: {line}"
      l := { l with rom := l.rom.insert p (decodeEntry ns.toArray) }
    | ["skip", pc] =>
      let some p := hexNat pc | throw s!"bad line: {line}"
      l := { l with rom := l.rom.insert p none }
    | ["elf", path] => l := { l with elf := some path }
    | [""] => pure ()
    | _ => throw s!"bad line: {line}"
  return l

def hex (n : Nat) : String := String.ofList (Nat.toDigits 16 n)

structure Tally where
  described : Nat := 0
  undescribed : Nat := 0
  differences : Nat := 0
  /-- Words at a segment's end whose translation reads past it (`tailOk`). -/
  tails : Nat := 0

def check (name : String) (l : Listing) : IO Tally := do
  let mut t : Tally := {}
  -- The program's ROM from the spec (Transpiler/Rom.lean): every word of the
  -- code translated, internal addresses from the counter, as the Rust
  -- builds it.
  let segs := l.code.toList.map fun (base, data) =>
    (BitVec.ofNat 64 base, (Array.range (data.size / 4)).map fun k =>
      BitVec.ofNat 32 (data[4 * k]!.toNat + 256 * data[4 * k + 1]!.toNat +
        65536 * data[4 * k + 2]!.toNat + 16777216 * data[4 * k + 3]!.toNat))
  -- The words the spec does not describe: they have no entry in it.
  let mut undesc : Std.HashSet Nat := {}
  for (base, ws) in segs do
    for k in [0:ws.size] do
      let pc := base + BitVec.ofNat 64 (4 * k)
      if (convert pc ws[k]! ws[k + 1]? 0 ws[k + 2]?).isSome then
        t := { t with described := t.described + 1 }
      else
        t := { t with undescribed := t.undescribed + 1 }
        undesc := undesc.insert pc.toNat
  let mut seen : Std.HashSet Nat := {}
  for (a, i) in programRom segs do
    if seen.contains a.toNat then
      t := { t with differences := t.differences + 1 }
      IO.println s!"{name}: two instructions at {hex a.toNat} in the spec's ROM"
    seen := seen.insert a.toNat
    match l.rom.get? a.toNat with
    | some (some r) =>
      if r != i then
        t := { t with differences := t.differences + 1 }
        IO.println s!"{name}: at {hex a.toNat}:\n  spec {repr i}\n  rust {repr r}"
    | some none =>
      t := { t with differences := t.differences + 1 }
      IO.println s!"{name}: at {hex a.toNat}, the Rust's instruction is one the model does not describe"
    | none =>
      t := { t with differences := t.differences + 1 }
      IO.println s!"{name}: at {hex a.toNat}, no instruction in the Rust's ROM"
  -- And the Rust's program ROM has nothing more, but at a word the spec
  -- does not describe.
  for (a, _) in l.rom do
    if 0x9000_0000 ≤ a && a < 0x97E0_0000 && !seen.contains a && !undesc.contains a then
      t := { t with differences := t.differences + 1 }
      IO.println s!"{name}: at {hex a}, an instruction the spec's ROM has not"
  -- The BIOS, for the entry the Rust's BIOS calls.
  if let some (some call) := l.rom.get? 0x1018 then
    for (a, i) in bios call.b_imm do
      t := { t with described := t.described + 1 }
      match l.rom.get? a.toNat with
      | some (some r) =>
        if r != i then
          t := { t with differences := t.differences + 1 }
          IO.println s!"{name}: BIOS at {hex a.toNat}:\n  spec {repr i}\n  rust {repr r}"
      | _ =>
        t := { t with differences := t.differences + 1 }
        IO.println s!"{name}: BIOS at {hex a.toNat}: no instruction in the Rust's ROM"
  -- What the theorems ask of the segments' ends (`ProgramOk.of_tail`): no
  -- translation there reads past the segment, where the transpiler reads
  -- nothing and memory holds whatever follows.
  for (base, ws) in segs do
    for k in [0:ws.size] do
      unless k + 2 < ws.size || (!usesNext2 ws[k]! && (k + 1 < ws.size || !usesNext ws[k]!)) do
        t := { t with tails := t.tails + 1 }
        IO.println s!"{name}: the word at {hex (base + BitVec.ofNat 64 (4 * k)).toNat}, at its segment's end, \
          reads past it ({hex ws[k]!.toNat}): the theorems do not cover this program (tailOk)"
  -- The code as the Lean loader reads the ELF (what the theorem `zisk_elf`
  -- takes): ELFSage's parse, the Rust's checks, the same executable segments
  -- as the Rust's, byte for byte.
  if let some path := l.elf then
    match mkRawELFFile? (← IO.FS.readBinFile path) with
    | .ok (.elf64 elf) =>
      match ZiskRiscv.Elf.segments elf with
      | .ok segs =>
        let mine := (segs.filter (·.exec)).map fun s => (s.base, s.data.toList)
        let rust := l.code.toList.map fun (a, d) => (a, d.toList)
        if mine != rust then
          t := { t with differences := t.differences + 1 }
          IO.println s!"{name}: the Lean loader's code segments are not the Rust's"
      | .error e =>
        t := { t with differences := t.differences + 1 }
        IO.println s!"{name}: the Lean loader rejects {path}: {e}"
    | _ =>
      t := { t with differences := t.differences + 1 }
      IO.println s!"{name}: ELFSage does not read {path} as a 64-bit ELF"
  -- No instruction at an address 2 mod 4: a jump there, which sail-riscv
  -- traps on, finds none on ZisK either (Equiv/MisJumpSim.lean).
  for (a, _) in l.rom do
    if a % 4 == 2 then
      t := { t with differences := t.differences + 1 }
      IO.println s!"{name}: an instruction at {hex a}, an address 2 mod 4"
  return t

def main (args : List String) : IO UInt32 := do
  if args.isEmpty then
    IO.eprintln "usage: transpiler_check <listing>..."
    return 2
  let mut total : Tally := {}
  for file in args do
    let l ← match parseListing (← IO.FS.readFile file) with
      | .ok l => pure l
      | .error e => IO.eprintln s!"{file}: {e}"; return 2
    let t ← check file l
    total := { described := total.described + t.described, undescribed := total.undescribed + t.undescribed,
               differences := total.differences + t.differences, tails := total.tails + t.tails }
  IO.println s!"{args.length} programs: {total.described} instructions the spec describes, \
    {total.undescribed} it does not, {total.differences} differences, \
    {total.tails} words at a segment's end that read past it (outside the theorems)"
  return if total.differences == 0 then 0 else 1
