/-
  The pilot for every modeled external op: `add`, `ltu`, `eq`, `and`.

  `AddPilot.lean` proved slot 1 of Main executing an `add`. The argument did
  not depend on `add` beyond two facts: what the op does (`zisk_exec`, here a
  `sem` on 64-bit values) and that every prover the operation bus can answer
  from computes it (`OpProves code sem`). `slot1_external` states it once,
  with those two as hypotheses; the four instances below supply them.

  For `ltu` and `eq` the flag can be set, so the step may branch: the model's
  next pc is `pc + jmp_off1` when the flag is set, and Main's pc transition
  says the same through `flag·(jmp_offset1 − jmp_offset2)`.
-/
import Proofs.Ops

open Out.Functions

namespace Correspondence

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- What one step of the model does for a modeled external op with the
pilot's sources: `(c, flag)` is the op of the sources, and the pc branches on
the flag. -/
theorem execute_ext (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_ADD () ∨ inst.op = .ZOP_LTU () ∨ inst.op = .ZOP_EQ () ∨
      inst.op = .ZOP_AND ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false) :
    let r := zisk_exec inst.op (srcA inst zc) (srcB inst zc)
    ∃ σ', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some r.1 ∧
      σ'.regs.get? Register.Zflag = some r.2 ∧
      σ'.regs.get? Register.PC = some (bif r.2 then pc + inst.jmp_off1 else pc + inst.jmp_off2) ∧
      σ'.regs.get? Register.Step = some (Sail.BitVec.addInt st 1) := by
  rcases ha with ha | ha <;> rcases hb with hb | hb <;>
  rcases hop with hop | hop | hop | hop <;> cases hsp : inst.store_pc <;>
  · cases hfl : (zisk_exec inst.op (srcA inst zc) (srcB inst zc)).2 <;>
    simp only [hop, ha, hb, srcA, srcB] at hfl <;>
    simp [zisk_execute, source_a, source_b, store_c, value_to_store, next_pc,
      ha, hb, hop, hstore, hset, hend, hsp, srcA, srcB, bind, EStateM.bind, pure, EStateM.pure,
      readReg_eq, writeReg_run, hpc, hzc, hst, Std.ExtDHashMap.get?_insert, hfl]

section Pilot

open Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- Lane `l`'s operation-bus lookup, `Main.bus46`, `bus59`, `bus72` and
`bus85`: the same tuple, at each lane's columns. -/
def opArg : Nat → BusArg F (Main.Ctx F) (Main.Trace F)
  | 0 => Main.bus46
  | 1 => Main.bus59
  | 2 => Main.bus72
  | _ => Main.bus85

omit [Lean.Grind.IsCharP F goldilocks] in
theorem opArg_reps {l : Nat} (hl : l < 4) (x : Main.Ctx F) (t : Main.Trace F) (i : Int) :
    (opArg l).reps x t i = (t i).is_external_op l + 0 := by
  rcases l with _ | _ | _ | _ | l
  all_goals first | rfl | omega

omit [Lean.Grind.IsCharP F goldilocks] in
theorem opArg_bus (l : Nat) (x : Main.Ctx F) (t : Main.Trace F) (i : Int) :
    (opArg l).bus x t i = 5000 := by
  rcases l with _ | _ | _ | l <;> rfl

