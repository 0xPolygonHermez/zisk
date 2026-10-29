/-
  The run's access conditions failing alike: a load or a store, any
  registers, at an address no PMA region permits (`AccFault`) and that
  ZisK's memory map refuses too. sail-riscv's step traps — its access
  fault, to the trap vector (Equiv/RiscvTrap.lean) — and sail-zisk's step
  on the spec's instruction fails, its `read_mem` or `write_mem` asserting
  (Equiv/ZiskFault.lean).
-/
import Equiv.X0Sim
import Equiv.RiscvTrap
import Equiv.ZiskFault

namespace Equiv.Sim

open Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Equiv.Zisk (regv)

/-- An access of w bytes at a that no PMA region permits. -/
def AccFault (rs : List PMA_Region) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (w : Nat) : Prop :=
  matching_pma_region rs (.Physaddr a) w = none ∨
    ∃ r, matching_pma_region rs (.Physaddr a) w = some r ∧ permits r.attributes acc = false

theorem LKind.w_cases (k : LKind) : k.w = 1 ∨ k.w = 2 ∨ k.w = 4 ∨ k.w = 8 := by
  cases k <;> simp [LKind.w]

theorem SKind.w_cases (k : SKind) : k.w = 1 ∨ k.w = 2 ∨ k.w = 4 ∨ k.w = 8 := by
  cases k <;> simp [SKind.w]

set_option maxHeartbeats 4000000 in
/-- `ld`, any registers, at an address neither side may read: sail-riscv's
    step traps, sail-zisk's fails. -/
theorem ldt_fault (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LdInstr) (next : Option (BitVec 32))
    (hi : LdtAt s rs pc zrom i next) (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hf : AccFault rs (.Load .Data) (v1 + i.imm.signExtend 64) 8)
    (hzr : _root_.Zisk.Functions.readable (v1 + i.imm.signExtend 64) 8 = false) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧ ∃ e z', (zstep zrom).run z = .error e z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, -, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  obtain ⟨h14, h13⟩ := ld_f3 _ hf3
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  refine ⟨try_step_trap s rs hr pc' hpc _ _ _ _ hw (.LOAD (i.imm, .Regidx i.rs1, .Regidx i.rd, false, 8)) ?_ ?_ n, ?_⟩
  · intro t htp ⟨w, hw'⟩ _ _
    have := decode_load0 t _ hop (by rw [h13, h14]; decide) htp w hw'
    rw [himm, hr1x, hrdx, h13, h14] at this; exact this
  · intro t he _ hmr hsec hwf _
    rw [execute_load_eq]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    obtain ⟨c, p, h⟩ := execute_load_fault t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rd i.imm v1
      (getX_of_agree he l1 hv1) 8 (by decide) false hf
    exact ⟨(c, p), h⟩
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]
    have ha := Equiv.Zisk.srcA_ASrc xs _ l1
    rw [hr1] at ha
    exact Equiv.Zisk.execute_load_fault_z z _ (.ZOP_COPYB ()) i.rd.toNat 4 _ 8 xs hx v1 ha hzr

set_option maxHeartbeats 4000000 in
/-- A narrow load, any registers, at an address neither side may read. -/
theorem lwt_fault (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : LwInstr) (next : Option (BitVec 32))
    (hi : LwtAt s rs pc zrom i next) (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hf : AccFault rs (.Load .Data) (v1 + i.imm.signExtend 64) i.k.w)
    (hzr : _root_.Zisk.Functions.readable (v1 + i.imm.signExtend 64) i.k.w = false) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧ ∃ e z', (zstep zrom).run z = .error e z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, -, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, himm, hrdx, hr1x, hc⟩ := i.fields
  obtain ⟨hu, hwd⟩ := i.k.dec _ hf3
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hvalid := i.k.valid
  rw [← hu, ← hwd] at hvalid
  refine ⟨try_step_trap s rs hr pc' hpc _ _ _ _ hw
    (.LOAD (i.imm, .Regidx i.rs1, .Regidx i.rd, i.k.u, (i.k.w : Int))) ?_ ?_ n, ?_⟩
  · intro t htp ⟨w, hw'⟩ _ _
    have := decode_load0 t _ hop hvalid htp w hw'
    rw [himm, hr1x, hrdx, hu, hwd] at this; exact this
  · intro t he _ hmr hsec hwf _
    rw [execute_load_w, Int.toNat_natCast]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    obtain ⟨c, p, h⟩ := execute_load_fault t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rd i.imm v1
      (getX_of_agree he l1 hv1) i.k.w i.k.w_cases i.k.u hf
    exact ⟨(c, p), h⟩
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]
    have ha := Equiv.Zisk.srcA_ASrc xs _ l1
    rw [hr1] at ha
    exact Equiv.Zisk.execute_load_fault_z z _ i.k.op i.rd.toNat 4 _ i.k.w xs hx v1 ha hzr

