/-
  The register file through one lane's step.

  `RegInv M xs b`: the model's registers `x1`–`x31` (the vector `xs`) hold what
  Main's register chain holds at memory step `b` — each one `regVal` of its
  chain. At a lane whose memory steps start at `b`:

  * `reg_srcs`: a register source's value is in the lane's `a` or `b` columns —
    the access reads the chain's value at its step, which `RegInv` makes the
    model's;
  * `reg_update`: after the step, `RegInv` holds at `b + 4` for the model's
    new register file — the lane's store, if it stores to a register, sets it
    to the value the model stores, and every other access is a read.
-/
import Proofs.RegRun
import Proofs.StepRegs

open Out.Functions

namespace Correspondence

open Pil.Zisk MemCheck

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

set_option linter.unusedSectionVars false

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- The model's registers `x1`–`x31` hold the chain's values at memory step `b`. -/
def RegInv (M : MainSegs F) (xs : Vector (BitVec 64) 32) (b : Nat) : Prop :=
  ∀ i, i < 31 → (lo (xs[i + 1]!), hi (xs[i + 1]!)) = regVal (0, 0) (M.es i) b

theorem regIdx_toNat {v : BitVec 64} (h : v.toNat < 32) : (regIdx v).toNat = v.toNat := by
  simp [regIdx, Sail.BitVec.extractLsb, BitVec.extractLsb_toNat]
  omega

theorem lo_small {v : BitVec 64} (h : v.toNat < 32) : lo v = ((v.toNat : Nat) : F) := by
  simp only [lo]; congr 1; omega

theorem off_small {v : BitVec 64} (h : v.toNat < 32) : off v = ((v.toNat : Nat) : F) := by
  have : v.toInt = (v.toNat : Int) := by
    have := BitVec.toInt_eq_toNat_cond v
    split at this <;> omega
  rw [off, this, Lean.Grind.Ring.intCast_natCast]

theorem natCast_lt32_inj {a b : Nat} (ha : a < 32) (hb : b < 32) (h : ((a : Nat) : F) = ((b : Nat) : F)) :
    a = b :=
  natCast_inj (F := F) (by unfold goldilocks; omega) (by unfold goldilocks; omega) h

theorem lane_mem {l κ : Nat} (hl : l < 4) (hκ : κ < 3) : PairTag.lane l κ ∈ mainTags := by
  rcases l with _ | _ | _ | _ | l <;> rcases κ with _ | _ | _ | κ
  all_goals first | decide | omega

/-- A register chain's facts, for `x1` to `x31`. -/
def ChainOk (M : MainSegs F) : Prop :=
  ∀ i, i < 31 → (proved (M.es i)).Perm (assumed (0, 0) (M.es i)) ∧
    (∀ e ∈ M.es i, e.prev < e.step) ∧ ((M.es i).map (·.step)).Nodup

/-- Index `v` as a register, `x1`–`x31`, read as `v.toNat`. -/
theorem reg_pos {v : BitVec 64} (h32 : v.toNat < 32) (hne : regIdx v ≠ 0) :
    1 ≤ v.toNat ∧ (regIdx v).toNat = v.toNat := by
  have hidx := regIdx_toNat h32
  refine ⟨?_, hidx⟩
  apply Nat.pos_of_ne_zero; intro h0; apply hne
  apply BitVec.eq_of_toNat_eq; rw [hidx, h0]; rfl

