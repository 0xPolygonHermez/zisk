/-
  sail-riscv's loads, in ZisK's profile: `ld rd, imm(rs1)` at an aligned
  address in a readable region, in machine mode with no pointer masking,
  reads the 8 bytes there into rd — through sail-riscv's whole load path
  (pointer masking, translation, the PMA check, the page split, the read).
-/
import Equiv.RiscvStep

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- With no pointer masking (pmlen 0), the effective address is the address. -/
theorem pm_id (a : BitVec 64) :
    zero_extend (m := 64) (Sail.BitVec.extractLsb a ((Functions.xlen : Int) - ((Int.toNat 0 : Nat) : Int) - 1).toNat 0)
      = a := by
  simp only [zero_extend, Sail.BitVec.zeroExtend, Sail.BitVec.extractLsb, Functions.xlen]
  apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [hi]

/-- An access of 8 bytes at an 8-aligned address does not cross a page. -/
theorem split_aligned8_run (s : RVState) (a : BitVec 64) (hal : a.toNat % 8 = 0) :
    (interpretSailEffects (split_on_page_boundary a 8)).run s = .ok (8, 0) s := by
  have h3 : a.extractLsb 2 0 = 0#3 := by
    apply BitVec.eq_of_toNat_eq; simp [BitVec.extractLsb, BitVec.extractLsb']; omega
  simp [split_on_page_boundary, Sail.BitVec.length]
  rw [if_pos]
  · rfl
  · simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', ones, zeros, BitVec.subInt, BitVec.addInt,
      Functions.pagesize_bits, sail_ones, BitVec.zero_eq]
    bv_decide

/-- vmem_read_addr's access width when it does not split: the width. -/
theorem width8 : (if (SATPMode.Bare != SATPMode.Bare && ((0 >b 0) && !false)) = true then (8 : Int)
    else ((8 : Nat) : Int)).toNat = 8 := rfl

set_option maxHeartbeats 4000000 in
/-- `ld rd, imm(rs1)`, at an 8-aligned address in a readable region, in
    machine mode with no pointer masking: rd gets the 8 bytes there. -/
theorem execute_ld (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 8 = some r)
    (hrd : r.attributes.readable = true) (hal : (v1 + sign_extend (m := 64) imm).toNat % 8 = 0)
    (v : BitVec 64)
    (hv : (Sail.ArchSem.readBytes 8 (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) false 8)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat v) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hrd' := mem_read_run t rs hr hh _ 8 (by omega) _ hacc r hm (by simpa [permits] using hrd)
    (by rw [tmod_nat]; omega) v hv
  have hal' : is_aligned_vaddr (.Virtaddr (v1 + sign_extend (m := 64) imm)) 8 = true := by
    simp only [is_aligned_vaddr, BitVec.toNatInt, Int.ofNat_eq_natCast, tmod_nat, hal]; rfl
  have hw := wX_run t rd.toNat rd.lt v
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, hal', bits_of_virtaddr]; try dsimp only)
    | (rw [split_aligned8_run _ _ hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  erw [wX_run t rd.toNat rd.lt]
  simp only [interp_pure, run_pure', EStateM.Result.ok.injEq, true_and]
  congr 1
  change sign_extend (m := 64) (Sail.BitVec.updateSubrange (0#64) 63 0
    (BitVec.setWidth (63 - 0 + 1) (from_bytes_le (n := 8) (bitvec_to_vecbytes v)))) = v
  rw [rt8]
  simp only [sign_extend, Sail.BitVec.signExtend, Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange']
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [hi]

end Equiv.Riscv
