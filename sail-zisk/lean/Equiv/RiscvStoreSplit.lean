/-
  sail-riscv's stores across a 16-byte granule, in ZisK's profile: as with
  loads (Equiv/RiscvSplit.lean), the PMA check passes them as splittable,
  and `checked_mem_write` writes them in pieces of 2^min(ctz addr, ctz width)
  bytes — which make the same bytes as the whole write (`cmw_split_*`). So
  `sd`, `sw`, `sh` and `sb` inside a writable region write their bytes at
  any alignment.
-/
import Equiv.RiscvStoreW
import Equiv.RiscvSplit

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- One piece of a split write: K bytes at x. -/
theorem write_piece (s : RVState) (x : BitVec 64) (K : Nat) (hK : K = 1 ∨ K = 2 ∨ K = 4) (val : BitVec (8 * K)) :
    (interpretSailEffects (write_ram .Write_plain (.Store .Data) (.Physaddr x) K val ())).run s
      = .ok true { s with mem := Equiv.memWrite (n := K) s.mem x.toNat val } := by
  rcases hK with rfl | rfl | rfl
  · exact write_ram_run1 s _ x val
  · exact write_ram_run2 s _ x val
  · exact write_ram_run4 s _ x val

/-- Two writes side by side, the second's address given as any equal one. -/
theorem memWrite_append' {p q : Nat} (m : Equiv.Mem) (a b : Nat) (hb : b = a + p) (lo : BitVec (8 * p))
    (hi : BitVec (8 * q)) :
    Equiv.memWrite (Equiv.memWrite m a lo) b hi =
      Equiv.memWrite (n := p + q) m a ((show 8 * q + 8 * p = 8 * (p + q) by omega) ▸ (hi ++ lo)) := by
  subst hb; exact Equiv.memWrite_append m a lo hi

/-- The announcement of one piece of a split write: nothing changes. -/
theorem write_ea_piece (s : RVState) (p : physaddr) (K : Nat) :
    (interpretSailEffects (write_ram_ea .Write_plain (.Store .Data) p K)).run s = .ok () s := by
  simp [write_ram_ea, LeanZiskRV.ArchSem.sail_mem_address_announce, Sail.ArchSem.PreSail.sail_mem_address_announce,
    interpretSailEffects, FreeM.liftM, interpretInstructionEffect]

set_option maxHeartbeats 0 in
/-- A store of 2 bytes across its granule, its address 1 mod 2 (pieces of 1). -/
theorem cmw_split_2_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 2 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 2 ≠ 0) (hnw : allowed_misaligned a 2 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 2 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) (v : BitVec 16) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 2 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 2) s.mem a.toNat v } := by
  have hp := pmaCheck_split s rs hr a 2 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 2) = 1 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (1 : Int) = 1 := by decide
  have htd : Int.tdiv 2 1 = 2 := by decide
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have ea0 : (a + 0#64).toNat = a.toNat := by simp
  erw [ea0]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  congr 1
  simp only [Equiv.rec_cast, Sail.BitVec.extractLsb]
  bv_decide

set_option maxHeartbeats 0 in
/-- The PMA check and announcement of a store of 2 bytes across its granule (pieces of 1). -/
theorem mwe_split_2_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 2 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 2 ≠ 0) (hnw : allowed_misaligned a 2 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 2 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 2 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hp := pmaCheck_split s rs hr a 2 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 2) = 1 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (1 : Int) = 1 := by decide
  have htd : Int.tdiv 2 1 = 2 := by decide
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  repeat (first
    | (erw [readReg_run _ _ _ hh]; try dsimp only)
    | (erw [write_ea_piece]; try dsimp only)
    | simp (config := { decide := true }) [interp_bind, run_bind', monadLift_self, liftM,
        interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
        within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, LeanZiskRV.not])

set_option maxHeartbeats 0 in
/-- A store of 4 bytes across its granule, its address 1 mod 2 (pieces of 1). -/
theorem cmw_split_4_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 4 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 4 ≠ 0) (hnw : allowed_misaligned a 4 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) (v : BitVec 32) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 4 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 4) s.mem a.toNat v } := by
  have hp := pmaCheck_split s rs hr a 4 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 4) = 2 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (2 : Int) = 1 := by decide
  have htd : Int.tdiv 4 1 = 4 := by decide
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have ea0 : (a + 0#64).toNat = a.toNat := by simp
  erw [ea0]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  congr 1
  simp only [Equiv.rec_cast, Sail.BitVec.extractLsb]
  bv_decide

set_option maxHeartbeats 0 in
/-- The PMA check and announcement of a store of 4 bytes across its granule (pieces of 1). -/
theorem mwe_split_4_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 4 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 4 ≠ 0) (hnw : allowed_misaligned a 4 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 4 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hp := pmaCheck_split s rs hr a 4 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 4) = 2 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (2 : Int) = 1 := by decide
  have htd : Int.tdiv 4 1 = 4 := by decide
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  repeat (first
    | (erw [readReg_run _ _ _ hh]; try dsimp only)
    | (erw [write_ea_piece]; try dsimp only)
    | simp (config := { decide := true }) [interp_bind, run_bind', monadLift_self, liftM,
        interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
        within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, LeanZiskRV.not])

