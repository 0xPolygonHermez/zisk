/-
  The simulation, one step, on a store: `sd rs2, imm(rs1)` and the
  transpiler's store_op (copyb of r[rs2] to the 8 bytes at r[rs1] + imm).
  Both sides write the same 8 bytes at the same address of the same memory,
  so their memories stay equal.

  As for loads, what the store needs when it runs is a hypothesis of the
  step: the address aligned and in a writable region on both sides (RAM),
  off ZisK's register file.
-/
import Equiv.LoadSim
import Equiv.RiscvStore
import Equiv.RiscvStepStore
import Equiv.ZiskStore

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- An `sd`, by its fields; the immediate in the two parts the S format
    splits it into (taken whole, as `immHi ++ immLo`, so that no equation
    between a 7 + 5 and a 12-bit word is ever stated: Lean would insert a
    coercion between the widths). -/
structure SdInstr where
  rs1 : BitVec 5
  rs2 : BitVec 5
  immHi : BitVec 7
  immLo : BitVec 5

def SdInstr.encode (i : SdInstr) : BitVec 32 :=
  i.immHi ++ i.rs2 ++ i.rs1 ++ 3#3 ++ i.immLo ++ 0x23#7

theorem SdInstr.fields (i : SdInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x23#7 ∧ v.extractLsb 14 12 = 3#3 ∧
    v.extractLsb 31 25 = i.immHi ∧ v.extractLsb 11 7 = i.immLo ∧
    v.extractLsb 24 20 = i.rs2 ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, SdInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- `sd` instruction `i` at pc: its encoding in executable memory there,
    rs1, rs2 ≠ x0, and the transpiler's instruction for it at pc. -/
structure SdAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SdInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rs1 : i.rs1 ≠ 0
  rs2 : i.rs2 ≠ 0
  rom : zrom pc = Equiv.Zisk.storeInst i.rs1.toNat i.rs2.toNat ((i.immHi ++ i.immLo).signExtend 64) 8

/-- What the store needs when it runs, at address a: aligned, in a writable
    region on both sides, off ZisK's register file. -/
structure SdOk (rs : List PMA_Region) (a : BitVec 64) : Prop where
  region : ∃ r, matching_pma_region rs (.Physaddr a) 8 = some r ∧ r.attributes.writable = true ∧
    AlignOk r (.Store .Data) a 8
  zwrite : _root_.Zisk.Functions.writable a 8 = true
  zregs : _root_.Zisk.Functions.hits_registers a 8 = false

set_option maxHeartbeats 4000000 in
/-- One step of each on an `sd` keeps the states related, when the store's
    address `a` (rs1 + imm) is good to write. -/
theorem sdstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SdInstr) (hi : SdAt s rs pc zrom i)
    (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (hv2 : getX s i.rs2.toNat = some v2)
    (hok : SdOk rs (v1 + (i.immHi ++ i.immLo).signExtend 64)) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = Equiv.memWrite (n := 8) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat v2 := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, ⟨st, hst, hze⟩, hzm⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hhi, hlo, hr2x, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have e1 := ne_zero_toNat hi.rs1
  have e2 := ne_zero_toNat hi.rs2
  have a1 := agree i.rs1.toNat (by omega) l1
  have a2 := agree i.rs2.toNat (by omega) l2
  have hxv1 : xs[i.rs1.toNat]! = v1 := by rw [hv1] at a1; exact (Option.some.inj a1).symm
  have hxv2 : xs[i.rs2.toNat]! = v2 := by rw [hv2] at a2; exact (Option.some.inj a2).symm
  obtain ⟨⟨r, hm, hwr, hal⟩, hzw, hzn⟩ := hok
  obtain ⟨s', hrun, hmem, hwf, hpc'', hxs', ho⟩ :=
    try_step_store s rs hr pc' hpc _ _ _ _ hw (.STORE (i.immHi ++ i.immLo, .Regidx i.rs2, .Regidx i.rs1, 8))
      (by intro t htp ⟨w, hw'⟩ hmt _
          have := decode_sd t _ hop hf3 htp w hw'
          rw [hhi, hlo, hr2x, hr1x] at this; exact this)
      (Equiv.memWrite (n := 8) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat v2)
      (by intro t he htm hmr hsec hwf'
          rw [execute_store_eq]
          obtain ⟨sv, hsv, hpm⟩ := hr.pmm
          rw [← htm]
          exact execute_sd t hwf' rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rs2 (i.immHi ++ i.immLo) v1 v2
            (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) r hm hwr hal) n
  have hxv1' : xs[(i.rs1.toNat : Int)]! = v1 := by rw [getElem!_int]; exact hxv1
  have hxv2' : xs[(i.rs2.toNat : Int)]! = v2 := by rw [getElem!_int]; exact hxv2
  have hz := Equiv.Zisk.execute_sd z i.rs1.toNat i.rs2.toNat ((i.immHi ++ i.immLo).signExtend 64) (by omega) (by omega)
    xs pc' st hx hzpc hst (by rw [hxv1']; exact hzw) (by rw [hxv1']; exact hzn)
  refine ⟨s', _, hrun, by rw [zstep_run zrom z pc' hzpc, hi.rom]; exact hz, ⟨hr.keep hwf ho, ⟨xs, ?_, ?_⟩,
    ⟨_, hpc'', by simp⟩, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩,
    ?_⟩, hpc'', hmem⟩
  · simp [Std.ExtDHashMap.get?_insert, hx]
  · intro k hk1 hk2; rw [hxs' k hk2]; exact agree k hk1 hk2
  · rw [hmem]; simp only [hxv1', hxv2']; exact Equiv.normIn_memWrite hzm _ _

end Equiv.Sim
