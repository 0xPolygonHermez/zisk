/-
  Totality, the memory accesses: loads, stores, AMOs, lr and sc. Each
  access is permitted on both sides (a step the simulation covers, the
  free input's `ld` among them), refused by both (`FaultAtR`), or breaks
  one of the contract's preconditions (R, I, A, M, the marchid cell).
-/
import Equiv.TotalityJump

namespace Equiv.Final

open Equiv.Riscv Equiv.Sim Equiv.Machine Equiv.Top
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Transpiler (decode convert opcode funct3)

/-! ## ZisK's memory map, and its PMA regions -/

/-- ZisK's memory map reads [a, a + w) where it lies in one of its regions. -/
theorem zreadable_iff (a : BitVec 64) (w : Nat) :
    _root_.Zisk.Functions.readable a w = true ↔
      (0x1000_0000 ≤ a.toNat ∧ a.toNat + w ≤ 0x9000_0000) ∨ (0x9000_0000 ≤ a.toNat ∧ a.toNat + w ≤ 0x9800_0000) ∨
        (0xA000_0000 ≤ a.toNat ∧ a.toNat + w ≤ 0xC000_0000) := by
  simp only [_root_.Zisk.Functions.readable, _root_.Zisk.Functions.mem_region_of, BitVec.toNatInt]
  split <;> rename_i h <;> (repeat' split at h) <;> simp_all <;> omega

/-- ZisK's memory map writes [a, a + w) where it lies in the RAM. -/
theorem zwritable_iff (a : BitVec 64) (w : Nat) :
    _root_.Zisk.Functions.writable a w = true ↔ (0xA000_0000 ≤ a.toNat ∧ a.toNat + w ≤ 0xC000_0000) := by
  simp only [_root_.Zisk.Functions.writable, _root_.Zisk.Functions.mem_region_of, BitVec.toNatInt]
  split <;> rename_i h <;> (repeat' split at h) <;> simp_all <;> omega

/-- A PMA region of ZisK's that holds [a, a + w): ZisK's memory map reads
    there, and writes there if the region is writable. -/
theorem match_zisk {a : BitVec 64} {w : Nat} (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8) {r : PMA_Region}
    (hm : matching_pma_region ziskRegions (.Physaddr a) w = some r) :
    _root_.Zisk.Functions.readable a w = true ∧
      (r.attributes.writable = true → _root_.Zisk.Functions.writable a w = true) := by
  have ha := a.isLt
  rw [zreadable_iff, zwritable_iff]
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions, range_subset, zopz0zIzJ_u,
    zero_extend, bits_of_physaddr, Sail.BitVec.zeroExtend, to_bits, slice_w, BitVec.toNatInt] at hm
  split at hm
  · rename_i h1; cases hm
    simp [BitVec.toNat_sub, BitVec.toNat_add] at h1
    rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
  split at hm
  · rename_i h1; cases hm
    simp [BitVec.toNat_sub, BitVec.toNat_add] at h1
    rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
  split at hm
  · rename_i h1; cases hm
    simp [BitVec.toNat_sub, BitVec.toNat_add] at h1
    rcases hw with rfl | rfl | rfl | rfl <;> simp at * <;> omega
  · cases hm

theorem region_readable {a : BitVec 64} {w : Nat} {r : PMA_Region}
    (hm : matching_pma_region ziskRegions (.Physaddr a) w = some r) : r.attributes.readable = true := by
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions] at hm
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  · cases hm

theorem region_atomic {a : BitVec 64} {w : Nat} {r : PMA_Region}
    (hm : matching_pma_region ziskRegions (.Physaddr a) w = some r) : r.attributes.atomic_support = .AMOCASQ := by
  simp only [matching_pma_region, matching_pma_region_bits_range, ziskRegions] at hm
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  split at hm
  · cases hm; rfl
  · cases hm

theorem inputAddr_toNat : _root_.Zisk.Functions.INPUT_ADDR.toNat = 0x1000_0000 := rfl

/-! ## An access, permitted or refused -/

/-- A read ZisK's memory map makes, off its registers and the free input:
    sail-riscv's region reads it too, and memory holds its bytes. -/
theorem lwOk_of {s : RVState} {a : BitVec 64} {n : Nat} (hn : n = 1 ∨ n = 2 ∨ n = 4 ∨ n = 8)
    (hz : _root_.Zisk.Functions.readable a n = true) (hr : _root_.Zisk.Functions.hits_registers a n = false)
    (hio : Equiv.InOff a n) (hf : Equiv.Full s.mem) : ∃ v, LwOk s ziskRegions a n v := by
  obtain ⟨r, hm, hrd⟩ := readable_match a n hn hz
  have hb := (zreadable_iff a n).1 hz
  obtain ⟨v, hv⟩ := Equiv.memRead_some s.mem n a.toNat (fun i hi => hf _ (by
    simp only [ZiskRiscv.inRegions]; simp; omega))
  exact ⟨v, ⟨r, hm, hrd, splitOk_zisk hm (by omega)⟩, hz, hr, hio, hv⟩

/-- A write ZisK's memory map makes, off its registers: sail-riscv's
    region writes it too. -/
theorem swOk_of {a : BitVec 64} {n : Nat} (hn : n = 1 ∨ n = 2 ∨ n = 4 ∨ n = 8)
    (hz : _root_.Zisk.Functions.writable a n = true) (hr : _root_.Zisk.Functions.hits_registers a n = false) :
    SwOk ziskRegions a n := by
  obtain ⟨r, hm, hwr⟩ := writable_match a n hn hz
  have hb := (zwritable_iff a n).1 hz
  exact ⟨⟨r, hm, hwr, splitOk_zisk hm (by omega)⟩, hz, hr⟩

/-- A load's access: on the registers' slots (R), on the free input but
    an 8-byte load of it (I), the free input's `ld`, refused by both sides,
    or permitted by both, off the free input. -/
theorem load_cases {s : RVState} {w : BitVec 32} {a : BitVec 64} {n : Nat} (hn : n = 1 ∨ n = 2 ∨ n = 4 ∨ n = 8)
    (hacc : access s w = some (.load, a, n)) (hf : Equiv.Full s.mem) :
    BreakR s w ∨ BreakI s w ∨ (a = _root_.Zisk.Functions.INPUT_ADDR ∧ n = 8) ∨
      AccFault ziskRegions (.Load .Data) a n ∨
      ((∃ v, LwOk s ziskRegions a n v) ∧ a ≠ _root_.Zisk.Functions.INPUT_ADDR) := by
  cases hr : _root_.Zisk.Functions.hits_registers a n
  case true => exact .inl ⟨_, _, _, hacc, hr⟩
  by_cases hi : Overlaps a n _root_.Zisk.Functions.INPUT_ADDR 8
  · by_cases he : a = _root_.Zisk.Functions.INPUT_ADDR ∧ n = 8
    · exact .inr (.inr (.inl he))
    · exact .inr (.inl ⟨_, _, _, hacc, hi, fun h => he ⟨h.2.1, h.2.2⟩⟩)
  have hio : Equiv.InOff a n := by
    simp only [Overlaps, inputAddr_toNat] at hi; unfold Equiv.InOff Equiv.IN_LO; omega
  cases hz : _root_.Zisk.Functions.readable a n
  · refine .inr (.inr (.inr (.inl ?_)))
    cases hm : matching_pma_region ziskRegions (.Physaddr a) n with
    | none => exact .inl hm
    | some r => rw [(match_zisk hn hm).1] at hz; cases hz
  · refine .inr (.inr (.inr (.inr ⟨lwOk_of hn hz hr hio hf, ?_⟩)))
    intro h; subst h; simp [Equiv.InOff, Equiv.IN_LO, inputAddr_toNat] at hio; rcases hn with rfl | rfl | rfl | rfl <;> simp at hio

/-- A store's or an AMO's access: on the registers' slots (R), on the free
    input (I), on marchid's cell, refused by both sides, or permitted by
    both. -/
theorem store_cases {s : RVState} {w : BitVec 32} {k : AccKind} {a : BitVec 64} {n : Nat}
    (hn : n = 1 ∨ n = 2 ∨ n = 4 ∨ n = 8) (hacc : access s w = some (k, a, n)) (hk : k ≠ .load) :
    BreakR s w ∨ BreakI s w ∨ BreakMarchid s w ∨ AccFault ziskRegions (.Store .Data) a n ∨
      SwOk ziskRegions a n := by
  cases hr : _root_.Zisk.Functions.hits_registers a n
  case true => exact .inl ⟨_, _, _, hacc, hr⟩
  by_cases hi : Overlaps a n _root_.Zisk.Functions.INPUT_ADDR 8
  · exact .inr (.inl ⟨_, _, _, hacc, hi, fun h => hk h.1⟩)
  by_cases hc : Overlaps a n MARCHID_CELL 8
  · exact .inr (.inr (.inl ⟨_, _, _, hacc, hk, hc⟩))
  cases hz : _root_.Zisk.Functions.writable a n
  · refine .inr (.inr (.inr (.inl ?_)))
    cases hm : matching_pma_region ziskRegions (.Physaddr a) n with
    | none => exact .inl hm
    | some r =>
      refine .inr ⟨r, hm, ?_⟩
      cases hw : r.attributes.writable
      · simp [permits, hw]
      · rw [(match_zisk hn hm).2 hw] at hz; cases hz
  · exact .inr (.inr (.inr (.inr (swOk_of hn hz hr))))

/-- An AMO's, an lr's or an sc's access: on the registers' slots (R), off
    the RAM (M), or in the RAM, off the registers. -/
theorem ram_cases {s : RVState} {w : BitVec 32} {k : AccKind} {a : BitVec 64} {n : Nat}
    (hacc : access s w = some (k, a, n))
    (hd : (∃ op d, decode w = some (.amo op d)) ∨ (∃ d, decode w = some (.lr d)) ∨
      (∃ d, decode w = some (.sc d))) :
    BreakR s w ∨ BreakM s w ∨
      ((0xA000_0000 ≤ a.toNat ∧ a.toNat + n ≤ 0xC000_0000) ∧ _root_.Zisk.Functions.hits_registers a n = false) := by
  cases hr : _root_.Zisk.Functions.hits_registers a n
  case true => exact .inl ⟨_, _, _, hacc, hr⟩
  by_cases hram : 0xA000_0000 ≤ a.toNat ∧ a.toNat + n ≤ 0xC000_0000
  · exact .inr (.inr ⟨hram, rfl⟩)
  · exact .inr (.inl ⟨hd, _, _, _, hacc, hram⟩)

/-! ## The instructions, from their words -/

def _root_.Transpiler.LOp.f3 : Transpiler.LOp → BitVec 3
  | .lb => 0 | .lh => 1 | .lw => 2 | .ld => 3 | .lbu => 4 | .lhu => 5 | .lwu => 6

def _root_.Transpiler.SzOp.f3 : Transpiler.SzOp → BitVec 3
  | .sb => 0 | .sh => 1 | .sw => 2 | .sd => 3

set_option maxHeartbeats 2000000 in
theorem decode_load_fields {w : BitVec 32} {op : Transpiler.LOp} (h : decode w = some (.load op)) :
    opcode w = 3#7 ∧ funct3 w = op.f3 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1; simp_all [BitVec.toNat_eq, Transpiler.LOp.f3])
    | (cases h; simp_all [BitVec.toNat_eq, Transpiler.LOp.f3])

set_option maxHeartbeats 2000000 in
theorem decode_store_fields {w : BitVec 32} {op : Transpiler.SzOp} (h : decode w = some (.store op)) :
    opcode w = 35#7 ∧ funct3 w = op.f3 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1; simp_all [BitVec.toNat_eq, Transpiler.SzOp.f3])
    | (cases h; simp_all [BitVec.toNat_eq, Transpiler.SzOp.f3])

/-- What the decoder reads of an atomic: opcode, funct3 (its width) and
    funct5 (its op). -/
def FieldsA (w : BitVec 32) (d : Bool) (f5 : BitVec 5) : Prop :=
  opcode w = 47#7 ∧ funct3 w = (if d then 3#3 else 2#3) ∧ w.extractLsb 31 27 = f5

set_option maxHeartbeats 2000000 in
theorem decode_amo_fields {w : BitVec 32} {op : Transpiler.AmoOp} {d : Bool} (h : decode w = some (.amo op d)) :
    FieldsA w d op.f5 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [FieldsA, BitVec.toNat_eq, Transpiler.AmoOp.f5]; split <;> simp_all)
    | (cases h; simp_all [FieldsA, BitVec.toNat_eq, Transpiler.AmoOp.f5])

