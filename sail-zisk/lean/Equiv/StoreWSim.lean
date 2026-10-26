/-
  The simulation, one step, on the narrow stores: sb, sh and sw, and the
  transpiler's store_op for each (copyb of r[rs2] to the 1, 2 or 4 bytes at
  r[rs1] + imm), as Equiv/StoreSim.lean has it for sd. Both sides write the
  same low bytes of rs2 at the same address of the same memory.
-/
import Equiv.StoreSim
import Equiv.LoadWSim
import Equiv.RiscvStoreW
import Equiv.RiscvDecodeStore
import Equiv.ZiskStoreW

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

/-- The narrow stores. -/
inductive SKind | sb | sh | sw

/-- Its funct3. -/
def SKind.f3 : SKind → BitVec 3
  | .sb => 0 | .sh => 1 | .sw => 2

/-- How many bytes it writes. -/
def SKind.w : SKind → Nat
  | .sb => 1 | .sh => 2 | .sw => 4

/-- sail-riscv's decoder on each narrow store. -/
theorem SKind.decode (k : SKind) (t : RVState) (w : BitVec 32)
    (hop : w.extractLsb 6 0 = 0x23#7) (hf3 : w.extractLsb 14 12 = k.f3)
    (hp : t.regs.get? Register.cur_privilege = some .Machine)
    (sec : RegisterType Register.mseccfg) (hsec : t.regs.get? Register.mseccfg = some sec) :
    (interpretSailEffects (ext_decode w)).run t
      = .ok (.STORE (w.extractLsb 31 25 ++ w.extractLsb 11 7, .Regidx (w.extractLsb 24 20),
          .Regidx (w.extractLsb 19 15), (k.w : Int))) t := by
  cases k
  · exact decode_sb t w hop hf3 hp sec hsec
  · exact decode_sh t w hop hf3 hp sec hsec
  · exact decode_sw t w hop hf3 hp sec hsec

/-- A narrow store, by its kind and fields; the immediate in its two parts,
    as for `sd`. -/
structure SwInstr where
  k : SKind
  rs1 : BitVec 5
  rs2 : BitVec 5
  immHi : BitVec 7
  immLo : BitVec 5

def SwInstr.encode (i : SwInstr) : BitVec 32 :=
  i.immHi ++ i.rs2 ++ i.rs1 ++ i.k.f3 ++ i.immLo ++ 0x23#7

theorem SwInstr.fields (i : SwInstr) :
    let v := byte i.encode 3 ++ byte i.encode 2 ++ (byte i.encode 1 ++ byte i.encode 0)
    v.extractLsb 6 0 = 0x23#7 ∧ v.extractLsb 14 12 = i.k.f3 ∧
    v.extractLsb 31 25 = i.immHi ∧ v.extractLsb 11 7 = i.immLo ∧
    v.extractLsb 24 20 = i.rs2 ∧ v.extractLsb 19 15 = i.rs1 ∧
    (byte i.encode 0).extractLsb 1 0 = 3#2 := by
  intro v; simp only [v, SwInstr.encode, byte]; refine ⟨?_, ?_, ?_, ?_, ?_, ?_, ?_⟩ <;> bv_decide

/-- A narrow store `i` at pc: its encoding in executable memory there, rs1,
    rs2 ≠ x0, and the transpiler's instruction for it at pc. -/
structure SwAt (s : RVState) (rs : List PMA_Region) (pc : BitVec 64)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SwInstr) : Prop where
  al : pc.toNat % 4 = 0
  x1 : ExecAt rs pc
  x2 : ExecAt rs (pc + 2)
  h0 : s.mem.get? pc.toNat = some (byte i.encode 0)
  h1 : s.mem.get? (pc.toNat + 1) = some (byte i.encode 1)
  h2 : s.mem.get? (pc.toNat + 2) = some (byte i.encode 2)
  h3 : s.mem.get? (pc.toNat + 3) = some (byte i.encode 3)
  rs1 : i.rs1 ≠ 0
  rs2 : i.rs2 ≠ 0
  rom : zrom pc = Equiv.Zisk.storeInst i.rs1.toNat i.rs2.toNat ((i.immHi ++ i.immLo).signExtend 64) i.k.w

/-- What a store of w bytes needs when it runs, at address a: in a
    writable region on both sides, off ZisK's register file. -/
