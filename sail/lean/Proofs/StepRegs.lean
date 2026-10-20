/-
  One step with register sources and stores.

  `step_core` (`Run.lean`) is one step of the pilot's programs: immediate or
  `c` sources, no store. Here sources and the store may also be registers.
  The model's side is `execute_gen` (`ModelRegs.lean`); Main's is the lane
  over known source columns (`LaneGen.lean`). What a register source puts in
  the lane's columns is an input (`hAreg`, `hBreg`): the register chain gives
  it, at the level of a run.
-/
import Proofs.LaneGen

open Out.Functions

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- An instruction the register-aware pilot covers: the six modeled ops,
immediate, `c` or register sources, no store or a register store, register
indices `x1`–`x31` written as such, `set_pc` with a short offset, on any op,
not the end. -/
structure RegPilot (inst : zisk_inst) : Prop where
  a : ASrcOk inst
  b : BSrcOk inst
  store : StoreOk inst
  op : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG () ∨ inst.op = .ZOP_ADD () ∨
    inst.op = .ZOP_LTU () ∨ inst.op = .ZOP_EQ () ∨ inst.op = .ZOP_AND ()
  set_pc : inst.set_pc = true → ShortJump inst.jmp_off1
  is_end : inst.is_end = false
  a_idx : inst.a_src = .A_REG → inst.a_imm.toNat < 32
  b_idx : inst.b_src = .B_REG → inst.b_imm.toNat < 32
  s_idx : inst.store = .ST_REG → inst.store_off.toNat < 32

/-- Every instruction of `prog` is one the register-aware pilot covers, with
both jumps landing in the 32-bit pc range. -/
def RegProg (prog : BitVec 64 → Option zisk_inst) : Prop :=
  ∀ pc inst, prog pc = some inst → RegPilot inst ∧ Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2

section Step

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- What an external op computes, as the operation bus's provers prove it. -/
theorem ext_cols (inst : zisk_inst)
    (hop : inst.op = .ZOP_ADD () ∨ inst.op = .ZOP_LTU () ∨ inst.op = .ZOP_EQ () ∨
      inst.op = .ZOP_AND ())
    {xm : Main.Ctx F} {tm : Main.Trace F} {i : Int} {l : Nat} (hl4 : l < 4)
    (ps : List (Prover F)) (hops : LookupHolds (opArg l) xm tm ps) (hpr : OpsProved ps)
    {pc : BitVec 64} (hslot : SlotHolds (tm i) l inst pc)
    {A B : BitVec 64} (hA : ACols (tm i) l A) (hB : BCols (tm i) l B) :
    (tm i).c l 0 = lo (zisk_exec inst.op A B).1 ∧ (tm i).c l 1 = hi (zisk_exec inst.op A B).1 ∧
      (tm i).flag l = bit (zisk_exec inst.op A B).2 := by
  rcases hop with h | h | h | h
  · rw [h, exec_add]
    exact lane_ext_cols 10 addSem hl4 ps hops (fun p hp => (hpr p hp).add) hslot
      (by rw [h]; rfl) (by rw [h]; rfl) hA hB
  · rw [h, exec_ltu]
    exact lane_ext_cols 6 ltuSem hl4 ps hops (fun p hp => (hpr p hp).ltu) hslot
      (by rw [h]; rfl) (by rw [h]; rfl) hA hB
  · rw [h, exec_eq]
    exact lane_ext_cols 9 eqSem hl4 ps hops (fun p hp => (hpr p hp).eq) hslot
      (by rw [h]; rfl) (by rw [h]; rfl) hA hB
  · rw [h, exec_and]
    exact lane_ext_cols 14 andSem hl4 ps hops (fun p hp => (hpr p hp).and) hslot
      (by rw [h]; rfl) (by rw [h]; rfl) hA hB

