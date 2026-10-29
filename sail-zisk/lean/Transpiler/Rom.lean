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

/-- The program's code words, its segments in order (each its base and its
    words): each word's address, the word, and the two words after it in
    its segment. -/
def codeWords (segs : List (BitVec 64 × Array (BitVec 32))) :
    List (BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)) :=
  segs.flatMap fun (base, ws) =>
    (List.range ws.size).map fun k => (base + BitVec.ofNat 64 (4 * k), ws[k]!, ws[k + 1]?, ws[k + 2]?)

/-- The translations of code words, the counter at n. -/
def romFrom : List (BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)) → Nat →
    List (BitVec 64 × zisk_inst)
  | [], _ => []
  | (pc, w, next, next2) :: rest, n =>
    match convert pc w next (internalAddr n) next2 with
    | some l => l ++ romFrom rest (n + internals l)
    | none => romFrom rest n

/-- `romFrom`, tail-recursive, its entries appended to `acc`: what runs. -/
def romFromAcc (acc : Array (BitVec 64 × zisk_inst)) :
    List (BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32)) → Nat → Array (BitVec 64 × zisk_inst)
  | [], _ => acc
  | (pc, w, next, next2) :: rest, n =>
    match convert pc w next (internalAddr n) next2 with
    | some l => romFromAcc (acc ++ l.toArray) rest (n + internals l)
    | none => romFromAcc acc rest n

theorem romFromAcc_eq (acc : Array (BitVec 64 × zisk_inst)) (ws) (n : Nat) :
    (romFromAcc acc ws n).toList = acc.toList ++ romFrom ws n := by
  induction ws generalizing acc n with
  | nil => simp [romFromAcc, romFrom]
  | cons x rest ih =>
    obtain ⟨pc, w, next, next2⟩ := x
    simp only [romFromAcc, romFrom]
    split <;> simp [ih]

/-- What runs for `romFrom`. -/
def romFromImpl (ws : List (BitVec 64 × BitVec 32 × Option (BitVec 32) × Option (BitVec 32))) (n : Nat) :
    List (BitVec 64 × zisk_inst) :=
  (romFromAcc #[] ws n).toList

@[csimp] theorem romFrom_eq_impl : @romFrom = @romFromImpl := by
  funext ws n; simp [romFromImpl, romFromAcc_eq]

/-- The program's ROM entries, from its code segments in order. -/
def programRom (segs : List (BitVec 64 × Array (BitVec 32))) : List (BitVec 64 × zisk_inst) :=
  romFrom (codeWords segs) 0

end Transpiler