set_option maxHeartbeats 2000000 in
theorem decode_lr_fields {w : BitVec 32} {d : Bool} (h : decode w = some (.lr d)) :
    FieldsA w d 2#5 ∧ w.extractLsb 24 20 = 0#5 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [FieldsA, BitVec.toNat_eq, Transpiler.rs2]; split <;> simp_all)
    | (cases h; simp_all [FieldsA, BitVec.toNat_eq, Transpiler.rs2])

set_option maxHeartbeats 2000000 in
theorem decode_sc_fields {w : BitVec 32} {d : Bool} (h : decode w = some (.sc d)) : FieldsA w d 3#5 := by
  simp only [decode] at h
  repeat' (split at h)
  all_goals first
    | (simp at h; done)
    | (cases h; rename_i h1 h2; simp_all [FieldsA, BitVec.toNat_eq]; split <;> simp_all)
    | (cases h; simp_all [FieldsA, BitVec.toNat_eq])

def ldOfWord (w : BitVec 32) : LdInstr := ⟨w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 12⟩

theorem ldOfWord_encode {w : BitVec 32} (h : decode w = some (.load .ld)) : (ldOfWord w).encode = w := by
  have hf := decode_load_fields h
  simp only [opcode, funct3, BitVec.extractLsb, Transpiler.LOp.f3] at hf
  simp only [ldOfWord, LdInstr.encode]; revert hf; bv_decide

