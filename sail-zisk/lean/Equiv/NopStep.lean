/-
  `csrs ID, x0`, an access to a ZisK CSR number that ZisK makes nothing
  of: the machine's step moves pc on by 4 (`ZiskRiscv.Decoded.nop`), and
  ZisK's, on the spec's copyb(0, 0) that stores nothing, does the same. The
  states stay related.
-/
import Equiv.CallRun

namespace Equiv.Sim

open Equiv.Riscv Equiv.Machine
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- `csrs ID, x0`'s translation: copyb(0, 0), stored nowhere, on to pc + 4. -/
def csrNop : _root_.Zisk.zisk_inst :=
  Transpiler.mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) (j1 := 4) (j2 := 4)

/-- The machine's decoder makes a nop of a csrrs only. -/
theorem nop_kind {fetch : Nat → Option (BitVec 32)} {a : Nat} {w : BitVec 32} (hf : fetch a = some w)
    (hd : ZiskRiscv.decode fetch a = .nop) : ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrs := by
  simp only [ZiskRiscv.decode, hf] at hd
  revert hd
  cases ZiskRiscv.kind (ZiskRiscv.fields w) <;> simp
  all_goals simp only [ZiskRiscv.decodeCsrrsi, ZiskRiscv.decodeCsrrwi, ZiskRiscv.memsetPair]
  all_goals (repeat' split) <;> simp

theorem csrrs_fields {w : BitVec 32} (h : ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrs) :
    w.extractLsb' 0 7 = 0x73#7 ∧ w.extractLsb' 12 3 = 2#3 := by
  unfold ZiskRiscv.kind ZiskRiscv.fields at h
  split at h <;> (try split at h) <;> (try split at h) <;> simp_all <;> rename_i h1 h2 <;>
    exact ⟨BitVec.eq_of_toNat_eq h1, BitVec.eq_of_toNat_eq h2⟩

/-- The spec's decoder on a csrrs: a CSR instruction. -/
theorem decode_csrrs {w : BitVec 32} (h : ZiskRiscv.kind (ZiskRiscv.fields w) = .csrrs) :
    Transpiler.decode w = some .csr := by
  obtain ⟨h1, h2⟩ := csrrs_fields h
  have ho : (Transpiler.opcode w).toNat = 115 := by
    simp only [Transpiler.opcode, BitVec.extractLsb]; rw [h1]; rfl
  have hf : (Transpiler.funct3 w).toNat = 2 := by
    simp only [Transpiler.funct3, BitVec.extractLsb]; rw [h2]; rfl
  have n1 : (w == 0x00000073#32) = false := by simp; intro e; subst e; revert h2; decide
  have n2 : (w == 0x00100073#32) = false := by simp; intro e; subst e; revert h2; decide
  simp only [Transpiler.decode, ho, hf, n1, n2]; rfl

/-- The spec's translation of a word the machine makes a nop of: `csrNop`. -/
theorem nop_rom {zrom : BitVec 64 → _root_.Zisk.zisk_inst} {pc : BitVec 64} {w : BitVec 32}
    {next next2 : Option (BitVec 32)} (ht : Transpiled zrom pc w next next2)
    (hd : ∀ fetch : Nat → Option (BitVec 32), fetch pc.toNat = some w → ZiskRiscv.decode fetch pc.toNat = .nop) :
    zrom pc = csrNop := by
  obtain ⟨ia, l, hl, hz⟩ := ht
  have hf2 : Transpiler.fetch2 pc w next pc.toNat = some w := by simp [Transpiler.fetch2]
  have hk := nop_kind hf2 (hd _ hf2)
  have hcs : Transpiler.csr w pc next ia = some [(pc, csrNop)] ∨ Transpiler.csr w pc next ia = none := by
    simp only [Transpiler.csr, hd _ hf2]
    split <;> simp [csrNop]
  have hl' : Transpiler.csr w pc next ia = some l := by
    simp only [Transpiler.convert, decode_csrrs hk, Transpiler.csr3] at hl
    cases next2 with
    | none => exact hl
    | some n2 => simpa [hk] using hl
  rcases hcs with h | h <;> rw [h] at hl'
  · cases hl'; exact hz (pc, csrNop) (by simp)
  · cases hl'

set_option maxHeartbeats 4000000 in
/-- ZisK's step on `csrNop`, from related states: related to sail-riscv's
    state with pc on by 4, the fcall registers as they were. -/
theorem nop_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (hrom : zrom pc = csrNop) :
    ∃ z', (zstep zrom).run z = .ok false z' ∧
      Rel { s with regs := s.regs.insert Register.PC (pc + 4) } z' rs ∧ FKeep z z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  have hn : csrNop = Transpiler.mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) (Transpiler.storeReg 0) 4 4 := rfl
  have hex := Equiv.Zisk.execute_mk z (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) Equiv.Zisk.copyb_pure 0 (by omega)
      xs pc' st hx hzpc hst 0 0 (Or.inl rfl) (Or.inl rfl)
  have h2 : (zstep zrom).run z = (_root_.Zisk.Functions.zisk_execute
      (Transpiler.mk (.A_IMM, 0) (.B_IMM, 0) (.ZOP_COPYB ()) (Transpiler.storeReg 0) 4 4)).run z := by
    rw [zstep_run zrom z pc' hzpc, hrom, hn]
  rw [hex] at h2
  refine ⟨_, h2, ⟨?_, ⟨xs, ?_, fun k h1 h2 => ?_⟩, ⟨pc' + 4, ?_, ?_⟩, ⟨BitVec.addInt st 1, ?_, ?_⟩, ?_⟩,
    zstep_fkeep hzpc (by rw [hrom]; rfl) h2⟩
  · refine hr.keep ⟨fun r => ?_⟩ (fun r _ h2 _ _ _ => ?_)
    · simp only [Std.ExtDHashMap.get?_insert]; split
      · simp
      · exact hr.wf.all r
    · simp [Std.ExtDHashMap.get?_insert, Ne.symm h2]
  · simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs, hx]
  · rw [getX_setPC]; exact agree k h1 h2
  · simp
  · simp
  · simp [Std.ExtDHashMap.get?_insert]
  · simp [Std.ExtDHashMap.get?_insert, Equiv.Zisk.linkRegs, hze]
  · exact hzm

end Equiv.Sim