/-- A lane's source access (`κ` 0 or 1) to register `v` reads the model's
value of `v`, under `RegInv` at the lane's first memory step. -/
theorem slot_read {M : MainSegs F} (hch : ChainOk M) {k m l κ : Nat} (hk : k < M.K)
    (hm : m < Main.numRows) (hl : l < 4) (hκ : κ < 2) {v : BitVec 64} (h32 : v.toNat < 32)
    (hne : regIdx v ≠ 0)
    (h1 : ((PairTag.lane l κ).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 1)
    (h2 : el 1 (tupleOf ⟨(PairTag.lane l κ).preArg (F := F), (M.seg k).1, (M.seg k).2⟩ m) = lo v)
    {xs : Vector (BitVec 64) 32} (hinv : RegInv M xs (1 + 4 * (k * 16777216 + 4 * m + l))) :
    (el 4 (tupleOf ⟨(PairTag.lane l κ).preArg (F := F), (M.seg k).1, (M.seg k).2⟩ m),
      el 5 (tupleOf ⟨(PairTag.lane l κ).preArg (F := F), (M.seg k).1, (M.seg k).2⟩ m)) =
      (lo (xs[(regIdx v).toNat]!), hi (xs[(regIdx v).toNat]!)) := by
  obtain ⟨hpos, hidx⟩ := reg_pos h32 hne
  obtain ⟨hp1, hlt1, hnd1⟩ := hch (v.toNat - 1) (by omega)
  have h2' : el 1 (tupleOf ⟨(PairTag.lane l κ).preArg (F := F), (M.seg k).1, (M.seg k).2⟩ m) =
      ((v.toNat - 1 + 1 : Nat) : F) := by
    rw [h2, lo_small h32]; congr 1; omega
  have hr := M.lane_read hp1 hlt1 hnd1 hk (lane_mem hl (by omega)) hm h1 h2'
  have hkeep : regVal (0, 0) (M.es (v.toNat - 1)) (1 + 4 * (k * 16777216 + 4 * m + l) + κ) =
      regVal (0, 0) (M.es (v.toNat - 1)) (1 + 4 * (k * 16777216 + 4 * m + l)) := by
    rcases κ with _ | _ | κ
    · rw [Nat.add_zero]
    · exact M.reads_keep hp1 hlt1 hnd1 (by omega)
    · omega
  have hi := hinv (v.toNat - 1) (by omega)
  rw [hkeep, ← hi, show v.toNat - 1 + 1 = v.toNat by omega] at hr
  rw [hidx]
  exact hr

/-- What the register steps need of an instruction: its register indices
`x1`–`x31`, written as such. -/
structure RegIdx (inst : zisk_inst) : Prop where
  a : inst.a_src = .A_REG → regIdx inst.a_imm ≠ 0 ∧ inst.a_imm.toNat < 32
  b : inst.b_src = .B_REG → regIdx inst.b_imm ≠ 0 ∧ inst.b_imm.toNat < 32
  s : inst.store = .ST_REG → inst.store_off.toNat < 32

theorem RegPilot.idx {inst : zisk_inst} (hp : RegPilot inst) : RegIdx inst where
  a ha := ⟨by rcases hp.a with h | h | ⟨-, h⟩
              · cases h.symm.trans ha
              · cases h.symm.trans ha
              · exact h, hp.a_idx ha⟩
  b hb := ⟨by rcases hp.b with h | h | ⟨-, h⟩
              · cases h.symm.trans hb
              · cases h.symm.trans hb
              · exact h, hp.b_idx hb⟩
  s := hp.s_idx

/-- **Register sources read the model's registers.** With `RegInv` at the lane's
first memory step, a register source's value is in the lane's columns. -/
theorem reg_srcs {M : MainSegs F} (hch : ChainOk M) {k m l : Nat} (hk : k < M.K)
    (hm : m < Main.numRows) (hl : l < 4) {inst : zisk_inst} {pc : BitVec 64}
    (hslot : SlotHolds ((M.seg k).2 m) l inst pc) (hp : RegIdx inst)
    {xs : Vector (BitVec 64) 32} (hinv : RegInv M xs (1 + 4 * (k * 16777216 + 4 * m + l))) :
    (inst.a_src = .A_REG → ACols ((M.seg k).2 m) l (xs[(regIdx inst.a_imm).toNat]!)) ∧
      (inst.b_src = .B_REG → BCols ((M.seg k).2 m) l (xs[(regIdx inst.b_imm).toNat]!)) := by
  obtain ⟨c1, c2, c3, c4, c5, c6, c7, c8, -⟩ := lane_cols (F := F) hl (M.seg k).1 (M.seg k).2 m
  refine ⟨fun ha => ?_, fun hb => ?_⟩
  · have hr := slot_read hch hk hm hl (κ := 0) (by omega) (hp.a ha).2 (hp.a ha).1
      (by rw [c1, hslot.a_src_reg, ha]; simp [bit, isAReg, Lean.Grind.Semiring.add_zero])
      (by rw [c2, hslot.a_offset_imm0, Lean.Grind.Semiring.add_zero]) hinv
    rw [c3, c4, Lean.Grind.Semiring.add_zero, Lean.Grind.Semiring.add_zero, Prod.mk.injEq] at hr
    exact hr
  · have hr := slot_read hch hk hm hl (κ := 1) (by omega) (hp.b hb).2 (hp.b hb).1
      (by rw [c5, hslot.b_src_reg, hb]; simp [bit, isBReg, Lean.Grind.Semiring.add_zero])
      (by rw [c6, hslot.b_offset_imm0, Lean.Grind.Semiring.add_zero]) hinv
    rw [c7, c8, Lean.Grind.Semiring.add_zero, Lean.Grind.Semiring.add_zero, Prod.mk.injEq] at hr
    exact hr

/-- The value a lane's store writes, in its columns, is the model's. -/
theorem store_val_cols {r : Main.Row F} {l : Nat} {inst : zisk_inst} {pc c : BitVec 64}
    (hslot : SlotHolds r l inst pc) (hsto : StoreHolds r l inst) (hland : Lands pc inst.jmp_off2)
    (hc0 : r.c l 0 = lo c) (hc1 : r.c l 1 = hi c) :
    r.store_pc l * (r.pc l + r.jmp_offset2 l - r.c l 0) + r.c l 0 = lo (storeVal inst pc c) ∧
      (1 - r.store_pc l) * r.c l 1 = hi (storeVal inst pc c) := by
  rw [hsto.store_pc]
  cases h : inst.store_pc
  · simp only [bit, storeVal, h, cond_false, Bool.false_eq_true, if_false]
    rw [← hc0, ← hc1]
    constructor <;> grind
  · simp only [bit, storeVal, h, cond_true, if_true]
    have hlt := hland.lt
    have elo : lo (pc + inst.jmp_off2) = r.pc l + r.jmp_offset2 l := by
      rw [hslot.pc, hslot.jmp_offset2, ← pc_add pc inst.jmp_off2 hland.noWrap, lo,
        Nat.mod_eq_of_lt hlt]
    have ehi : hi (pc + inst.jmp_off2) = (0 : F) := by
      rw [hi, Nat.div_eq_of_lt hlt, Lean.Grind.Semiring.natCast_zero]
    rw [elo, ehi]
    constructor <;> grind

/-- **The register file after a lane's step.** With `RegInv` at the lane's
first memory step `b`, and the lane's `c` the model's, `RegInv` holds at `b + 4`
for the register file after the model's store. -/
theorem reg_update {M : MainSegs F} (hch : ChainOk M) (hK : M.K < 1048576) {k m l : Nat}
    (hk : k < M.K) (hm : m < Main.numRows) (hl : l < 4) {inst : zisk_inst} {pc c : BitVec 64}
    (hslot : SlotHolds ((M.seg k).2 m) l inst pc) (hsto : StoreHolds ((M.seg k).2 m) l inst)
    (hp : RegIdx inst) (hland : Lands pc inst.jmp_off2)
    (hc0 : ((M.seg k).2 m).c l 0 = lo c) (hc1 : ((M.seg k).2 m).c l 1 = hi c)
    {xs : Vector (BitVec 64) 32} (hinv : RegInv M xs (1 + 4 * (k * 16777216 + 4 * m + l))) :
    RegInv M (storeXs inst pc c xs) (1 + 4 * (k * 16777216 + 4 * m + l) + 4) := by
  obtain ⟨-, -, -, -, -, -, -, -, c9, c10, c11, c12, c13, c14⟩ :=
    lane_cols (F := F) hl (M.seg k).1 (M.seg k).2 m
  intro i hi
  obtain ⟨hp1, hlt1, hnd1⟩ := hch i hi
  -- The flush window's step and the lane's `a` and `b` steps are reads.
  have k3 := M.reads_keep hp1 hlt1 hnd1 (s := 1 + 4 * (k * 16777216 + 4 * m + l) + 3) (by omega)
  have k1 := M.reads_keep hp1 hlt1 hnd1 (s := 1 + 4 * (k * 16777216 + 4 * m + l) + 1) (by omega)
  have k0 := M.reads_keep hp1 hlt1 hnd1 (s := 1 + 4 * (k * 16777216 + 4 * m + l)) (by omega)
  rw [show 1 + 4 * (k * 16777216 + 4 * m + l) + 4 = 1 + 4 * (k * 16777216 + 4 * m + l) + 3 + 1
    by omega, k3]
  have hpre : ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).pre.p =
      ⟨(PairTag.lane l 2).preArg (F := F), (M.seg k).1, (M.seg k).2⟩ := rfl
  have hcur : ((PairTag.lane l 2).pair k (M.seg k).1 (M.seg k).2).cur.p =
      ⟨(PairTag.lane l 2).curArg (F := F), (M.seg k).1, (M.seg k).2⟩ := rfl
  by_cases hs : inst.store = .ST_REG ∧ inst.store_off.toNat = i + 1
  · -- The lane stores to `x(i+1)`.
    obtain ⟨hs, hoff⟩ := hs
    have h32 := hp.s hs
    have hidx := regIdx_toNat h32
    have hreg : ((M.seg k).2 m).store_reg l = 1 := by
      rw [hsto.store_reg, hs]; rfl
    have hmem : ((M.seg k).2 m).store_mem l = 0 := by
      rw [hsto.store_mem, hs]; rfl
    have hind : ((M.seg k).2 m).store_ind l = 0 := by
      rw [hsto.store_ind, hs]; rfl
    have hset := M.store_set hnd1 hk (lane_mem hl (by omega)) hm
      (by show ((PairTag.lane l 2).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 1
          rw [c9, hreg, Lean.Grind.Semiring.add_zero])
      (by rw [hpre, c10, hsto.store_offset, off_small h32, hoff, Lean.Grind.Semiring.add_zero])
      (by show ((PairTag.lane l 2).curArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 1
          rw [c11, hreg, hmem, hind]; grind)
      (by rw [hcur, c12, hreg, hmem, hind]; grind)
    rw [show 1 + 4 * (k * 16777216 + 4 * m + l) + 3 = 1 + 4 * (k * 16777216 + 4 * m + l) + 2 + 1
      by omega, hset, hcur, c13, c14]
    obtain ⟨v0, v1⟩ := store_val_cols hslot hsto hland hc0 hc1
    rw [v0, v1]
    simp only [storeXs, hs, hidx, hoff]
    rw [vec_get_set xs (by omega) (by omega), if_pos rfl]
  · -- It does not: the store's step keeps `x(i+1)`, and so does the model.
    have hno := M.store_none (n := i) hK hnd1 hk hl hm (by
      rintro ⟨h1, h2⟩
      change ((PairTag.lane l 2).preArg (F := F)).reps (M.seg k).1 (M.seg k).2 m = 1 at h1
      rw [c9, hsto.store_reg, Lean.Grind.Semiring.add_zero] at h1
      rw [hpre, c10, hsto.store_offset, Lean.Grind.Semiring.add_zero] at h2
      cases hst : inst.store with
      | ST_REG =>
        have h32 := hp.s hst
        rw [off_small h32] at h2
        exact hs ⟨hst, natCast_lt32_inj h32 (by omega) h2⟩
      | _ =>
        rw [hst] at h1
        simp only [bit, isStReg, Bool.false_eq_true, if_false] at h1
        exact absurd h1 (by grind))
    rw [show 1 + 4 * (k * 16777216 + 4 * m + l) + 3 = 1 + 4 * (k * 16777216 + 4 * m + l) + 2 + 1
      by omega, hno, show 1 + 4 * (k * 16777216 + 4 * m + l) + 2 =
        1 + 4 * (k * 16777216 + 4 * m + l) + 1 + 1 by omega, k1, k0, ← hinv i hi]
    have : (storeXs inst pc c xs)[i + 1]! = xs[i + 1]! := by
      unfold storeXs
      split
      · rename_i hst
        have h32 := hp.s hst
        rw [regIdx_toNat h32, vec_get_set xs h32 (by omega), if_neg]
        intro e; exact hs ⟨hst, e⟩
      · rfl
    rw [this]

end Correspondence