/-- A narrow load's kind (`ld` taken apart). -/
def _root_.Transpiler.LOp.k : Transpiler.LOp → LKind
  | .lb => .lb | .lh => .lh | .lw | .ld => .lw | .lbu => .lbu | .lhu => .lhu | .lwu => .lwu

def lwOfWord (op : Transpiler.LOp) (w : BitVec 32) : LwInstr :=
  ⟨op.k, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 12⟩

theorem lwOfWord_encode {w : BitVec 32} {op : Transpiler.LOp} (h : decode w = some (.load op)) (hne : op ≠ .ld) :
    (lwOfWord op w).encode = w := by
  have hf := decode_load_fields h
  cases op <;> simp only [ne_eq, not_true_eq_false] at hne <;>
    simp only [opcode, funct3, BitVec.extractLsb, Transpiler.LOp.f3] at hf <;>
    simp only [lwOfWord, LwInstr.encode, Transpiler.LOp.k, LKind.f3] <;> (revert hf; bv_decide)

def sdOfWord (w : BitVec 32) : SdInstr :=
  ⟨w.extractLsb' 15 5, w.extractLsb' 20 5, w.extractLsb' 25 7, w.extractLsb' 7 5⟩

theorem sdOfWord_encode {w : BitVec 32} (h : decode w = some (.store .sd)) : (sdOfWord w).encode = w := by
  have hf := decode_store_fields h
  simp only [opcode, funct3, BitVec.extractLsb, Transpiler.SzOp.f3] at hf
  simp only [sdOfWord, SdInstr.encode]; revert hf; bv_decide

