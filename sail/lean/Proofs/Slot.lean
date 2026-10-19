/-
  One slot of the Main AIR against one step of the model: the pilot.

  This is the first statement that relates the two generated definitions,
  `Out.Functions.zisk_execute` from the Sail model and `Pil.Zisk.Main.everyRow`
  from the pilout. It is deliberately narrow, to test the shape of the
  correspondence before it is scaled up:

  * **Slot 1 of a row.** Main packs four steps per row. Slot 1 reads the
    previous step's `c` from slot 0 and hands its `pc` to slot 2, all inside one
    row; slot 0 would reach across a row (or a segment) boundary instead. The
    proof is now done once for any lane (`LaneHolds`, `lane_internal`), with
    the neighbours as parameters, and `slot1_internal` is its slot-1 case.
  * **The internal ops, `copyb` and `flag`.** These are the only ops whose `c`
    and `flag` Main itself determines. For every other op Main sends
    `(op, a, b, c, flag)` over the operation bus, and another AIR constrains it;
    nothing in Main says `c = a + b`. So `add` cannot be the pilot: the
    correspondence for an external op needs the bus.
  * **Sources Main determines alone, immediate and `c`**, and no store. Memory
    and register sources and stores also go through a bus, the memory one.
  * **No `set_pc`, and not the last instruction.**

  What the theorem says: if slot 1 holds the instruction the model executes, at
  the model's pc, and slot 0's `c` is the model's `c`, then the constraints
  force slot 1's `c` and `flag`, and slot 2's `pc`, to be exactly the post-state
  `zisk_execute` produces.

  "Slot 1 holds the instruction" is a hypothesis, `SlotHolds`. In the AIR it is
  the ROM lookup, a bus argument the generator does not extract, so here it is
  assumed rather than derived — the same way the bus-borne parts would be.
-/
import Out
import Pil.Main

open Out.Functions PreSail Sail

namespace Correspondence

/-! ### Running the model -/

abbrev State := SequentialState RegisterType trivialChoiceSource

/-- Reading a register is a lookup in the state's register map. -/
theorem readReg_eq (r : Register) (σ : State) :
    (Sail.readReg r : SailM _) σ =
      match σ.regs.get? r with
      | some v => .ok v σ
      | none => .error .Unreachable σ := by
  cases h : σ.regs.get? r <;>
  simp [Sail.readReg, PreSail.readReg, h, bind, EStateM.bind, get, getThe, MonadStateOf.get,
    EStateM.get, pure, EStateM.pure, throw, throwThe, MonadExceptOf.throw, EStateM.throw]

/-- Writing one is an insertion. -/
theorem writeReg_run (r : Register) (v : RegisterType r) (σ : State) :
    (Sail.writeReg r v : SailM _) σ = .ok () { σ with regs := σ.regs.insert r v } := by
  simp [Sail.writeReg, PreSail.writeReg, modify, modifyGet, MonadStateOf.modifyGet,
    EStateM.modifyGet]

/-- `a` for the sources the pilot covers: the immediate, or the previous `c`. -/
def srcA (inst : zisk_inst) (zc : BitVec 64) : BitVec 64 :=
  match inst.a_src with
  | .A_IMM => inst.a_imm
  | _ => zc

/-- `b` likewise. -/
def srcB (inst : zisk_inst) (zc : BitVec 64) : BitVec 64 :=
  match inst.b_src with
  | .B_IMM => inst.b_imm
  | _ => zc

