/-
  The program's part of the ROM, from the spec: the transpiler's walk over
  the code (riscv_interpreter.rs, its `compressed` feature off), a word at
  every 4-byte boundary, each translated by `convert` with the two words
  after it, when its segment holds them (riscv2zisk_context.rs:
  `next_instructions`, the rest of the segment); and the internal
  addresses, from the ROM's counter (zisk_rom.rs, `get_internal_address`):
  ROM_ADDR + 1, + 3, + 5, ..., handed out in the order the translations ask
  for them, across the segments in order.

  A word the spec does not describe has no instructions here, and takes no
  internal address.
-/
import Transpiler.Spec

namespace Transpiler

open _root_.Zisk

/-- The program's first ROM address (core/src/mem.rs). -/
def ROM_ADDR : BitVec 64 := 0x9000_0000

/-- The internal address the counter hands out n-th (from 0): odd, from ROM_ADDR + 1. -/
def internalAddr (n : Nat) : BitVec 64 := ROM_ADDR + BitVec.ofNat 64 (2 * n + 1)

/-- How many internal addresses a translation takes: its instructions at odd addresses. -/
def internals (l : List (BitVec 64 × zisk_inst)) : Nat := (l.filter (·.1.getLsbD 0)).length

/-- A segment's translations, appended to `acc`, the counter at `n`: the
    ROM entries and the counter after them. -/
def segRom (base : BitVec 64) (ws : Array (BitVec 32)) (acc : Array (BitVec 64 × zisk_inst)) (n : Nat) :
    Array (BitVec 64 × zisk_inst) × Nat :=
  (List.range ws.size).foldl (fun (acc, n) k =>
    match convert (base + BitVec.ofNat 64 (4 * k)) ws[k]! ws[k + 1]? (internalAddr n) ws[k + 2]? with
    | some l => (acc ++ l.toArray, n + internals l)
    | none => (acc, n)) (acc, n)

/-- The program's ROM entries, from its code segments in order (each its
    base and its words). -/
def programRom (segs : List (BitVec 64 × Array (BitVec 32))) : Array (BitVec 64 × zisk_inst) :=
  (segs.foldl (fun (acc, n) (base, ws) => segRom base ws acc n) (#[], 0)).1

end Transpiler
