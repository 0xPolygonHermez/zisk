/-
  sail-riscv's stores, in ZisK's profile: `sd rs2, imm(rs1)` at an aligned
  address in a writable region, in machine mode with no pointer masking,
  writes rs2's 8 bytes there — through sail-riscv's whole store path.
-/
import Equiv.RiscvLoad
import Equiv.MemWrite

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

set_option maxHeartbeats 4000000 in
/-- sail-riscv's bits-to-bytes-and-back on a store: the identity. (Slow to
    check: about four minutes, unfolding eight vector updates.) -/
theorem wt8 (v : BitVec 64) : vecbytes_to_bitvec (to_bytes_le (n := 8) v) = v := by
  simp [to_bytes_le, vecbytes_to_bitvec, Id.run, forIn, ForIn'.forIn', IntRange.forIn']
  repeat (rw [IntRange.forIn'.loop]; simp (config := { decide := true }) [Membership.mem])
  simp only [vectorUpdate, vectorInit, pure]
  change ((((((((0#64 ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 7 0) <<< (0 * 8)) ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 15 8) <<< (1 * 8)) ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 23 16) <<< (2 * 8)) ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 31 24) <<< (3 * 8)) ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 39 32) <<< (4 * 8)) ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 47 40) <<< (5 * 8)) ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 55 48) <<< (6 * 8)) ||| BitVec.setWidth 64 (Sail.BitVec.extractLsb v 63 56) <<< (7 * 8)) = v
  simp only [Sail.BitVec.extractLsb]
  bv_decide

/-- A plain write to RAM: the bytes, little-endian. -/
theorem write_ram_run (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) (v : BitVec 64) :
    (interpretSailEffects (write_ram .Write_plain acc (.Physaddr a) 8 v ())).run s
      = .ok true { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hw := Equiv.riscv_writeBytes (n := 8) s a.toNat v
  simp [write_ram, LeanZiskRV.ArchSem.sail_mem_write, Sail.ArchSem.PreSail.sail_mem_write, interpretSailEffects,
    FreeM.liftM, interpretInstructionEffect, Mem_request.toArchSem, wt8]
  erw [hw]

theorem write_ram_ea_run (s : RVState) (acc : MemoryAccessType mem_payload) (a : BitVec 64) :
    (interpretSailEffects (write_ram_ea .Write_plain acc (.Physaddr a) 8)).run s = .ok () s := by
  simp [write_ram_ea, LeanZiskRV.ArchSem.sail_mem_address_announce, Sail.ArchSem.PreSail.sail_mem_address_announce,
    interpretSailEffects, FreeM.liftM, interpretInstructionEffect]

set_option maxHeartbeats 4000000 in
/-- The PMA check and announcement of an aligned write inside a region that
    permits it: nothing changes. -/
theorem mem_write_ea_run (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : r.attributes.writable = true) (hal : Int.tmod (a.toNat : Int) 8 = 0) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 8 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 8 _ hacc r hm (by simpa [permits] using hperm) hal
  have hea := write_ram_ea_run s (.Store .Data) a
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  erw [h0, hea]

set_option maxHeartbeats 4000000 in
/-- An aligned write inside a region that permits it: the bytes, little-endian. -/
theorem checked_mem_write_run (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : r.attributes.writable = true) (hal : Int.tmod (a.toNat : Int) 8 = 0) (v : BitVec 64) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 8 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have hp := pmaCheck_run s rs hr a 8 _ hacc r hm (by simpa [permits] using hperm) hal
  have hw := write_ram_run s (.Store .Data) a v
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have h0 : a + 0#64 = a := by simp
  have hw' := write_ram_run s (.Store .Data) a (Sail.BitVec.extractLsb v 63 0)
  erw [h0, hw']
  have hx : Sail.BitVec.extractLsb v 63 0 = v := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [Sail.BitVec.extractLsb]; omega
  simp only [hx]

theorem mem_write_value_run (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : r.attributes.writable = true) (hal : Int.tmod (a.toNat : Int) 8 = 0) (v : BitVec 64) :
    (interpretSailEffects (mem_write_value (.Physaddr a) 8 v .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have h := checked_mem_write_run s rs hr hh a r hm hperm hal v
  simp [mem_write_value, mem_write_value_priv_meta, h]

theorem execute_store_eq (imm : BitVec 12) (a b : regidx) :
    execute (.STORE (imm, a, b, 8)) = execute_STORE imm a b 8 := by
  simp [execute]

set_option maxHeartbeats 4000000 in
/-- `sd rs2, imm(rs1)`, at an 8-aligned address in a writable region, in
    machine mode with no pointer masking: rs2's 8 bytes go there. -/
theorem execute_sd (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 8 = some r)
    (hwr : r.attributes.writable = true) (hal : (v1 + sign_extend (m := 64) imm).toNat % 8 = 0) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 8)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 8) t.mem (v1 + sign_extend (m := 64) imm).toNat v2 } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have htm : Int.tmod ((v1 + sign_extend (m := 64) imm).toNat : Int) 8 = 0 := by
    rw [show (8 : Int) = ((8 : Nat) : Int) from rfl, tmod_nat]; omega
  have hea := mem_write_ea_run t rs hr hh _ r hm hwr htm
  have hal' : is_aligned_vaddr (.Virtaddr (v1 + sign_extend (m := 64) imm)) 8 = true := by
    simp only [is_aligned_vaddr, BitVec.toNatInt, Int.ofNat_eq_natCast, tmod_nat, hal]; rfl
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
    | (simp only [pmm_mode_backwards, pm_id, hal', bits_of_virtaddr]; try dsimp only)
    | (rw [split_aligned8_run _ _ hal]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_run t rs hr hh _ r hm hwr htm]; try dsimp only))
  congr 3
  change BitVec.setWidth 64 (Sail.BitVec.extractLsb (BitVec.setWidth 64 (Sail.BitVec.extractLsb v2 63 0)) 63 0) = v2
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]

end Equiv.Riscv