/-- A narrow store's kind (`sd` taken apart). -/
def _root_.Transpiler.SzOp.k : Transpiler.SzOp → SKind
  | .sb => .sb | .sh => .sh | .sw | .sd => .sw

def swOfWord (op : Transpiler.SzOp) (w : BitVec 32) : SwInstr :=
  ⟨op.k, w.extractLsb' 15 5, w.extractLsb' 20 5, w.extractLsb' 25 7, w.extractLsb' 7 5⟩

theorem swOfWord_encode {w : BitVec 32} {op : Transpiler.SzOp} (h : decode w = some (.store op))
    (hne : op ≠ .sd) : (swOfWord op w).encode = w := by
  have hf := decode_store_fields h
  cases op <;> simp only [ne_eq, not_true_eq_false] at hne <;>
    simp only [opcode, funct3, BitVec.extractLsb, Transpiler.SzOp.f3] at hf <;>
    simp only [swOfWord, SwInstr.encode, Transpiler.SzOp.k, SKind.f3] <;> (revert hf; bv_decide)

def amoOfWord (op : Transpiler.AmoOp) (d : Bool) (w : BitVec 32) : AmoInstr :=
  ⟨op, d, w.extractLsb' 26 1, w.extractLsb' 25 1, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem amoOfWord_encode {w : BitVec 32} {op : Transpiler.AmoOp} {d : Bool} (h : decode w = some (.amo op d)) :
    (amoOfWord op d w).encode = w := by
  have hf := decode_amo_fields h
  simp only [FieldsA, opcode, funct3, BitVec.extractLsb] at hf
  simp only [amoOfWord, AmoInstr.encode, AmoInstr.f3]
  generalize op.f5 = f at hf ⊢
  cases d <;> simp only [if_true, if_false, Bool.false_eq_true] at hf ⊢ <;> (revert hf; bv_decide)

def lrOfWord (d : Bool) (w : BitVec 32) : LrInstr :=
  ⟨d, w.extractLsb' 26 1, w.extractLsb' 25 1, w.extractLsb' 7 5, w.extractLsb' 15 5⟩

theorem lrOfWord_encode {w : BitVec 32} {d : Bool} (h : decode w = some (.lr d)) : (lrOfWord d w).encode = w := by
  have hf := decode_lr_fields h
  simp only [FieldsA, opcode, funct3, BitVec.extractLsb] at hf
  simp only [lrOfWord, LrInstr.encode, LrInstr.f3]
  cases d <;> simp only [if_true, if_false, Bool.false_eq_true] at hf ⊢ <;> (revert hf; bv_decide)

