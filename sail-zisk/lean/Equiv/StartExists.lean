/-
  The theorems' start is not empty: for any entry and initial memory, a
  RISC-V state meets the contract's start (`Start`, doc/riscv_contract.md
  § 3) — every register sail-riscv has at its type's default, but those
  the start, and what a step needs of the state (`StepReady`), fix. The
  state sail-riscv's own reset reaches cannot stand in for it: the reset
  calls `cancel_reservation`, which sail-riscv's Lean leaves undefined.
-/
import Equiv.BiosSim
import Equiv.Regions

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

open Lean Elab Term Meta in
/-- sail-riscv's registers: `Register`'s constructors, as a list. -/
elab "all_registers%" : term => do
  let some (.inductInfo i) := (← getEnv).find? ``LeanZiskRV.Register | throwError "no Register"
  mkListLit (Lean.mkConst ``LeanZiskRV.Register) (i.ctors.map (Lean.mkConst ·))

def allRegisters : List Register := all_registers%

set_option maxRecDepth 20000 in
theorem mem_allRegisters : ∀ r : Register, r ∈ allRegisters := by
  intro r; cases r <;> decide

instance dfltReg (r : Register) : Inhabited (RegisterType r) := by
  cases r <;> exact inferInstance

/-- Every register at its type's default. -/
def defaultRegs : Std.ExtDHashMap Register RegisterType :=
  allRegisters.foldl (fun m r => m.insert r default) ∅

theorem foldl_insert_get? (l : List Register) (m : Std.ExtDHashMap Register RegisterType) (r : Register) :
    (l.foldl (fun m r => m.insert r default) m).get? r = if r ∈ l then some default else m.get? r := by
  induction l generalizing m with
  | nil => simp
  | cons a l ih =>
    simp only [List.foldl_cons, ih, Std.ExtDHashMap.get?_insert, List.mem_cons]
    by_cases h : r ∈ l
    · simp [h]
    · by_cases e : a = r
      · subst e; simp
      · simp [h, e, Ne.symm e]

theorem defaultRegs_get? (r : Register) : defaultRegs.get? r = some default := by
  simp [defaultRegs, foldl_insert_get?, mem_allRegisters]

/-- misa with I, M, A and B set, MXL 64: what `StepReady` reads of it. -/
def startMisa : BitVec 64 := 0x8000000000001103

/-- A RISC-V state at the contract's start (doc/riscv_contract.md § 3): every
    register at its type's default (0 for the bit vectors), but those the
    start fixes. -/
def startState (entry : BitVec 64) (m0 : Mem) : RVState where
  regs := (((((((((((((((defaultRegs
    |>.insert .hart_state (.HART_ACTIVE ()))
    |>.insert .cur_privilege .Machine)
    |>.insert .mstatus 0)
    |>.insert .misa startMisa)
    |>.insert .htif_tohost_base none)
    |>.insert .pma_regions ziskRegions)
    |>.insert .mip 0)
    |>.insert .mie 0)
    |>.insert .elp (landing_pad_bits_backwards .NO_LP_EXPECTED))
    |>.insert .marchid 0x0FFFEEEE)
    |>.insert .mtvec 0x4)
    |>.insert .vtype 0x8000000000000000)
    |>.insert .PC entry)
    |>.insert .x1 Transpiler.OUTPUT_BLOCK)
    |>.insert .x10 Transpiler.INPUT_ADDR)
    |>.insert .x11 Transpiler.OUTPUT_ADDR
  mem := startMem m0
  tags := ()
  cycleCount := 0
  sailOutput := #[]

set_option maxRecDepth 20000 in
/-- `startState` is at the contract's start. -/
theorem start_exists (entry : BitVec 64) (m0 : Mem) : Start (startState entry m0) ziskRegions entry m0 := by
  refine ⟨⟨⟨?_⟩, ?_, ?_, ?_, ?_, ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩,
    ?_, ?_, ?_, ?_, fun k h1 h2 n1 n10 n11 => ?_, rfl⟩
  · simp only [startState]
    repeat (refine isSome_insert _ _ _ ?_)
    intro r; simp [defaultRegs_get?]
  all_goals try (simp (config := { decide := true }) [startState, Std.ExtDHashMap.get?_insert, defaultRegs_get?]; done)
  · exact ⟨0x8000000000000000, by simp (config := { decide := true }) [startState, Std.ExtDHashMap.get?_insert], by simp only [VtypeOk]; decide⟩
  all_goals try (simp (config := { decide := true }) [getX, xreg, startState, Std.ExtDHashMap.get?_insert]; done)
  have : k = 2 ∨ k = 3 ∨ k = 4 ∨ k = 5 ∨ k = 6 ∨ k = 7 ∨ k = 8 ∨ k = 9 ∨ k = 12 ∨ k = 13 ∨ k = 14 ∨ k = 15 ∨ k = 16 ∨ k = 17 ∨ k = 18 ∨ k = 19 ∨ k = 20 ∨ k = 21 ∨ k = 22 ∨ k = 23 ∨ k = 24 ∨ k = 25 ∨ k = 26 ∨ k = 27 ∨ k = 28 ∨ k = 29 ∨ k = 30 ∨ k = 31 := by omega
  rcases this with rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp_all (config := { decide := true }) [getX, xreg, startState, Std.ExtDHashMap.get?_insert,
    defaultRegs_get?] <;> rfl

end Equiv.Sim
