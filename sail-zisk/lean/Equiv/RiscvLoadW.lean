/-
  sail-riscv's narrower loads, in ZisK's profile: lb, lbu, lh, lhu, lw and
  lwu at an aligned address in a readable region read the 1, 2 or 4 bytes
  there into rd, sign- or zero-extended — as Equiv/RiscvLoad.lean's `ld`.
-/
import Equiv.RiscvLoad

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- An access of w ∈ {1, 2, 4} bytes at a w-aligned address does not cross
    a page. -/
theorem split_aligned_run (s : RVState) (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4)
    (hal : a.toNat % w = 0) :
    (interpretSailEffects (split_on_page_boundary a w)).run s = .ok ((w : Int), 0) s := by
  rcases hw with rfl | rfl | rfl
  · simp [split_on_page_boundary, Sail.BitVec.length]
    rw [if_pos]
    · rfl
    · simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', ones, zeros, BitVec.subInt,
        BitVec.addInt, Functions.pagesize_bits, sail_ones, BitVec.zero_eq]
      bv_decide
  · have h1 : a.extractLsb 0 0 = 0#1 := by
      apply BitVec.eq_of_toNat_eq; simp [BitVec.extractLsb, BitVec.extractLsb']; omega
    simp [split_on_page_boundary, Sail.BitVec.length]
    rw [if_pos]
    · rfl
    · simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', ones, zeros, BitVec.subInt,
        BitVec.addInt, Functions.pagesize_bits, sail_ones, BitVec.zero_eq]
      bv_decide
  · have h1 : a.extractLsb 1 0 = 0#2 := by
      apply BitVec.eq_of_toNat_eq; simp [BitVec.extractLsb, BitVec.extractLsb']; omega
    simp [split_on_page_boundary, Sail.BitVec.length]
    rw [if_pos]
    · rfl
    · simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', ones, zeros, BitVec.subInt,
        BitVec.addInt, Functions.pagesize_bits, sail_ones, BitVec.zero_eq]
      bv_decide

/-- The page split of a 1-, 2- or 4-byte access the PMA check passes unsplit. -/
theorem splitw_run (s : RVState) (r : PMA_Region) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (w : Nat)
    (hw : w = 1 ∨ w = 2 ∨ w = 4) (hal : AlignOk r acc a w) :
    (interpretSailEffects (split_on_page_boundary a w)).run s = .ok ((w : Int), 0) s := by
  rcases hal with h | ⟨-, -, -, h⟩
  · exact split_aligned_run s a w hw (by rw [tmod_nat] at h; omega)
  · exact split_gran_run s a w (by omega) h

