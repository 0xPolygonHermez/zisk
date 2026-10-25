/-
  sail-riscv's narrow stores, in ZisK's profile: `sw`, `sh` and `sb` as
  Equiv/RiscvStore.lean has `sd` — rs2's low bytes at an aligned address in
  a writable region, through sail-riscv's whole store path.
-/
import Equiv.RiscvStore
import Equiv.RiscvLoadW

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 4000000 in
/-- sail-riscv's bits-to-bytes-and-back on a store: the identity. -/
theorem wt4 (v : BitVec 32) : vecbytes_to_bitvec (to_bytes_le (n := 4) v) = v := by
  simp [to_bytes_le, vecbytes_to_bitvec, Id.run, forIn, ForIn'.forIn', IntRange.forIn']
  repeat (rw [IntRange.forIn'.loop]; simp (config := { decide := true }) [Membership.mem])
  simp only [vectorUpdate, vectorInit, pure]
  change ((((0#32 ||| BitVec.setWidth 32 (Sail.BitVec.extractLsb v 7 0) <<< (0 * 8)) ||| BitVec.setWidth 32 (Sail.BitVec.extractLsb v 15 8) <<< (1 * 8)) ||| BitVec.setWidth 32 (Sail.BitVec.extractLsb v 23 16) <<< (2 * 8)) ||| BitVec.setWidth 32 (Sail.BitVec.extractLsb v 31 24) <<< (3 * 8)) = v
  simp only [Sail.BitVec.extractLsb]
  bv_decide

/-- A plain write to RAM: the bytes, little-endian. -/
theorem write_ram_run4 (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (v : BitVec 32) :
    (interpretSailEffects (write_ram .Write_plain acc (.Physaddr a) 4 v ())).run s
      = .ok true { s with mem := Equiv.memWrite (n := 4) s.mem a.toNat v } := by
  have hw := Equiv.riscv_writeBytes (n := 4) s a.toNat v
  simp [write_ram, LeanZiskRV.ArchSem.sail_mem_write, Sail.ArchSem.PreSail.sail_mem_write, interpretSailEffects,
    FreeM.liftM, interpretInstructionEffect, Mem_request.toArchSem, wt4]
  erw [hw]

theorem write_ram_ea_run4 (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) :
    (interpretSailEffects (write_ram_ea .Write_plain acc (.Physaddr a) 4)).run s = .ok () s := by
  simp [write_ram_ea, LeanZiskRV.ArchSem.sail_mem_address_announce, Sail.ArchSem.PreSail.sail_mem_address_announce,
    interpretSailEffects, FreeM.liftM, interpretInstructionEffect]

set_option maxHeartbeats 4000000 in
/-- The PMA check and announcement of a write, aligned or in-granule, inside a region that
    permits it: nothing changes. -/
theorem mem_write_ea_run4 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 4) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 4 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 4 _ hacc r hm (by simpa [permits] using hperm) hal
  have hea := write_ram_ea_run4 s (.Store .Data) a
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  erw [h0, hea]

set_option maxHeartbeats 4000000 in
/-- An aligned write inside a region that permits it: the bytes, little-endian. -/
theorem checked_mem_write_run4 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 4) (v : BitVec 32) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 4 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 4) s.mem a.toNat v } := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 4 _ hacc r hm (by simpa [permits] using hperm) hal
  have hw := write_ram_run4 s (.Store .Data) a v
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  have hw' := write_ram_run4 s (.Store .Data) a (Sail.BitVec.extractLsb v 31 0)
  erw [h0, hw']
  have hx : Sail.BitVec.extractLsb v 31 0 = v := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [Sail.BitVec.extractLsb]; omega
  simp only [hx]

theorem mem_write_value_run4 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 4) (v : BitVec 32) :
    (interpretSailEffects (mem_write_value (.Physaddr a) 4 v .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 4) s.mem a.toNat v } := by
  have h := checked_mem_write_run4 s rs hr hh a r hm hperm hal v
  simp [mem_write_value, mem_write_value_priv_meta, h]

theorem execute_store_eq4 (imm : BitVec 12) (a b : regidx) :
    execute (.STORE (imm, a, b, 4)) = execute_STORE imm a b 4 := by
  simp [execute]

set_option maxHeartbeats 4000000 in
/-- `sw rs2, imm(rs1)`, at a 4-aligned address in a writable region, in
    machine mode with no pointer masking: rs2's low 4 bytes go there. -/
theorem execute_sw (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 4 = some r)
    (hwr : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) (v1 + sign_extend (m := 64) imm) 4) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 4)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 4) t.mem (v1 + sign_extend (m := 64) imm).toNat (v2.setWidth 32) } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hea := mem_write_ea_run4 t rs hr hh _ r hm hwr hal
  simp only [execute_STORE, vmem_write, vmem_write_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_write_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (rw [r2]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_st, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 4 (by decide) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_run4 t rs hr hh _ r hm hwr hal]; try dsimp only))
  congr 3
  change BitVec.setWidth 32 (Sail.BitVec.extractLsb (BitVec.setWidth 32 (Sail.BitVec.extractLsb v2 31 0)) 31 0) = v2.setWidth 32
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]