/-- What one step of the model does, for the instructions the pilot covers: it
succeeds, does not end the program, and leaves `c`, `flag` and `pc` as the op
and the jump offsets say. -/
theorem execute_internal (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false) :
    let r := zisk_exec inst.op (srcA inst zc) (srcB inst zc)
    ∃ σ', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some r.1 ∧
      σ'.regs.get? Register.Zflag = some r.2 ∧
      σ'.regs.get? Register.PC = some (bif r.2 then pc + inst.jmp_off1 else pc + inst.jmp_off2) ∧
      σ'.regs.get? Register.Step = some (BitVec.addInt st 1) := by
  rcases ha with ha | ha <;> rcases hb with hb | hb <;> rcases hop with hop | hop <;>
  cases hsp : inst.store_pc <;>
  · simp [zisk_execute, source_a, source_b, store_c, value_to_store, next_pc, zisk_exec,
      ha, hb, hop, hstore, hset, hend, hsp, srcA, srcB, bind, EStateM.bind, pure, EStateM.pure,
      readReg_eq, writeReg_run, hpc, hzc, hst, Std.ExtDHashMap.get?_insert]

/-- What one step of the model does for an `add` with the pilot's sources:
`c = a + b`, no flag, and the pc falls through. -/
theorem execute_add (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_ADD ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false) :
    ∃ σ', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some (srcA inst zc + srcB inst zc) ∧
      σ'.regs.get? Register.Zflag = some false ∧
      σ'.regs.get? Register.PC = some (pc + inst.jmp_off2) := by
  rcases ha with ha | ha <;> rcases hb with hb | hb <;> cases hsp : inst.store_pc <;>
  · simp [zisk_execute, source_a, source_b, store_c, value_to_store, next_pc, zisk_exec,
      ha, hb, hop, hstore, hset, hend, hsp, srcA, srcB, bind, EStateM.bind, pure, EStateM.pure,
      readReg_eq, writeReg_run, hpc, hzc, hst, Std.ExtDHashMap.get?_insert]

/-! ### Encoding the model's values as columns

A 64-bit value is two 32-bit limbs, low first, as `a`, `b` and `c` are in
Main. A jump offset is a signed field element. The pc column is 32 bits wide,
so a pc is its value as a natural number. -/

section Encoding

open Pil.Zisk.Main

variable {F : Type} [PilField F]

/- `Lean.Grind.CommRing` carries its casts as fields, not instances. -/
attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

def lo (v : BitVec 64) : F := ((v.toNat % 2 ^ 32 : Nat) : F)
def hi (v : BitVec 64) : F := ((v.toNat / 2 ^ 32 : Nat) : F)
def bit (b : Bool) : F := if b then 1 else 0
def off (v : BitVec 64) : F := ((v.toInt : Int) : F)

/-- Sail's `zeros()`, the `c` of `flag`, is the zero limb pair. -/
theorem lo_zeros : lo (zeros (n := 64)) = (0 : F) := by
  simp [lo, zeros, Lean.Grind.Semiring.natCast_zero]
theorem hi_zeros : hi (zeros (n := 64)) = (0 : F) := by
  simp [hi, zeros, Lean.Grind.Semiring.natCast_zero]

/-- The opcode Main's `op` column carries, from `core/src/zisk_ops.rs`. -/
def opcode : zisk_op → Nat
  | .ZOP_FLAG () => 0x00
  | .ZOP_COPYB () => 0x01
  | .ZOP_LTU () => 0x06
  | .ZOP_EQ () => 0x09
  | .ZOP_ADD () => 0x0a
  | .ZOP_AND () => 0x0e
  | .ZOP_PUBOUT () => 0xa0
  | .ZOP_HALT () => 0xff

/-- Whether Main hands the op to another AIR over the operation bus:
everything but the `Internal` (and `Fcall`) ops, as `ZiskInstBuilder` sets
`is_external_op`. -/
def isExternal : zisk_op → Bool
  | .ZOP_FLAG () | .ZOP_COPYB () | .ZOP_HALT () => false
  | _ => true

/-- The source each selector column flags. -/
def isAImm : a_src → Bool | .A_IMM => true | _ => false
def isAMem : a_src → Bool | .A_MEM => true | _ => false
def isAReg : a_src → Bool | .A_REG => true | _ => false
def isBImm : b_src → Bool | .B_IMM => true | _ => false
def isBMem : b_src → Bool | .B_MEM => true | _ => false
def isBInd : b_src → Bool | .B_IND => true | _ => false
def isBReg : b_src → Bool | .B_REG => true | _ => false

