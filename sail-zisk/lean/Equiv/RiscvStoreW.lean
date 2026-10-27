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

end Equiv.Riscv
