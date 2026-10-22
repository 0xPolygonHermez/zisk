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


/-- Register `n` of a RISC-V state, as sail-riscv's `rX` reads it (x0 is 0). -/
def getX (s : RVState) : Nat → Option (BitVec 64)
  | 0 => some 0
  | 1 => s.regs.get? Register.x1
  | 2 => s.regs.get? Register.x2
  | 3 => s.regs.get? Register.x3
  | 4 => s.regs.get? Register.x4
  | 5 => s.regs.get? Register.x5
  | 6 => s.regs.get? Register.x6
  | 7 => s.regs.get? Register.x7
  | 8 => s.regs.get? Register.x8
  | 9 => s.regs.get? Register.x9
  | 10 => s.regs.get? Register.x10
  | 11 => s.regs.get? Register.x11
  | 12 => s.regs.get? Register.x12
  | 13 => s.regs.get? Register.x13
  | 14 => s.regs.get? Register.x14
  | 15 => s.regs.get? Register.x15
  | 16 => s.regs.get? Register.x16
  | 17 => s.regs.get? Register.x17
  | 18 => s.regs.get? Register.x18
  | 19 => s.regs.get? Register.x19
  | 20 => s.regs.get? Register.x20
  | 21 => s.regs.get? Register.x21
  | 22 => s.regs.get? Register.x22
  | 23 => s.regs.get? Register.x23
  | 24 => s.regs.get? Register.x24
  | 25 => s.regs.get? Register.x25
  | 26 => s.regs.get? Register.x26
  | 27 => s.regs.get? Register.x27
  | 28 => s.regs.get? Register.x28
  | 29 => s.regs.get? Register.x29
  | 30 => s.regs.get? Register.x30
  | _ => s.regs.get? Register.x31

/-- The state with register `n` set to `v`, as sail-riscv's `wX` writes it. -/
def setX (s : RVState) (n : Nat) (v : BitVec 64) : RVState :=
  match n with
  | 0 => s
  | 1 => { s with regs := s.regs.insert Register.x1 v }
  | 2 => { s with regs := s.regs.insert Register.x2 v }
  | 3 => { s with regs := s.regs.insert Register.x3 v }
  | 4 => { s with regs := s.regs.insert Register.x4 v }
  | 5 => { s with regs := s.regs.insert Register.x5 v }
  | 6 => { s with regs := s.regs.insert Register.x6 v }
  | 7 => { s with regs := s.regs.insert Register.x7 v }
  | 8 => { s with regs := s.regs.insert Register.x8 v }
  | 9 => { s with regs := s.regs.insert Register.x9 v }
  | 10 => { s with regs := s.regs.insert Register.x10 v }
  | 11 => { s with regs := s.regs.insert Register.x11 v }
  | 12 => { s with regs := s.regs.insert Register.x12 v }
  | 13 => { s with regs := s.regs.insert Register.x13 v }
  | 14 => { s with regs := s.regs.insert Register.x14 v }
  | 15 => { s with regs := s.regs.insert Register.x15 v }
  | 16 => { s with regs := s.regs.insert Register.x16 v }
  | 17 => { s with regs := s.regs.insert Register.x17 v }
  | 18 => { s with regs := s.regs.insert Register.x18 v }
  | 19 => { s with regs := s.regs.insert Register.x19 v }
  | 20 => { s with regs := s.regs.insert Register.x20 v }
  | 21 => { s with regs := s.regs.insert Register.x21 v }
  | 22 => { s with regs := s.regs.insert Register.x22 v }
  | 23 => { s with regs := s.regs.insert Register.x23 v }
  | 24 => { s with regs := s.regs.insert Register.x24 v }
  | 25 => { s with regs := s.regs.insert Register.x25 v }
  | 26 => { s with regs := s.regs.insert Register.x26 v }
  | 27 => { s with regs := s.regs.insert Register.x27 v }
  | 28 => { s with regs := s.regs.insert Register.x28 v }
  | 29 => { s with regs := s.regs.insert Register.x29 v }
  | 30 => { s with regs := s.regs.insert Register.x30 v }
  | _ => { s with regs := s.regs.insert Register.x31 v }

macro "reg_simp" : tactic => `(tactic| (simp_all [getX, setX, interpretSailEffects, LeanZiskRV.readReg,
    Sail.ArchSem.PreSail.readReg, LeanZiskRV.writeReg, Sail.ArchSem.PreSail.writeReg, FreeM.liftM,
    interpretInstructionEffect, regval_from_reg, regval_into_reg, zero_reg, zeros, xreg_write_callback,
    reg_name_forwards, encdec_reg_forwards_matches, get_config_use_abi_names, LeanZiskRV.not,
    xreg_full_write_callback] <;> try rfl))

theorem rX_run (s : RVState) (n : Nat) (hn : n < 32) (v : BitVec 64) (h : getX s n = some v) :
    (interpretSailEffects (rX (.Regno n))).run s = .ok v s := by
  simp only [rX]
  split
  all_goals first
    | reg_simp
    | (have : n = 31 := by omega
       subst this; reg_simp)

theorem wX_run (s : RVState) (n : Nat) (hn : n < 32) (v : BitVec 64) :
    (interpretSailEffects (wX (.Regno n) v)).run s = .ok () (setX s n v) := by
  simp only [wX]
  split
  all_goals first
    | reg_simp
    | (have : n = 31 := by omega
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