def scOfWord (d : Bool) (w : BitVec 32) : ScInstr :=
  ⟨d, w.extractLsb' 26 1, w.extractLsb' 25 1, w.extractLsb' 7 5, w.extractLsb' 15 5, w.extractLsb' 20 5⟩

theorem scOfWord_encode {w : BitVec 32} {d : Bool} (h : decode w = some (.sc d)) : (scOfWord d w).encode = w := by
  have hf := decode_sc_fields h
  simp only [FieldsA, opcode, funct3, BitVec.extractLsb] at hf
  simp only [scOfWord, ScInstr.encode, ScInstr.f3]
  cases d <;> simp only [if_true, if_false, Bool.false_eq_true] at hf ⊢ <;> (revert hf; bv_decide)

/-! ## The access each makes -/

theorem regOf_eq {s : RVState} {r : Nat} {v : BitVec 64} (h : getX s r = some v) : regOf s r = v := by
  simp [regOf, h]

theorem access_load {s : RVState} {w : BitVec 32} {op : Transpiler.LOp} {v1 : BitVec 64}
    (h : decode w = some (.load op)) (hv : getX s (w.extractLsb' 15 5).toNat = some v1) :
    access s w = some (.load, v1 + (w.extractLsb' 20 12).signExtend 64, LOp.width op) := by
  unfold access; rw [h]
  rw [show Transpiler.rs1 w = (w.extractLsb' 15 5).toNat from rfl, regOf_eq hv]; rfl

theorem access_store {s : RVState} {w : BitVec 32} {op : Transpiler.SzOp} {v1 : BitVec 64}
    (h : decode w = some (.store op)) (hv : getX s (w.extractLsb' 15 5).toNat = some v1) :
    access s w = some (.store, v1 + (w.extractLsb' 25 7 ++ w.extractLsb' 7 5).signExtend 64, SzOp.width op) := by
  unfold access; rw [h]
  rw [show Transpiler.rs1 w = (w.extractLsb' 15 5).toNat from rfl, regOf_eq hv]; rfl

theorem access_amo {s : RVState} {w : BitVec 32} {op : Transpiler.AmoOp} {d : Bool} {v1 : BitVec 64}
    (h : decode w = some (.amo op d)) (hv : getX s (w.extractLsb' 15 5).toNat = some v1) :
    access s w = some (.amo, v1, aW d) := by
  unfold access; rw [h]
  rw [show Transpiler.rs1 w = (w.extractLsb' 15 5).toNat from rfl, regOf_eq hv]; rfl

theorem access_lr {s : RVState} {w : BitVec 32} {d : Bool} {v1 : BitVec 64}
    (h : decode w = some (.lr d)) (hv : getX s (w.extractLsb' 15 5).toNat = some v1) :
    access s w = some (.load, v1, aW d) := by
  unfold access; rw [h]
  rw [show Transpiler.rs1 w = (w.extractLsb' 15 5).toNat from rfl, regOf_eq hv]; rfl

theorem access_sc {s : RVState} {w : BitVec 32} {d : Bool} {v1 : BitVec 64}
    (h : decode w = some (.sc d)) (hv : getX s (w.extractLsb' 15 5).toNat = some v1) :
    access s w = some (.store, v1, aW d) := by
  unfold access; rw [h]
  rw [show Transpiler.rs1 w = (w.extractLsb' 15 5).toNat from rfl, regOf_eq hv]; rfl

theorem aW_cases (d : Bool) : aW d = 1 ∨ aW d = 2 ∨ aW d = 4 ∨ aW d = 8 := by cases d <;> simp [aW]

/-! ## The step's facts from the context -/

/-- A memory instruction's facts from the context: as `ctx_at`, the
    instruction in the memory group. -/
theorem ctx_at_mem {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {i : Transpiler.Inst} (hd : decode w = some i) (hg : group i = .mem) {e : BitVec 32} (he : e = w) :
    WordsAt S.rv ziskRegions pc zrom e (nextWord S.rv pc) := by
  rw [he]
  have hb := readWord_bytes hx.word
  have hn2 : Transpiler.usesNext2 w = false := by
    cases i <;> simp only [group, reduceCtorEq] at hg <;> simp [Transpiler.usesNext2, hd]
  have hdesc : ∃ ia l, convert pc w (nextWord S.rv pc) ia = some l := by
    refine ⟨0, ?_⟩
    cases i <;> simp only [group, reduceCtorEq] at hg <;> simp only [convert, hd] <;> (repeat' split) <;>
      exact ⟨_, rfl⟩
  exact ⟨hx.aligned pc hx.code, hx.exec pc hx.code, exec_add2 (hx.exec pc hx.code) (hx.aligned pc hx.code),
    hb.1, hb.2.1, hb.2.2.1, hb.2.2.2, ctx_transpiled hx hn2 hdesc⟩

/-- The step's cases, from a precondition broken at the word at pc. -/
theorem StepCase.ofBroken {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    (h : Broken S.rv pc w) : StepCase zrom C S :=
  .inr (.inr (.inr (.inr (.inr (.inl ⟨pc, w, hx.atPc, hx.word, h⟩)))))

/-- The step's cases, from an access both sides refuse. -/
theorem StepCase.ofFault {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} (h : FaultAtR zrom S.rv) : StepCase zrom C S :=
  .inr (.inr (.inr (.inl (.inr (.inr (.inl h))))))

/-- The step's cases, from an `ld` of the free input. -/
theorem StepCase.ofFreeLd {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} (h : FreeLdAt ziskRegions zrom S) : StepCase zrom C S :=
  .inl (.inr (.inr (.inl h)))

/-! ## Loads and stores -/

theorem LOp.width_k {op : Transpiler.LOp} (h : op ≠ .ld) : LOp.width op = op.k.w := by
  cases op <;> simp_all [LOp.width, Transpiler.LOp.k, LKind.w]

theorem SzOp.width_k {op : Transpiler.SzOp} (h : op ≠ .sd) : SzOp.width op = op.k.w := by
  cases op <;> simp_all [SzOp.width, Transpiler.SzOp.k, SKind.w]

/-- A load at pc: its access permitted, refused, the free input's `ld`, or a
    precondition broken. -/
theorem kind_load {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} {op : Transpiler.LOp}
    (hx : Ctx zrom C S pc w) (hd : decode w = some (.load op)) : StepCase zrom C S := by
  obtain ⟨v1, hv1⟩ := getX_ready hx.ready (w.extractLsb' 15 5).toNat
  have hacc := access_load hd hv1
  by_cases hop : op = .ld
  · subst hop
    have h := ctx_at_mem hx hd rfl (ldOfWord_encode hd)
    have hi : LdtAt S.rv ziskRegions pc zrom (ldOfWord w) (nextWord S.rv pc) :=
      ⟨h.al, h.x1, h.x2, h.h0, h.h1, h.h2, h.h3, h.tr⟩
    rcases load_cases (by decide) hacc hx.full with hb | hb | ⟨ha, -⟩ | hb | ⟨⟨v, hok⟩, hne⟩
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inl hb))))
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inl hb)))))
    · have hz : _root_.Zisk.Functions.readable _root_.Zisk.Functions.INPUT_ADDR 8 = true :=
        (zreadable_iff _ _).2 (.inl (by rw [inputAddr_toNat]; omega))
      obtain ⟨r, hm, hrd⟩ := readable_match _ 8 (by decide) hz
      refine StepCase.ofFreeLd ⟨pc, ldOfWord w, hx.atPc, hi, v1, hv1, ha, r, ?_, hrd, ?_⟩
      · erw [ha]; exact hm
      · erw [ha]; exact splitOk_zisk hm (by rw [inputAddr_toNat]; omega)
    · exact StepCase.ofFault ⟨pc, hx.atPc, .inl ⟨ldOfWord w, hi, v1, hv1, hb⟩⟩
    · exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inl ⟨.ld (ldOfWord w), hi, v1, v, hv1,
        ⟨hok.region, hok.zread, hok.zregs, hne, hok.inOff, hok.bytes⟩⟩⟩
  · have h := ctx_at_mem hx hd rfl (lwOfWord_encode hd hop)
    have hi : LwtAt S.rv ziskRegions pc zrom (lwOfWord op w) (nextWord S.rv pc) :=
      ⟨h.al, h.x1, h.x2, h.h0, h.h1, h.h2, h.h3, h.tr⟩
    rw [LOp.width_k hop] at hacc
    rcases load_cases (LKind.w_cases _) hacc hx.full with hb | hb | ⟨-, hn⟩ | hb | ⟨⟨v, hok⟩, -⟩
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inl hb))))
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inl hb)))))
    · cases op <;> simp_all [Transpiler.LOp.k, LKind.w]
    · exact StepCase.ofFault ⟨pc, hx.atPc, .inr (.inl ⟨lwOfWord op w, hi, v1, hv1, hb⟩)⟩
    · exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inl ⟨.lw (lwOfWord op w), hi, v1, v, hv1, hok⟩⟩

/-- A store at pc: its access permitted, refused, or a precondition broken. -/
theorem kind_store {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} {op : Transpiler.SzOp}
    (hx : Ctx zrom C S pc w) (hd : decode w = some (.store op)) : StepCase zrom C S := by
  obtain ⟨v1, hv1⟩ := getX_ready hx.ready (w.extractLsb' 15 5).toNat
  obtain ⟨v2, hv2⟩ := getX_ready hx.ready (w.extractLsb' 20 5).toNat
  have hacc := access_store hd hv1
  by_cases hop : op = .sd
  · subst hop
    have h := ctx_at_mem hx hd rfl (sdOfWord_encode hd)
    have hi : SdtAt S.rv ziskRegions pc zrom (sdOfWord w) (nextWord S.rv pc) :=
      ⟨h.al, h.x1, h.x2, h.h0, h.h1, h.h2, h.h3, h.tr⟩
    rcases store_cases (by decide) hacc (by decide) with hb | hb | hb | hb | hok
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inl hb))))
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inl hb)))))
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inr (.inr (.inr hb))))))))
    · exact StepCase.ofFault ⟨pc, hx.atPc, .inr (.inr (.inl ⟨sdOfWord w, hi, v1, hv1, hb⟩))⟩
    · exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inl ⟨.sd (sdOfWord w), hi, v1, v2, hv1, hv2,
        ⟨hok.region, hok.zwrite, hok.zregs⟩⟩⟩
  · have h := ctx_at_mem hx hd rfl (swOfWord_encode hd hop)
    have hi : SwtAt S.rv ziskRegions pc zrom (swOfWord op w) (nextWord S.rv pc) :=
      ⟨h.al, h.x1, h.x2, h.h0, h.h1, h.h2, h.h3, h.tr⟩
    rw [SzOp.width_k hop] at hacc
    rcases store_cases (SKind.w_cases _) hacc (by decide) with hb | hb | hb | hb | hok
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inl hb))))
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inl hb)))))
    · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inr (.inr (.inr hb))))))))
    · exact StepCase.ofFault ⟨pc, hx.atPc, .inr (.inr (.inr ⟨swOfWord op w, hi, v1, hv1, hb⟩))⟩
    · exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inl ⟨.sw (swOfWord op w), hi, v1, v2, hv1, hv2, hok⟩⟩

