/-
  One lane over known source columns.

  The lane lemmas of `Slot.lean`, `ExtPilot.lean` and `Run.lean` derive a
  lane's source columns themselves, for immediate and `c` sources. With
  register sources the columns come from elsewhere — the memory bus, through
  the register chain. So here the sources are an input: `SrcCols r l A B`, the
  lane's `a` and `b` columns hold `A` and `B`; from it, the lane's `c` and
  `flag` are `zisk_exec inst.op A B`'s, for the internal ops by Main's
  constraints (`lane_int_cols`) and for the external ones by the operation bus
  (`lane_ext_cols`). And the sources an immediate or `c` gives satisfy it
  (`a_cols_imm_c`, `b_cols_imm_c`).
-/
import Proofs.Run
import Proofs.ModelRegs

open Out.Functions

namespace Correspondence

open Pil.Zisk

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The lane's `a` columns hold `A`. -/
def ACols (r : Main.Row F) (l : Nat) (A : BitVec 64) : Prop := r.a l 0 = lo A ∧ r.a l 1 = hi A

/-- The lane's `b` columns hold `B`. -/
def BCols (r : Main.Row F) (l : Nat) (B : BitVec 64) : Prop := r.b l 0 = lo B ∧ r.b l 1 = hi B

omit [Lean.Grind.IsCharP F goldilocks] in
/-- An immediate or `c` source puts its value in the lane's `a` columns. -/
theorem a_cols_imm_c {r : Main.Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    {inst : zisk_inst} {pc zc : BitVec 64} (hslot : SlotHolds r l inst pc)
    (hprev : p0 = lo zc ∧ p1 = hi zc) (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (xs : Vector (BitVec 64) 32) : ACols r l (aVal inst zc xs) := by
  obtain ⟨h41, -, h43, -, -, -, h47, -, h49, -, -, -, -, -, -, -, -⟩ := hl
  obtain ⟨rfl, rfl⟩ := hprev
  rcases hslot with ⟨s1, s2, s3, s4, s5, -, -, -, -, -, -, -, -, -, -, -, -, -⟩
  rcases ha with ha | ha <;>
  · simp only [ha, bit, isAImm, isAMem, isAReg, Bool.false_eq_true, if_false, if_true] at s1 s2 s3
    simp only [s1, s2, s3, s4, s5] at h41 h43 h47 h49
    simp only [ACols, aVal, ha]
    refine ⟨?_, ?_⟩ <;> grind

omit [Lean.Grind.IsCharP F goldilocks] in
/-- An immediate or `c` source puts its value in the lane's `b` columns. -/
theorem b_cols_imm_c {r : Main.Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    {inst : zisk_inst} {pc zc : BitVec 64} (hslot : SlotHolds r l inst pc)
    (hprev : p0 = lo zc ∧ p1 = hi zc) (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (xs : Vector (BitVec 64) 32) : BCols r l (bVal inst zc xs) := by
  obtain ⟨-, h42, -, h44, -, -, -, h48, -, h50, -, -, -, -, -, -, -⟩ := hl
  obtain ⟨rfl, rfl⟩ := hprev
  rcases hslot with ⟨-, -, -, -, -, s6, s7, s8, s9, s10, s11, -, -, -, -, -, -, -⟩
  rcases hb with hb | hb <;>
  · simp only [hb, bit, isBImm, isBMem, isBInd, isBReg, Bool.false_eq_true, if_false, if_true]
      at s6 s7 s8 s9
    simp only [s6, s7, s8, s9, s10, s11] at h42 h44 h48 h50
    simp only [BCols, bVal, hb]
    refine ⟨?_, ?_⟩ <;> grind

omit [Lean.Grind.IsCharP F goldilocks] in
/-- **An internal op over known sources.** The lane's `c` and `flag` are
`zisk_exec inst.op A B`'s, by Main's internal-op constraints. -/
theorem lane_int_cols {r : Main.Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    {inst : zisk_inst} {pc : BitVec 64} (hslot : SlotHolds r l inst pc)
    (hop : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ()) {A B : BitVec 64}
    (hB : BCols r l B) :
    r.c l 0 = lo (zisk_exec inst.op A B).1 ∧ r.c l 1 = hi (zisk_exec inst.op A B).1 ∧
      r.flag l = bit (zisk_exec inst.op A B).2 := by
  obtain ⟨-, -, -, -, h45, h46, -, -, -, -, h51, h52, h53, h54, -, -, -⟩ := hl
  obtain ⟨hb0, hb1⟩ := hB
  have s12 := hslot.is_external_op
  have s13 := hslot.op
  rcases hop with hop | hop <;>
  · simp only [hop, isExternal, opcode, bit, Bool.false_eq_true, if_false,
      Lean.Grind.Semiring.natCast_zero, Lean.Grind.Semiring.natCast_one] at s12 s13
    rw [s12, s13] at h45 h46 h51 h52 h53 h54
    simp only [hop, zisk_exec, lo_zeros, hi_zeros, bit, if_true, if_false, Bool.false_eq_true]
    refine ⟨?_, ?_, ?_⟩ <;> grind

/-- **An external op over known sources.** The lane's `c` and `flag` are
`sem A B`'s, from whichever prover answers its operation-bus lookup. -/
theorem lane_ext_cols (code : Nat) (sem : BitVec 64 → BitVec 64 → BitVec 64 × Bool)
    {xm : Main.Ctx F} {tm : Main.Trace F} {i : Int} {l : Nat} (hl4 : l < 4)
    (ps : List (Prover F)) (hops : LookupHolds (opArg l) xm tm ps)
    (hpr : ∀ p ∈ ps, OpProves code sem p)
    {inst : zisk_inst} {pc : BitVec 64} (hslot : SlotHolds (tm i) l inst pc)
    (hext : isExternal inst.op = true) (hcode : opcode inst.op = code)
    {A B : BitVec 64} (hA : ACols (tm i) l A) (hB : BCols (tm i) l B) :
    (tm i).c l 0 = lo (sem A B).1 ∧ (tm i).c l 1 = hi (sem A B).1 ∧
      (tm i).flag l = bit (sem A B).2 := by
  obtain ⟨hA0, hA1⟩ := hA
  obtain ⟨hB0, hB1⟩ := hB
  obtain ⟨-, -, -, -, -, -, -, -, -, -, -, s12, s13, -, -, -, -, s18⟩ := hslot
  have hm32 : isM32 inst.op = false := rfl
  rw [hext] at s12
  rw [hcode] at s13
  rw [hm32] at s18
  simp only [bit, if_true, if_false, Bool.false_eq_true] at s12 s18
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
  obtain ⟨r5, r6, r7⟩ := hprove (by rw [e0, s13]) A B
    (by rw [e1, hA0]) (by rw [e2, s18, one_sub_zero_mul, hA1])
    (by rw [e3, hB0]) (by rw [e4, s18, one_sub_zero_mul, hB1])
  rw [e5] at r5
  rw [e6] at r6
  rw [e7] at r7
  exact ⟨r5, r6, r7⟩

/-- **The next pc over a known `c` and flag.** The next step's pc column is the
model's next pc: `c + jmp_off1` for `set_pc` (given the column holds a 32-bit
value, as the next step's ROM lookup makes it), else `pc + jmp_off1` or
`pc + jmp_off2` by the flag. -/
theorem lane_npc {r : Main.Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    {inst : zisk_inst} {pc : BitVec 64} (hslot : SlotHolds r l inst pc) {c : BitVec 64}
    {flag : Bool} (hc0 : r.c l 0 = lo c) (hc1 : r.c l 1 = hi c) (hf : r.flag l = bit flag)
    (hj : inst.set_pc = true → ShortJump inst.jmp_off1)
    (hland : inst.set_pc = false → Lands pc inst.jmp_off1 ∧ Lands pc inst.jmp_off2)
    (hq : ∃ q : BitVec 64, q.toNat < 2 ^ 32 ∧ np = ((q.toNat : Nat) : F)) :
    np = (((nextPcV inst pc c flag).toNat : Nat) : F) ∧ (nextPcV inst pc c flag).toNat < 2 ^ 32 := by
  have s14 := hslot.set_pc
  have s15 := hslot.jmp_offset1
  have s16 := hslot.jmp_offset2
  have s17 := hslot.pc
  have hn := hl.npc
  cases hsp : inst.set_pc
  · -- No set_pc: the flag picks the offset.
    obtain ⟨l1, l2⟩ := hland hsp
    rw [hsp] at s14
    simp only [bit, Bool.false_eq_true, if_false] at s14
    rw [s14, s17, s15, s16, hf] at hn
    cases flag
    · simp only [nextPcV, hsp, cond_false]
      refine ⟨?_, l2.lt⟩
      rw [pc_add pc inst.jmp_off2 l2.noWrap, hn]
      simp only [bit, Bool.false_eq_true, if_false]
      grind
    · simp only [nextPcV, hsp, cond_false, cond_true]
      refine ⟨?_, l1.lt⟩
      rw [pc_add pc inst.jmp_off1 l1.noWrap, hn]
      simp only [bit, if_true]
      grind
  · -- set_pc: the target is `c + jmp_off1`, the 32-bit `q`.
    obtain ⟨q, hq32, hnq⟩ := hq
    have hjs := hj hsp
    rw [hsp] at s14
    simp only [bit, if_true] at s14
    have hfs := hl.fs
    have hsc := hl.sc1
    rw [s14] at hfs hsc hn
    have hf0 : r.flag l = 0 := by grind
    have hcz : r.c l 1 = 0 := by grind
    have hhi : c.toNat / 2 ^ 32 = 0 := by
      have h' : ((c.toNat / 2 ^ 32 : Nat) : F) = ((0 : Nat) : F) := by
        rw [Lean.Grind.Semiring.natCast_zero, ← hcz, hc1, hi]
      exact natCast_inj (F := F) (by have := c.isLt; unfold goldilocks; omega)
        (by unfold goldilocks; omega) h'
    have hfield : ((q.toNat : Nat) : F) = ((c.toNat % 2 ^ 32 : Nat) : F) + off inst.jmp_off1 := by
      rw [← hnq, hn, hf0, ← s15, hc0, lo]; grind
    have hint : (((q.toNat : Nat) : Int) : F) =
        ((((c.toNat % 2 ^ 32 : Nat) : Int) + inst.jmp_off1.toInt : Int) : F) := by
      rw [Lean.Grind.Ring.intCast_natCast, Lean.Grind.Ring.intCast_add,
        Lean.Grind.Ring.intCast_natCast, hfield, off]
    have hmod := (Lean.Grind.IsCharP.intCast_ext_iff (p := goldilocks)).mp hint
    obtain ⟨hj0, hj1⟩ := hjs
    have hc := c.isLt
    have heq : ((q.toNat : Nat) : Int) = ((c.toNat % 2 ^ 32 : Nat) : Int) + inst.jmp_off1.toInt := by
      unfold goldilocks at hmod; omega
    have hcq : c + inst.jmp_off1 = q := by
      apply BitVec.eq_of_toNat_eq
      have hz : (((c + inst.jmp_off1).toNat : Nat) : Int) = (c.toNat : Int) + inst.jmp_off1.toInt := by
        rw [BitVec.toNat_add]
        have := BitVec.toInt_eq_toNat_cond inst.jmp_off1
        have := inst.jmp_off1.isLt
        split at * <;> omega
      omega
    simp only [nextPcV, hsp, cond_true, hcq]
    exact ⟨hnq, hq32⟩

end Correspondence