set_option maxHeartbeats 4000000 in
/-- sail-riscv's bits-to-bytes-and-back on a store: the identity. -/
theorem wt2 (v : BitVec 16) : vecbytes_to_bitvec (to_bytes_le (n := 2) v) = v := by
  simp [to_bytes_le, vecbytes_to_bitvec, Id.run, forIn, ForIn'.forIn', IntRange.forIn']
  repeat (rw [IntRange.forIn'.loop]; simp (config := { decide := true }) [Membership.mem])
  simp only [vectorUpdate, vectorInit, pure]
  change ((0#16 ||| BitVec.setWidth 16 (Sail.BitVec.extractLsb v 7 0) <<< (0 * 8)) ||| BitVec.setWidth 16 (Sail.BitVec.extractLsb v 15 8) <<< (1 * 8)) = v
  simp only [Sail.BitVec.extractLsb]
  bv_decide

/-- A plain write to RAM: the bytes, little-endian. -/
theorem write_ram_run2 (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (v : BitVec 16) :
    (interpretSailEffects (write_ram .Write_plain acc (.Physaddr a) 2 v ())).run s
      = .ok true { s with mem := Equiv.memWrite (n := 2) s.mem a.toNat v } := by
  have hw := Equiv.riscv_writeBytes (n := 2) s a.toNat v
  simp [write_ram, LeanZiskRV.ArchSem.sail_mem_write, Sail.ArchSem.PreSail.sail_mem_write, interpretSailEffects,
    FreeM.liftM, interpretInstructionEffect, Mem_request.toArchSem, wt2]
  erw [hw]

theorem write_ram_ea_run2 (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) :
    (interpretSailEffects (write_ram_ea .Write_plain acc (.Physaddr a) 2)).run s = .ok () s := by
  simp [write_ram_ea, LeanZiskRV.ArchSem.sail_mem_address_announce, Sail.ArchSem.PreSail.sail_mem_address_announce,
    interpretSailEffects, FreeM.liftM, interpretInstructionEffect]

set_option maxHeartbeats 4000000 in
/-- The PMA check and announcement of a write, aligned or in-granule, inside a region that
    permits it: nothing changes. -/
theorem mem_write_ea_run2 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 2 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 2) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 2 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 2 _ hacc r hm (by simpa [permits] using hperm) hal
  have hea := write_ram_ea_run2 s (.Store .Data) a
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  erw [h0, hea]

set_option maxHeartbeats 4000000 in
/-- An aligned write inside a region that permits it: the bytes, little-endian. -/
theorem checked_mem_write_run2 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 2 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 2) (v : BitVec 16) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 2 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 2) s.mem a.toNat v } := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 2 _ hacc r hm (by simpa [permits] using hperm) hal
  have hw := write_ram_run2 s (.Store .Data) a v
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  have hw' := write_ram_run2 s (.Store .Data) a (Sail.BitVec.extractLsb v 15 0)
  erw [h0, hw']
  have hx : Sail.BitVec.extractLsb v 15 0 = v := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [Sail.BitVec.extractLsb]; omega
  simp only [hx]

theorem mem_write_value_run2 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 2 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 2) (v : BitVec 16) :
    (interpretSailEffects (mem_write_value (.Physaddr a) 2 v .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 2) s.mem a.toNat v } := by
  have h := checked_mem_write_run2 s rs hr hh a r hm hperm hal v
  simp [mem_write_value, mem_write_value_priv_meta, h]

theorem execute_store_eq2 (imm : BitVec 12) (a b : regidx) :
    execute (.STORE (imm, a, b, 2)) = execute_STORE imm a b 2 := by
  simp [execute]

set_option maxHeartbeats 4000000 in
/-- `sh rs2, imm(rs1)`, at a 2-aligned address in a writable region, in
    machine mode with no pointer masking: rs2's low 2 bytes go there. -/
theorem execute_sh (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 2 = some r)
    (hwr : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) (v1 + sign_extend (m := 64) imm) 2) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 2)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 2) t.mem (v1 + sign_extend (m := 64) imm).toNat (v2.setWidth 16) } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hea := mem_write_ea_run2 t rs hr hh _ r hm hwr hal
  simp only [execute_STORE, vmem_write, vmem_write_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_write_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (rw [r2]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_st, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 2 (by decide) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_run2 t rs hr hh _ r hm hwr hal]; try dsimp only))
  congr 3
  change BitVec.setWidth 16 (Sail.BitVec.extractLsb (BitVec.setWidth 16 (Sail.BitVec.extractLsb v2 15 0)) 15 0) = v2.setWidth 16
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]

