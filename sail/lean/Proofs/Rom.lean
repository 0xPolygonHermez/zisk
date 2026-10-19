/-
  The ROM lookup: where `SlotHolds` comes from.

  The pilot in `Slot.lean` takes "slot 1 holds this instruction" as a
  hypothesis. In the AIR that fact is the ROM lookup: each of Main's four slots
  looks up its instruction columns on bus 7890, and the Rom AIR proves the
  program's rows there. This file derives `SlotHolds` from that lookup, so what
  remains assumed is no longer a statement about Main's columns but two
  statements about the system around it:

  * `LookupHolds` (`Bus.lean`) — what a sound lookup argument guarantees:
    every tuple Main assumes is a tuple the Rom proves, at a row it proves with
    nonzero multiplicity. It is stated over the generated bus arguments
    (`Main.bus87`, `Rom.bus0`), with the Rom as the bus's one prover.
  * `RomHolds prog` — the Rom's committed `rom` columns encode `prog`, the
    program. That commitment is fixed per program (its root is a public of the
    proof), so this is a statement about the setup, not something an AIR could
    establish. Its encoding is the one `state-machines/rom/src/custom_rom.rs`
    writes.

  From those, with Main's booleanity constraints and the Rom's
  `is_data * multiplicity = 0`, the slot's columns are the program's encoding of
  the instruction at its pc. That needs field laws (a nonzero multiplicity is
  invertible) and the characteristic (small numerals are distinct), so `F` is a
  field of the base characteristic here, not an arbitrary ring.
-/
import Proofs.Slot
import Proofs.Bus
import Pil.Rom

open Out.Functions Correspondence

namespace Correspondence

/-! ### What a lookup guarantees -/

/-! ### The ROM's encoding of an instruction -/

section Encoding

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

def isStMem : c_store → Bool | .ST_MEM => true | _ => false
def isStInd : c_store → Bool | .ST_IND => true | _ => false
def isStReg : c_store → Bool | .ST_REG => true | _ => false

/-- The fifteen flag bits above bit 0 of `ZiskInst::get_flags`, low first. The
model has no precompiles, so `is_precompiled` is always clear. -/
def flagBits (inst : zisk_inst) : List Bool :=
  [isAImm inst.a_src, isAMem inst.a_src, false, isBImm inst.b_src, isBMem inst.b_src,
   isExternal inst.op, inst.store_pc, isStMem inst.store, isStInd inst.store, inst.set_pc,
   isM32 inst.op, isBInd inst.b_src, isAReg inst.a_src, isBReg inst.b_src, isStReg inst.store]

/-- Bits as a number, low first. -/
def packNat : List Bool → Nat
  | [] => 0
  | b :: bs => (if b then 1 else 0) + 2 * packNat bs

/-- `ZiskInst::get_flags`: bit 0 always set, so that an all-zero padding row
never matches a lookup. -/
def flagsEnc (inst : zisk_inst) : Nat := 1 + 2 * packNat (flagBits inst)

variable {F : Type} [PilField F]

/-- Row `r` of the Rom holds `inst` at `pc`, as `custom_rom.rs` fills it.
`ind_width` is left out: the ROM commits the raw width, which the exporter
normalizes when the instruction does not use it, and nothing here needs it. -/
structure RomRowEncodes (r : Pil.Zisk.Rom.Row F) (pc : BitVec 64) (inst : zisk_inst) : Prop where
  line : r.line = ((pc.toNat : Nat) : F)
  a_offset_imm0 : r.a_offset_imm0 = lo inst.a_imm
  a_imm1 : r.a_imm1 = hi inst.a_imm
  b_offset_imm0 : r.b_offset_imm0 = lo inst.b_imm
  b_imm1 : r.b_imm1 = hi inst.b_imm
  op : r.op = ((opcode inst.op : Nat) : F)
  store_offset : r.store_offset = off inst.store_off
  jmp_offset1 : r.jmp_offset1 = off inst.jmp_off1
  jmp_offset2 : r.jmp_offset2 = off inst.jmp_off2
  flags : r.flags = ((flagsEnc inst : Nat) : F)

