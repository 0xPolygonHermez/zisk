/-
  The `add` pilot: one slot of Main executing an external op.

  `Slot.lean` and `Rom.lean` proved one slot of Main against one step of the
  model for the internal ops, whose result Main computes itself. This file does
  it for `add`, whose result Main does not compute: it sends `(op, a, b, c,
  flag)` over the operation bus (5000), and one of forty AIRs proves the tuple.
  So the theorem needs a fact about whichever AIR answers, and the files before
  this one supply it for every kind:

  * the 21 adder lanes (`Adders.lean`) and the 7 Binary lanes (`Binary.lean`)
    prove `AddsAt` at every row;
  * the 33 provers that cannot prove an `add` (`OpBus.lean`) never send op 10,
    so `AddsAt` holds for them vacuously;
  * the FROPS tables hold, for every `add` row, `c = a + b`, which
    `check_tables.py` checks (`FropsAdds`).

  `OpAdds p` is the common form: on the operation bus, `p` proves `add`
  correctly at every row. `opAdds_of_adds`, `opAdds_of_not` and
  `opAdds_of_frops` turn each of the three into it, and `slot1_add` takes it as
  a hypothesis over whatever list of provers the lookup answers from.
-/
import Proofs.Binary
import Proofs.OpBus

namespace Correspondence

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

section Provers

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- On the operation bus, `p` proves `add` correctly at every row. -/
def OpAdds (p : Prover F) : Prop :=
  ∀ j, p.arg.bus p.x p.t j = 5000 → AddsAt p j

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A prover that proves `add` correctly everywhere (an adder, a Binary lane). -/
theorem opAdds_of_adds {p : Prover F} (h : ∀ j, AddsAt p j) : OpAdds p :=
  fun j _ => h j

omit [Lean.Grind.IsCharP F goldilocks] in
/-- The op a bus argument sends is its tuple's element 0. -/
theorem opOf_tupleAt {C T : Type} (a : BusArg F C T) (x : C) (t : T) (i : Int) :
    opOf a x t i = tupleAt a.tuple x t i 0 := by
  unfold opOf tupleAt
  cases a.tuple <;> rfl

omit [Lean.Grind.IsCharP F goldilocks] in
/-- A prover whose op is never `add` (the exclusions of `OpBus.lean`). -/
theorem opAdds_of_not {p : Prover F}
    (h : ∀ j, opOf p.arg p.x p.t j ≠ ((10 : Int) : F)) : OpAdds p := by
  intro j _ hop
  exfalso
  apply h j
  rw [intCast_lit, opOf_tupleAt]
  exact hop

/-- What `check_tables.py` checks of a FROPS table: every row on the operation
bus whose op is `add` carries 32-bit limbs with `c = a + b` modulo 2^64, and
flag 0. -/
def FropsAdds (p : Prover F) : Prop :=
  ∀ j, p.arg.bus p.x p.t j = 5000 → tupleAt p.arg.tuple p.x p.t j 0 = 10 →
    ∃ a0 a1 b0 b1 c0 c1 : Nat, a0 < 2 ^ 32 ∧ a1 < 2 ^ 32 ∧ b0 < 2 ^ 32 ∧ b1 < 2 ^ 32 ∧
      c0 < 2 ^ 32 ∧ c1 < 2 ^ 32 ∧
      tupleAt p.arg.tuple p.x p.t j 1 = (a0 : F) ∧ tupleAt p.arg.tuple p.x p.t j 2 = (a1 : F) ∧
      tupleAt p.arg.tuple p.x p.t j 3 = (b0 : F) ∧ tupleAt p.arg.tuple p.x p.t j 4 = (b1 : F) ∧
      tupleAt p.arg.tuple p.x p.t j 5 = (c0 : F) ∧ tupleAt p.arg.tuple p.x p.t j 6 = (c1 : F) ∧
      tupleAt p.arg.tuple p.x p.t j 7 = 0 ∧
      c0 + 4294967296 * c1 = (a0 + 4294967296 * a1 + (b0 + 4294967296 * b1)) % 2 ^ 64