/-- Whether the op works on 32 bits (`m32`). None of the modeled ops does. -/
def isM32 : zisk_op → Bool | _ => false

/-- Slot `l` of row `r` holds `inst`, run at `pc`. This is what the ROM lookup
establishes in the AIR; the pilot takes it as a hypothesis. -/
structure SlotHolds (r : Row F) (l : Nat) (inst : zisk_inst) (pc : BitVec 64) : Prop where
  a_src_imm : r.a_src_imm l = bit (isAImm inst.a_src)
  a_src_mem : r.a_src_mem l = bit (isAMem inst.a_src)
  a_src_reg : r.a_src_reg l = bit (isAReg inst.a_src)
  a_offset_imm0 : r.a_offset_imm0 l = lo inst.a_imm
  a_imm1 : r.a_imm1 l = hi inst.a_imm
  b_src_imm : r.b_src_imm l = bit (isBImm inst.b_src)
  b_src_mem : r.b_src_mem l = bit (isBMem inst.b_src)
  b_src_ind : r.b_src_ind l = bit (isBInd inst.b_src)
  b_src_reg : r.b_src_reg l = bit (isBReg inst.b_src)
  b_offset_imm0 : r.b_offset_imm0 l = lo inst.b_imm
  b_imm1 : r.b_imm1 l = hi inst.b_imm
  is_external_op : r.is_external_op l = bit (isExternal inst.op)
  op : r.op l = ((opcode inst.op : Nat) : F)
  set_pc : r.set_pc l = bit inst.set_pc
  jmp_offset1 : r.jmp_offset1 l = off inst.jmp_off1
  jmp_offset2 : r.jmp_offset2 l = off inst.jmp_off2
  pc : r.pc l = ((pc.toNat : Nat) : F)
  m32 : r.m32 l = bit (isM32 inst.op)

/-- A jump from `pc` by `j` stays inside the 64-bit address space. -/
def NoWrap (pc j : BitVec 64) : Prop := 0 ≤ (pc.toNat : Int) + j.toInt ∧ (pc.toNat : Int) + j.toInt < 2 ^ 64

/-- Without a wrap, the model's 64-bit pc arithmetic is the field's. -/
theorem pc_add (pc j : BitVec 64) (h : NoWrap pc j) :
    (((pc + j).toNat : Nat) : F) = ((pc.toNat : Nat) : F) + off j := by
  have hz : (((pc + j).toNat : Nat) : Int) = (pc.toNat : Int) + j.toInt := by
    rw [BitVec.toNat_add]
    have := BitVec.toInt_eq_toNat_cond j
    have := j.isLt
    have := pc.isLt
    unfold NoWrap at h
    split at * <;> omega
  rw [off, ← Lean.Grind.Ring.intCast_natCast, hz, Lean.Grind.Ring.intCast_add,
    Lean.Grind.Ring.intCast_natCast]

end Encoding

/-! ### One lane, in general