/-- The Rom's committed columns encode `prog`: every code row (`is_data = 0`) is
an instruction of the program, at a pc that fits the 32-bit `pc` column. -/
def RomHolds (prog : BitVec 64 → Option zisk_inst) (t : Pil.Zisk.Rom.Trace F) : Prop :=
  ∀ j, (t j).is_data = 0 →
    ∃ pc inst, prog pc = some inst ∧ pc.toNat < 2 ^ 32 ∧ RomRowEncodes (t j) pc inst

end Encoding

/-! ### Unpacking the flags -/

section Unpack

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

theorem packNat_lt (bs : List Bool) : packNat bs < 2 ^ bs.length := by
  induction bs with
  | nil => simp [packNat]
  | cons b bs ih => simp only [packNat, List.length_cons, Nat.pow_succ]; split <;> omega

theorem packNat_inj : ∀ (bs cs : List Bool), bs.length = cs.length →
    packNat bs = packNat cs → bs = cs
  | [], [], _, _ => rfl
  | b :: bs, c :: cs, hl, h => by
    simp only [packNat] at h
    have hb : b = c := by
      cases b <;> cases c <;> simp only [if_true, if_false, Bool.false_eq_true] at h ⊢ <;> omega
    subst hb
    have : packNat bs = packNat cs := by cases b <;> simp at h <;> omega
    rw [packNat_inj bs cs (by simpa using hl) this]
  | [], _ :: _, hl, _ | _ :: _, [], hl, _ => by simp at hl

variable {F : Type} [Lean.Grind.Field F]

/- `grind` in this toolchain identifies the numeral `2` with the cast `↑(2 : Nat)`
but builds a proof the kernel rejects, so casts of numerals are normalized
before it runs. -/
theorem natCast_two : ((2 : Nat) : F) = 2 := (Lean.Grind.Semiring.ofNat_eq_natCast 2).symm

/-- A field element `s` with `s * (1 - s) = 0` is a bit. -/
theorem bool_of_idem {s : F} (h : s * (1 - s) = 0) : ∃ b : Bool, s = bit b := by
  by_cases h0 : s = 0
  · exact ⟨false, by simp [bit, h0]⟩
  · refine ⟨true, ?_⟩
    simp only [bit, if_true]
    have : (1 - s) = 0 := by
      have := congrArg (fun y => s⁻¹ * y) h
      simp only at this
      grind
    grind

/-- Bits, read in the field low first. -/
def packF : List F → F
  | [] => 0
  | x :: xs => x + 2 * packF xs

theorem packF_bits (bs : List Bool) : packF (bs.map (bit (F := F))) = ((packNat bs : Nat) : F) := by
  induction bs with
  | nil => simp [packF, packNat, Lean.Grind.Semiring.natCast_zero]
  | cons b bs ih =>
    simp only [List.map_cons, packF, packNat, ih, Lean.Grind.Semiring.natCast_add,
      Lean.Grind.Semiring.natCast_mul, natCast_two]
    cases b <;> simp [bit, Lean.Grind.Semiring.natCast_zero, Lean.Grind.Semiring.natCast_one]

/-- Fifteen field bits packed as a number below the characteristic determine
each bit. -/
theorem unpack {p : Nat} [Lean.Grind.IsCharP F p] (hp : 2 ^ 16 < p)
    (bs cs : List Bool) (hl : bs.length = 15) (hc : cs.length = 15)
    (h : (1 + 2 * packF (bs.map (bit (F := F)))) = ((1 + 2 * packNat cs : Nat) : F)) :
    bs = cs := by
  rw [packF_bits] at h
  have h' : ((1 + 2 * packNat bs : Nat) : F) = ((1 + 2 * packNat cs : Nat) : F) := by
    rw [← h, Lean.Grind.Semiring.natCast_add, Lean.Grind.Semiring.natCast_mul,
      Lean.Grind.Semiring.natCast_one, natCast_two]
  have lb := packNat_lt bs
  have lc := packNat_lt cs
  rw [hl] at lb; rw [hc] at lc
  have := (Lean.Grind.IsCharP.natCast_eq_iff_of_lt (p := p) (by omega) (by omega)).mp h'
  exact packNat_inj bs cs (by rw [hl, hc]) (by omega)

end Unpack

/-! ### Slot 1 holds the program's instruction -/

section Derive

attribute [local instance] Lean.Grind.Semiring.natCast Lean.Grind.Ring.intCast