structure SwOk (rs : List PMA_Region) (a : BitVec 64) (w : Nat) : Prop where
  region : ∃ r, matching_pma_region rs (.Physaddr a) w = some r ∧ r.attributes.writable = true ∧
    AlignOk r (.Store .Data) a w
  zwrite : _root_.Zisk.Functions.writable a w = true
  zregs : _root_.Zisk.Functions.hits_registers a w = false

theorem execute_store_w (imm : BitVec 12) (a b : regidx) (w : Int) :
    execute (.STORE (imm, a, b, w)) = execute_STORE imm a b w.toNat := by
  simp [execute]

/-- sail-riscv's execution of each narrow store. -/
theorem SKind.exec (k : SKind) (t : RVState) (hwf : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType Register.mseccfg) (hsec : t.regs.get? Register.mseccfg = some sec)
    (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) k.w = some r)
    (hwr : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) (v1 + sign_extend (m := 64) imm) k.w) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) k.w)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := (Equiv.memWrite (n := k.w) t.mem (v1 + sign_extend (m := 64) imm).toNat
              (v2.setWidth (8 * k.w))) } := by
  cases k
  · exact execute_sb t hwf rs hs sec hsec hpm rs1 rs2 imm v1 v2 h1 h2 r hm hwr hal
  · exact execute_sh t hwf rs hs sec hsec hpm rs1 rs2 imm v1 v2 h1 h2 r hm hwr hal
  · exact execute_sw t hwf rs hs sec hsec hpm rs1 rs2 imm v1 v2 h1 h2 r hm hwr hal

set_option maxHeartbeats 4000000 in
/-- One step of each on a narrow store keeps the states related, when the
    store's address `a` (rs1 + imm) is good to write. -/
theorem swstep_rel (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SwInstr) (hi : SwAt s rs pc zrom i)
    (v1 v2 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1) (hv2 : getX s i.rs2.toNat = some v2)
    (hok : SwOk rs (v1 + (i.immHi ++ i.immLo).signExtend 64) i.k.w) (n : Nat) :
    ∃ s' z', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      (zstep zrom).run z = .ok false z' ∧ Rel s' z' rs ∧
      s'.regs.get? Register.PC = some (pc + 4) ∧
      s'.mem = Equiv.memWrite (n := i.k.w) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat
        (v2.setWidth (8 * i.k.w)) := by
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
    try_step_store s rs hr pc' hpc _ _ _ _ hw
      (.STORE (i.immHi ++ i.immLo, .Regidx i.rs2, .Regidx i.rs1, (i.k.w : Int)))
      (by intro t htp ⟨w, hw'⟩ hmt _
          have := i.k.decode t _ hop hf3 htp w hw'
          rw [hhi, hlo, hr2x, hr1x] at this; exact this)
      (Equiv.memWrite (n := i.k.w) s.mem (v1 + (i.immHi ++ i.immLo).signExtend 64).toNat (v2.setWidth (8 * i.k.w)))
      (by intro t he htm hmr hsec hwf'
          rw [execute_store_w, Int.toNat_natCast]
          obtain ⟨sv, hsv, hpm⟩ := hr.pmm
          rw [← htm]
          exact i.k.exec t hwf' rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rs2 (i.immHi ++ i.immLo) v1 v2
            (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) r hm hwr hal) n
  have hxv1' : xs[(i.rs1.toNat : Int)]! = v1 := by rw [getElem!_int]; exact hxv1
  have hxv2' : xs[(i.rs2.toNat : Int)]! = v2 := by rw [getElem!_int]; exact hxv2
  have hz := Equiv.Zisk.execute_store z i.rs1.toNat i.rs2.toNat ((i.immHi ++ i.immLo).signExtend 64) i.k.w
    (by omega) (by omega) xs pc' st hx hzpc hst (by rw [hxv1']; exact hzw) (by rw [hxv1']; exact hzn)
  refine ⟨s', _, hrun, by rw [zstep_run zrom z pc' hzpc, hi.rom]; exact hz, ⟨hr.keep hwf ho, ⟨xs, ?_, ?_⟩,
    ⟨_, hpc'', by simp⟩, ⟨BitVec.addInt st 1, by simp [Std.ExtDHashMap.get?_insert], by simp [Std.ExtDHashMap.get?_insert, hze]⟩,
    ?_⟩, hpc'', hmem⟩
  · simp [Std.ExtDHashMap.get?_insert, hx]
  · intro k hk1 hk2; rw [hxs' k hk2]; exact agree k hk1 hk2
  · rw [hmem]; simp only [hxv1', hxv2']; exact Equiv.normIn_memWrite hzm _ _

end Equiv.Sim