/-- A FROPS table's rows are correct `add`s. -/
theorem opAdds_of_frops {p : Prover F} (h : FropsAdds p) : OpAdds p := by
  intro j hbus hop A B ha0 ha1 hb0 hb1
  obtain ⟨a0, a1, b0, b1, c0, c1, la0, la1, lb0, lb1, lc0, lc1, e1, e2, e3, e4, e5, e6, e7, hsum⟩ :=
    h j hbus hop
  have hA := A.isLt
  have hB := B.isLt
  have nA0 : a0 = A.toNat % 2 ^ 32 := nat_eq (by omega) (by omega) (e1.symm.trans ha0)
  have nA1 : a1 = A.toNat / 2 ^ 32 := nat_eq (by omega) (by omega) (e2.symm.trans ha1)
  have nB0 : b0 = B.toNat % 2 ^ 32 := nat_eq (by omega) (by omega) (e3.symm.trans hb0)
  have nB1 : b1 = B.toNat / 2 ^ 32 := nat_eq (by omega) (by omega) (e4.symm.trans hb1)
  have hs := sum_limbs A B
  have m0 : c0 = (A + B).toNat % 2 ^ 32 := by omega
  have m1 : c1 = (A + B).toNat / 2 ^ 32 := by omega
  exact ⟨by rw [e5, lo, m0], by rw [e6, hi, m1], e7⟩

end Provers

section Pilot

open Out.Functions Pil.Zisk

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

omit [Lean.Grind.IsCharP F goldilocks] in
theorem one_sub_zero_mul (x : F) : (1 - 0) * x = x := by grind

/-- Slot 1's `a` and `b` limbs are the model's sources, for immediate and `c`
sources: Main's source constraints, `main.pil:456`–`461`. -/
theorem slot1_sources (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (inst : zisk_inst) (pc zc : BitVec 64) (hslot : SlotHolds (tm i) 1 inst pc)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hprev : (tm i).c 0 0 = lo zc ∧ (tm i).c 0 1 = hi zc) :
    ((tm i).a 1 0 = lo (srcA inst zc) ∧ (tm i).a 1 1 = hi (srcA inst zc)) ∧
      ((tm i).b 1 0 = lo (srcB inst zc) ∧ (tm i).b 1 1 = hi (srcB inst zc)) := by
  have h41 := hm.c41; have h42 := hm.c42; have h43 := hm.c43; have h44 := hm.c44
  have h47 := hm.c47; have h48 := hm.c48; have h49 := hm.c49; have h50 := hm.c50
  simp only [Main.c41, Main.c42, Main.c43, Main.c44, Main.c47, Main.c48, Main.c49, Main.c50,
    Main.Main_a_src_c_1, Main.Main_b_src_c_1, Main.Zisk_direct_gsum_e_15, Main.Zisk_direct_gsum_e_16,
    Main.Zisk_direct_gsum_e_17] at h41 h42 h43 h44 h47 h48 h49 h50
  obtain ⟨hc0, hc1⟩ := hprev
  rcases hslot with ⟨s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, -, -, -, -, -, -, -⟩
  rcases ha with ha | ha <;> rcases hb with hb | hb <;>
  · simp only [ha, hb, bit, isAImm, isAMem, isAReg, isBImm, isBMem, isBInd, isBReg,
      Bool.false_eq_true, if_false, if_true, srcA, srcB] at s1 s2 s3 s6 s7 s8 s9 ⊢
    simp only [s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] at h41 h42 h43 h44 h47 h48 h49 h50
    refine ⟨⟨?_, ?_⟩, ?_, ?_⟩ <;> grind

/-- **One slot, one `add`.** If slot 1 of Main holds an `add` the model is
about to execute — reached through the ROM lookup, as in `slot1_internal_rom` —
with immediate or `c` sources, and every prover the operation bus can answer
from proves `add` correctly (`OpAdds`, which the adders, Binary, the FROPS
tables and the exclusions each give), then slot 1's `c` and `flag`, and slot 2's
`pc`, are exactly what `zisk_execute` computes: `c = a + b`, no flag, the next
instruction.

