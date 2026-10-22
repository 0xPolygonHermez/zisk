/-
  The simulation: a RISC-V state and a ZisK state agree, and each instruction
  the transpiler translates keeps them agreeing.

  So far the registers: RISC-V's x1..x31 are ZisK's r1..r31 (the transpiler
  maps xk to rk, core/src/zisk_inst_builder.rs). r32..r39 are ZisK's alone,
  the transpiler's scratch, and RegsAgree says nothing of them. And one
  instruction: `add rd, rs1, rs2`, whose sail-riscv execution
  (Equiv/Riscv.lean) and whose transpilation's sail-zisk execution
  (Equiv/Zisk.lean) leave the registers agreeing.
-/
import Equiv.Riscv
import Equiv.Zisk

namespace Equiv.Sim

open Equiv.Riscv (RVState RegBits getX setX getX_setX)
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- RISC-V's x1..x31 hold what ZisK's r1..r31 do. -/
def RegsAgree (r : RVState) (xs : Vector (BitVec 64) 40) : Prop :=
  ∀ k, 1 ≤ k → k < 32 → getX r k = some xs[k]!

/-- ZisK's registers are read with an Int index (sail-zisk's rX); on a
    natural number it is the Nat one. -/
theorem getElem!_int (xs : Vector (BitVec 64) 40) (k : Nat) :
    xs[(k : Int)]! = xs[k]! := by
  simp only [getElem!_def]; rfl

theorem set!_eq (xs : Vector α n) (i : Nat) (x : α) : xs.set! i x = xs.setIfInBounds i x := rfl

/-- A ZisK register after an update. -/
theorem getElem!_update (xs : Vector (BitVec 64) 40) (n k : Nat) (w : BitVec 64) (hn : n < 40) :
    (vectorUpdate xs n w)[k]! = if k = n then w else xs[k]! := by
  simp only [vectorUpdate, set!_eq, getElem!_def, Vector.getElem?_setIfInBounds]
  by_cases e : k = n
  · subst e; simp [hn]
  · simp [Ne.symm e, e]

/-- `add rd, rs1, rs2`, rs1, rs2 ≠ x0: from agreeing registers, sail-riscv's
    execution and sail-zisk's execution of its transpilation write the same
    sum to rd, and the registers agree after. -/
theorem add_regs (r : RVState) (xs : Vector (BitVec 64) 40) (rs1 rs2 rd : RegBits)
    (h1 : rs1.toNat ≠ 0) (h2 : rs2.toNat ≠ 0) (hd : rd.toNat ≠ 0) (agree : RegsAgree r xs) :
    let sum := xs[(rs1.toNat : Int)]! + xs[(rs2.toNat : Int)]!
    (interpretSailEffects (execute_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) .ADD)).run r
        = .ok RETIRE_SUCCESS (setX r rd.toNat sum) ∧
      RegsAgree (setX r rd.toNat sum) (vectorUpdate xs rd.toNat sum) := by
  intro sum
  have l1 := rs1.lt; have l2 := rs2.lt; have ld := rd.lt
  have a1 := agree rs1.toNat (by omega) l1
  have a2 := agree rs2.toNat (by omega) l2
  refine ⟨?_, ?_⟩
  · simp only [sum, getElem!_int]
    exact Equiv.Riscv.execute_add r rs1 rs2 rd _ _ a1 a2
  · intro k hk1 hk2
    rw [getX_setX r rd.toNat k sum ld hk2, getElem!_update xs rd.toNat k sum (by omega)]
    by_cases e : k = rd.toNat
    · rw [if_pos ⟨hd, e⟩, if_pos e]
    · rw [if_neg (fun h => e h.2), if_neg e]; exact agree k hk1 hk2

/-- The pilot: `add rd, rs1, rs2`, rs1, rs2, rd ≠ x0. From a RISC-V state and
    a ZisK state whose registers agree, sail-riscv's execution of the
    instruction and sail-zisk's execution of its transpilation both succeed,
    and their registers agree after. -/
theorem add_sim (r : RVState) (z : Equiv.Zisk.ZState) (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (hpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    (hst : z.regs.get? _root_.Zisk.Register.Step = some st)
    (rs1 rs2 rd : RegBits) (h1 : rs1.toNat ≠ 0) (h2 : rs2.toNat ≠ 0) (hd : rd.toNat ≠ 0)
    (agree : RegsAgree r xs) :
    ∃ r' z' xs',
      (interpretSailEffects (execute_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) .ADD)).run r
        = .ok RETIRE_SUCCESS r' ∧
      (_root_.Zisk.Functions.zisk_execute (Equiv.Zisk.addInst rd.toNat rs1.toNat rs2.toNat)).run z
        = .ok false z' ∧
      z'.regs.get? _root_.Zisk.Register.Xs = some xs' ∧
      RegsAgree r' xs' := by
  have l1 := rs1.lt; have l2 := rs2.lt; have ld := rd.lt
  obtain ⟨hr, ha⟩ := add_regs r xs rs1 rs2 rd h1 h2 hd agree
  refine ⟨_, _, _, hr,
    Equiv.Zisk.execute_add z rd.toNat rs1.toNat rs2.toNat (by omega) (by omega) (by omega) xs pc st hx hpc hst,
    ?_, ha⟩
  simp [Std.ExtDHashMap.get?_insert]

end Equiv.Sim