Main's four lanes are constrained alike, each by its own copy of
`main.pil:456`–`503`; only the neighbours differ. A lane reads the previous
step's `c` — the lane before it, or for lane 0 the last lane of the previous
row (or, on a segment's first row, the airval `segment_previous_c`) — and
determines the next step's `pc` — the lane after it, or for lane 3 lane 0 of
the next row (or, on a segment's last row, `segment_next_pc`). `LaneHolds`
states one lane's constraints with those neighbours as parameters, so the
correspondence is proved once and holds at every lane. -/

section Lane

open Pil.Zisk.Main

variable {F : Type} [PilField F]

/-- Main's constraints on lane `l` of row `r`, as equations over its columns,
with the previous step's `c` as `p0`, `p1` and the next step's `pc` as `np`:
the `c` and immediate sources, the internal ops, and the pc transition. -/
structure LaneHolds (r : Row F) (l : Nat) (p0 p1 np : F) : Prop where
  a0c : (1 - r.a_src_mem l - r.a_src_imm l - r.a_src_reg l) * (r.a l 0 - p0) = 0
  b0c : (1 - r.b_src_mem l - r.b_src_imm l - r.b_src_ind l - r.b_src_reg l) * (r.b l 0 - p0) = 0
  a0i : r.a_src_imm l * (r.a l 0 - r.a_offset_imm0 l) = 0
  b0i : r.b_src_imm l * (r.b l 0 - r.b_offset_imm0 l) = 0
  c0z : (1 - r.is_external_op l) * (1 - r.op l) * r.c l 0 = 0
  c0b : (1 - r.is_external_op l) * r.op l * (r.b l 0 - r.c l 0) = 0
  a1c : (1 - r.a_src_mem l - r.a_src_imm l - r.a_src_reg l) * (r.a l 1 - p1) = 0
  b1c : (1 - r.b_src_mem l - r.b_src_imm l - r.b_src_ind l - r.b_src_reg l) * (r.b l 1 - p1) = 0
  a1i : r.a_src_imm l * (r.a l 1 - r.a_imm1 l) = 0
  b1i : r.b_src_imm l * (r.b l 1 - r.b_imm1 l) = 0
  c1z : (1 - r.is_external_op l) * (1 - r.op l) * r.c l 1 = 0
  c1b : (1 - r.is_external_op l) * r.op l * (r.b l 1 - r.c l 1) = 0
  fz : (1 - r.is_external_op l) * (1 - r.op l) * (1 - r.flag l) = 0
  fb : (1 - r.is_external_op l) * r.op l * r.flag l = 0
  npc : np = r.set_pc l * (r.c l 0 + r.jmp_offset1 l) + (1 - r.set_pc l) * (r.pc l + r.jmp_offset2 l)
    + r.flag l * (r.jmp_offset1 l - r.jmp_offset2 l)

theorem laneHolds_0 {x : Ctx F} {t : Trace F} {i : Int} (h : everyRow x t i) :
    LaneHolds (t i) 0 (Main_previous_c_0_0 x t i) (Main_previous_c_0_1 x t i) ((t i).pc 1) where
  a0c := by have e := h.c16; simp only [c16, Zisk_direct_gsum_e_3] at e; grind
  b0c := by have e := h.c17; simp only [c17, Zisk_direct_gsum_e_4] at e; grind
  a0i := by have e := h.c18; simp only [c18] at e; grind
  b0i := by have e := h.c19; simp only [c19] at e; grind
  c0z := by have e := h.c20; simp only [c20, Zisk_direct_gsum_e_5] at e; grind
  c0b := by have e := h.c21; simp only [c21] at e; grind
  a1c := by have e := h.c22; simp only [c22, Zisk_direct_gsum_e_6, Zisk_direct_gsum_e_3] at e; grind
  b1c := by have e := h.c23; simp only [c23, Zisk_direct_gsum_e_4] at e; grind
  a1i := by have e := h.c24; simp only [c24] at e; grind
  b1i := by have e := h.c25; simp only [c25] at e; grind
  c1z := by have e := h.c26; simp only [c26, Zisk_direct_gsum_e_7] at e; grind
  c1b := by have e := h.c27; simp only [c27] at e; grind
  fz := by have e := h.c28; simp only [c28, Zisk_direct_gsum_e_8] at e; grind
  fb := by have e := h.c29; simp only [c29] at e; grind
  npc := by have e := h.c57; simp only [c57, Main_expected_current_pc_1, Zisk_direct_gsum_e_20] at e; grind

theorem laneHolds_1 {x : Ctx F} {t : Trace F} {i : Int} (h : everyRow x t i) :
    LaneHolds (t i) 1 ((t i).c 0 0) ((t i).c 0 1) ((t i).pc 2) where
  a0c := by have e := h.c41; simp only [c41, Main_a_src_c_1] at e; grind
  b0c := by have e := h.c42; simp only [c42, Main_b_src_c_1, Zisk_direct_gsum_e_15] at e; grind
  a0i := by have e := h.c43; simp only [c43] at e; grind
  b0i := by have e := h.c44; simp only [c44, Zisk_direct_gsum_e_16] at e; grind
  c0z := by have e := h.c45; simp only [c45] at e; grind
  c0b := by have e := h.c46; simp only [c46] at e; grind
  a1c := by have e := h.c47; simp only [c47, Zisk_direct_gsum_e_17, Main_a_src_c_1] at e; grind
  b1c := by have e := h.c48; simp only [c48, Main_b_src_c_1, Zisk_direct_gsum_e_15] at e; grind
  a1i := by have e := h.c49; simp only [c49] at e; grind
  b1i := by have e := h.c50; simp only [c50] at e; grind
  c1z := by have e := h.c51; simp only [c51, Zisk_direct_gsum_e_18] at e; grind
  c1b := by have e := h.c52; simp only [c52] at e; grind
  fz := by have e := h.c53; simp only [c53, Zisk_direct_gsum_e_19] at e; grind
  fb := by have e := h.c54; simp only [c54] at e; grind
  npc := by have e := h.c81; simp only [c81, Main_expected_current_pc_2, Zisk_direct_gsum_e_30] at e; grind

theorem laneHolds_2 {x : Ctx F} {t : Trace F} {i : Int} (h : everyRow x t i) :
    LaneHolds (t i) 2 ((t i).c 1 0) ((t i).c 1 1) ((t i).pc 3) where
  a0c := by have e := h.c65; simp only [c65, Main_a_src_c_2, Zisk_direct_gsum_e_24] at e; grind
  b0c := by have e := h.c66; simp only [c66, Zisk_direct_gsum_e_25, Main_b_src_c_2] at e; grind
  a0i := by have e := h.c67; simp only [c67] at e; grind
  b0i := by have e := h.c68; simp only [c68] at e; grind
  c0z := by have e := h.c69; simp only [c69, Zisk_direct_gsum_e_26] at e; grind
  c0b := by have e := h.c70; simp only [c70] at e; grind
  a1c := by have e := h.c71; simp only [c71, Main_a_src_c_2, Zisk_direct_gsum_e_24] at e; grind
  b1c := by have e := h.c72; simp only [c72, Main_b_src_c_2] at e; grind
  a1i := by have e := h.c73; simp only [c73, Zisk_direct_gsum_e_27] at e; grind
  b1i := by have e := h.c74; simp only [c74] at e; grind
  c1z := by have e := h.c75; simp only [c75] at e; grind
  c1b := by have e := h.c76; simp only [c76, Zisk_direct_gsum_e_28] at e; grind
  fz := by have e := h.c77; simp only [c77] at e; grind
  fb := by have e := h.c78; simp only [c78, Zisk_direct_gsum_e_29] at e; grind
  npc := by have e := h.c105; simp only [c105, Zisk_direct_gsum_e_40, Main_expected_current_pc_3, Zisk_direct_gsum_e_39] at e; grind

/-- Lane 3 hands its pc to lane 0 of the next row, when that row is in the
same segment (`main.pil:498`, which the first row of a segment skips). -/
theorem laneHolds_3 {x : Ctx F} {t : Trace F} {i : Int} (h : everyRow x t i)
    (h' : everyRow x t (i + 1)) (hl : (t (i + 1)).__L1__ = 0) :
    LaneHolds (t i) 3 ((t i).c 2 0) ((t i).c 2 1) ((t (i + 1)).pc 0) where
  a0c := by have e := h.c89; simp only [c89, Zisk_direct_gsum_e_34, Main_a_src_c_3] at e; grind
  b0c := by have e := h.c90; simp only [c90, Main_b_src_c_3] at e; grind
  a0i := by have e := h.c91; simp only [c91] at e; grind
  b0i := by have e := h.c92; simp only [c92, Zisk_direct_gsum_e_35] at e; grind
  c0z := by have e := h.c93; simp only [c93] at e; grind
  c0b := by have e := h.c94; simp only [c94, Zisk_direct_gsum_e_36] at e; grind
  a1c := by have e := h.c95; simp only [c95, Main_a_src_c_3] at e; grind
  b1c := by have e := h.c96; simp only [c96, Main_b_src_c_3] at e; grind
  a1i := by have e := h.c97; simp only [c97] at e; grind
  b1i := by have e := h.c98; simp only [c98] at e; grind
  c1z := by have e := h.c99; simp only [c99, Zisk_direct_gsum_e_37] at e; grind
  c1b := by have e := h.c100; simp only [c100] at e; grind
  fz := by have e := h.c101; simp only [c101, Zisk_direct_gsum_e_38] at e; grind
  fb := by have e := h.c102; simp only [c102] at e; grind
  npc := by
    have e := h'.c32
    simp only [c32, Zisk_direct_gsum_e_10, Main_expected_current_pc_0, Zisk_direct_gsum_e_9, hl, Int.add_sub_cancel] at e
    grind

/-- On the last row of a segment, lane 3 hands its pc to the next segment, as
the airval `segment_next_pc` (`main.pil:526`). -/
theorem laneHolds_3_last {x : Ctx F} {t : Trace F} {i : Int} (h : everyRow x t i)
    (hl : (t (i + 1)).__L1__ = 1) :
    LaneHolds (t i) 3 ((t i).c 2 0) ((t i).c 2 1) x.Main_segment_next_pc where
  a0c := by have e := h.c89; simp only [c89, Zisk_direct_gsum_e_34, Main_a_src_c_3] at e; grind
  b0c := by have e := h.c90; simp only [c90, Main_b_src_c_3] at e; grind
  a0i := by have e := h.c91; simp only [c91] at e; grind
  b0i := by have e := h.c92; simp only [c92, Zisk_direct_gsum_e_35] at e; grind
  c0z := by have e := h.c93; simp only [c93] at e; grind
  c0b := by have e := h.c94; simp only [c94, Zisk_direct_gsum_e_36] at e; grind
  a1c := by have e := h.c95; simp only [c95, Main_a_src_c_3] at e; grind
  b1c := by have e := h.c96; simp only [c96, Main_b_src_c_3] at e; grind
  a1i := by have e := h.c97; simp only [c97] at e; grind
  b1i := by have e := h.c98; simp only [c98] at e; grind
  c1z := by have e := h.c99; simp only [c99, Zisk_direct_gsum_e_37] at e; grind
  c1b := by have e := h.c100; simp only [c100] at e; grind
  fz := by have e := h.c101; simp only [c101, Zisk_direct_gsum_e_38] at e; grind
  fb := by have e := h.c102; simp only [c102] at e; grind
  npc := by
    have e := h.c33
    simp only [c33, Main_expected_current_pc_0_', Zisk_direct_gsum_e_11, hl] at e
    grind

/-- Lane 0 reads the last lane of the previous row, inside a segment. -/
theorem previous_c_0 {x : Ctx F} {t : Trace F} {i : Int} (hl : (t i).__L1__ = 0) :
    Main_previous_c_0_0 x t i = (t (i - 1)).c 3 0 ∧ Main_previous_c_0_1 x t i = (t (i - 1)).c 3 1 := by
  simp only [Main_previous_c_0_0, Main_previous_c_0_1, hl]; constructor <;> grind

/-- On a segment's first row, it reads what the previous segment handed over. -/
theorem previous_c_0_first {x : Ctx F} {t : Trace F} {i : Int} (hl : (t i).__L1__ = 1) :
    Main_previous_c_0_0 x t i = x.Main_segment_previous_c 0 ∧
      Main_previous_c_0_1 x t i = x.Main_segment_previous_c 1 := by
  simp only [Main_previous_c_0_0, Main_previous_c_0_1, hl]; constructor <;> grind

end Lane

/-! ### The pilot -/

section Pilot

open Pil.Zisk.Main

variable {F : Type} [PilField F]

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- **One lane, one step.** If lane `l` of row `r` holds an internal-op
instruction the model is about to execute, and the previous step's `c` is the
model's `c`, then the lane's constraints force its `c` and `flag`, and the next
step's `pc`, to encode exactly the state `zisk_execute` leaves. -/
theorem lane_internal {r : Row F} {l : Nat} {p0 p1 np : F} (hl : LaneHolds r l p0 p1 np)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hslot : SlotHolds r l inst pc) (hprev : p0 = lo zc ∧ p1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      σ'.regs.get? Register.Step = some (BitVec.addInt st 1) ∧
      (pc' = pc + inst.jmp_off1 ∨ pc' = pc + inst.jmp_off2) ∧
      r.c l 0 = lo c ∧ r.c l 1 = hi c ∧ r.flag l = bit flag ∧
      np = ((pc'.toNat : Nat) : F) := by
  obtain ⟨σ', hrun, hc, hf, hp, hs⟩ :=
    execute_internal inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend
  refine ⟨σ', _, _, _, hrun, hc, hf, hp, hs, by cases (zisk_exec inst.op _ _).2 <;> simp, ?_⟩
  obtain ⟨h41, h42, h43, h44, h45, h46, h47, h48, h49, h50, h51, h52, h53, h54, h81⟩ := hl
  obtain ⟨hw1, hw2⟩ := hwrap
  have p1' := pc_add (F := F) pc inst.jmp_off1 hw1
  have p2' := pc_add (F := F) pc inst.jmp_off2 hw2
  obtain ⟨rfl, rfl⟩ := hprev
  rcases hslot with ⟨s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, s16, s17, -⟩
  rcases ha with ha | ha <;> rcases hb with hb | hb <;> rcases hop with hop | hop <;>
  · -- The lane's columns are now constants: 0/1 selectors, the opcode, the offsets.
    simp only [ha, hb, hop, hset, bit, opcode, isExternal, isAImm, isAMem, isAReg, isBImm,
      isBMem, isBInd, isBReg, Bool.false_eq_true, if_false, if_true, Lean.Grind.Semiring.natCast_zero,
      Lean.Grind.Semiring.natCast_one] at s1 s2 s3 s6 s7 s8 s9 s12 s13 s14
    simp only [s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, s16, s17]
      at h41 h42 h43 h44 h45 h46 h47 h48 h49 h50 h51 h52 h53 h54 h81
    simp only [srcA, srcB, ha, hb, hop, zisk_exec, cond_true, cond_false, lo_zeros, hi_zeros,
      bit, if_true, if_false, Bool.false_eq_true, p1', p2']
    grind

/-- **One slot, one step**: `lane_internal` at slot 1, whose neighbours are
slots 0 and 2 of the same row. The constraints used are `main.pil:456`–`503`
for lanes 1 and 2. -/
theorem slot1_internal (x : Ctx F) (t : Trace F) (i : Int) (h : everyRow x t i)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hslot : SlotHolds (t i) 1 inst pc)
    (hprev : (t i).c 0 0 = lo zc ∧ (t i).c 0 1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      (t i).c 1 0 = lo c ∧ (t i).c 1 1 = hi c ∧ (t i).flag 1 = bit flag ∧
      (t i).pc 2 = ((pc'.toNat : Nat) : F) := by
  obtain ⟨σ', c, f, pc', h1, h2, h3, h4, -, -, h5⟩ :=
    lane_internal (laneHolds_1 h) inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend hslot
      hprev hwrap
  exact ⟨σ', c, f, pc', h1, h2, h3, h4, h5⟩

end Pilot

end Correspondence
