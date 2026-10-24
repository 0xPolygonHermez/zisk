/-
  transpiler_check: the transpiler's spec (Transpiler/Spec.lean) against the
  transpiler itself, on real programs.

    transpiler_check <listing>...

  Each listing is what `sail-difftest rom -e <elf>` writes: the ELF's code
  sections, and the ROM the Rust transpiler built from it, as ROM table
  entries (sail-zisk/model/zisk_rom_table.sail). The code is walked as
  riscv_interpreter.rs walks it; for each 32-bit instruction the spec
  describes, every ZisK instruction it gives must be in the ROM, at the same
  address, field for field — the proofs' hypothesis `SpecRom`
  (Equiv/SpecRom.lean), its code addresses the ones walked here and the
  next word the four bytes after. The internal address a translation continues
  at is the Rust ROM's (see the spec's header). Exits 1 on any difference.
-/
import Transpiler.Spec
import Transpiler.Bios

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
    | [""] => pure ()
    | _ => throw s!"bad line: {line}"
  return l

/-- The instructions of a code section, as riscv_interpreter.rs splits it:
    each address with its 32-bit word, or none for what the interpreter
    turns into a halt (a 16-bit parcel, the word 0xffffffff). -/
partial def parcels (base : Nat) (data : ByteArray) : Array (Nat × Option (BitVec 32)) :=
  go 0 #[]
where
  half (k : Nat) : Nat := data[k]!.toNat + 256 * data[k + 1]!.toNat
  go (k : Nat) (acc : Array (Nat × Option (BitVec 32))) : Array (Nat × Option (BitVec 32)) :=
    if k + 2 > data.size then acc
    else if half k % 4 != 3 then go (k + 2) (acc.push (base + k, none))
    else if k + 4 > data.size then acc
    else
      let w := half k + 65536 * half (k + 2)
      go (k + 4) (acc.push (base + k, if w == 0xffffffff then none else some (BitVec.ofNat 32 w)))

def hex (n : Nat) : String := String.ofList (Nat.toDigits 16 n)

structure Tally where
  described : Nat := 0
  undescribed : Nat := 0
  differences : Nat := 0

def check (name : String) (l : Listing) : IO Tally := do
  let mut t : Tally := {}
  for (base, data) in l.code do
    let ps := parcels base data
    for k in [0:ps.size] do
      let (pc, some w) := ps[k]! | continue
      -- The next word as `SpecRom` (Equiv/SpecRom.lean) reads it: the four
      -- bytes at pc + 4, whatever they are, when the section holds them.
      let o := pc - base
      let next : Option (BitVec 32) :=
        if o + 8 ≤ data.size then
          some (BitVec.ofNat 32 (data[o + 4]!.toNat + 256 * data[o + 5]!.toNat +
            65536 * data[o + 6]!.toNat + 16777216 * data[o + 7]!.toNat))
        else none
      -- The internal address the Rust's translation continues at, if any.
      let ia : BitVec 64 := match l.rom.get? pc with
        | some (some i) => BitVec.ofNat 64 pc + i.jmp_off1
        | _ => BitVec.ofNat 64 (pc + 1)
      match convert (BitVec.ofNat 64 pc) w next ia with
      | none => t := { t with undescribed := t.undescribed + 1 }
      | some insts =>
        t := { t with described := t.described + 1 }
        for (a, i) in insts do
          match l.rom.get? a.toNat with
          | some (some r) =>
            if r != i then
              t := { t with differences := t.differences + 1 }
              IO.println s!"{name}: {repr (decode w)} at {hex pc} \
                ({hex w.toNat}), at {hex a.toNat}:\
                \n  spec {repr i}\n  rust {repr r}"
          | some none =>
            t := { t with differences := t.differences + 1 }
            IO.println s!"{name}: at {hex a.toNat}, the Rust's instruction is one the model does not describe"
          | none =>
            t := { t with differences := t.differences + 1 }
            IO.println s!"{name}: at {hex a.toNat}, no instruction in the Rust's ROM"
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
               differences := total.differences + t.differences }
  IO.println s!"{args.length} programs: {total.described} instructions the spec describes, \
    {total.undescribed} it does not, {total.differences} differences"
  return if total.differences == 0 then 0 else 1