set_option maxHeartbeats 0 in
/-- A store of 4 bytes across its granule, its address 2 mod 4 (pieces of 2). -/
theorem cmw_split_4_1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 4 = 2) (hwr : a.toNat + 4 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 4 ≠ 0) (hnw : allowed_misaligned a 4 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) (v : BitVec 32) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 4 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 4) s.mem a.toNat v } := by
  have hp := pmaCheck_split s rs hr a 4 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 1 := by
    have hb : (a &&& 3#64).toNat = a.toNat % 4 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 2
    have h' : a &&& 3#64 = 2#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 1#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 4) = 2 := by decide
  have hpw : (2 : Int) ^ min (1 : Int) (2 : Int) = 2 := by decide
  have htd : Int.tdiv 4 2 = 2 := by decide
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [write_piece _ _ 2 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 2 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have ea0 : (a + 0#64).toNat = a.toNat := by simp
  erw [ea0]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  congr 1
  simp only [Equiv.rec_cast, Sail.BitVec.extractLsb]
  bv_decide

set_option maxHeartbeats 0 in
/-- The PMA check and announcement of a store of 4 bytes across its granule (pieces of 2). -/
theorem mwe_split_4_1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 4 = 2) (hwr : a.toNat + 4 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 4 ≠ 0) (hnw : allowed_misaligned a 4 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 4 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hp := pmaCheck_split s rs hr a 4 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 1 := by
    have hb : (a &&& 3#64).toNat = a.toNat % 4 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 2
    have h' : a &&& 3#64 = 2#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 1#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 4) = 2 := by decide
  have hpw : (2 : Int) ^ min (1 : Int) (2 : Int) = 2 := by decide
  have htd : Int.tdiv 4 2 = 2 := by decide
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  repeat (first
    | (erw [readReg_run _ _ _ hh]; try dsimp only)
    | (erw [write_ea_piece]; try dsimp only)
    | simp (config := { decide := true }) [interp_bind, run_bind', monadLift_self, liftM,
        interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
        within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, LeanZiskRV.not])

set_option maxHeartbeats 0 in
/-- A store of 8 bytes across its granule, its address 1 mod 2 (pieces of 1). -/
theorem cmw_split_8_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) (v : BitVec 64) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 8 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (3 : Int) = 1 := by decide
  have htd : Int.tdiv 8 1 = 8 := by decide
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 1 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have ea0 : (a + 0#64).toNat = a.toNat := by simp
  erw [ea0]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  congr 1
  simp only [Equiv.rec_cast, Sail.BitVec.extractLsb]
  bv_decide

set_option maxHeartbeats 0 in
/-- The PMA check and announcement of a store of 8 bytes across its granule (pieces of 1). -/
theorem mwe_split_8_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 8 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (3 : Int) = 1 := by decide
  have htd : Int.tdiv 8 1 = 8 := by decide
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  repeat (first
    | (erw [readReg_run _ _ _ hh]; try dsimp only)
    | (erw [write_ea_piece]; try dsimp only)
    | simp (config := { decide := true }) [interp_bind, run_bind', monadLift_self, liftM,
        interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
        within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, LeanZiskRV.not])

set_option maxHeartbeats 0 in
/-- A store of 8 bytes across its granule, its address 2 mod 4 (pieces of 2). -/
theorem cmw_split_8_1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 4 = 2) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) (v : BitVec 64) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 8 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 1 := by
    have hb : (a &&& 3#64).toNat = a.toNat % 4 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 2
    have h' : a &&& 3#64 = 2#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 1#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (1 : Int) (3 : Int) = 2 := by decide
  have htd : Int.tdiv 8 2 = 4 := by decide
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [write_piece _ _ 2 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 2 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 2 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 2 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have ea0 : (a + 0#64).toNat = a.toNat := by simp
  erw [ea0]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  congr 1
  simp only [Equiv.rec_cast, Sail.BitVec.extractLsb]
  bv_decide

set_option maxHeartbeats 0 in
/-- The PMA check and announcement of a store of 8 bytes across its granule (pieces of 2). -/
theorem mwe_split_8_1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 4 = 2) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 8 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 1 := by
    have hb : (a &&& 3#64).toNat = a.toNat % 4 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 2
    have h' : a &&& 3#64 = 2#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 1#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (1 : Int) (3 : Int) = 2 := by decide
  have htd : Int.tdiv 8 2 = 4 := by decide
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  repeat (first
    | (erw [readReg_run _ _ _ hh]; try dsimp only)
    | (erw [write_ea_piece]; try dsimp only)
    | simp (config := { decide := true }) [interp_bind, run_bind', monadLift_self, liftM,
        interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
        within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, LeanZiskRV.not])

set_option maxHeartbeats 0 in
/-- A store of 8 bytes across its granule, its address 4 mod 8 (pieces of 4). -/
theorem cmw_split_8_2 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 8 = 4) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) (v : BitVec 64) :
    (interpretSailEffects (checked_mem_write (.Physaddr a) 8 v (.Store .Data) .PBMT_PMA .Machine () false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 2 := by
    have hb : (a &&& 7#64).toNat = a.toNat % 8 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 3
    have h' : a &&& 7#64 = 4#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 2#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (2 : Int) (3 : Int) = 4 := by decide
  have htd : Int.tdiv 8 4 = 2 := by decide
  simp (config := { decide := true }) [checked_mem_write, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [write_piece _ _ 4 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  rw [write_piece _ _ 4 (by decide)]; dsimp only
  try (rw [readReg_match]; dsimp only; simp only [hh])
  simp (config := { decide := true }) [readReg_match, hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
    within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
  have ea0 : (a + 0#64).toNat = a.toNat := by simp
  erw [ea0]
  rw [memWrite_append' _ _ _ (by first | (erw [BitVec.toNat_add]; simp; omega) | (erw [BitVec.toNat_add, BitVec.toNat_add]; simp; omega))]
  congr 1
  simp only [Equiv.rec_cast, Sail.BitVec.extractLsb]
  bv_decide

set_option maxHeartbeats 0 in
/-- The PMA check and announcement of a store of 8 bytes across its granule (pieces of 4). -/
theorem mwe_split_8_2 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 8 = 4) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Store .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) 8 .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inr rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 2 := by
    have hb : (a &&& 7#64).toNat = a.toNat % 8 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 3
    have h' : a &&& 7#64 = 4#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 2#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (2 : Int) (3 : Int) = 4 := by decide
  have htd : Int.tdiv 8 4 = 2 := by decide
  simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
    readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  repeat (first
    | (erw [readReg_run _ _ _ hh]; try dsimp only)
    | (erw [write_ea_piece]; try dsimp only)
    | simp (config := { decide := true }) [interp_bind, run_bind', monadLift_self, liftM,
        interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_writable,
        within_clint, within_sig, within_htif_writable, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, LeanZiskRV.not])


set_option maxHeartbeats 4000000 in
/-- The PMA check and announcement of a store inside a writable region, of
    1, 2, 4 or 8 bytes, at any alignment: nothing changes. -/
theorem mem_write_ea_any (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8) (hwr : a.toNat + w < 2 ^ 64)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : r.attributes.writable = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) w .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok ()) s := by
  have hp : permits r.attributes (.Store .Data) = true := by simpa [permits] using hperm
  by_cases hal : AlignOk r (.Store .Data) a w
  · rcases hw with rfl | rfl | rfl | rfl
    · exact mem_write_ea_run1 s rs hr hh a r hm hperm hal
    · exact mem_write_ea_run2 s rs hr hh a r hm hperm hal
    · exact mem_write_ea_run4 s rs hr hh a r hm hperm hal
    · exact mem_write_ea_run s rs hr hh a r hm hperm hal
  have hna : Int.tmod (a.toNat : Int) w ≠ 0 := fun h => hal (.inl h)
  have hnw : allowed_misaligned a w 4 = false := by
    cases h : allowed_misaligned a w 4
    · rfl
    · exact absurd (.inr ⟨by simp, hg, by omega, h⟩) hal
  have hmod : a.toNat % w ≠ 0 := by
    intro e; apply hna; rw [show (w : Int) = ((w : Nat) : Int) from rfl, tmod_nat, e]; rfl
  rcases hw with rfl | rfl | rfl | rfl
  · exact absurd (Nat.mod_one _) hmod
  · exact mwe_split_2_0 s rs hr hh a (by omega) hwr hna hnw r hm hp hg hex
  · by_cases ho : a.toNat % 2 = 1
    · exact mwe_split_4_0 s rs hr hh a ho hwr hna hnw r hm hp hg hex
    · exact mwe_split_4_1 s rs hr hh a (by omega) hwr hna hnw r hm hp hg hex
  · by_cases ho : a.toNat % 2 = 1
    · exact mwe_split_8_0 s rs hr hh a ho hwr hna hnw r hm hp hg hex
    · by_cases h4 : a.toNat % 4 = 2
      · exact mwe_split_8_1 s rs hr hh a h4 hwr hna hnw r hm hp hg hex
      · exact mwe_split_8_2 s rs hr hh a (by omega) hwr hna hnw r hm hp hg hex

set_option maxHeartbeats 4000000 in
/-- A store inside a writable region, of 1, 2, 4 or 8 bytes, at any
    alignment: its bytes, little-endian. Aligned or within its 16-byte
    granule, sail-riscv writes it whole; across the granule, in pieces
    (`cmw_split_*`), which make the same bytes. -/
theorem mem_write_value_any (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8) (hwr : a.toNat + w < 2 ^ 64)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : r.attributes.writable = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none) (v : BitVec (8 * w)) :
    (interpretSailEffects (mem_write_value (.Physaddr a) w v .Machine (.Store .Data) .PBMT_PMA false false false)).run s
      = .ok (.Ok true) { s with mem := Equiv.memWrite (n := w) s.mem a.toNat v } := by
  have hp : permits r.attributes (.Store .Data) = true := by simpa [permits] using hperm
  by_cases hal : AlignOk r (.Store .Data) a w
  · rcases hw with rfl | rfl | rfl | rfl
    · exact mem_write_value_run1 s rs hr hh a r hm hperm hal v
    · exact mem_write_value_run2 s rs hr hh a r hm hperm hal v
    · exact mem_write_value_run4 s rs hr hh a r hm hperm hal v
    · exact mem_write_value_run s rs hr hh a r hm hperm hal v
  have hna : Int.tmod (a.toNat : Int) w ≠ 0 := fun h => hal (.inl h)
  have hnw : allowed_misaligned a w 4 = false := by
    cases h : allowed_misaligned a w 4
    · rfl
    · exact absurd (.inr ⟨by simp, hg, by omega, h⟩) hal
  have hmod : a.toNat % w ≠ 0 := by
    intro e; apply hna; rw [show (w : Int) = ((w : Nat) : Int) from rfl, tmod_nat, e]; rfl
  rcases hw with rfl | rfl | rfl | rfl
  · exact absurd (Nat.mod_one _) hmod
  · have h := cmw_split_2_0 s rs hr hh a (by omega) hwr hna hnw r hm hp hg hex v
    simp [mem_write_value, mem_write_value_priv_meta, h]
  · by_cases ho : a.toNat % 2 = 1
    · have h := cmw_split_4_0 s rs hr hh a ho hwr hna hnw r hm hp hg hex v
      simp [mem_write_value, mem_write_value_priv_meta, h]
    · have h := cmw_split_4_1 s rs hr hh a (by omega) hwr hna hnw r hm hp hg hex v
      simp [mem_write_value, mem_write_value_priv_meta, h]
  · by_cases ho : a.toNat % 2 = 1
    · have h := cmw_split_8_0 s rs hr hh a ho hwr hna hnw r hm hp hg hex v
      simp [mem_write_value, mem_write_value_priv_meta, h]
    · by_cases h4 : a.toNat % 4 = 2
      · have h := cmw_split_8_1 s rs hr hh a h4 hwr hna hnw r hm hp hg hex v
        simp [mem_write_value, mem_write_value_priv_meta, h]
      · have h := cmw_split_8_2 s rs hr hh a (by omega) hwr hna hnw r hm hp hg hex v
        simp [mem_write_value, mem_write_value_priv_meta, h]

set_option maxHeartbeats 4000000 in
/-- `sd rs2, imm(rs1)`, at any address in a writable region (no wrap past the end),
    in machine mode with no pointer masking: rs2's 8 bytes go there. -/
theorem execute_sd (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 8 = some r)
    (hwr : r.attributes.writable = true) (hal : SplitOk r (v1 + sign_extend (m := 64) imm) 8) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 8)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 8) t.mem (v1 + sign_extend (m := 64) imm).toNat v2 } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hea := mem_write_ea_any t rs hr hh _ 8 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1
  obtain ⟨sp, hsp⟩ := split_any (v1 + sign_extend (m := 64) imm) 8 (by decide)
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
    | (rw [hsp]; try dsimp only)
    | (simp only [bare_ne, Bool.false_and, Bool.and_false, Bool.false_eq_true, if_false]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_any t rs hr hh _ 8 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1]; try dsimp only))
  congr 3
  change BitVec.setWidth 64 (Sail.BitVec.extractLsb (BitVec.setWidth 64 (Sail.BitVec.extractLsb v2 63 0)) 63 0) = v2
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]

set_option maxHeartbeats 4000000 in
/-- `sw rs2, imm(rs1)`, at any address in a writable region (no wrap past the end),
    in machine mode with no pointer masking: rs2's low 4 bytes go there. -/
theorem execute_sw (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 4 = some r)
    (hwr : r.attributes.writable = true) (hal : SplitOk r (v1 + sign_extend (m := 64) imm) 4) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 4)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 4) t.mem (v1 + sign_extend (m := 64) imm).toNat (v2.setWidth 32) } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hea := mem_write_ea_any t rs hr hh _ 4 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1
  obtain ⟨sp, hsp⟩ := split_any (v1 + sign_extend (m := 64) imm) 4 (by decide)
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
    | (rw [hsp]; try dsimp only)
    | (simp only [bare_ne, Bool.false_and, Bool.and_false, Bool.false_eq_true, if_false]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_any t rs hr hh _ 4 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1]; try dsimp only))
  congr 3
  change BitVec.setWidth 32 (Sail.BitVec.extractLsb (BitVec.setWidth 32 (Sail.BitVec.extractLsb v2 31 0)) 31 0) = v2.setWidth 32
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]