/-! ## The atomics -/

/-- An access in the RAM: ZisK's map reads and writes it, off the free
    input. -/
theorem ram_facts {a : BitVec 64} {n : Nat} (h : 0xA000_0000 ≤ a.toNat ∧ a.toNat + n ≤ 0xC000_0000) :
    _root_.Zisk.Functions.readable a n = true ∧ _root_.Zisk.Functions.writable a n = true ∧
      Equiv.InOff a n ∧ a ≠ _root_.Zisk.Functions.INPUT_ADDR :=
  ⟨(zreadable_iff a n).2 (.inr (.inr h)), (zwritable_iff a n).2 h, .inr (by unfold Equiv.IN_LO; omega),
    fun e => by rw [e, inputAddr_toNat] at h; omega⟩

/-- An instruction in the ROM sits below every RAM access. -/
theorem below_ram {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} (hx : Ctx zrom C S pc w)
    {a : BitVec 64} (h : 0xA000_0000 ≤ a.toNat) : pc.toNat + 4 ≤ a.toNat := by
  have := exec_rom (hx.exec pc hx.code); omega

theorem amo_allows (op : Transpiler.AmoOp) (d : Bool) :
    pma_allows_atomic_op .AMOCASQ op.r (aW d) = true := by
  cases op <;> cases d <;> decide