open Pil.Zisk

/-- The Goldilocks prime, the base field's characteristic. -/
abbrev goldilocks : Nat := 18446744069414584321

variable {F : Type} [Lean.Grind.Field F] [Lean.Grind.IsCharP F goldilocks]

/-- Lane `l`'s ROM lookup, `Main.bus86` to `Main.bus89`: the same tuple, at
each lane's columns. -/
def romArg : Nat → BusArg F (Main.Ctx F) (Main.Trace F)
  | 0 => Main.bus86
  | 1 => Main.bus87
  | 2 => Main.bus88
  | _ => Main.bus89

/-- Lane `l`'s packed ROM flags, `Main_rom_flags_l`. -/
def romFlags (r : Main.Row F) (l : Nat) : F :=
  1 + 2 * r.a_src_imm l + 4 * r.a_src_mem l + 8 * r.is_precompiled l + 16 * r.b_src_imm l +
    32 * r.b_src_mem l + 64 * r.is_external_op l + 128 * r.store_pc l + 256 * r.store_mem l +
    512 * r.store_ind l + 1024 * r.set_pc l + 2048 * r.m32 l + 4096 * r.b_src_ind l +
    8192 * r.a_src_reg l + 16384 * r.b_src_reg l + 32768 * r.store_reg l

omit [Lean.Grind.IsCharP F goldilocks] in
theorem romArg_reps (l : Nat) (x : Main.Ctx F) (t : Main.Trace F) (i : Int) :
    (romArg l).reps x t i = 1 := by
  rcases l with _ | _ | _ | l <;> rfl

omit [Lean.Grind.IsCharP F goldilocks] in
theorem romArg_bus (l : Nat) (x : Main.Ctx F) (t : Main.Trace F) (i : Int) :
    (romArg l).bus x t i = 7890 := by
  rcases l with _ | _ | _ | l <;> rfl

omit [Lean.Grind.IsCharP F goldilocks] in
/-- The elements of lane `l`'s ROM tuple the proof reads. -/
theorem romArg_tuple {l : Nat} (hl : l < 4) (x : Main.Ctx F) (t : Main.Trace F) (i : Int) :
    tupleAt (romArg l).tuple x t i 0 = (t i).pc l + 0 ∧
      tupleAt (romArg l).tuple x t i 1 = (t i).a_offset_imm0 l + 0 ∧
      tupleAt (romArg l).tuple x t i 2 = (t i).a_imm1 l + 0 ∧
      tupleAt (romArg l).tuple x t i 3 = (t i).b_offset_imm0 l + 0 ∧
      tupleAt (romArg l).tuple x t i 4 = (t i).b_imm1 l + 0 ∧
      tupleAt (romArg l).tuple x t i 6 = (t i).op l + 0 ∧
      tupleAt (romArg l).tuple x t i 8 = (t i).jmp_offset1 l + 0 ∧
      tupleAt (romArg l).tuple x t i 9 = (t i).jmp_offset2 l + 0 ∧
      tupleAt (romArg l).tuple x t i 10 = romFlags (t i) l + 0 := by
  rcases l with _ | _ | _ | _ | l
  all_goals first
    | exact ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩
    | omega