set_option maxHeartbeats 4000000 in
/-- `sd`, any registers, at an address neither side may write. -/
theorem sdt_fault (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SdInstr) (next : Option (BitVec 32))
    (hi : SdtAt s rs pc zrom i next) (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hf : AccFault rs (.Store .Data) (v1 + (i.immHi ++ i.immLo).signExtend 64) 8)
    (hzw : _root_.Zisk.Functions.writable (v1 + (i.immHi ++ i.immLo).signExtend 64) 8 = false) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧ ∃ e z', (zstep zrom).run z = .error e z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, -, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hhi, hlo, hr2x, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hv2 := getX_regv agree l2
  refine ⟨try_step_trap s rs hr pc' hpc _ _ _ _ hw (.STORE (i.immHi ++ i.immLo, .Regidx i.rs2, .Regidx i.rs1, 8))
    ?_ ?_ n, ?_⟩
  · intro t htp ⟨w, hw'⟩ _ _
    have := decode_sd t _ hop hf3 htp w hw'
    rw [hhi, hlo, hr2x, hr1x] at this; exact this
  · intro t he _ hmr hsec hwf _
    rw [execute_store_eq]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    obtain ⟨c, p, h⟩ := execute_store_fault t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rs2 (i.immHi ++ i.immLo)
      v1 _ (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) 8 (by decide) hf
    exact ⟨(c, p), h⟩
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]
    have ha := Equiv.Zisk.srcA_ASrc xs _ l1
    have hb := Equiv.Zisk.srcB_BSrc xs _ l2
    rw [hr1] at ha
    exact Equiv.Zisk.execute_store_fault_z z _ _ _ 8 4 xs hx v1 _ ha hb hzw

set_option maxHeartbeats 4000000 in
/-- A narrow store, any registers, at an address neither side may write. -/
theorem swt_fault (s : RVState) (z : Equiv.Zisk.ZState) (rs : List PMA_Region) (hrel : Rel s z rs)
    (pc : BitVec 64) (hpc : s.regs.get? Register.PC = some pc)
    (zrom : BitVec 64 → _root_.Zisk.zisk_inst) (i : SwInstr) (next : Option (BitVec 32))
    (hi : SwtAt s rs pc zrom i next) (v1 : BitVec 64) (hv1 : getX s i.rs1.toNat = some v1)
    (hf : AccFault rs (.Store .Data) (v1 + (i.immHi ++ i.immLo).signExtend 64) i.k.w)
    (hzw : _root_.Zisk.Functions.writable (v1 + (i.immHi ++ i.immLo).signExtend 64) i.k.w = false) (n : Nat) :
    (∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧
      s'.regs.get? Register.PC = some 0x4#64) ∧ ∃ e z', (zstep zrom).run z = .error e z' := by
  obtain ⟨hr, ⟨xs, hx, agree⟩, ⟨pc', hpc', hzpc⟩, -, -⟩ := hrel
  have : pc' = pc := by rw [hpc] at hpc'; exact (Option.some.inj hpc').symm
  subst this
  obtain ⟨hop, hf3, hhi, hlo, hr2x, hr1x, hc⟩ := i.fields
  have hw : WordAt s rs pc' (byte i.encode 0) (byte i.encode 1) (byte i.encode 2) (byte i.encode 3) :=
    ⟨hi.al, hi.x1, hi.x2, hi.h0, hi.h1, hi.h2, hi.h3, hc⟩
  have l1 : i.rs1.toNat < 32 := i.rs1.isLt
  have l2 : i.rs2.toNat < 32 := i.rs2.isLt
  have hr1 : regv xs i.rs1.toNat = v1 := by
    have := getX_regv agree l1; rw [hv1] at this; exact (Option.some.inj this).symm
  have hv2 := getX_regv agree l2
  refine ⟨try_step_trap s rs hr pc' hpc _ _ _ _ hw
    (.STORE (i.immHi ++ i.immLo, .Regidx i.rs2, .Regidx i.rs1, (i.k.w : Int))) ?_ ?_ n, ?_⟩
  · intro t htp ⟨w, hw'⟩ _ _
    have := i.k.decode t _ hop hf3 htp w hw'
    rw [hhi, hlo, hr2x, hr1x] at this; exact this
  · intro t he _ hmr hsec hwf _
    rw [execute_store_w, Int.toNat_natCast]
    obtain ⟨sv, hsv, hpm⟩ := hr.pmm
    obtain ⟨c, p, h⟩ := execute_store_fault t hwf rs hmr sv (hsec.trans hsv) hpm i.rs1 i.rs2 (i.immHi ++ i.immLo)
      v1 _ (getX_of_agree he l1 hv1) (getX_of_agree he l2 hv2) i.k.w i.k.w_cases hf
    exact ⟨(c, p), h⟩
  · rw [zstep_run zrom z pc' hzpc, i.zisk_of_spec' hi.tr]
    have ha := Equiv.Zisk.srcA_ASrc xs _ l1
    have hb := Equiv.Zisk.srcB_BSrc xs _ l2
    rw [hr1] at ha
    exact Equiv.Zisk.execute_store_fault_z z _ _ _ i.k.w 4 xs hx v1 _ ha hb hzw

end Equiv.Sim