/-- An AMO at pc: its access permitted, or a precondition broken. -/
theorem kind_amo {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} {op : Transpiler.AmoOp} {d : Bool}
    (hx : Ctx zrom C S pc w) (hd : decode w = some (.amo op d)) : StepCase zrom C S := by
  obtain ⟨v1, hv1⟩ := getX_ready hx.ready (w.extractLsb' 15 5).toNat
  have hacc := access_amo hd hv1
  have h := ctx_at_mem hx hd rfl (amoOfWord_encode hd)
  rcases ram_cases hacc (.inl ⟨op, d, hd⟩) with hb | hb | ⟨hram, hr⟩
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inl hb))))
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inr (.inl hb)))))))
  by_cases hc : Overlaps v1 (aW d) MARCHID_CELL 8
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inr (.inr (.inr
      ⟨_, _, _, hacc, by decide, hc⟩))))))))
  by_cases hA : v1.toNat / 16 ≠ (v1.toNat + aW d - 1) / 16
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inl ⟨_, _, hacc, hA⟩))))))
  obtain ⟨hzr, hzw, hio, hne⟩ := ram_facts hram
  obtain ⟨r, hm, hwr⟩ := writable_match v1 (aW d) (aW_cases d) hzw
  obtain ⟨v, hok⟩ := lwOk_of (s := S.rv) (aW_cases d) hzr hr hio hx.full
  have hg : v1.toNat % 16 + aW d ≤ 16 := by cases d <;> simp only [aW] at hA ⊢ <;> omega
  refine StepCase.ofOneOkT ⟨pc, hx.atPc, .inr (.inr (.inr (.inl ⟨amoOfWord op d w, h, v1, hv1,
    ⟨⟨r, hm, ⟨region_readable hm, hwr, ?_⟩, alignOk_gran hm (by simp [amoAcc]) (aW_cases d) hg⟩,
      hzr, hzw, hr, ?_, hio, v, hok.bytes⟩⟩)))⟩
  · rw [region_atomic hm]; exact amo_allows op d
  · simp [hne]