omit [Lean.Grind.IsCharP F goldilocks] in
/-- Every flag of lane `l` is a bit, by Main's booleanity constraints
(`main.pil:552`–`566`). -/
theorem lane_bits {x : Main.Ctx F} {t : Main.Trace F} {i : Int} (h : Main.everyRow x t i)
    {l : Nat} (hl : l < 4) :
      (t i).a_src_imm l * (1 - (t i).a_src_imm l) = 0 ∧
      (t i).a_src_mem l * (1 - (t i).a_src_mem l) = 0 ∧
      (t i).is_precompiled l * (1 - (t i).is_precompiled l) = 0 ∧
      (t i).b_src_imm l * (1 - (t i).b_src_imm l) = 0 ∧
      (t i).b_src_mem l * (1 - (t i).b_src_mem l) = 0 ∧
      (t i).is_external_op l * (1 - (t i).is_external_op l) = 0 ∧
      (t i).store_pc l * (1 - (t i).store_pc l) = 0 ∧
      (t i).store_mem l * (1 - (t i).store_mem l) = 0 ∧
      (t i).store_ind l * (1 - (t i).store_ind l) = 0 ∧
      (t i).set_pc l * (1 - (t i).set_pc l) = 0 ∧
      (t i).m32 l * (1 - (t i).m32 l) = 0 ∧
      (t i).b_src_ind l * (1 - (t i).b_src_ind l) = 0 ∧
      (t i).a_src_reg l * (1 - (t i).a_src_reg l) = 0 ∧
      (t i).b_src_reg l * (1 - (t i).b_src_reg l) = 0 ∧
      (t i).store_reg l * (1 - (t i).store_reg l) = 0 := by
  rcases l with _ | _ | _ | _ | l
  · exact ⟨h.bool_a_src_imm_0, h.bool_a_src_mem_0, h.bool_is_precompiled_0, h.bool_b_src_imm_0, h.bool_b_src_mem_0, h.bool_is_external_op_0, h.bool_store_pc_0, h.bool_store_mem_0, h.bool_store_ind_0, h.bool_set_pc_0, h.bool_m32_0, h.bool_b_src_ind_0, h.bool_a_src_reg_0, h.bool_b_src_reg_0, h.bool_store_reg_0⟩
  · exact ⟨h.bool_a_src_imm_1, h.bool_a_src_mem_1, h.bool_is_precompiled_1, h.bool_b_src_imm_1, h.bool_b_src_mem_1, h.bool_is_external_op_1, h.bool_store_pc_1, h.bool_store_mem_1, h.bool_store_ind_1, h.bool_set_pc_1, h.bool_m32_1, h.bool_b_src_ind_1, h.bool_a_src_reg_1, h.bool_b_src_reg_1, h.bool_store_reg_1⟩
  · exact ⟨h.bool_a_src_imm_2, h.bool_a_src_mem_2, h.bool_is_precompiled_2, h.bool_b_src_imm_2, h.bool_b_src_mem_2, h.bool_is_external_op_2, h.bool_store_pc_2, h.bool_store_mem_2, h.bool_store_ind_2, h.bool_set_pc_2, h.bool_m32_2, h.bool_b_src_ind_2, h.bool_a_src_reg_2, h.bool_b_src_reg_2, h.bool_store_reg_2⟩
  · exact ⟨h.bool_a_src_imm_3, h.bool_a_src_mem_3, h.bool_is_precompiled_3, h.bool_b_src_imm_3, h.bool_b_src_mem_3, h.bool_is_external_op_3, h.bool_store_pc_3, h.bool_store_mem_3, h.bool_store_ind_3, h.bool_set_pc_3, h.bool_m32_3, h.bool_b_src_ind_3, h.bool_a_src_reg_3, h.bool_b_src_reg_3, h.bool_store_reg_3⟩
  · omega

