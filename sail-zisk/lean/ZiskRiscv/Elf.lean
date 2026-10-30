/-
  A RISC-V ELF as ZisK loads it (transpilers/riscv/src/elf_extraction.rs):
  its `PT_LOAD` segments, checked as the Rust checks them, and the memory
  each side starts with. ELFSage parses the file; the checks, the code and
  the memories are the Rust's, read from the program headers.

  The two memories differ in one way: an executable segment's bytes are in
  RISC-V's memory always, as its loader writes every segment, but in ZisK's
  only when the segment is also readable (`PF_X | PF_R`). An execute-only
  segment's bytes (`xOnly`) are code to ZisK, and nothing in its memory
  (contract § 2, precondition X).
-/
import ELFSage
import ZiskRiscv.Machine

namespace ZiskRiscv.Elf

def PT_LOAD : Nat := 1
def PF_X : Nat := 1
def PF_W : Nat := 2
def PF_R : Nat := 4
def EM_RISCV : Nat := 243
def ET_EXEC : Nat := 2

def ROM_ADDR : Nat := 0x9000_0000
def ROM_SIZE : Nat := 0x0800_0000
def RAM_ADDR : Nat := 0xA000_0000
def RAM_SIZE : Nat := 0x2000_0000
def INPUT_ADDR : Nat := 0x1000_0000

/-- A loadable segment, checked: its address, its file bytes, its size in
    memory (the rest zero), and its permissions. -/
structure Seg where
  base : Nat
  data : ByteArray
  memsz : Nat
  exec : Bool
  read : Bool
  write : Bool

def flag (flags bit : Nat) : Bool := flags / bit % 2 == 1

/-- A loadable segment's checks, as the Rust makes them: no overflow, W^X,
    aligned (4 bytes for code, 2 else), in the ROM or the RAM, disjoint from
    the segments `loaded` before it, its file bytes no more than its size
    and its body those bytes; code in the ROM, all file bytes, of even
    length; writable data in the RAM. -/
def segValid (loaded : List Seg) (start memsz filesz body : Nat) (exec write : Bool) : Bool :=
  let stop := start + memsz
  let inRom := ROM_ADDR ≤ start && stop ≤ ROM_ADDR + ROM_SIZE
  let inRam := RAM_ADDR ≤ start && stop ≤ RAM_ADDR + RAM_SIZE
  stop < 2 ^ 64 && !(write && exec) && start % (if exec then 4 else 2) == 0 && (inRom || inRam) &&
    !loaded.any (fun o => start < o.base + o.memsz && o.base < stop) && filesz ≤ memsz && body == filesz &&
    (!exec || (inRom && memsz == filesz && filesz % 2 == 0)) && (exec || !write || inRam)

/-- Why a segment fails `segValid`: the Rust's message for its first failed
    check. -/
def segError (loaded : List Seg) (start memsz filesz body : Nat) (exec write : Bool) : String :=
  let stop := start + memsz
  let inRom := ROM_ADDR ≤ start && stop ≤ ROM_ADDR + ROM_SIZE
  let inRam := RAM_ADDR ≤ start && stop ≤ RAM_ADDR + RAM_SIZE
  if stop ≥ 2 ^ 64 then s!"PT_LOAD segment at {start} overflows the address space"
  else if write && exec then s!"PT_LOAD segment at {start} is both writable and executable"
  else if start % (if exec then 4 else 2) != 0 then s!"PT_LOAD segment address {start} is not aligned"
  else if !inRom && !inRam then s!"PT_LOAD segment at {start} is outside ZisK's addressable space"
  else if loaded.any (fun o => start < o.base + o.memsz && o.base < stop) then
    s!"overlapping PT_LOAD segments at {start}"
  else if filesz > memsz then s!"PT_LOAD segment at {start} has p_filesz greater than p_memsz"
  else if body != filesz then s!"PT_LOAD segment at {start}: its body is not p_filesz bytes"
  else if exec && !inRom then s!"executable PT_LOAD segment at {start} is outside the ROM"
  else if exec && memsz != filesz then s!"executable PT_LOAD segment at {start} has a zero-fill tail"
  else if exec && filesz % 2 != 0 then s!"executable PT_LOAD segment at {start} has an odd byte length"
  else s!"writable PT_LOAD segment at {start} is outside RAM"