omit [Lean.Grind.IsCharP F goldilocks] in
/-- The elements of lane `l`'s operation tuple: `(op, a, b, c, flag)`, the high
limbs of `a` and `b` masked by `m32`. -/
theorem opArg_tuple {l : Nat} (hl : l < 4) (x : Main.Ctx F) (t : Main.Trace F) (i : Int) :
    tupleAt (opArg l).tuple x t i 0 = (t i).op l + 0 ∧
      tupleAt (opArg l).tuple x t i 1 = (t i).a l 0 + 0 ∧
      tupleAt (opArg l).tuple x t i 2 = (1 - (t i).m32 l) * (t i).a l 1 ∧
      tupleAt (opArg l).tuple x t i 3 = (t i).b l 0 + 0 ∧
      tupleAt (opArg l).tuple x t i 4 = (1 - (t i).m32 l) * (t i).b l 1 ∧
      tupleAt (opArg l).tuple x t i 5 = (t i).c l 0 + 0 ∧
      tupleAt (opArg l).tuple x t i 6 = (t i).c l 1 + 0 ∧
      tupleAt (opArg l).tuple x t i 7 = (t i).flag l + 0 := by
  rcases l with _ | _ | _ | _ | l
  all_goals first
    | exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
    | omega

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A lane's `a` and `b` limbs are the model's sources, for immediate and `c`
sources: its source constraints, `main.pil:456`–`461`. -/
theorem lane_sources {r : Main.Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    (inst : zisk_inst) (pc zc : BitVec 64) (hslot : SlotHolds r l inst pc)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hprev : p0 = lo zc ∧ p1 = hi zc) :
    (r.a l 0 = lo (srcA inst zc) ∧ r.a l 1 = hi (srcA inst zc)) ∧
      (r.b l 0 = lo (srcB inst zc) ∧ r.b l 1 = hi (srcB inst zc)) := by
  obtain ⟨h41, h42, h43, h44, -, -, h47, h48, h49, h50, -, -, -, -, -⟩ := hl
  obtain ⟨rfl, rfl⟩ := hprev
  rcases hslot with ⟨s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, -, -, -, -, -, -, -⟩
  rcases ha with ha | ha <;> rcases hb with hb | hb <;>
  · simp only [ha, hb, bit, isAImm, isAMem, isAReg, isBImm, isBMem, isBInd, isBReg,
      Bool.false_eq_true, if_false, if_true, srcA, srcB] at s1 s2 s3 s6 s7 s8 s9 ⊢
    simp only [s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] at h41 h42 h43 h44 h47 h48 h49 h50
    refine ⟨⟨?_, ?_⟩, ?_, ?_⟩ <;> grind