/-- **The ROM lookup gives the instruction, at any lane.** If Main's lane `l`
looks its instruction up on the ROM bus, the Rom holds the program `prog`, and
the lane's `pc` column is a 32-bit `pc`, then the program has an instruction
at `pc`, and the lane holds it, column by column. -/
theorem rom_lane (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    {l : Nat} (hl : l < 4)
    (hbus : LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩]) (hrom : RomHolds prog tr)
    (pc : BitVec 64) (hpc32 : pc.toNat < 2 ^ 32) (hpc : (tm i).pc l = ((pc.toNat : Nat) : F)) :
    ∃ inst, prog pc = some inst ∧ SlotHolds (tm i) l inst pc := by
  obtain ⟨-, -, -, -, hb⟩ := hbus
  -- The lookup: the lane's tuple is the tuple of some Rom row `j` in use.
  obtain ⟨p, hp, j, hmult, -, htup⟩ := hb i (by rw [romArg_reps]; exact one_ne_zero')
  rw [List.mem_singleton] at hp
  subst hp
  simp only [Rom.bus0] at hmult
  -- A row in use is a code row: `is_data * multiplicity = 0`.
  have hdata : (tr j).is_data = 0 := by
    have h0 := (hr j).c0
    simp only [Rom.c0] at h0
    have hm' : (tr j).multiplicity ≠ 0 := by intro h; apply hmult; rw [h]; grind
    have := congrArg (fun y => y * ((tr j).multiplicity)⁻¹) h0
    simp only at this
    grind
  obtain ⟨pc', inst, hprog', hlt', enc⟩ := hrom j hdata
  obtain ⟨r0, r1, r2, r3, r4, r6, r8, r9, r10⟩ := romArg_tuple hl xm tm i
  have t_pc := htup 0
  have t_a0 := htup 1
  have t_a1 := htup 2
  have t_b0 := htup 3
  have t_b1 := htup 4
  have t_op := htup 6
  have t_j1 := htup 8
  have t_j2 := htup 9
  have t_flags := htup 10
  rw [r0] at t_pc; rw [r1] at t_a0; rw [r2] at t_a1; rw [r3] at t_b0; rw [r4] at t_b1
  rw [r6] at t_op; rw [r8] at t_j1; rw [r9] at t_j2; rw [r10] at t_flags
  simp only [tupleAt, Rom.bus0, List.getD_cons_zero, List.getD_cons_succ] at t_pc t_a0 t_a1 t_b0
  simp only [tupleAt, Rom.bus0, List.getD_cons_zero, List.getD_cons_succ] at t_b1 t_op t_j1 t_j2
  simp only [tupleAt, Rom.bus0, List.getD_cons_zero, List.getD_cons_succ] at t_flags
  -- The row's pc is the lane's pc, so its instruction is `prog pc`.
  have hpcs : pc' = pc := by
    have h := t_pc
    rw [hpc, enc.line] at h
    have h' : ((pc.toNat : Nat) : F) = ((pc'.toNat : Nat) : F) := by grind
    have := (Lean.Grind.IsCharP.natCast_eq_iff_of_lt (p := goldilocks)
      (by unfold goldilocks; omega) (by unfold goldilocks; omega)).mp h'
    exact BitVec.eq_of_toNat_eq this.symm
  rw [hpcs] at hprog' enc
  refine ⟨inst, hprog', ?_⟩
  -- Every selector is a bit, by Main's booleanity constraints.
  obtain ⟨q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11, q12, q13, q14, q15⟩ := lane_bits hm hl
  obtain ⟨b1, e1⟩ := bool_of_idem q1
  obtain ⟨b2, e2⟩ := bool_of_idem q2
  obtain ⟨b3, e3⟩ := bool_of_idem q3
  obtain ⟨b4, e4⟩ := bool_of_idem q4
  obtain ⟨b5, e5⟩ := bool_of_idem q5
  obtain ⟨b6, e6⟩ := bool_of_idem q6
  obtain ⟨b7, e7⟩ := bool_of_idem q7
  obtain ⟨b8, e8⟩ := bool_of_idem q8
  obtain ⟨b9, e9⟩ := bool_of_idem q9
  obtain ⟨b10, e10⟩ := bool_of_idem q10
  obtain ⟨b11, e11⟩ := bool_of_idem q11
  obtain ⟨b12, e12⟩ := bool_of_idem q12
  obtain ⟨b13, e13⟩ := bool_of_idem q13
  obtain ⟨b14, e14⟩ := bool_of_idem q14
  obtain ⟨b15, e15⟩ := bool_of_idem q15
  -- The packed flags are the program's, so each bit is the instruction's.
  have hbits : [b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15]
      = flagBits inst := by
    have hf : romFlags (tm i) l = ((flagsEnc inst : Nat) : F) := by
      rw [← enc.flags]; grind
    apply unpack (F := F) (p := goldilocks) (by decide) _ _ rfl rfl
    show _ = ((flagsEnc inst : Nat) : F)
    rw [← hf]
    simp only [romFlags, e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13,
      e14, e15, List.map_cons, List.map_nil, packF]
    grind
  simp only [flagBits, List.cons.injEq] at hbits
  obtain ⟨h1, h2, -, h4, h5, h6, -, -, -, h10, h11, h12, h13, h14, -⟩ := hbits
  subst h1 h2 h4 h5 h6 h10 h11 h12 h13 h14
  exact {
    a_src_imm := e1, a_src_mem := e2, a_src_reg := e13,
    a_offset_imm0 := by rw [← enc.a_offset_imm0]; grind
    a_imm1 := by rw [← enc.a_imm1]; grind
    b_src_imm := e4, b_src_mem := e5, b_src_ind := e12, b_src_reg := e14,
    b_offset_imm0 := by rw [← enc.b_offset_imm0]; grind
    b_imm1 := by rw [← enc.b_imm1]; grind
    is_external_op := e6, set_pc := e10,
    op := by rw [← enc.op]; grind
    jmp_offset1 := by rw [← enc.jmp_offset1]; grind
    jmp_offset2 := by rw [← enc.jmp_offset2]; grind
    pc := hpc, m32 := e11 }