set_option maxHeartbeats 4000000 in
/-- `lb rd, imm(rs1)`: rd gets the 1 bytes there, sign-extended. -/
theorem execute_lb (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 1 = some r)
    (hrd : r.attributes.readable = true) (hal : AlignOk r (.Load .Data) (v1 + sign_extend (m := 64) imm) 1)
    (v : BitVec (8 * 1))
    (hv : (Sail.ArchSem.readBytes 1 (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) false 1)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (extend_value false v)) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hrd' := mem_read_run t rs hr hh _ 1 (by omega) _ hacc r hm (by simpa [permits] using hrd) hal v hv
  have hw := wX_run t rd.toNat rd.lt (extend_value false v)
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_ld, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 1 (by omega) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  erw [wX_run t rd.toNat rd.lt]
  simp only [interp_pure, run_pure', EStateM.Result.ok.injEq, true_and]
  congr 1
  change sign_extend (m := 64) (Sail.BitVec.updateSubrange (0#(8 * 1)) (8 * 1 - 1) 0
    (BitVec.setWidth (8 * 1 - 1 - 0 + 1) (from_bytes_le (n := 1) (bitvec_to_vecbytes v)))) = extend_value false v
  rw [rt1]
  simp only [extend_value, Bool.false_eq_true, ↓reduceIte]
  congr 1
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', hi]
set_option maxHeartbeats 4000000 in
/-- `lbu rd, imm(rs1)`: rd gets the 1 bytes there, zero-extended. -/
theorem execute_lbu (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 1 = some r)
    (hrd : r.attributes.readable = true) (hal : AlignOk r (.Load .Data) (v1 + sign_extend (m := 64) imm) 1)
    (v : BitVec (8 * 1))
    (hv : (Sail.ArchSem.readBytes 1 (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) true 1)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (extend_value true v)) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hrd' := mem_read_run t rs hr hh _ 1 (by omega) _ hacc r hm (by simpa [permits] using hrd) hal v hv
  have hw := wX_run t rd.toNat rd.lt (extend_value true v)
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_ld, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 1 (by omega) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  erw [wX_run t rd.toNat rd.lt]
  simp only [interp_pure, run_pure', EStateM.Result.ok.injEq, true_and]
  congr 1
  change zero_extend (m := 64) (Sail.BitVec.updateSubrange (0#(8 * 1)) (8 * 1 - 1) 0
    (BitVec.setWidth (8 * 1 - 1 - 0 + 1) (from_bytes_le (n := 1) (bitvec_to_vecbytes v)))) = extend_value true v
  rw [rt1]
  simp only [extend_value, ↓reduceIte]
  congr 1
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', hi]
set_option maxHeartbeats 4000000 in
/-- `lh rd, imm(rs1)`: rd gets the 2 bytes there, sign-extended. -/
theorem execute_lh (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 2 = some r)
    (hrd : r.attributes.readable = true) (hal : AlignOk r (.Load .Data) (v1 + sign_extend (m := 64) imm) 2)
    (v : BitVec (8 * 2))
    (hv : (Sail.ArchSem.readBytes 2 (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) false 2)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (extend_value false v)) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hrd' := mem_read_run t rs hr hh _ 2 (by omega) _ hacc r hm (by simpa [permits] using hrd) hal v hv
  have hw := wX_run t rd.toNat rd.lt (extend_value false v)
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_ld, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 2 (by omega) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  erw [wX_run t rd.toNat rd.lt]
  simp only [interp_pure, run_pure', EStateM.Result.ok.injEq, true_and]
  congr 1
  change sign_extend (m := 64) (Sail.BitVec.updateSubrange (0#(8 * 2)) (8 * 2 - 1) 0
    (BitVec.setWidth (8 * 2 - 1 - 0 + 1) (from_bytes_le (n := 2) (bitvec_to_vecbytes v)))) = extend_value false v
  rw [rt2]
  simp only [extend_value, Bool.false_eq_true, ↓reduceIte]
  congr 1
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', hi]
set_option maxHeartbeats 4000000 in
/-- `lhu rd, imm(rs1)`: rd gets the 2 bytes there, zero-extended. -/
theorem execute_lhu (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 2 = some r)
    (hrd : r.attributes.readable = true) (hal : AlignOk r (.Load .Data) (v1 + sign_extend (m := 64) imm) 2)
    (v : BitVec (8 * 2))
    (hv : (Sail.ArchSem.readBytes 2 (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) true 2)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (extend_value true v)) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hrd' := mem_read_run t rs hr hh _ 2 (by omega) _ hacc r hm (by simpa [permits] using hrd) hal v hv
  have hw := wX_run t rd.toNat rd.lt (extend_value true v)
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_ld, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 2 (by omega) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  erw [wX_run t rd.toNat rd.lt]
  simp only [interp_pure, run_pure', EStateM.Result.ok.injEq, true_and]
  congr 1
  change zero_extend (m := 64) (Sail.BitVec.updateSubrange (0#(8 * 2)) (8 * 2 - 1) 0
    (BitVec.setWidth (8 * 2 - 1 - 0 + 1) (from_bytes_le (n := 2) (bitvec_to_vecbytes v)))) = extend_value true v
  rw [rt2]
  simp only [extend_value, ↓reduceIte]
  congr 1
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', hi]
set_option maxHeartbeats 4000000 in
/-- `lw rd, imm(rs1)`: rd gets the 4 bytes there, sign-extended. -/
theorem execute_lw (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 4 = some r)
    (hrd : r.attributes.readable = true) (hal : AlignOk r (.Load .Data) (v1 + sign_extend (m := 64) imm) 4)
    (v : BitVec (8 * 4))
    (hv : (Sail.ArchSem.readBytes 4 (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) false 4)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (extend_value false v)) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hrd' := mem_read_run t rs hr hh _ 4 (by omega) _ hacc r hm (by simpa [permits] using hrd) hal v hv
  have hw := wX_run t rd.toNat rd.lt (extend_value false v)
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_ld, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 4 (by omega) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  erw [wX_run t rd.toNat rd.lt]
  simp only [interp_pure, run_pure', EStateM.Result.ok.injEq, true_and]
  congr 1
  change sign_extend (m := 64) (Sail.BitVec.updateSubrange (0#(8 * 4)) (8 * 4 - 1) 0
    (BitVec.setWidth (8 * 4 - 1 - 0 + 1) (from_bytes_le (n := 4) (bitvec_to_vecbytes v)))) = extend_value false v
  rw [rt4]
  simp only [extend_value, Bool.false_eq_true, ↓reduceIte]
  congr 1
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', hi]
set_option maxHeartbeats 4000000 in
/-- `lwu rd, imm(rs1)`: rd gets the 4 bytes there, zero-extended. -/
theorem execute_lwu (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rd : RegBits) (imm : BitVec 12) (v1 : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 4 = some r)
    (hrd : r.attributes.readable = true) (hal : AlignOk r (.Load .Data) (v1 + sign_extend (m := 64) imm) 4)
    (v : BitVec (8 * 4))
    (hv : (Sail.ArchSem.readBytes 4 (v1 + sign_extend (m := 64) imm).toNat :
      Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_LOAD imm (.Regidx rs1) (.Regidx rd) true 4)).run t
      = .ok RETIRE_SUCCESS (setX t rd.toNat (extend_value true v)) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have hacc : PlainAccess (.Load .Data) := Or.inr (Or.inl rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hrd' := mem_read_run t rs hr hh _ 4 (by omega) _ hacc r hm (by simpa [permits] using hrd) hal v hv
  have hw := wX_run t rd.toNat rd.lt (extend_value true v)
  simp only [execute_LOAD, vmem_read, vmem_read_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_read_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not, translate_and_read_value, wX_bits, extend_value]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_ld, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 4 (by omega) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (rw [hrd']; try dsimp only)
    | (erw [hrd']; try dsimp only))
  erw [wX_run t rd.toNat rd.lt]
  simp only [interp_pure, run_pure', EStateM.Result.ok.injEq, true_and]
  congr 1
  change zero_extend (m := 64) (Sail.BitVec.updateSubrange (0#(8 * 4)) (8 * 4 - 1) 0
    (BitVec.setWidth (8 * 4 - 1 - 0 + 1) (from_bytes_le (n := 4) (bitvec_to_vecbytes v)))) = extend_value true v
  rw [rt4]
  simp only [extend_value, ↓reduceIte]
  congr 1
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', hi]
end Equiv.Riscv
