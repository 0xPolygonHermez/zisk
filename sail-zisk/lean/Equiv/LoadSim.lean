/-
  The simulation, one step, on a load: `ld rd, imm(rs1)` and the transpiler's
  load_op (copyb of the 8 bytes at r[rs1] + imm). Both models keep memory as
  the same map of bytes (`Rel.mem`), so both read the same 8 bytes.

  What a load needs at the moment it runs depends on the registers, so it
  is a hypothesis of the step, not part of the code at pc: the address in
  a readable region, aligned or within its 16-byte granule (sail-riscv's
  PMA check, which leaves such an access unsplit), off the
  register file and not the free input's (sail-zisk's read_mem), and the
  bytes there.
-/
import Equiv.Program
import Equiv.RiscvLoad
import Equiv.RiscvDecodeMem
import Equiv.ZiskLoad

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- An `ld`, by its fields. -/
structure LdInstr where
  rd : BitVec 5
  rs1 : BitVec 5
  imm : BitVec 12

def LdInstr.encode (i : LdInstr) : BitVec 32 :=
  i.imm ++ i.rs1 ++ 3#3 ++ i.rd ++ 0x03#7

theorem LdInstr.fields (i : LdInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x03#7 ∧ v.extractLsb 14 12 = 3#3 ∧ v.extractLsb 31 20 = i.imm ∧
    v.extractLsb 11 7 = i.rd ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, LdInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- `ld` instruction `i` at pc: its encoding in executable memory there, rd,
    rs1 ≠ x0, and the transpiler's instruction for it at pc. -/
structure LdAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LdInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rd : i.rd ≠ 0
  rs1 : i.rs1 ≠ 0
  rom : zrom pc = Equiv.Zisk.loadInst (.ZOP_COPYB ()) i.rd.toNat i.rs1.toNat (i.imm.signExtend 64) 8

/-- What the load needs when it runs, at address a: in a readable region on
    both sides, aligned or within the region's granule (`AlignOk`), off ZisK's register file and free input, and the
    8 bytes there. -/
structure LdOk (s : RVState) (rs : List PMA_Region) (a : BitVec 64) (v : BitVec 64) : Prop where
  region : ∃ r, matching_pma_region rs (.Physaddr a) 8 = some r ∧ r.attributes.readable = true ∧
    AlignOk r (.Load .Data) a 8
  zread : _root_.Zisk.Functions.readable a 8 = true
  zregs : _root_.Zisk.Functions.hits_registers a 8 = false
  zin : a ≠ _root_.Zisk.Functions.INPUT_ADDR
  bytes : Equiv.memRead s.mem 8 a.toNat = some v

theorem execute_load_eq (imm : BitVec 12) (a b : regidx) (u : Bool) :
    execute (.LOAD (imm, a, b, u, 8)) = execute_LOAD imm a b u 8 := by
  simp [execute]

set_option maxHeartbeats 4000000 in
/-- One step of each on an `ld` keeps the states related, when the load's
    address `a` (rs1 + imm) is good to read and holds `v`. -/
theorem ldstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LdInstr) (hi : LdAt s rs pc zrom i)
    (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (v : BitVec 64)
    (hok : LdOk s rs (v1 + i.imm.signExtend 64) v) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧ s'.mem = s.mem := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have ld : i.rd.toNat < 32 := i.rd.isLt
  have e1 := ne_zero_toNat hi.rs1
  have ed := ne_zero_toNat hi.rd
  have a1 := agree i.rs1.toNat (by omega) l1
  have hxv : xs[i.rs1.toNat]! = v1 := by rw [hv1] at a1; exact (Option.some.inj a1).symm
  have hrd5 : (byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)).extractLsb 11 7 ≠ 0#5 := by
    rw [hrdx]; exact hi.rd
  obtain ⟨⟨r, hm, hrb, hal⟩, hzr, hzn, hzi, hbytes⟩ := hok
  refine alu_step_rel s z rs hr xs hx agree pc' hpc hzpc _ _ _ _ hw
    (.LOAD (i.imm, .Regidx i.rs1, .Regidx i.rd, false, 8)) ?_ i.rd.toNat ed ld v ?_ zrom hzm ?_ n
  · intro t htp ⟨w, hw'⟩ hmt _
    have := decode_ld t _ hop hf3 hrd5 htp w hw'
    rw [himm, hr1x, hrdx] at this; exact this
  · intro t he htm hmr hsec hwf
    rw [execute_load_eq]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    have hv := Equiv.riscv_readBytes t 8 _ v (htm ▸ hbytes)
    exact execute_ld t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rd i.imm v1 (getX_of_agree he l1 hv1)
      r hm hrb hal v hv
  · rw [zstep_run zrom z pc' hzpc, hi.rom]
    have hxv' : xs[(i.rs1.toNat : Int)]! = v1 := by rw [getElem!_int]; exact hxv
    refine ⟨_, Equiv.Zisk.execute_ld z i.rd.toNat i.rs1.toNat _ (by omega) (by omega) xs pc' st hx hzpc hst v
      (by rw [hxv']; exact hzr) (by rw [hxv']; exact hzn) (by rw [hxv']; exact hzi)
      (by rw [hxv', hzm]; exact hbytes), by simp [Std.ExtDHashMap.get?_insert], by simp,
      ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩, rfl⟩

end Equiv.Sim
