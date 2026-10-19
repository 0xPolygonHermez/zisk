/-
  The model's step with register sources and stores, in general.

  `Slot.lean` and `ExtPilot.lean` run `zisk_execute` by cases on the op, which
  is what made `execute_ext` slow. Here the op stays abstract: one step's
  result is `zisk_exec inst.op a b`, whatever the op, as long as it is not
  `halt`. And the sources and the store now include registers: `rX` reads the
  register file `Xs`, `wX` writes it, for an index other than `x0`.
-/
import Proofs.Slot

open Out.Functions PreSail Sail

namespace Correspondence

/-! ### The monad, one step at a time -/

theorem bind_ok {α β : Type} {m : SailM α} {f : α → SailM β} {σ σ' : State} {a : α}
    (h : m σ = .ok a σ') : (m >>= f) σ = f a σ' := by
  show EStateM.bind m f σ = _
  simp [EStateM.bind, h]

/-- Reading register `r ≠ x0`. -/
theorem rX_ok (r : BitVec 5) (σ : State) (xs : Vector (BitVec 64) 32)
    (hxs : σ.regs.get? Register.Xs = some xs) (hr : r ≠ 0) :
    rX r σ = .ok (xs[r.toNat]!) σ := by
  have h0 : (r = 0#5) = False := by simpa using hr
  simp [rX, PreSail.assert, h0, bind, EStateM.bind, pure, EStateM.pure, readReg_eq, hxs]

/-- Writing register `r ≠ x0`. -/
theorem wX_ok (r : BitVec 5) (v : BitVec 64) (σ : State) (xs : Vector (BitVec 64) 32)
    (hxs : σ.regs.get? Register.Xs = some xs) (hr : r ≠ 0) :
    wX r v σ = .ok () { σ with regs := σ.regs.insert Register.Xs (xs.set! r.toNat v) } := by
  have h0 : (r = 0#5) = False := by simpa using hr
  simp [wX, PreSail.assert, h0, bind, EStateM.bind, pure, EStateM.pure, readReg_eq, hxs, writeReg_run,
    vectorUpdate]

/-! ### Sources and stores -/

/-- The register an index field names: its low 5 bits. -/
def regIdx (v : BitVec 64) : BitVec 5 := Sail.BitVec.extractLsb v 4 0

/-- Source `a`'s value: the immediate, the previous `c`, or a register. -/
def aVal (inst : zisk_inst) (zc : BitVec 64) (xs : Vector (BitVec 64) 32) : BitVec 64 :=
  match inst.a_src with
  | .A_IMM => inst.a_imm
  | .A_REG => xs[(regIdx inst.a_imm).toNat]!
  | _ => zc

/-- Source `b`'s value. -/
def bVal (inst : zisk_inst) (zc : BitVec 64) (xs : Vector (BitVec 64) 32) : BitVec 64 :=
  match inst.b_src with
  | .B_IMM => inst.b_imm
  | .B_REG => xs[(regIdx inst.b_imm).toNat]!
  | _ => zc

/-- The sources the step covers: immediate, `c`, or a register other than x0. -/
def ASrcOk (inst : zisk_inst) : Prop :=
  inst.a_src = .A_IMM ∨ inst.a_src = .A_C ∨ (inst.a_src = .A_REG ∧ regIdx inst.a_imm ≠ 0)

def BSrcOk (inst : zisk_inst) : Prop :=
  inst.b_src = .B_IMM ∨ inst.b_src = .B_C ∨ (inst.b_src = .B_REG ∧ regIdx inst.b_imm ≠ 0)

/-- The stores the step covers: none, or a register other than x0. -/
def StoreOk (inst : zisk_inst) : Prop :=
  inst.store = .ST_NONE ∨ (inst.store = .ST_REG ∧ regIdx inst.store_off ≠ 0)

theorem source_a_ok (inst : zisk_inst) (σ : State) (zc : BitVec 64) (xs : Vector (BitVec 64) 32)
    (hzc : σ.regs.get? Register.Zc = some zc) (hxs : σ.regs.get? Register.Xs = some xs)
    (ha : ASrcOk inst) : source_a inst σ = .ok (aVal inst zc xs) σ := by
  rcases ha with ha | ha | ⟨ha, hr⟩
  · simp [source_a, aVal, ha, pure, EStateM.pure]
  · simp [source_a, aVal, ha, readReg_eq, hzc]
  · simp only [source_a, aVal, ha]
    exact rX_ok _ σ xs hxs hr

theorem source_b_ok (inst : zisk_inst) (a : BitVec 64) (σ : State) (zc : BitVec 64)
    (xs : Vector (BitVec 64) 32)
    (hzc : σ.regs.get? Register.Zc = some zc) (hxs : σ.regs.get? Register.Xs = some xs)
    (hb : BSrcOk inst) : source_b inst a σ = .ok (bVal inst zc xs) σ := by
  rcases hb with hb | hb | ⟨hb, hr⟩
  · simp [source_b, bVal, hb, pure, EStateM.pure]
  · simp [source_b, bVal, hb, readReg_eq, hzc]
  · simp only [source_b, bVal, hb]
    exact rX_ok _ σ xs hxs hr

/-- The value a store writes: `c`, or `pc + jmp_off2` for `store_pc`. -/
def storeVal (inst : zisk_inst) (pc c : BitVec 64) : BitVec 64 :=
  bif inst.store_pc then pc + inst.jmp_off2 else c

/-- The register file after the store. -/
def storeXs (inst : zisk_inst) (pc c : BitVec 64) (xs : Vector (BitVec 64) 32) :
    Vector (BitVec 64) 32 :=
  match inst.store with
  | .ST_REG => xs.set! (regIdx inst.store_off).toNat (storeVal inst pc c)
  | _ => xs

theorem store_c_ok (inst : zisk_inst) (a c : BitVec 64) (σ : State) (pc : BitVec 64)
    (xs : Vector (BitVec 64) 32)
    (hpc : σ.regs.get? Register.PC = some pc) (hxs : σ.regs.get? Register.Xs = some xs)
    (hs : StoreOk inst) :
    ∃ σ₁, store_c inst a c σ = .ok () σ₁ ∧
      σ₁.regs.get? Register.PC = σ.regs.get? Register.PC ∧
      σ₁.regs.get? Register.Zc = σ.regs.get? Register.Zc ∧
      σ₁.regs.get? Register.Step = σ.regs.get? Register.Step ∧
      σ₁.regs.get? Register.Xs = some (storeXs inst pc c xs) := by
  have hv : value_to_store inst c σ = .ok (storeVal inst pc c) σ := by
    unfold value_to_store storeVal
    cases inst.store_pc <;> simp [readReg_eq, hpc, bind, EStateM.bind, pure, EStateM.pure]
  rcases hs with hs | ⟨hs, hr⟩
  · refine ⟨σ, ?_, rfl, rfl, rfl, by simp [storeXs, hs, hxs]⟩
    unfold store_c
    rw [bind_ok hv]
    simp [hs, pure, EStateM.pure]
  · have e : store_c inst a c σ = .ok () { σ with regs := σ.regs.insert Register.Xs (xs.set! (regIdx inst.store_off).toNat (storeVal inst pc c)) } := by
      unfold store_c
      rw [bind_ok hv]
      simp only [hs]
      exact wX_ok _ _ σ xs hxs hr
    exact ⟨_, e, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert],
      by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, storeXs, hs]⟩

/-- The pc after a step: `c + jmp_off1` for `set_pc`, else the flag's branch. -/
def nextPcV (inst : zisk_inst) (pc c : BitVec 64) (flag : Bool) : BitVec 64 :=
  bif inst.set_pc then c + inst.jmp_off1 else bif flag then pc + inst.jmp_off1 else pc + inst.jmp_off2

/-- **One step, in general.** For any op but `halt`, with immediate, `c` or
register sources, no store or a register store, and not the end: the step
computes `zisk_exec inst.op a b` of its sources, stores, and moves on. -/
theorem execute_gen (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (xs : Vector (BitVec 64) 32)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st) (hxs : σ.regs.get? Register.Xs = some xs)
    (ha : ASrcOk inst) (hb : BSrcOk inst) (hs : StoreOk inst)
    (hop : inst.op ≠ .ZOP_HALT ()) (hend : inst.is_end = false) :
    ∃ σ', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).1 ∧
      σ'.regs.get? Register.Zflag = some (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).2 ∧
      σ'.regs.get? Register.PC = some (nextPcV inst pc
        (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).1
        (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).2) ∧
      σ'.regs.get? Register.Step = some (BitVec.addInt st 1) ∧
      σ'.regs.get? Register.Xs = some (storeXs inst pc
        (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).1 xs) := by
  generalize hr : zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs) = r
  obtain ⟨c, flag⟩ := r
  obtain ⟨σ₁, hs₁, p₁, z₁, t₁, x₁⟩ := store_c_ok inst (aVal inst zc xs) c σ pc xs hpc hxs hs
  have ea := source_a_ok inst σ zc xs hzc hxs ha
  have eb := source_b_ok inst (aVal inst zc xs) σ zc xs hzc hxs hb
  simp only [zisk_execute, bind, EStateM.bind, ea, eb, hr, hs₁, pure, EStateM.pure]
  rw [hpc] at p₁
  rw [hzc] at z₁
  rw [hst] at t₁
  cases hsp : inst.set_pc <;> cases flag <;>
  · simp [writeReg_run, readReg_eq, Std.ExtDHashMap.get?_insert, t₁, x₁, hend, nextPcV, next_pc,
      p₁, hsp, bind, EStateM.bind, pure, EStateM.pure]

end Correspondence