What remains assumed: the soundness of the two lookups (`LookupHolds`), the
ROM (`RomHolds`), each prover's `OpAdds`, slot 1's pc and slot 0's `c` being
the model's, and the pilot's scope. -/
theorem slot1_add (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hrom : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hromh : RomHolds prog tr)
    (ps : List (Prover F)) (hops : LookupHolds Main.bus59 xm tm ps)
    (hadds : ∀ p ∈ ps, OpAdds p)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (hprog : prog pc = some inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpcol : (tm i).pc 1 = ((pc.toNat : Nat) : F))
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_ADD ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hprev : (tm i).c 0 0 = lo zc ∧ (tm i).c 0 1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      (tm i).c 1 0 = lo c ∧ (tm i).c 1 1 = hi c ∧ (tm i).flag 1 = bit flag ∧
      (tm i).pc 2 = ((pc'.toNat : Nat) : F) := by
  -- The model: `c = a + b`, no flag, fall through.
  obtain ⟨σ', hrun, hc, hf, hp⟩ :=
    execute_add inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend
  refine ⟨σ', _, _, _, hrun, hc, hf, hp, ?_⟩
  -- The slot holds the program's `add`, and its sources are the model's.
  have hslot := slotHolds_of_rom prog xm tm i hm xr tr hr hrom hromh pc inst hpc32 hpcol hprog
  obtain ⟨⟨hA0, hA1⟩, hB0, hB1⟩ := slot1_sources xm tm i hm inst pc zc hslot ha hb hprev
  obtain ⟨-, -, -, -, -, -, -, -, -, -, -, s12, s13, s14, -, s16, s17, s18⟩ := hslot
  simp only [hop, hset, isExternal, opcode, isM32, bit, if_true, if_false, Bool.false_eq_true]
    at s12 s13 s14 s18
  -- The operation bus: some prover answers slot 1's tuple, and it adds.
  obtain ⟨-, -, -, -, hb'⟩ := hops
  obtain ⟨p, hp', j, -, hpb, htup⟩ := hb' i (by
    simp only [Main.bus59]; rw [s12, Lean.Grind.Semiring.add_zero]; exact one_ne_zero')
  have tup : ∀ k, tupleAt Main.bus59.tuple xm tm i k = tupleAt p.arg.tuple p.x p.t j k := htup
  have hadd := hadds p hp' j (by rw [hpb]; rfl)
  -- Main's side of each tuple element the prover's `AddsAt` speaks of.
  have e0 := (tup 0).symm; have e1 := (tup 1).symm; have e2 := (tup 2).symm
  have e3 := (tup 3).symm; have e4 := (tup 4).symm; have e5 := (tup 5).symm
  have e6 := (tup 6).symm; have e7 := (tup 7).symm
  simp only [tupleAt, Main.bus59, List.getD_cons_zero, List.getD_cons_succ,
    Lean.Grind.Semiring.add_zero] at e0 e1 e2 e3 e4 e5 e6 e7
  obtain ⟨r5, r6, r7⟩ := hadd
    (by unfold tupleAt; rw [e0, s13, ← natCast_ofNat' 10])
    (srcA inst zc) (srcB inst zc)
    (by unfold tupleAt; rw [e1, hA0])
    (by unfold tupleAt; rw [e2, s18, one_sub_zero_mul, hA1])
    (by unfold tupleAt; rw [e3, hB0])
    (by unfold tupleAt; rw [e4, s18, one_sub_zero_mul, hB1])
  unfold tupleAt at r5 r6 r7
  rw [e5] at r5
  rw [e6] at r6
  rw [e7] at r7
  -- The pc: no `set_pc` and no flag, so the next slot runs at `pc + jmp_off2`.
  have h81 := hm.c81
  simp only [Main.c81, Main.Main_expected_current_pc_2, Main.Zisk_direct_gsum_e_30] at h81
  have p2 := pc_add (F := F) pc inst.jmp_off2 hwrap
  refine ⟨r5, r6, by rw [r7]; simp [bit], ?_⟩
  rw [p2, ← s17, ← s16]
  rw [s14, r7] at h81
  grind

end Pilot

end Correspondence