/-- One program header, as the Rust checks it: none for a segment that is
    not `PT_LOAD` or is empty; an error for what the Rust rejects. `loaded`,
    the segments before it, for the overlap check. -/
def checkSeg (loaded : List Seg) (ph : ELF64ProgramHeaderTableEntry) (seg : InterpretedSegment) :
    Except String (Option Seg) :=
  let flags := ProgramHeaderTableEntry.p_flags ph
  let start := ProgramHeaderTableEntry.p_vaddr ph
  let memsz := ProgramHeaderTableEntry.p_memsz ph
  let filesz := ProgramHeaderTableEntry.p_filesz ph
  let exec := flag flags PF_X
  let write := flag flags PF_W
  if ProgramHeaderTableEntry.p_type ph != PT_LOAD || (memsz == 0 && filesz == 0) then .ok none
  else if segValid loaded start memsz filesz seg.segment_body.size exec write then
    .ok (some { base := start, data := seg.segment_body, memsz, exec, read := flag flags PF_R, write })
  else .error (segError loaded start memsz filesz seg.segment_body.size exec write)

/-- Each program header, checked, the segments kept in order. -/
def loadSegs : List (ELF64ProgramHeaderTableEntry × InterpretedSegment) → List Seg → Except String (List Seg)
  | [], acc => .ok acc
  | (ph, seg) :: rest, acc =>
    match checkSeg acc ph seg with
    | .error e => .error e
    | .ok (some s) => loadSegs rest (acc ++ [s])
    | .ok none => loadSegs rest acc

/-- The ELF's loadable segments, checked: the header (a RISC-V executable,
    little-endian), each segment, and the entry, 4-aligned in an executable
    segment. -/
def segments (elf : ELF64File) : Except String (List Seg) :=
  let h := elf.file_header
  let entry := h.e_entry.toNat
  if h.e_ident.bytes.get! 5 != 1 then .error "ELF is not little-endian"
  else if h.e_machine.toNat != EM_RISCV then .error "ELF machine is not EM_RISCV"
  else if h.e_type.toNat != ET_EXEC then .error "ELF type is not ET_EXEC"
  else match loadSegs elf.interpreted_segments [] with
    | .error e => .error e
    | .ok segs =>
      if entry % 4 != 0 then .error s!"entry point {entry} is not 4-byte aligned"
      else if !segs.any (fun s => s.exec && s.base ≤ entry && entry < s.base + s.data.size) then
        .error s!"entry point {entry} is in no executable segment"
      else .ok segs

/-- The code, as the transpiler reads it: each executable segment's 32-bit
    words, little-endian, a trailing 16-bit parcel left out. -/
def code (segs : List Seg) : List (BitVec 64 × Array (BitVec 32)) :=
  segs.filter (·.exec) |>.map fun s =>
    (BitVec.ofNat 64 s.base, (Array.range (s.data.size / 4)).map fun k =>
      BitVec.ofNat 32 (s.data[4 * k]!.toNat + 256 * s.data[4 * k + 1]!.toNat +
        65536 * s.data[4 * k + 2]!.toNat + 16777216 * s.data[4 * k + 3]!.toNat))

/-- A segment's bytes in memory: its file bytes, then zeros to its size. -/
def load (m : Mem) (s : Seg) : Mem :=
  (List.range s.memsz).foldl (fun m i =>
    m.insert (s.base + i) (if h : i < s.data.size then (s.data[i]'h).toBitVec else 0)) m

/-- The input's bytes, behind the free input's 8 at INPUT_ADDR. -/
def loadInput (m : Mem) (input : ByteArray) : Mem :=
  (List.range input.size).foldl (fun m i => m.insert (INPUT_ADDR + 8 + i) (input.get! i).toBitVec) m

/-- RISC-V's memory: every segment. -/
def riscvMem (segs : List Seg) (input : ByteArray) : Mem :=
  loadInput (segs.foldl load ∅) input

/-- ZisK's memory: every segment but the execute-only ones. -/
def ziskMem (segs : List Seg) (input : ByteArray) : Mem :=
  loadInput ((segs.filter fun s => !s.exec || s.read).foldl load ∅) input

/-- The execute-only segments' bytes: in RISC-V's memory, not in ZisK's. -/
def xOnly (segs : List Seg) (a : Nat) : Bool :=
  segs.any fun s => s.exec && !s.read && s.base ≤ a && a < s.base + s.memsz

end ZiskRiscv.Elf
