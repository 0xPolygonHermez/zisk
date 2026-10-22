/-
  sail-riscv's semantics, instruction by instruction, in ZisK's profile: what
  `execute` does to a RISC-V state, as equations on lean-sail's sequential
  interpreter — the facts the equivalence proofs rest on, on the RISC-V side.

  sail-riscv's Lean is a free monad over effects (register and memory
  reads and writes), which lean-sail interprets as a state monad over a
  map of registers and a map of bytes. The lemmas here: interpreting
  distributes over bind; a register read and a write, generic in the
  register; and, for `add`, `addi` and `beq`, what the instruction does, for
  any registers. They need no more of the state than the registers they
  read to hold values — `misa`, too, for a branch, since sail-riscv checks
  the compressed extension before a jump.

  Loads and stores are still to come: sail-riscv takes them through address
  translation, PMP and PMA checks and misaligned splitting, which want
  lemmas of their own under machine mode, PMP and region hypotheses.
-/
import LeanZiskRV

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

abbrev RVState := Sail.ArchSem.SequentialState RegisterType

section FreeM
variable {Eff : Type} [Effect Eff] {m : Type → Type} [Monad m]
  (interp : (e : Eff) → m (Effect.ret e))

@[simp] theorem FreeM.liftM_pure' (a : α) : FreeM.liftM interp (pure a : FreeM Eff α) = pure a := rfl

@[simp] theorem FreeM.liftM_bind' [LawfulMonad m] (x : FreeM Eff α) (f : α → FreeM Eff β) :
    FreeM.liftM interp (x >>= f) = FreeM.liftM interp x >>= fun a => FreeM.liftM interp (f a) := by
  induction x with
  | pure a => simp [FreeM.liftM, bind, FreeM.bind]
  | impure e k ih =>
    show FreeM.liftM interp (FreeM.bind (FreeM.impure e k) f) = _
    simp only [FreeM.bind, FreeM.liftM, bind_assoc]
    congr 1; funext r; exact ih r
end FreeM

@[simp] theorem toNatInt_toNat (x : BitVec w) : (BitVec.toNatInt x).toNat = x.toNat := by
  simp [BitVec.toNatInt]

theorem readReg_run (s : RVState) (r : Register) (v : RegisterType r) (h : s.regs.get? r = some v) :
    (interpretSailEffects (readReg r : SailM _)).run s = .ok v s := by
  simp [interpretSailEffects, LeanZiskRV.readReg, Sail.ArchSem.PreSail.readReg, FreeM.liftM,
    interpretInstructionEffect, h]
  done


theorem writeReg_run (s : RVState) (r : Register) (v : RegisterType r) :
    (interpretSailEffects (writeReg r v : SailM _)).run s = .ok () { s with regs := s.regs.insert r v } := by
  simp [interpretSailEffects, LeanZiskRV.writeReg, Sail.ArchSem.PreSail.writeReg, FreeM.liftM,
    interpretInstructionEffect]
  rfl


/-- sail-riscv's register for x_k, k = 1..31, as `rX` and `wX` pick it (31 for
    anything past 30, as they do). -/
def xreg : Nat → Register
  | 1 => Register.x1
  | 2 => Register.x2
  | 3 => Register.x3
  | 4 => Register.x4
  | 5 => Register.x5
  | 6 => Register.x6
  | 7 => Register.x7
  | 8 => Register.x8
  | 9 => Register.x9
  | 10 => Register.x10
  | 11 => Register.x11
  | 12 => Register.x12
  | 13 => Register.x13
  | 14 => Register.x14
  | 15 => Register.x15
  | 16 => Register.x16
  | 17 => Register.x17
  | 18 => Register.x18
  | 19 => Register.x19
  | 20 => Register.x20
  | 21 => Register.x21
  | 22 => Register.x22
  | 23 => Register.x23
  | 24 => Register.x24
  | 25 => Register.x25
  | 26 => Register.x26
  | 27 => Register.x27
  | 28 => Register.x28
  | 29 => Register.x29
  | 30 => Register.x30
  | _ => Register.x31

theorem xreg_ty (k : Nat) : RegisterType (xreg k) = BitVec 64 := by
  unfold xreg; split <;> rfl

theorem xreg_inj : ∀ a b : Fin 32, a.val ≠ 0 → b.val ≠ 0 → xreg a.val = xreg b.val → a = b := by
  decide

/-- Register k of a RISC-V state, as sail-riscv's `rX` reads it (x0 is 0). -/
def getX (s : RVState) (k : Nat) : Option (BitVec 64) :=
  if k = 0 then some 0 else (s.regs.get? (xreg k)).map (cast (xreg_ty k))

/-- The state with register k set to v, as sail-riscv's `wX` writes it (x0 stays). -/
def setX (s : RVState) (k : Nat) (v : BitVec 64) : RVState :=
  if k = 0 then s else { s with regs := s.regs.insert (xreg k) (cast (xreg_ty k).symm v) }

