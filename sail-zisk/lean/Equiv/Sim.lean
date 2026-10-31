/-
  The simulation: a RISC-V state and a ZisK state agree, and each instruction
  the transpiler translates keeps them agreeing.

  So far the registers: RISC-V's x1..x31 are ZisK's r1..r31 (the transpiler
  maps xk to rk, core/src/zisk_inst_builder.rs). r32..r39 are ZisK's alone,
  the transpiler's scratch, and RegsAgree says nothing of them. And the
  R-type instructions (add, sub, sll, slt, sltu, xor, srl, sra, or, and),
  rs1, rs2, rd ≠ x0: sail-riscv's execution (Equiv/Riscv.lean) and the
  sail-zisk execution of their transpilation (Equiv/Zisk.lean) leave the
  registers agreeing. `add` first, the pilot; then all ten, through one
  ZisK lemma for any pure op and one value lemma per op.
-/
import Equiv.Riscv
import Equiv.Zisk

namespace Equiv.Sim

open Equiv.Riscv (RVState RegBits getX setX getX_setX rtypeVal)
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

/-- The ZisK op the transpiler emits for each R-type op (create_register_op). -/
def ziskOp : rop → _root_.Zisk.zisk_op
  | .ADD => .ZOP_ADD () | .SUB => .ZOP_SUB () | .SLL => .ZOP_SLL () | .SLT => .ZOP_LT ()
  | .SLTU => .ZOP_LTU () | .XOR => .ZOP_XOR () | .SRL => .ZOP_SRL () | .SRA => .ZOP_SRA ()
  | .OR => .ZOP_OR () | .AND => .ZOP_AND ()

theorem pureOp (op : rop) : Equiv.Zisk.PureOp (ziskOp op) := by
  cases op <;> refine ⟨fun a b e => ?_, ?_, fun u => ?_⟩ <;>
    first | rfl | simp [ziskOp] | decide

theorem shl_int (x : BitVec w) (n : Nat) : x <<< Int.ofNat n = x <<< n := rfl

theorem shr_int (x : BitVec w) (n : Nat) : x >>> Int.ofNat n = x >>> n := by
  cases n with
  | zero => show BitVec.shiftLeft x 0 = x >>> 0; simp
  | succ n => show x <<< (Int.negSucc n) = _; rfl

/-- ZisK's sra: sign-extend to 128 bits, shift, keep the low 64. -/
theorem sra_eq (a : BitVec 64) (x : BitVec 6) :
    ((a.signExtend 128) >>> x.toNat).extractLsb 63 0 = a.sshiftRight x.toNat := by
  rw [← BitVec.ushiftRight_eq', ← BitVec.sshiftRight_eq']
  bv_decide

theorem bool_c_fst (c : Bool) :
    (_root_.Zisk.Functions.bool_c c).1 = zero_extend (m := 64) (bool_to_bit c) := by
  cases c <;> rfl

theorem ziskVal (op : rop) (a b : BitVec 64) :
    (_root_.Zisk.Functions.zisk_exec (ziskOp op) a b).1 = rtypeVal op a b := by
  cases op <;> simp [ziskOp, rtypeVal, _root_.Zisk.Functions.zisk_exec]
  case SLT | SLTU => exact bool_c_fst _
  case SLL | SRL =>
    simp only [Sail.BitVec.toNatInt, shift_bits_left, shift_bits_right, shl_int, shr_int, Functions.log2_xlen]
    all_goals rfl
  case SRA =>
    simp only [Sail.BitVec.toNatInt, Sail.BitVec.extractLsb, shift_bits_right_arith,
      _root_.Zisk.Functions.sra_bits, _root_.Zisk.Functions.sign_extend, Sail.BitVec.signExtend,
      Functions.log2_xlen, shr_int]
    exact sra_eq a (b.extractLsb 5 0)
/-- An R-type instruction, rs1, rs2, rd ≠ x0: from agreeing registers,
    sail-riscv's execution and sail-zisk's execution of its transpilation
    write the same value to rd. -/
theorem rtype_regs (r : RVState) (xs : Vector (BitVec 64) 40) (op : rop) (rs1 rs2 rd : RegBits)
    (h1 : rs1.toNat ≠ 0) (h2 : rs2.toNat ≠ 0) (hd : rd.toNat ≠ 0) (agree : RegsAgree r xs) :
    let c := (_root_.Zisk.Functions.zisk_exec (ziskOp op) xs[(rs1.toNat : Int)]! xs[(rs2.toNat : Int)]!).1
    (interpretSailEffects (execute_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run r
        = .ok RETIRE_SUCCESS (setX r rd.toNat c) ∧
      RegsAgree (setX r rd.toNat c) (vectorUpdate xs rd.toNat c) := by
  intro c
  have l1 := rs1.lt; have l2 := rs2.lt; have ld := rd.lt
  have a1 := agree rs1.toNat (by omega) l1
  have a2 := agree rs2.toNat (by omega) l2
  refine ⟨?_, ?_⟩
  · simp only [c, ziskVal, getElem!_int]
    exact Equiv.Riscv.execute_rtype r op rs1 rs2 rd _ _ a1 a2
  · intro k hk1 hk2
    rw [getX_setX r rd.toNat k c ld hk2, getElem!_update xs rd.toNat k c (by omega)]
    by_cases e : k = rd.toNat
    · rw [if_pos ⟨hd, e⟩, if_pos e]
    · rw [if_neg (fun h => e h.2), if_neg e]; exact agree k hk1 hk2

/-- Every R-type instruction (add, sub, sll, slt, sltu, xor, srl, sra, or,
    and), rs1, rs2, rd ≠ x0: from a RISC-V state and a ZisK state whose
    registers agree, sail-riscv's execution of the instruction and sail-zisk's
    execution of its transpilation both succeed, and their registers agree
    after. -/
theorem rtype_sim (r : RVState) (z : Equiv.Zisk.ZState) (xs : Vector (BitVec 64) 40) (pc st : BitVec 64)
    (hx : z.regs.get? _root_.Zisk.Register.Xs = some xs) (hpc : z.regs.get? _root_.Zisk.Register.PC = some pc)
    (hst : z.regs.get? _root_.Zisk.Register.Step = some st)
    (op : rop) (rs1 rs2 rd : RegBits) (h1 : rs1.toNat ≠ 0) (h2 : rs2.toNat ≠ 0) (hd : rd.toNat ≠ 0)
    (agree : RegsAgree r xs) :
    ∃ r' z' xs',
      (interpretSailEffects (execute_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run r
        = .ok RETIRE_SUCCESS r' ∧
      (_root_.Zisk.Functions.zisk_execute (Equiv.Zisk.regInst (ziskOp op) rd.toNat rs1.toNat rs2.toNat)).run z
        = .ok false z' ∧
      z'.regs.get? _root_.Zisk.Register.Xs = some xs' ∧
      RegsAgree r' xs' := by
  have l1 := rs1.lt; have l2 := rs2.lt; have ld := rd.lt
  obtain ⟨hr, ha⟩ := rtype_regs r xs op rs1 rs2 rd h1 h2 hd agree
  refine ⟨_, _, _, hr,
    Equiv.Zisk.execute_regop z (ziskOp op) (pureOp op) rd.toNat rs1.toNat rs2.toNat
      (by omega) (by omega) (by omega) xs pc st hx hpc hst,
    ?_, ha⟩
  simp [Std.ExtDHashMap.get?_insert]

end Equiv.Sim