/-- **One step with registers, at one lane.** As `step_core`, with register
sources and stores: given what the lane's columns hold for a register source,
the step executes the program's instruction as `zisk_execute` does, with the
model's register file, and leaves the lane's `c` and the next pc as the model
does. -/
theorem step_gen (prog : BitVec 64 → Option zisk_inst) (hprog : RegProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) {i : Int} (hmi : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    {l : Nat} (hl4 : l < 4) (hbus : LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds (opArg l) xm tm ps) (hpr : OpsProved ps)
    {p0 p1 np : F} (hlane : LaneHolds (tm i) l p0 p1 np)
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ np = ((q.toNat : Nat) : F))
    (σ : State) (pc zc : BitVec 64) (xs : Vector (BitVec 64) 32) (hσ : Ready σ pc zc)
    (hxs : σ.regs.get? Register.Xs = some xs)
    (hcol : (tm i).pc l = ((pc.toNat : Nat) : F)) (hprev : p0 = lo zc ∧ p1 = hi zc)
    (hAreg : ∀ inst, prog pc = some inst → inst.a_src = .A_REG →
      ACols (tm i) l (xs[(regIdx inst.a_imm).toNat]!))
    (hBreg : ∀ inst, prog pc = some inst → inst.b_src = .B_REG →
      BCols (tm i) l (xs[(regIdx inst.b_imm).toNat]!)) :
    ∃ inst σ' pc' c, prog pc = some inst ∧ SlotHolds (tm i) l inst pc ∧
      StoreHolds (tm i) l inst ∧ RegPilot inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' c ∧ σ'.regs.get? Register.Xs = some (storeXs inst pc c xs) ∧
      c = (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).1 ∧
      np = ((pc'.toNat : Nat) : F) ∧ (tm i).c l 0 = lo c ∧ (tm i).c l 1 = hi c := by
  obtain ⟨hpc, hzc, ⟨st, hst⟩, hpc32⟩ := hσ
  obtain ⟨inst, hinst, hslot, hsto⟩ :=
    rom_lane' prog xm tm i hmi xr tr hr hl4 hbus hrom pc hpc32 hcol
  obtain ⟨hpil, hl1, hl2⟩ := hprog pc inst hinst
  -- The source columns.
  have hA : ACols (tm i) l (aVal inst zc xs) := by
    rcases hpil.a with h | h | ⟨h, -⟩
    · exact a_cols_imm_c hlane hslot hprev (.inl h) xs
    · exact a_cols_imm_c hlane hslot hprev (.inr h) xs
    · have := hAreg inst hinst h
      simp only [aVal, h]
      exact this
  have hB : BCols (tm i) l (bVal inst zc xs) := by
    rcases hpil.b with h | h | ⟨h, -⟩
    · exact b_cols_imm_c hlane hslot hprev (.inl h) xs
    · exact b_cols_imm_c hlane hslot hprev (.inr h) xs
    · have := hBreg inst hinst h
      simp only [bVal, h]
      exact this
  -- The lane's `c` and flag.
  have hcf : (tm i).c l 0 = lo (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).1 ∧
      (tm i).c l 1 = hi (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).1 ∧
      (tm i).flag l = bit (zisk_exec inst.op (aVal inst zc xs) (bVal inst zc xs)).2 := by
    rcases hpil.op with h | h | h | h | h | h
    · exact lane_int_cols hlane hslot (.inl h) hB
    · exact lane_int_cols hlane hslot (.inr h) hB
    · exact ext_cols inst (.inl h) hl4 ps hops hpr hslot hA hB
    · exact ext_cols inst (.inr (.inl h)) hl4 ps hops hpr hslot hA hB
    · exact ext_cols inst (.inr (.inr (.inl h))) hl4 ps hops hpr hslot hA hB
    · exact ext_cols inst (.inr (.inr (.inr h))) hl4 ps hops hpr hslot hA hB
  obtain ⟨hc0, hc1, hf⟩ := hcf
  -- The next pc.
  obtain ⟨hnp, hnp32⟩ := lane_npc hlane hslot hc0 hc1 hf hpil.set_pc
    (fun _ => ⟨hl1, hl2⟩) hq
  -- The model.
  have hop : inst.op ≠ .ZOP_HALT () := by
    rcases hpil.op with h | h | h | h | h | h <;> rw [h] <;> exact fun e => nomatch e
  obtain ⟨σ', hrun, hz, -, hp, hs, hx⟩ :=
    execute_gen inst σ pc zc st xs hpc hzc hst hxs hpil.a hpil.b hpil.store hop hpil.is_end
  exact ⟨inst, σ', _, _, hinst, hslot, hsto, hpil, hrun, ⟨hp, hz, ⟨_, hs⟩, hnp32⟩, hx, rfl, hnp,
    hc0, hc1⟩

end Step

end Correspondence