/-- **The ROM lookup gives `SlotHolds`**, at any lane, for the instruction the
program has there. -/
theorem slotHolds_of_rom_lane (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    {l : Nat} (hl : l < 4)
    (hbus : LookupHolds (romArg l) xm tm [⟨Rom.bus0, xr, tr⟩]) (hrom : RomHolds prog tr)
    (pc : BitVec 64) (inst : zisk_inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpc : (tm i).pc l = ((pc.toNat : Nat) : F)) (hprog : prog pc = some inst) :
    SlotHolds (tm i) l inst pc := by
  obtain ⟨inst', h', hs⟩ := rom_lane prog xm tm i hm xr tr hr hl hbus hrom pc hpc32 hpc
  rw [hprog] at h'
  rw [Option.some.inj h']
  exact hs

/-- **The ROM lookup gives `SlotHolds`**, at slot 1. -/
theorem slotHolds_of_rom (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hrom : RomHolds prog tr)
    (pc : BitVec 64) (inst : zisk_inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpc : (tm i).pc 1 = ((pc.toNat : Nat) : F)) (hprog : prog pc = some inst) :
    SlotHolds (tm i) 1 inst pc :=
  slotHolds_of_rom_lane prog xm tm i hm xr tr hr (l := 1) (by decide) hbus hrom pc inst hpc32 hpc
    hprog

/-- **The pilot, with the ROM lookup in place of `SlotHolds`.** The same
conclusion as `slot1_internal`: slot 1's `c` and `flag`, and slot 2's `pc`,
are what `zisk_execute` computes — but the instruction is now the program's,
`prog pc`, reached through the lookup, rather than assumed to sit in the slot.

What remains assumed: the lookup argument is sound (`LookupHolds`), the Rom
commits the program (`RomHolds`), slot 1's pc and slot 0's `c` are the model's
(the previous step's job), and the scope of the pilot (`copyb` and `flag`,
immediate and `c` sources, no store, no `set_pc`, no wrapping jump). -/
theorem slot1_internal_rom (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hrom : RomHolds prog tr)
    (inst : zisk_inst) (σ : State) (pc zc st : BitVec 64)
    (hpc : σ.regs.get? Register.PC = some pc) (hzc : σ.regs.get? Register.Zc = some zc)
    (hst : σ.regs.get? Register.Step = some st)
    (hprog : prog pc = some inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpcol : (tm i).pc 1 = ((pc.toNat : Nat) : F))
    (ha : inst.a_src = .A_IMM ∨ inst.a_src = .A_C)
    (hb : inst.b_src = .B_IMM ∨ inst.b_src = .B_C)
    (hop : inst.op = .ZOP_COPYB () ∨ inst.op = .ZOP_FLAG ())
    (hstore : inst.store = .ST_NONE) (hset : inst.set_pc = false) (hend : inst.is_end = false)
    (hprev : (tm i).c 0 0 = lo zc ∧ (tm i).c 0 1 = hi zc)
    (hwrap : NoWrap pc inst.jmp_off1 ∧ NoWrap pc inst.jmp_off2) :
    ∃ σ' c flag pc', zisk_execute inst σ = .ok false σ' ∧
      σ'.regs.get? Register.Zc = some c ∧
      σ'.regs.get? Register.Zflag = some flag ∧
      σ'.regs.get? Register.PC = some pc' ∧
      (tm i).c 1 0 = lo c ∧ (tm i).c 1 1 = hi c ∧ (tm i).flag 1 = bit flag ∧
      (tm i).pc 2 = ((pc'.toNat : Nat) : F) :=
  slot1_internal xm tm i hm inst σ pc zc st hpc hzc hst ha hb hop hstore hset hend
    (slotHolds_of_rom prog xm tm i hm xr tr hr hbus hrom pc inst hpc32 hpcol hprog) hprev hwrap

end Derive

end Correspondence
