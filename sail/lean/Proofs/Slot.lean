/-
  One slot of the Main AIR against one step of the model: the pilot.

  This is the first statement that relates the two generated definitions,
  `Out.Functions.zisk_execute` from the Sail model and `Pil.Zisk.Main.everyRow`
  from the pilout. It is deliberately narrow, to test the shape of the
  correspondence before it is scaled up:

  * **Slot 1 of a row.** Main packs four steps per row. Slot 1 reads the
    previous step's `c` from slot 0 and hands its `pc` to slot 2, all inside one
    row; slot 0 would reach across a row (or a segment) boundary instead.
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
      σ'.regs.get? Register.PC = some (bif r.2 then pc + inst.jmp_off1 else pc + inst.jmp_off2) := by
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

/-! ### The pilot -/

section Pilot

open Pil.Zisk.Main

variable {F : Type} [PilField F]

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

/-- **One slot, one step.** If slot 1 of row `i` holds an internal-op
instruction the model is about to execute, and slot 0's `c` is the model's
`c`, then the constraints Main asserts at that row force slot 1's `c` and
`flag` and slot 2's `pc` to encode exactly the state `zisk_execute` leaves.

The constraints used are `main.pil:456`–`503` for lanes 1 and 2: the `c` and
immediate sources, the internal-op semantics, and the pc transition. -/
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
  obtain ⟨σ', hrun, hc, hf, hp⟩ :=
    execute_internal inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend
  refine ⟨σ', _, _, _, hrun, hc, hf, hp, ?_⟩
  -- The constraints of lanes 1 and 2 this uses, as equations.
  have h41 := h.c41; have h42 := h.c42; have h43 := h.c43; have h44 := h.c44
  have h45 := h.c45; have h46 := h.c46; have h47 := h.c47; have h48 := h.c48
  have h49 := h.c49; have h50 := h.c50; have h51 := h.c51; have h52 := h.c52
  have h53 := h.c53; have h54 := h.c54; have h81 := h.c81
  simp only [c41, c42, c43, c44, c45, c46, c47, c48, c49, c50, c51, c52, c53, c54, c81,
    Main_a_src_c_1, Main_b_src_c_1, Main_expected_current_pc_2, Zisk_direct_gsum_e_15,
    Zisk_direct_gsum_e_16, Zisk_direct_gsum_e_17, Zisk_direct_gsum_e_18, Zisk_direct_gsum_e_19,
    Zisk_direct_gsum_e_30] at *
  obtain ⟨hw1, hw2⟩ := hwrap
  have p1 := pc_add (F := F) pc inst.jmp_off1 hw1
  have p2 := pc_add (F := F) pc inst.jmp_off2 hw2
  obtain ⟨hc0, hc1⟩ := hprev
  rcases hslot with ⟨s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, s16, s17, -⟩
  rcases ha with ha | ha <;> rcases hb with hb | hb <;> rcases hop with hop | hop <;>
  · -- The slot's columns are now constants: 0/1 selectors, the opcode, the offsets.
    simp only [ha, hb, hop, hset, bit, opcode, isExternal, isAImm, isAMem, isAReg, isBImm,
      isBMem, isBInd, isBReg, Bool.false_eq_true, if_false, if_true, Lean.Grind.Semiring.natCast_zero,
      Lean.Grind.Semiring.natCast_one] at s1 s2 s3 s6 s7 s8 s9 s12 s13 s14
    simp only [s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, s16, s17]
      at h41 h42 h43 h44 h45 h46 h47 h48 h49 h50 h51 h52 h53 h54 h81
    simp only [srcA, srcB, ha, hb, hop, zisk_exec, cond_true, cond_false, lo_zeros, hi_zeros,
      bit, if_true, if_false, Bool.false_eq_true, p1, p2]
    grind

end Pilot

end Correspondence