/-- **One lane, one external op.** If lane `l` of row `i` holds a modeled
external op the model is about to execute, with immediate or `c` sources, the
previous step's `c` is the model's, and every prover the operation bus can
answer from computes the op (`OpProves code sem`), then the lane's `c` and
`flag`, and the next step's `pc`, are exactly what `zisk_execute` computes. -/
theorem lane_external (code : Nat) (sem : BitVec 64 → BitVec 64 → BitVec 64 × Bool)
    {xm : Main.Ctx F} {tm : Main.Trace F} {i : Int} {l : Nat} {p0 p1 np : F} (hl4 : l < 4)
    (hl : LaneHolds (tm i) l p0 p1 np)
    (ps : List (Prover F)) (hops : LookupHolds (opArg l) xm tm ps)
    (hpr : ∀ p ∈ ps, OpProves code sem p)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (hslot : SlotHolds (tm i) l inst pc)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_ADD () ∨ inst.op = .ZOP_LTU () ∨ inst.op = .ZOP_EQ () ∨
      inst.op = .ZOP_AND ())
    (hcode : opcode inst.op = code) (hsem : ∀ a b, zisk_exec inst.op a b = sem a b)
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hprev : p0 = lo zc ∧ p1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      σ'.regs.get? Register.Step = some (Sail.BitVec.addInt st 1) ∧
      (pc' = pc + inst.jmp_off1 ∨ pc' = pc + inst.jmp_off2) ∧
      (tm i).c l 0 = lo c ∧ (tm i).c l 1 = hi c ∧ (tm i).flag l = bit flag ∧
      np = ((pc'.toNat : Nat) : F) := by
  -- The model.
  obtain ⟨σ', hrun, hc, hf, hp, hs⟩ :=
    execute_ext inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend
  refine ⟨σ', _, _, _, hrun, hc, hf, hp, hs, by cases (zisk_exec inst.op _ _).2 <;> simp, ?_⟩
  rw [hsem]
  -- The lane's sources are the model's.
  obtain ⟨⟨hA0, hA1⟩, hB0, hB1⟩ := lane_sources hl inst pc zc hslot ha hb hprev
  have hnpc := hl.npc
  obtain ⟨-, -, -, -, -, -, -, -, -, -, -, s12, s13, s14, s15, s16, s17, s18⟩ := hslot
  have hext : isExternal inst.op = true := by
    rcases hop with h | h | h | h <;> rw [h] <;> rfl
  have hm32 : isM32 inst.op = false := rfl
  rw [hext] at s12
  rw [hcode] at s13
  rw [hm32] at s18
  simp only [hset, bit, if_true, if_false, Bool.false_eq_true] at s12 s14 s18
  -- The operation bus: some prover answers the lane's tuple, and computes the op.
  obtain ⟨t0, t1, t2, t3, t4, t5, t6, t7⟩ := opArg_tuple hl4 xm tm i
  obtain ⟨-, -, -, -, hb'⟩ := hops
  obtain ⟨p, hp', j, hreps, hpb, htup⟩ := hb' i (by
    rw [opArg_reps hl4, s12, Lean.Grind.Semiring.add_zero]; exact one_ne_zero')
  have hprove := hpr p hp' j (by rw [hpb, opArg_bus]) hreps
  have e0 := (htup 0).symm; have e1 := (htup 1).symm; have e2 := (htup 2).symm
  have e3 := (htup 3).symm; have e4 := (htup 4).symm; have e5 := (htup 5).symm
  have e6 := (htup 6).symm; have e7 := (htup 7).symm
  rw [t0, Lean.Grind.Semiring.add_zero] at e0
  rw [t1, Lean.Grind.Semiring.add_zero] at e1
  rw [t2] at e2
  rw [t3, Lean.Grind.Semiring.add_zero] at e3
  rw [t4] at e4
  rw [t5, Lean.Grind.Semiring.add_zero] at e5
  rw [t6, Lean.Grind.Semiring.add_zero] at e6
  rw [t7, Lean.Grind.Semiring.add_zero] at e7
  obtain ⟨r5, r6, r7⟩ := hprove
    (by rw [e0, s13])
    (srcA inst zc) (srcB inst zc)
    (by rw [e1, hA0])
    (by rw [e2, s18, one_sub_zero_mul, hA1])
    (by rw [e3, hB0])
    (by rw [e4, s18, one_sub_zero_mul, hB1])
  rw [e5] at r5
  rw [e6] at r6
  rw [e7] at r7
  refine ⟨r5, r6, r7, ?_⟩
  -- The pc: no `set_pc`, so the next step runs at `pc + jmp_off2`, or `jmp_off1` on a flag.
  rw [s14, r7, s17, s15, s16] at hnpc
  cases hfl : (sem (srcA inst zc) (srcB inst zc)).2
  · rw [cond_false, pc_add pc inst.jmp_off2 hwrap.2]
    simp only [hfl, bit, Bool.false_eq_true, if_false] at hnpc
    grind
  · rw [cond_true, pc_add pc inst.jmp_off1 hwrap.1]
    simp only [hfl, bit, if_true] at hnpc
    grind

/-- **One slot, one external op.** `lane_external` at slot 1, with the
instruction reached through the ROM lookup: as `slot1_add`, for any modeled
external op whose result is `sem` and whose every prover on the operation bus
computes it. -/
theorem slot1_external (code : Nat) (sem : BitVec 64 → BitVec 64 → BitVec 64 × Bool)
    (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hrom : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hromh : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds Main.bus59 xm tm ps)
    (hpr : ∀ p ∈ ps, OpProves code sem p)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (hprog : prog pc = some inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpcol : (tm i).pc 1 = ((pc.toNat : Nat) : F))
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_ADD () ∨ inst.op = .ZOP_LTU () ∨ inst.op = .ZOP_EQ () ∨
      inst.op = .ZOP_AND ())
    (hcode : opcode inst.op = code) (hsem : ∀ a b, zisk_exec inst.op a b = sem a b)
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hprev : (tm i).c 0 0 = lo zc ∧ (tm i).c 0 1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      (tm i).c 1 0 = lo c ∧ (tm i).c 1 1 = hi c ∧ (tm i).flag 1 = bit flag ∧
      (tm i).pc 2 = ((pc'.toNat : Nat) : F) := by
  obtain ⟨σ', c, f, pc', h1, h2, h3, h4, -, -, h5⟩ :=
    lane_external code sem (l := 1) (by decide) (laneHolds_1 hm) ps hops hpr inst σ pc zc st
      hpc hzc hst (slotHolds_of_rom prog xm tm i hm xr tr hr hrom hromh pc inst hpc32 hpcol hprog)
      ha hb hop hcode hsem hstore hset hend hprev hwrap
  exact ⟨σ', c, f, pc', h1, h2, h3, h4, h5⟩

/-! ### The four modeled external ops -/

/-- `ltu` — reached through the operation bus. -/
theorem slot1_ltu (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hrom : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hromh : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds Main.bus59 xm tm ps)
    (hpr : ∀ p ∈ ps, OpProves 6 ltuSem p)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (hprog : prog pc = some inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpcol : (tm i).pc 1 = ((pc.toNat : Nat) : F))
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_LTU ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hprev : (tm i).c 0 0 = lo zc ∧ (tm i).c 0 1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      (tm i).c 1 0 = lo c ∧ (tm i).c 1 1 = hi c ∧ (tm i).flag 1 = bit flag ∧
      (tm i).pc 2 = ((pc'.toNat : Nat) : F) :=
  slot1_external 6 ltuSem prog xm tm i hm xr tr hr hrom hromh ps hops hpr inst σ pc zc st
    hpc hzc hst hprog hpc32 hpcol ha hb (Or.inr (Or.inl hop)) (by rw [hop]; rfl)
    (by rw [hop]; exact exec_ltu) hstore hset hend hprev hwrap

/-- `eq` — reached through the operation bus. -/
theorem slot1_eq (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hrom : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hromh : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds Main.bus59 xm tm ps)
    (hpr : ∀ p ∈ ps, OpProves 9 eqSem p)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (hprog : prog pc = some inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpcol : (tm i).pc 1 = ((pc.toNat : Nat) : F))
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_EQ ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hprev : (tm i).c 0 0 = lo zc ∧ (tm i).c 0 1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      (tm i).c 1 0 = lo c ∧ (tm i).c 1 1 = hi c ∧ (tm i).flag 1 = bit flag ∧
      (tm i).pc 2 = ((pc'.toNat : Nat) : F) :=
  slot1_external 9 eqSem prog xm tm i hm xr tr hr hrom hromh ps hops hpr inst σ pc zc st
    hpc hzc hst hprog hpc32 hpcol ha hb (Or.inr (Or.inr (Or.inl hop))) (by rw [hop]; rfl)
    (by rw [hop]; exact exec_eq) hstore hset hend hprev hwrap

/-- `and` — reached through the operation bus. -/
theorem slot1_and (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hrom : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hromh : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds Main.bus59 xm tm ps)
    (hpr : ∀ p ∈ ps, OpProves 14 andSem p)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (hprog : prog pc = some inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpcol : (tm i).pc 1 = ((pc.toNat : Nat) : F))
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_AND ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hprev : (tm i).c 0 0 = lo zc ∧ (tm i).c 0 1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      (tm i).c 1 0 = lo c ∧ (tm i).c 1 1 = hi c ∧ (tm i).flag 1 = bit flag ∧
      (tm i).pc 2 = ((pc'.toNat : Nat) : F) :=
  slot1_external 14 andSem prog xm tm i hm xr tr hr hrom hromh ps hops hpr inst σ pc zc st
    hpc hzc hst hprog hpc32 hpcol ha hb (Or.inr (Or.inr (Or.inr hop))) (by rw [hop]; rfl)
    (by rw [hop]; exact exec_and) hstore hset hend hprev hwrap

end Pilot

end Correspondence
