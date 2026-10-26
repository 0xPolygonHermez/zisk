/-
  The simulation on reading `marchid` (contract § 6): `csrr rd, marchid`,
  which the machine leaves to sail-riscv (rd gets the register marchid,
  ZisK's ID in ZisK's profile), and the transpiler makes a copy of
  marchid's cell (the BIOS writes the ID there). The cell holding the ID
  when the read runs is the run's condition: the contract has no program
  store to it.
-/
import Equiv.LrScSim
import Equiv.RiscvDecodeCsr
import Equiv.RiscvCsr
import Equiv.ZiskCsr

set_option linter.unusedSimpArgs false

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv)

/-- `csrr rd, marchid`: `csrrs rd, 0xF12, x0`. -/
structure MarchidInstr where
  rd : BitVec 5

def MarchidInstr.encode (i : MarchidInstr) : BitVec 32 := 0xF12#12 ++ 0#5 ++ 2#3 ++ i.rd ++ 0x73#7

/-- ZisK's ID, which marchid holds on both sides. -/
def ARCH_ID : BitVec 64 := 0x0FFFEEEE#64

/-- marchid's cell (`CSR_ADDR + 8 · 0xF12`), where the BIOS writes the ID. -/
def MARCHID_CELL : BitVec 64 := Transpiler.CSR_ADDR + 0xF12 * 8