set_option maxHeartbeats 4000000 in
/-- `sh rs2, imm(rs1)`, at any address in a writable region (no wrap past the end),
    in machine mode with no pointer masking: rs2's low 2 bytes go there. -/
theorem execute_sh (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 2 = some r)
    (hwr : r.attributes.writable = true) (hal : SplitOk r (v1 + sign_extend (m := 64) imm) 2) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 2)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 2) t.mem (v1 + sign_extend (m := 64) imm).toNat (v2.setWidth 16) } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hea := mem_write_ea_any t rs hr hh _ 2 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1
  obtain ⟨sp, hsp⟩ := split_any (v1 + sign_extend (m := 64) imm) 2 (by decide)
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
    | (rw [hsp]; try dsimp only)
    | (simp only [bare_ne, Bool.false_and, Bool.and_false, Bool.false_eq_true, if_false]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_any t rs hr hh _ 2 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1]; try dsimp only))
  congr 3
  change BitVec.setWidth 16 (Sail.BitVec.extractLsb (BitVec.setWidth 16 (Sail.BitVec.extractLsb v2 15 0)) 15 0) = v2.setWidth 16
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]

set_option maxHeartbeats 4000000 in
/-- `sb rs2, imm(rs1)`, at any address in a writable region (no wrap past the end),
    in machine mode with no pointer masking: rs2's low 1 bytes go there. -/