/-- An lr at pc: its access permitted, or a precondition broken. -/
theorem kind_lr {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} {d : Bool}
    (hx : Ctx zrom C S pc w) (hd : decode w = some (.lr d)) : StepCase zrom C S := by
  obtain ⟨v1, hv1⟩ := getX_ready hx.ready (w.extractLsb' 15 5).toNat
  have hacc := access_lr hd hv1
  have h := ctx_at_mem hx hd rfl (lrOfWord_encode hd)
  rcases ram_cases hacc (.inr (.inl ⟨d, hd⟩)) with hb | hb | ⟨hram, hr⟩
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inl hb))))
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inr (.inl hb)))))))
  obtain ⟨hzr, -, hio, hne⟩ := ram_facts hram
  exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inr (.inr (.inr (.inr (.inl (.inl ⟨lrOfWord d w, h, v1, hv1,
    lwOk_of (aW_cases d) hzr hr hio hx.full, by simp [hne], .inr (below_ram hx hram.1)⟩)))))⟩

/-- An sc at pc: its access permitted, or a precondition broken. -/
theorem kind_sc {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop}
    {S : ZiskRiscv.State model} {pc : BitVec 64} {w : BitVec 32} {d : Bool}
    (hx : Ctx zrom C S pc w) (hd : decode w = some (.sc d)) : StepCase zrom C S := by
  obtain ⟨v1, hv1⟩ := getX_ready hx.ready (w.extractLsb' 15 5).toNat
  obtain ⟨v2, hv2⟩ := getX_ready hx.ready (w.extractLsb' 20 5).toNat
  have hacc := access_sc hd hv1
  have h := ctx_at_mem hx hd rfl (scOfWord_encode hd)
  rcases ram_cases hacc (.inr (.inr ⟨d, hd⟩)) with hb | hb | ⟨hram, hr⟩
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inl hb))))
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inr (.inl hb)))))))
  by_cases hc : Overlaps v1 (aW d) MARCHID_CELL 8
  · exact StepCase.ofBroken hx (.inr (.inr (.inr (.inr (.inr (.inr (.inr (.inr
      ⟨_, _, _, hacc, by decide, hc⟩))))))))
  obtain ⟨-, hzw, -, -⟩ := ram_facts hram
  exact StepCase.ofOneOkT ⟨pc, hx.atPc, .inr (.inr (.inr (.inr (.inl (.inr ⟨scOfWord d w, h, v1, v2, hv1, hv2,
    swOk_of (aW_cases d) hzw hr, .inr (below_ram hx hram.1)⟩)))))⟩

/-! ## The memory group, assembled -/

/-- `KindCases.mem`: a memory access at pc, the step's cases. -/
theorem kind_mem {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {C : BitVec 64 → Prop} :
    ∀ S pc w i, Ctx zrom C S pc w → decode w = some i → group i = .mem → StepCase zrom C S := by
  intro S pc w i hx hd hg
  cases i with
  | load op => exact kind_load hx hd
  | store op => exact kind_store hx hd
  | amo op d => exact kind_amo hx hd
  | lr d => exact kind_lr hx hd
  | sc d => exact kind_sc hx hd
  | _ => simp [group] at hg

end Equiv.Final