theorem MarchidInstr.fields (i : MarchidInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x73#7 ∧ v.extractLsb 14 12 = 2#3 ∧ v.extractLsb 31 20 = 0xF12#12 ∧
    v.extractLsb 19 15 = 0#5 ∧ v.extractLsb 11 7 = i.rd ∧ (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, MarchidInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- The fields the machine's decoder reads. -/
theorem MarchidInstr.fields_m (i : MarchidInstr) :
    (ZiskRiscv.fields i.encode).opcode = 0x73 ∧ (ZiskRiscv.fields i.encode).funct3 = 2 ∧
    (ZiskRiscv.fields i.encode).csr = 0xF12 ∧ (ZiskRiscv.fields i.encode).rs1 = 0 ∧
    (ZiskRiscv.fields i.encode).rd = i.rd.toNat := by
  simp only [ZiskRiscv.fields]
  rw [show i.encode.extractLsb' 0 7 = 0x73#7 by simp only [MarchidInstr.encode]; bv_decide,
    show i.encode.extractLsb' 12 3 = 2#3 by simp only [MarchidInstr.encode]; bv_decide,
    show i.encode.extractLsb' 20 12 = 0xF12#12 by simp only [MarchidInstr.encode]; bv_decide,
    show i.encode.extractLsb' 15 5 = 0#5 by simp only [MarchidInstr.encode]; bv_decide,
    show i.encode.extractLsb' 7 5 = i.rd by simp only [MarchidInstr.encode]; bv_decide]
  exact ⟨rfl, rfl, rfl, rfl, rfl⟩

theorem MarchidInstr.kind (i : MarchidInstr) : ZiskRiscv.kind (ZiskRiscv.fields i.encode) = .csrrs := by
  obtain ⟨h1, h2, -, -, -⟩ := i.fields_m
  simp only [ZiskRiscv.kind, h1, h2]

/-- The machine leaves it to sail-riscv: a read of a CSR that is no ZisK call. -/
theorem MarchidInstr.plain (i : MarchidInstr) (hrd : i.rd ≠ 0) : Equiv.Machine.Plain i.encode := by
  intro fetch a hf
  obtain ⟨-, -, h3, h4, h5⟩ := i.fields_m
  have hr : i.rd.toNat ≠ 0 := fun e => hrd (BitVec.eq_of_toNat_eq (by simpa using e))
  simp only [ZiskRiscv.decode, hf, i.kind, ZiskRiscv.decodeCsrrs, h3, h4, h5]
  simp [hr]

/-- The spec's translation: the copy of marchid's cell into rd. -/
theorem MarchidInstr.spec (i : MarchidInstr) (hrd : i.rd ≠ 0) {zrom : BitVec 64 → _root_.Zisk.zisk_inst}
    {pc : BitVec 64} {next : Option (BitVec 32)} (h : Transpiled zrom pc i.encode next) :
    zrom pc = Transpiler.mk (.A_IMM, 0) (.B_MEM, MARCHID_CELL) (.ZOP_COPYB ()) (Transpiler.storeReg i.rd.toNat) 4 4 := by
  obtain ⟨-, -, h3, h4, h5⟩ := i.fields_m
  have hr : i.rd.toNat ≠ 0 := fun e => hrd (BitVec.eq_of_toNat_eq (by simpa using e))
  have ho : Transpiler.opcode i.encode = 0x73#7 := by simp only [Transpiler.opcode, MarchidInstr.encode]; bv_decide
  have hf3 : Transpiler.funct3 i.encode = 2#3 := by simp only [Transpiler.funct3, MarchidInstr.encode]; bv_decide
  have hne : (i.encode == 0x00000073#32) = false := by
    simp only [MarchidInstr.encode, beq_eq_false_iff_ne]; intro e; apply hrd; bv_decide
  have hd : ∀ next, ZiskRiscv.decode (Transpiler.fetch2 pc i.encode next) pc.toNat = .riscv := by
    intro next
    have := i.plain hrd (Transpiler.fetch2 pc i.encode next) pc.toNat (by simp [Transpiler.fetch2])
    exact this
  refine h.one fun ia => ?_
  simp only [Transpiler.convert, Transpiler.decode, ho, hf3, hne, Transpiler.csr3, Transpiler.csr, hd, i.kind, h3, h4, h5]
  simp [hr, MARCHID_CELL]
  rfl

set_option maxHeartbeats 16000000 in
/-- `csrr rd, marchid`, rd not x0, marchid's cell holding the ID: one step
    of the machine (sail-riscv's), and ZisK's copy, keep the states
    related. -/
theorem marchidstep_m (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : MarchidInstr) (hrd : i.rd ≠ 0) (next : Option (BitVec 32))
    (hi : WordsAt s rs pc zrom i.encode next)
    (hcell : Equiv.memRead s.mem 8 MARCHID_CELL.toNat = some ARCH_ID) (n : Nat) :
    ∃ s' z', MStep rs n s s' ∧ ZRun zrom 1 z z' ∧ Rel s' z' rs ∧ RamOnly s.mem s'.mem ∧ FKeep z z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hcsr, hr1x, hrdx, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have ld : i.rd.toNat < 32 := i.rd.isLt
  -- sail-riscv: the read of marchid
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_gen s rs hr pc' hpc _ _ _ _ hw (.CSRReg (0xF12#12, .Regidx 0, .Regidx i.rd, .CSRRS))
    (by
      intro t htp ⟨w, hw'⟩ _ _
      have := decode_csrrs t _ hop hf3 htp w hw'
      rw [hcsr, hr1x, hrdx] at this; exact this)
    i.rd.toNat ARCH_ID (pc' + 4)
    (by
      intro t _ _ hmr _ hwf hnext ho
      rw [insert_same t _ _ hnext]
      exact execute_marchid t hwf hmr.priv i.rd ARCH_ID
        (by rw [ho _ (by decide) (by decide) (by decide)]; exact hr.march)) n
  -- ZisK: the copy of the cell
  have hrom := i.spec hrd hi.tr
  have hex := Equiv.Zisk.execute_bmem z MARCHID_CELL i.rd.toNat (by omega) 4 xs pc' st hx hzpc hst ARCH_ID
    (by decide) (by decide) (by decide)
    (by rw [Equiv.memRead_normIn hzm (by right; decide)]; exact hcell)
  have hzr := hex
  rw [← hrom, ← zstep_run zrom z pc' hzpc] at hzr
  refine ⟨s', _, .plain pc' _ hpc (readWord_of' hi.h0 hi.h1 hi.h2 hi.h3) (i.plain hrd) hrun,
    .step hzr (.done _), ?_, RamOnly.of_eq hmem, zstep_fkeep hzpc (by rw [hrom]; rfl) hzr⟩
  refine ⟨hr.keep hwf ho, ⟨Equiv.Zisk.xsUpd xs i.rd.toNat ARCH_ID, ?_, fun k hk1 hk2 => ?_⟩,
    ⟨_, hpc'', by simp⟩,
    ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, ?_⟩
  · simp only [Std.ExtDHashMap.get?_insert]
    simp [linkRegs_xs _ xs _ _ hx]
  · rw [hxs' k hk2]
    exact regs_after0 s xs i.rd.toNat ARCH_ID ld agree k hk1 hk2
  · rw [hmem]; exact hzm

end Equiv.Sim
