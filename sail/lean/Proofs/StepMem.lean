/-
  One step with register and memory sources and stores.

  `step_gen` (`StepRegs.lean`) with memory: the model side is `execute_mem`
  (`ModelMem.lean`); what a register or memory source puts in the lane's
  columns, and where a memory access lands, are inputs — the register chain
  and the memory bus give them, at the level of a run.
-/
import Proofs.MemMain

open Out.Functions

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- An instruction the pilot with memory covers: the six modeled ops, `pubout`
(given `PubProved`), and `halt` (which no lane can hold);
immediate, `c`, register or memory sources — `B_IND` of width 8 — with 32-bit
memory addresses; no store, a register store, or a memory store — `ST_IND` of
width 8 — with an offset below `2^32`; register indices `x1`–`x31` written as
such; `set_pc` with a short offset; not the end. -/
structure MemPilot (inst : zisk_inst) : Prop where
  a : ASrcOk inst ∨ (inst.a_src = .A_MEM ∧ inst.a_imm.toNat < 2 ^ 32)
  b : BSrcOk inst ∨
    ((inst.b_src = .B_MEM ∨ (inst.b_src = .B_IND ∧ inst.ind_width = 8)) ∧ inst.b_imm.toNat < 2 ^ 32)
  store : MemStoreOk inst
  store_reg : inst.store = .ST_REG → regIdx inst.store_off ≠ 0
  store_ind : inst.store = .ST_IND → inst.ind_width = 8
  op : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG () ∨ inst.op = .ZOP_ADD () ∨
    inst.op = .ZOP_LTU () ∨ inst.op = .ZOP_EQ () ∨ inst.op = .ZOP_AND () ∨ inst.op = .ZOP_HALT () ∨
    inst.op = .ZOP_PUBOUT ()
  set_pc : inst.set_pc = true → ShortJump inst.jmp_off1
  is_end : inst.is_end = false
  a_idx : inst.a_src = .A_REG → inst.a_imm.toNat < 32
  b_idx : inst.b_src = .B_REG → inst.b_imm.toNat < 32
  s_idx : inst.store = .ST_REG → inst.store_off.toNat < 32

theorem MemPilot.idx {inst : zisk_inst} (hp : MemPilot inst) : RegIdx inst where
  a ha := by
    refine ⟨?_, hp.a_idx ha⟩
    rcases hp.a with (h | h | ⟨-, h⟩) | ⟨h, -⟩
    · cases h.symm.trans ha
    · cases h.symm.trans ha
    · exact h
    · cases h.symm.trans ha
  b hb := by
    refine ⟨?_, hp.b_idx hb⟩
    rcases hp.b with (h | h | ⟨-, h⟩) | ⟨(h | ⟨h, -⟩), -⟩
    · cases h.symm.trans hb
    · cases h.symm.trans hb
    · exact h
    · cases h.symm.trans hb
    · cases h.symm.trans hb
  s := hp.s_idx

/-- Every instruction of `prog` is one the pilot with memory covers, with both
jumps landing in the 32-bit pc range. -/
def MemProg (prog : BitVec 64 → Option zisk_inst) : Prop :=
  ∀ pc inst, prog pc = some inst → MemPilot inst ∧ Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2

section Step

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

