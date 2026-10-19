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

/-- Whether the op works on 32 bits (`m32`). None of the modeled ops does. -/
def isM32 : zisk_op → Bool | _ => false

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

/-- **The ROM lookup gives `SlotHolds`.** If Main's slot 1 looks its
instruction up on the ROM bus, the Rom holds the program `prog`, and slot 1's
`pc` column is `pc`, then slot 1 holds `prog pc`, column by column. -/
theorem slotHolds_of_rom (prog : BitVec 64 → Option zisk_inst)
    (xm : Main.Ctx F) (tm : Main.Trace F) (i : Int) (hm : Main.everyRow xm tm i)
    (xr : Rom.Ctx F) (tr : Rom.Trace F) (hr : ∀ j, Rom.everyRow xr tr j)
    (hbus : LookupHolds Main.bus87 xm tm [⟨Rom.bus0, xr, tr⟩]) (hrom : RomHolds prog tr)
    (pc : BitVec 64) (inst : zisk_inst) (hpc32 : pc.toNat < 2 ^ 32)
    (hpc : (tm i).pc 1 = ((pc.toNat : Nat) : F)) (hprog : prog pc = some inst) :
    SlotHolds (tm i) 1 inst pc := by
  obtain ⟨-, -, -, -, hb⟩ := hbus
  -- The lookup: slot 1's tuple is the tuple of some Rom row `j` in use.
  obtain ⟨p, hp, j, hmult, -, htup⟩ := hb i (by
    simp only [Main.bus87]; exact fun h => Lean.Grind.Field.zero_ne_one h.symm)
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
  obtain ⟨pc', inst', hprog', hlt', enc⟩ := hrom j hdata
  have tup := fun k => htup k
  simp only [tupleAt, Main.bus87, Rom.bus0] at tup
  have t_pc := tup 0
  have t_a0 := tup 1
  have t_a1 := tup 2
  have t_b0 := tup 3
  have t_b1 := tup 4
  have t_op := tup 6
  have t_j1 := tup 8
  have t_j2 := tup 9
  have t_flags := tup 10
  simp only [List.getD_cons_zero, List.getD_cons_succ] at t_pc t_a0 t_a1 t_b0 t_b1 t_op t_j1
  simp only [List.getD_cons_zero, List.getD_cons_succ] at t_j2 t_flags
  -- The row's pc is slot 1's pc, so its instruction is `prog pc`.
  have hpcs : pc' = pc := by
    have h := t_pc
    rw [hpc, enc.line] at h
    have h' : ((pc.toNat : Nat) : F) = ((pc'.toNat : Nat) : F) := by grind
    have := (Lean.Grind.IsCharP.natCast_eq_iff_of_lt (p := goldilocks)
      (by unfold goldilocks; omega) (by unfold goldilocks; omega)).mp h'
    exact BitVec.eq_of_toNat_eq this.symm
  rw [hpcs] at hprog' enc
  have hinst : inst' = inst := by rw [hprog] at hprog'; exact (Option.some.inj hprog').symm
  rw [hinst] at enc
  -- Every selector is a bit, by Main's booleanity constraints.
  obtain ⟨b1, e1⟩ := bool_of_idem (by have := hm.c125; simpa [Main.c125] using this)
  obtain ⟨b2, e2⟩ := bool_of_idem (by have := hm.c126; simpa [Main.c126] using this)
  obtain ⟨b3, e3⟩ := bool_of_idem (by have := hm.c127; simpa [Main.c127] using this)
  obtain ⟨b4, e4⟩ := bool_of_idem (by have := hm.c128; simpa [Main.c128] using this)
  obtain ⟨b5, e5⟩ := bool_of_idem (by have := hm.c129; simpa [Main.c129] using this)
  obtain ⟨b6, e6⟩ := bool_of_idem (by have := hm.c130; simpa [Main.c130] using this)
  obtain ⟨b7, e7⟩ := bool_of_idem (by have := hm.c131; simpa [Main.c131] using this)
  obtain ⟨b8, e8⟩ := bool_of_idem (by have := hm.c132; simpa [Main.c132] using this)
  obtain ⟨b9, e9⟩ := bool_of_idem (by have := hm.c133; simpa [Main.c133] using this)
  obtain ⟨b10, e10⟩ := bool_of_idem (by have := hm.c134; simpa [Main.c134] using this)
  obtain ⟨b11, e11⟩ := bool_of_idem (by have := hm.c135; simpa [Main.c135] using this)
  obtain ⟨b12, e12⟩ := bool_of_idem (by have := hm.c136; simpa [Main.c136] using this)
  obtain ⟨b13, e13⟩ := bool_of_idem (by have := hm.c137; simpa [Main.c137] using this)
  obtain ⟨b14, e14⟩ := bool_of_idem (by have := hm.c138; simpa [Main.c138] using this)
  obtain ⟨b15, e15⟩ := bool_of_idem (by have := hm.c139; simpa [Main.c139] using this)
  -- The packed flags are the program's, so each bit is the instruction's.
  have hbits : [b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15]
      = flagBits inst := by
    have hf : Main.Main_rom_flags_1 xm tm i = ((flagsEnc inst : Nat) : F) := by
      rw [← enc.flags]; grind
    apply unpack (F := F) (p := goldilocks) (by decide) _ _ rfl rfl
    show _ = ((flagsEnc inst : Nat) : F)
    rw [← hf]
    simp only [Main.Main_rom_flags_1, e1, e2, e3, e4, e5, e6, e7, e8, e9, e10, e11, e12, e13,
      e14, e15, List.map_cons, List.map_nil, packF]
    grind
  simp only [flagBits, List.cons.injEq] at hbits
  obtain ⟨h1, h2, -, h4, h5, h6, -, -, -, h10, -, h12, h13, h14, -⟩ := hbits
  subst h1 h2 h4 h5 h6 h10 h12 h13 h14
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
    pc := hpc }

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