/-- Reading a register after writing one. -/
theorem getX_setX (s : RVState) (n k : Nat) (v : BitVec 64) (hn : n < 32) (hk : k < 32) :
    getX (setX s n v) k = if n ≠ 0 ∧ k = n then some v else getX s k := by
  by_cases h0 : n = 0
  · simp [setX, h0]
  by_cases k0 : k = 0
  · simp [getX, k0]; omega
  by_cases e : k = n
  · subst e; simp [getX, setX, h0]
  · have ne : xreg n ≠ xreg k := fun h => e (by
      have := xreg_inj ⟨n, hn⟩ ⟨k, hk⟩ h0 k0 h; simp at this; omega)
    simp [getX, setX, h0, k0, e, Std.ExtDHashMap.get?_insert, ne]

macro "reg_simp" : tactic => `(tactic| (simp_all [getX, setX, xreg, interpretSailEffects, LeanZiskRV.readReg,
    Sail.ArchSem.PreSail.readReg, LeanZiskRV.writeReg, Sail.ArchSem.PreSail.writeReg, FreeM.liftM,
    interpretInstructionEffect, regval_from_reg, regval_into_reg, zero_reg, zeros, xreg_write_callback,
    reg_name_forwards, encdec_reg_forwards_matches, get_config_use_abi_names, LeanZiskRV.not,
    xreg_full_write_callback] <;> try rfl))

/-- sail-riscv's `rX` reads register n. -/
theorem rX_run (s : RVState) (n : Nat) (hn : n < 32) (v : BitVec 64) (h : getX s n = some v) :
    (interpretSailEffects (rX (.Regno n))).run s = .ok v s := by
  simp only [rX]
  split
  all_goals first
    | (reg_simp; done)
    | (have : n = 31 := by simp only [imp_false] at *; omega
       subst this; reg_simp)

/-- sail-riscv's `wX` writes register n. -/
theorem wX_run (s : RVState) (n : Nat) (hn : n < 32) (v : BitVec 64) :
    (interpretSailEffects (wX (.Regno n) v)).run s = .ok () (setX s n v) := by
  simp only [wX]
  split
  all_goals first
    | (reg_simp; done)
    | (have : n = 31 := by simp only [imp_false] at *; omega
       subst this; reg_simp)

@[simp] theorem interp_bind (x : SailM α) (f : α → SailM β) :
    interpretSailEffects (x >>= f) = interpretSailEffects x >>= fun a => interpretSailEffects (f a) := by
  simp [interpretSailEffects]

@[simp] theorem interp_pure (a : α) : interpretSailEffects (pure a : SailM α) = pure a := rfl

@[simp] theorem run_bind' (x : EStateM ε σ α) (g : α → EStateM ε σ β) (s : σ) :
    (x >>= g).run s = match x.run s with
      | .ok a s' => (g a).run s'
      | .error e s' => .error e s' := rfl

@[simp] theorem run_pure' (a : α) (s : σ) : (pure a : EStateM ε σ α).run s = .ok a s := rfl

/-- A register index, as sail-riscv types it: 5 bits, the base ISA not being E. -/
abbrev RegBits := BitVec (if (false : Bool) then 4 else 5)

theorem RegBits.lt (i : RegBits) : i.toNat < 32 := by simpa using i.isLt

/-- `add rd, rs1, rs2`, any registers: rd gets rs1 + rs2. -/
theorem execute_add (s : RVState) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2) :
    (interpretSailEffects (execute_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) .ADD)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (v1 + v2)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  have w := wX_run s rd.toNat rd.lt (v1 + v2)
  simp only [execute_RTYPE, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
    run_pure']
  rw [r1]; dsimp only; rw [r2]; dsimp only; rw [w]

/-- What an R-type instruction computes from its operands, as
    sail-riscv's `execute_RTYPE` computes it. -/
def rtypeVal : rop → BitVec 64 → BitVec 64 → BitVec 64
  | .ADD, a, b => a + b
  | .SLT, a, b => zero_extend (m := 64) (bool_to_bit (zopz0zI_s a b))
  | .SLTU, a, b => zero_extend (m := 64) (bool_to_bit (zopz0zI_u a b))
  | .AND, a, b => a &&& b
  | .OR, a, b => a ||| b
  | .XOR, a, b => a ^^^ b
  | .SLL, a, b => shift_bits_left a (Sail.BitVec.extractLsb b (Functions.log2_xlen -i 1) 0)
  | .SRL, a, b => shift_bits_right a (Sail.BitVec.extractLsb b (Functions.log2_xlen -i 1) 0)
  | .SUB, a, b => a - b
  | .SRA, a, b => shift_bits_right_arith a (Sail.BitVec.extractLsb b (Functions.log2_xlen -i 1) 0)

/-- Any R-type instruction, any registers: rd gets what it computes. -/
theorem execute_rtype (s : RVState) (op : rop) (rs1 rs2 rd : RegBits) (v1 v2 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2) :
    (interpretSailEffects (execute_RTYPE (.Regidx rs2) (.Regidx rs1) (.Regidx rd) op)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (rtypeVal op v1 v2)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  have w := wX_run s rd.toNat rd.lt (rtypeVal op v1 v2)
  cases op <;>
  · simp only [execute_RTYPE, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
      run_pure']
    repeat (first | rw [r1] | rw [r2]); dsimp only
    repeat (first | (rw [r1]; dsimp only) | (rw [r2]; dsimp only))
    simp only [rtypeVal] at w; rw [w]; rfl

/-- `addi rd, rs1, imm`: rd gets rs1 + sign-extended imm. -/
theorem execute_addi (s : RVState) (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) :
    (interpretSailEffects (execute_ITYPE imm (.Regidx rs1) (.Regidx rd) .ADDI)).run s
      = .ok RETIRE_SUCCESS (setX s rd.toNat (v1 + imm.signExtend 64)) := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have w := wX_run s rd.toNat rd.lt (v1 + imm.signExtend 64)
  simp only [execute_ITYPE, rX_bits, wX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind',
    run_pure', sign_extend, Sail.BitVec.signExtend]
  rw [r1]; dsimp only; rw [w]

section ME
open Sail.ArchSem.PreSail

/-- Running a lifted SailM computation, then an early-return continuation. -/
theorem me_run_lift_bind (x : SailM α) (f : α → SailME β β) :
    LeanZiskRV.SailME.run ((monadLift x : SailME β α) >>= f) = x >>= fun a => LeanZiskRV.SailME.run (f a) := by
  simp only [LeanZiskRV.SailME.run]
  induction x with
  | pure a => rfl
  | impure e k ih =>
    show PreSailME.run (FreeM.bind (instMonadLiftPreSailMPreSailME.lift (FreeM.impure e k)) f) = _
    simp only [instMonadLiftPreSailMPreSailME.lift, FreeM.bind, PreSailME.run]
    show FreeM.impure e _ = FreeM.impure e _
    congr 1; funext r; exact ih r

theorem me_run_pure (v : α) : LeanZiskRV.SailME.run (pure v : SailME α α) = pure v := rfl

theorem me_bind_pure (a : α) (f : α → SailME β γ) : ((pure a : SailME β α) >>= f) = f a := rfl
end ME

/-- `beq rs1, rs2, imm`, taken, to a 4-byte aligned target: nextPC gets pc + imm. -/
theorem execute_beq_taken (s : RVState) (rs1 rs2 : RegBits) (imm : BitVec 13) (v : BitVec 64)
    (pc : BitVec 64) (m : RegisterType Register.misa)
    (h1 : getX s rs1.toNat = some v) (h2 : getX s rs2.toNat = some v)
    (hpc : s.regs.get? Register.PC = some pc) (hmisa : s.regs.get? Register.misa = some m)
    (hal : (pc + imm.signExtend 64)[0] = false ∧ (pc + imm.signExtend 64)[1] = false) :
    (interpretSailEffects (execute_BTYPE imm (.Regidx rs2) (.Regidx rs1) .BEQ)).run s
      = .ok RETIRE_SUCCESS { s with regs := s.regs.insert Register.nextPC (pc + imm.signExtend 64) } := by
  have r1 := rX_run s rs1.toNat rs1.lt v h1
  have r2 := rX_run s rs2.toNat rs2.lt v h2
  simp only [execute_BTYPE, rX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only; rw [r2]; dsimp only
  simp [jump_to, ext_control_check_pc, currentlyEnabled, hartSupports, bit_to_bool, set_next_pc,
    redirect_callback, sign_extend, Sail.BitVec.signExtend, BitVec.access,
    interpretSailEffects, LeanZiskRV.readReg, Sail.ArchSem.PreSail.readReg, LeanZiskRV.writeReg,
    Sail.ArchSem.PreSail.writeReg, FreeM.liftM, interpretInstructionEffect, hpc, hmisa, hal,
    me_run_lift_bind, me_bind_pure, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert,
    bool_bit_backwards, LeanZiskRV.not]
  rfl

/-- `beq rs1, rs2, imm`, not taken: nothing changes; the step moves on. -/
theorem execute_beq_not_taken (s : RVState) (rs1 rs2 : RegBits) (imm : BitVec 13) (v1 v2 : BitVec 64)
    (h1 : getX s rs1.toNat = some v1) (h2 : getX s rs2.toNat = some v2) (hne : v1 ≠ v2) :
    (interpretSailEffects (execute_BTYPE imm (.Regidx rs2) (.Regidx rs1) .BEQ)).run s
      = .ok RETIRE_SUCCESS s := by
  have r1 := rX_run s rs1.toNat rs1.lt v1 h1
  have r2 := rX_run s rs2.toNat rs2.lt v2 h2
  simp only [execute_BTYPE, rX_bits, toNatInt_toNat, interp_bind, interp_pure, run_bind', run_pure']
  rw [r1]; dsimp only; rw [r2]; dsimp only
  simp [hne]

end Equiv.Riscv