attribute [local instance] Lean.Grind.Semiring.natCast in
/-- **Main cannot execute `halt`.** Main knows two internal ops, `flag`
(`op = 0`) and `copyb` (`op = 1`); for `halt`'s opcode, `0xff`, its
internal-op constraints force `flag = 0` and `flag = 1` at once. So no lane
holds a `halt`: the pilot admits it vacuously. -/
theorem halt_impossible {r : Main.Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    {inst : zisk_inst} {pc : BitVec 64} (hslot : SlotHolds r l inst pc) (hop : inst.op = .ZOP_HALT ()) :
    False := by
  have e10 : (1 : F) - 0 = 1 := by grind
  have s12 := hslot.is_external_op
  have s13 := hslot.op
  simp only [hop, isExternal, opcode, bit, Bool.false_eq_true, if_false] at s12 s13
  have fz := hl.fz
  have fb := hl.fb
  rw [s12, s13] at fz fb
  have h255 : ((255 : Nat) : F) ≠ 0 := by
    intro h0; rw [← Lean.Grind.Semiring.natCast_zero] at h0
    have := natCast_inj33 (F := F) (by omega) (by omega) h0; omega
  have h254 : ((254 : Nat) : F) ≠ 0 := by
    intro h0; rw [← Lean.Grind.Semiring.natCast_zero] at h0
    have := natCast_inj33 (F := F) (by omega) (by omega) h0; omega
  rw [e10, Lean.Grind.Semiring.one_mul] at fz fb
  -- `255·flag = 0`, so `flag = 0`.
  have hf : r.flag l = 0 := by
    have := congrArg (· * ((255 : Nat) : F)⁻¹) fb
    simp only [Lean.Grind.Semiring.zero_mul] at this
    rw [Lean.Grind.CommSemiring.mul_comm, ← Lean.Grind.Semiring.mul_assoc,
      Lean.Grind.Field.inv_mul_cancel h255, Lean.Grind.Semiring.one_mul] at this
    exact this
  -- Then `1 - 255 = 0`, so `255 = 1`.
  rw [hf, e10, Lean.Grind.Semiring.mul_one] at fz
  have e : ((255 : Nat) : F) = ((1 : Nat) : F) := by
    rw [Lean.Grind.Semiring.natCast_one]
    generalize ((255 : Nat) : F) = y at fz ⊢
    clear fb h255 h254 s12 s13 hl hslot
    grind
  have := natCast_inj33 (F := F) (by omega) (by omega) e
  omega

/-- **One step with registers and memory, at one lane.** As `step_gen`, with
memory sources and stores: given what the lane's columns hold for a register
or memory source, and that memory accesses land in `D`, the step executes the
program's instruction as `zisk_execute` does, with the model's register file
and memory. -/
theorem step_genM (prog : BitVec 64 → Option zisk_inst) (hprog : MemProg prog)
    (xm : Main.Ctx F) (tm : Main.Trace F) {i : Int} (hmi : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    {l : Nat} (hl4 : l < 4) (hbus : LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩])
    (hrom : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds (opArg l) xm tm ps) (hpr : OpsProved ps)
    (hpub : PubProved ps)
    {p0 p1 np : F} (hlane : LaneHolds (tm i) l p0 p1 np)
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ np = ((q.toNat : Nat) : F))
    (σ : State) (pc zc : BitVec 64) (xs : Vector (BitVec 64) 32) {W : Nat → BitVec 64}
    {D : Nat → Prop} (hσ : Ready σ pc zc) (hxs : σ.regs.get? Register.Xs = some xs)
    (hm : MemOk σ W D) (hD : ∀ b, D b → b % 8 = 0)
    (hcol : (tm i).pc l = ((pc.toNat : Nat) : F)) (hprev : p0 = lo zc ∧ p1 = hi zc)
    (hAx : ∀ inst, prog pc = some inst → (inst.a_src = .A_REG ∨ inst.a_src = .A_MEM) →
      ACols (tm i) l (aValM inst zc xs W) ∧ (inst.a_src = .A_MEM → MemAt D inst.a_imm))
    (hBx : ∀ inst, prog pc = some inst →
      (inst.b_src = .B_REG ∨ inst.b_src = .B_MEM ∨ inst.b_src = .B_IND) →
      BCols (tm i) l (bValM inst zc xs W (aValM inst zc xs W)) ∧
        (inst.b_src = .B_MEM → MemAt D inst.b_imm) ∧
        (inst.b_src = .B_IND → MemAt D (aValM inst zc xs W + inst.b_imm)))
    (hSx : ∀ inst, prog pc = some inst →
      (inst.store = .ST_MEM → MemAt D inst.store_off) ∧
        (inst.store = .ST_IND → MemAt D (aValM inst zc xs W + inst.store_off))) :
    ∃ inst σ' pc' c, prog pc = some inst ∧ SlotHolds (tm i) l inst pc ∧
      StoreHolds (tm i) l inst ∧ MemPilot inst ∧ zisk_execute inst σ = .ok false σ' ∧
      Ready σ' pc' c ∧ σ'.regs.get? Register.Xs = some (storeXs inst pc c xs) ∧
      MemOk σ' (storeW inst pc c (aValM inst zc xs W) W) D ∧
      c = (zisk_exec inst.op (aValM inst zc xs W) (bValM inst zc xs W (aValM inst zc xs W))).1 ∧
      np = ((pc'.toNat : Nat) : F) ∧ (tm i).c l 0 = lo c ∧ (tm i).c l 1 = hi c ∧
      ACols (tm i) l (aValM inst zc xs W) := by
  obtain ⟨hpc, hzc, ⟨st, hst⟩, hpc32⟩ := hσ
  obtain ⟨inst, hinst, hslot, hsto⟩ :=
    rom_lane' prog xm tm i hmi xr tr hr hl4 hbus hrom pc hpc32 hcol
  obtain ⟨hpil, hl1, hl2⟩ := hprog pc inst hinst
  have hop : inst.op ≠ .ZOP_HALT () := fun h => halt_impossible hlane hslot h
  -- The source columns.
  have hA : ACols (tm i) l (aValM inst zc xs W) := by
    rcases hpil.a with ((h | h | ⟨h, -⟩) | ⟨h, -⟩)
    · have := a_cols_imm_c hlane hslot hprev (.inl h) xs
      simp only [aValM, h]; exact this
    · have := a_cols_imm_c hlane hslot hprev (.inr h) xs
      simp only [aValM, h]; exact this
    · exact (hAx inst hinst (.inl h)).1
    · exact (hAx inst hinst (.inr h)).1
  have hB : BCols (tm i) l (bValM inst zc xs W (aValM inst zc xs W)) := by
    rcases hpil.b with ((h | h | ⟨h, -⟩) | ⟨(h | ⟨h, -⟩), -⟩)
    · have := b_cols_imm_c hlane hslot hprev (.inl h) xs
      simp only [bValM, h]; exact this
    · have := b_cols_imm_c hlane hslot hprev (.inr h) xs
      simp only [bValM, h]; exact this
    · exact (hBx inst hinst (.inl h)).1
    · exact (hBx inst hinst (.inr (.inl h))).1
    · exact (hBx inst hinst (.inr (.inr h))).1
  -- The lane's `c` and flag.
  have hcf : (tm i).c l 0 = lo (zisk_exec inst.op (aValM inst zc xs W)
        (bValM inst zc xs W (aValM inst zc xs W))).1 ∧
      (tm i).c l 1 = hi (zisk_exec inst.op (aValM inst zc xs W)
        (bValM inst zc xs W (aValM inst zc xs W))).1 ∧
      (tm i).flag l = bit (zisk_exec inst.op (aValM inst zc xs W)
        (bValM inst zc xs W (aValM inst zc xs W))).2 := by
    rcases hpil.op with h | h | h | h | h | h | h | h
    · exact lane_int_cols hlane hslot (.inl h) hB
    · exact lane_int_cols hlane hslot (.inr h) hB
    · exact ext_cols inst (.inl h) hl4 ps hops hpr hslot hA hB
    · exact ext_cols inst (.inr (.inl h)) hl4 ps hops hpr hslot hA hB
    · exact ext_cols inst (.inr (.inr (.inl h))) hl4 ps hops hpr hslot hA hB
    · exact ext_cols inst (.inr (.inr (.inr h))) hl4 ps hops hpr hslot hA hB
    · exact absurd h hop
    · rw [h, exec_pubout]
      exact lane_ext_cols 0xA0 pubSem hl4 ps hops hpub hslot (by rw [h]; rfl) (by rw [h]; rfl) hA hB
  obtain ⟨hc0, hc1, hf⟩ := hcf
  -- The next pc.
  obtain ⟨hnp, hnp32⟩ := lane_npc hlane hslot hc0 hc1 hf hpil.set_pc (fun _ => ⟨hl1, hl2⟩) hq
  -- The model.
  have ha : ASrcOkM D inst := by
    rcases hpil.a with h | ⟨h, -⟩
    · exact .inl h
    · exact .inr ⟨h, (hAx inst hinst (.inr h)).2 h⟩
  have hb : BSrcOkM D inst (aValM inst zc xs W) := by
    rcases hpil.b with h | ⟨(h | ⟨h, hw⟩), -⟩
    · exact .inl h
    · exact .inr (.inl ⟨h, (hBx inst hinst (.inr (.inl h))).2.1 h⟩)
    · exact .inr (.inr ⟨h, hw, (hBx inst hinst (.inr (.inr h))).2.2 h⟩)
  have hs : StoreOkM D inst (aValM inst zc xs W) := by
    rcases hpil.store with h | h | ⟨(h | h), -⟩
    · exact .inl (.inl h)
    · exact .inl (.inr ⟨h, hpil.store_reg h⟩)
    · exact .inr (.inl ⟨h, (hSx inst hinst).1 h⟩)
    · exact .inr (.inr ⟨h, hpil.store_ind h, (hSx inst hinst).2 h⟩)
  obtain ⟨σ', hrun, hz, hp, hs', hx, hmem⟩ :=
    execute_mem inst σ pc zc st xs hm hD hpc hzc hst hxs ha hb hs hop hpil.is_end
  exact ⟨inst, σ', _, _, hinst, hslot, hsto, hpil, hrun, ⟨hp, hz, ⟨_, hs'⟩, hnp32⟩, hx, hmem, rfl,
    hnp, hc0, hc1, hA⟩

end Step

end Correspondence