theorem execute_sb (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (rs1 rs2 : RegBits) (imm : BitVec 12) (v1 v2 : BitVec 64)
    (h1 : getX t rs1.toNat = some v1) (h2 : getX t rs2.toNat = some v2)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr (v1 + sign_extend (m := 64) imm)) 1 = some r)
    (hwr : r.attributes.writable = true) (hal : SplitOk r (v1 + sign_extend (m := 64) imm) 1) :
    (interpretSailEffects (execute_STORE imm (.Regidx rs2) (.Regidx rs1) 1)).run t
      = .ok RETIRE_SUCCESS
          { t with mem := Equiv.memWrite (n := 1) t.mem (v1 + sign_extend (m := 64) imm).toNat (v2.setWidth 8) } := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have hacc : PlainAccess (.Store .Data) := Or.inr (Or.inr rfl)
  have ht := translateAddr_run t (v1 + sign_extend (m := 64) imm) _ hacc mi hmi hp se hse me hme
  have hea := mem_write_ea_any t rs hr hh _ 1 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1
  obtain ⟨sp, hsp⟩ := split_any (v1 + sign_extend (m := 64) imm) 1 (by decide)
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
    | (rw [hsp]; try dsimp only)
    | (simp only [bare_ne, Bool.false_and, Bool.and_false, Bool.false_eq_true, if_false]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [mem_write_value_any t rs hr hh _ 1 (by decide) hal.2.2 r hm hwr hal.1 hal.2.1]; try dsimp only))
  congr 3
  change BitVec.setWidth 8 (Sail.BitVec.extractLsb (BitVec.setWidth 8 (Sail.BitVec.extractLsb v2 7 0)) 7 0) = v2.setWidth 8
  apply BitVec.eq_of_getLsbD_eq; intro i hi
  simp [Sail.BitVec.extractLsb, hi]


end Equiv.Riscv