set_option maxHeartbeats 4000000 in
/-- sail-riscv's bits-to-bytes-and-back on a store: the identity. -/
theorem wt1 (v : BitVec 8) : vecbytes_to_bitvec (to_bytes_le (n := 1) v) = v := by
  simp [to_bytes_le, vecbytes_to_bitvec, Id.run, forIn, ForIn'.forIn', IntRange.forIn']
  repeat (rw [IntRange.forIn'.loop]; simp (config := { decide := true }) [Membership.mem])
  simp only [vectorUpdate, vectorInit, pure]
  change (0#8 ||| BitVec.setWidth 8 (Sail.BitVec.extractLsb v 7 0) <<< (0 * 8)) = v
  simp only [Sail.BitVec.extractLsb]
  bv_decide

/-- A plain write to RAM: the bytes, little-endian. -/
theorem write_ram_run1 (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (v : BitVec 8) :
    (interpretSailEffects (write_ram .Write_plain acc (.Physaddr a) 1 v ())).run s
      = .ok true { s with mem := Equiv.memWrite (n := 1) s.mem a.toNat v } := by
  have hw := Equiv.riscv_writeBytes (n := 1) s a.toNat v
  simp [write_ram, LeanZiskRV.ArchSem.sail_mem_write, Sail.ArchSem.PreSail.sail_mem_write, interpretSailEffects,
    FreeM.liftM, interpretInstructionEffect, Mem_request.toArchSem, wt1]
  erw [hw]

theorem write_ram_ea_run1 (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) :
    (interpretSailEffects (write_ram_ea .Write_plain acc (.Physaddr a) 1)).run s = .ok () s := by
  simp [write_ram_ea, LeanZiskRV.ArchSem.sail_mem_address_announce, Sail.ArchSem.PreSail.sail_mem_address_announce,
    interpretSailEffects, FreeM.liftM, interpretInstructionEffect]

set_option maxHeartbeats 4000000 in
/-- The PMA check and announcement of a write, aligned or in-granule, inside a region that
    permits it: nothing changes. -/
theorem mem_write_ea_run1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 1 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 1) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 1 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 1 _ hacc r hm (by simpa [permits] using hperm) hal
  have hea := write_ram_ea_run1 s (.Store .Data) a
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  erw [h0, hea]

set_option maxHeartbeats 4000000 in
/-- An aligned write inside a region that permits it: the bytes, little-endian. -/
theorem checked_mem_write_run1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 1 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 1) (v : BitVec 8) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 1 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 1) s.mem a.toNat v } := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 1 _ hacc r hm (by simpa [permits] using hperm) hal
  have hw := write_ram_run1 s (.Store .Data) a v
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  have hw' := write_ram_run1 s (.Store .Data) a (Sail.BitVec.extractLsb v 7 0)
  erw [h0, hw']
  have hx : Sail.BitVec.extractLsb v 7 0 = v := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [Sail.BitVec.extractLsb]; omega
  simp only [hx]

theorem mem_write_value_run1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 1 = some r)
    (hperm : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) a 1) (v : BitVec 8) :
    (interpretSailEffects (mem_write_value (.Physaddr a) 1 v .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 1) s.mem a.toNat v } := by
  have h := checked_mem_write_run1 s rs hr hh a r hm hperm hal v
  simp [mem_write_value, mem_write_value_priv_meta, h]

theorem execute_store_eq1 (imm : BitVec 12) (a b : regidx) :
    execute (.STORE (imm, a, b, 1)) = execute_STORE imm a b 1 := by
  simp [execute]

set_option maxHeartbeats 4000000 in
/-- `sb rs2, imm(rs1)`, at a 1-aligned address in a writable region, in
    machine mode with no pointer masking: rs2's low 1 bytes go there. -/
theorem execute_sb (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 1 = some r)
    (hwr : r.attributes.writable = true) (hal : AlignOk r (.Store .Data) (v1 + sign_extend (m := 64) imm) 1) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 1)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 1) t.mem (v1 + sign_extend (m := 64) imm).toNat (v2.setWidth 8) } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hea := mem_write_ea_run1 t rs hr hh _ r hm hwr hal
  simp only [execute_STORE, vmem_write, vmem_write_explicit, default_access_info, get_pmlen,
    get_transformed_data_addr, ext_data_get_addr, transform_effective_address, vmem_write_addr,
    rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable, pm_transform_PA,
    Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode, currentlyEnabled,
    hartSupports, LeanZiskRV.not]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (rw [r2]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_st, bits_of_virtaddr, ite_self]; try dsimp only)
    | (rw [splitw_run _ _ _ _ 1 (by decide) hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_run1 t rs hr hh _ r hm hwr hal]; try dsimp only))
  congr 3
  change BitVec.setWidth 8 (Sail.BitVec.extractLsb (BitVec.setWidth 8 (Sail.BitVec.extractLsb v2 7 0)) 7 0) = v2.setWidth 8
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]

end Equiv.Riscv
