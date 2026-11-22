/-
  sail-riscv's loads across a 16-byte granule, in ZisK's profile: the PMA
  check passes them as splittable (no misaligned exception in ZisK's
  regions), and `checked_mem_read` reads them in pieces of
  2^min(ctz addr, ctz width) bytes — which make the same value as the
  whole read (`cmr_split_*`). So a load inside a region that permits it
  reads its bytes at any alignment (`mem_read_any`).
-/
import Equiv.RiscvMem
import Equiv.MemRead

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

theorem bare_ne : (SATPMode.Bare != SATPMode.Bare) = false := rfl

/-- What a load or store of w bytes at a, in region r, needs of r to run at
    any alignment: a 16-byte granule and no misaligned exception, as every
    ZisK region has; and no wrap past the address space's end. -/
def SplitOk (r : PMA_Region) (a : BitVec 64) (w : Nat) : Prop :=
  r.attributes.misaligned_atomicity_granule_size_exp = 4 ∧ r.attributes.misaligned_exceptions.load_store = none ∧
    a.toNat + w < 2 ^ 64

/-- The page split of any access of 1, 2, 4 or 8 bytes runs. -/
theorem split_any (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8) :
    ∃ p, ∀ s : RVState, (interpretSailEffects (split_on_page_boundary a w)).run s = .ok p s := by
  simp only [split_on_page_boundary, Sail.BitVec.length]
  split
  · exact ⟨_, fun _ => rfl⟩
  · next hn =>
    simp only [LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
    rw [if_pos ?c]
    · exact ⟨_, fun _ => rfl⟩
    revert hn
    simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', ones, zeros, BitVec.subInt,
      BitVec.addInt, Functions.pagesize_bits, sail_ones, BitVec.zero_eq, BitVec.toNatInt, Sail.BitVec.extractLsb]
    have hb : (a &&& 7#64).toNat = a.toNat % 8 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 3
    rcases hw with rfl | rfl | rfl | rfl <;> simp <;> intro hn <;> refine Classical.byContradiction fun hc => hn ?_
    · have h7 : a &&& 7#64 ≤ 7#64 := by rw [BitVec.le_def, hb]; simp; omega
      revert h7; bv_decide
    · have h7 : a &&& 7#64 ≤ 6#64 := by rw [BitVec.le_def, hb]; simp; omega
      revert h7; bv_decide
    · have h7 : a &&& 7#64 ≤ 4#64 := by rw [BitVec.le_def, hb]; simp; omega
      revert h7; bv_decide
    · have h7 : a &&& 7#64 ≤ 0#64 := by rw [BitVec.le_def, hb]; simp; omega
      revert h7; bv_decide

set_option maxHeartbeats 0 in
/-- The PMA check of a load or store across its granule, in a region that
    permits it with no misaligned exception: splittable, at the granule. -/
theorem pmaCheck_split (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (a : BitVec 64) (w : Nat) (hw8 : w ≤ 8) (acc : MemoryAccessType mem_payload) (hacc : acc = .Load .Data ∨ acc = .Store .Data)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : permits r.attributes acc = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (hna : Int.tmod (a.toNat : Int) w ≠ 0) (hnw : allowed_misaligned a w 4 = false) :
    (interpretSailEffects (pmaCheck (.Physaddr a) w acc .PBMT_PMA false)).run s
      = .ok (.Ok { splittable := .CanSplit, granule_size_exp := 4 }) s := by
  obtain ⟨base, size, attrs, dt⟩ := r
  have hw8' : (w ≤b 8) = true := by simp; omega
  simp only at hg hex
  rcases hacc with rfl | rfl <;>
  · simp only [permits] at hperm
    simp (config := { decide := true }) [pmaCheck, hm, override_PMA, mag_pma_check, within_pma_mag,
      mag_of_pma, hg, hnw, hperm, hw8, hw8', Functions.xlen_bytes, me_run_lift_bind, me_run_pure,
      readReg_run _ _ _ hr, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_mag_applicable_access,
      bind_assoc, is_vector_access, extract_full, is_aligned_paddr, BitVec.toNatInt, hna, pma_misaligned_exception, hex]

set_option maxHeartbeats 4000000 in
/-- One piece of a split read: k bytes at a + j·k, a part of the whole. -/
theorem read_piece (s : RVState) (a : BitVec 64) (w : Nat) (v : BitVec (8 * w))
    (hv : Equiv.memRead s.mem w a.toNat = some v) (hwr : a.toNat + w < 2 ^ 64) (k j : Nat) (hk : j * k + k ≤ w)
    (x : BitVec 64) (hx : x = a + BitVec.ofNat 64 (j * k)) :
    (interpretSailEffects (read_ram .Read_plain (.Load .Data) (.Physaddr x) k false)).run s
      = .ok (from_bytes_le (n := k) (bitvec_to_vecbytes (v.extractLsb' (8 * (j * k)) (8 * k))), default_meta) s := by
  subst hx
  apply read_ram_run
  apply Equiv.riscv_readBytes
  have e : (a + BitVec.ofNat 64 (j * k)).toNat = a.toNat + j * k := by
    rw [BitVec.toNat_add, BitVec.toNat_ofNat]
    rw [Nat.mod_eq_of_lt (a := j * k) (by omega)]; exact Nat.mod_eq_of_lt (by omega)
  rw [e]
  exact Equiv.memRead_piece hv (j * k) k hk

set_option maxHeartbeats 0 in
/-- A load of 2 bytes across its granule, its address 1 mod 2 (pieces of 1). -/
theorem cmr_split_2_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 2 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 2 ≠ 0) (hnw : allowed_misaligned a 2 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 2 = some r)
    (hperm : permits r.attributes (.Load .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (v : BitVec 16) (hv : Equiv.memRead s.mem 2 a.toNat = some v) :
    (interpretSailEffects (checked_mem_read (.Load .Data) .PBMT_PMA .Machine (.Physaddr a) 2 false false false false)).run s
      = .ok (.Ok (v, default_meta)) s := by
  have hp := pmaCheck_split s rs hr a 2 (by omega) _ (.inl rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 2) = 1 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (1 : Int) = 1 := by decide
  have htd : Int.tdiv 2 1 = 2 := by decide
  simp (config := { decide := true }) [checked_mem_read, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, read_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_readable, within_clint, within_sig, within_htif_readable,
    within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [read_piece s a 2 v hv hwr 1 0 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 2 v hv hwr 1 1 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  simp only [rt1, rt2, rt4]
  congr 3
  simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', zeros]
  bv_decide

set_option maxHeartbeats 0 in
/-- A load of 4 bytes across its granule, its address 1 mod 2 (pieces of 1). -/
theorem cmr_split_4_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 4 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 4 ≠ 0) (hnw : allowed_misaligned a 4 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : permits r.attributes (.Load .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (v : BitVec 32) (hv : Equiv.memRead s.mem 4 a.toNat = some v) :
    (interpretSailEffects (checked_mem_read (.Load .Data) .PBMT_PMA .Machine (.Physaddr a) 4 false false false false)).run s
      = .ok (.Ok (v, default_meta)) s := by
  have hp := pmaCheck_split s rs hr a 4 (by omega) _ (.inl rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 4) = 2 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (2 : Int) = 1 := by decide
  have htd : Int.tdiv 4 1 = 4 := by decide
  simp (config := { decide := true }) [checked_mem_read, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, read_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_readable, within_clint, within_sig, within_htif_readable,
    within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [read_piece s a 4 v hv hwr 1 0 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 4 v hv hwr 1 1 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 4 v hv hwr 1 2 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 4 v hv hwr 1 3 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  simp only [rt1, rt2, rt4]
  congr 3
  simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', zeros]
  bv_decide

set_option maxHeartbeats 0 in
/-- A load of 4 bytes across its granule, its address 2 mod 4 (pieces of 2). -/
theorem cmr_split_4_1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 4 = 2) (hwr : a.toNat + 4 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 4 ≠ 0) (hnw : allowed_misaligned a 4 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : permits r.attributes (.Load .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (v : BitVec 32) (hv : Equiv.memRead s.mem 4 a.toNat = some v) :
    (interpretSailEffects (checked_mem_read (.Load .Data) .PBMT_PMA .Machine (.Physaddr a) 4 false false false false)).run s
      = .ok (.Ok (v, default_meta)) s := by
  have hp := pmaCheck_split s rs hr a 4 (by omega) _ (.inl rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 1 := by
    have hb : (a &&& 3#64).toNat = a.toNat % 4 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 2
    have h' : a &&& 3#64 = 2#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 1#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 4) = 2 := by decide
  have hpw : (2 : Int) ^ min (1 : Int) (2 : Int) = 2 := by decide
  have htd : Int.tdiv 4 2 = 2 := by decide
  simp (config := { decide := true }) [checked_mem_read, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, read_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_readable, within_clint, within_sig, within_htif_readable,
    within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [read_piece s a 4 v hv hwr 2 0 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 4 v hv hwr 2 1 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  simp only [rt1, rt2, rt4]
  congr 3
  simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', zeros]
  bv_decide

set_option maxHeartbeats 0 in
/-- A load of 8 bytes across its granule, its address 1 mod 2 (pieces of 1). -/
theorem cmr_split_8_0 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 2 = 1) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Load .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (v : BitVec 64) (hv : Equiv.memRead s.mem 8 a.toNat = some v) :
    (interpretSailEffects (checked_mem_read (.Load .Data) .PBMT_PMA .Machine (.Physaddr a) 8 false false false false)).run s
      = .ok (.Ok (v, default_meta)) s := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inl rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 0 := by
    have hb : (a &&& 1#64).toNat = a.toNat % 2 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 1
    have h' : a &&& 1#64 = 1#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 0#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (0 : Int) (3 : Int) = 1 := by decide
  have htd : Int.tdiv 8 1 = 8 := by decide
  simp (config := { decide := true }) [checked_mem_read, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, read_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_readable, within_clint, within_sig, within_htif_readable,
    within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [read_piece s a 8 v hv hwr 1 0 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 1 1 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 1 2 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 1 3 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 1 4 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 1 5 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 1 6 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 1 7 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  simp only [rt1, rt2, rt4]
  congr 3
  simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', zeros]
  bv_decide

set_option maxHeartbeats 0 in
/-- A load of 8 bytes across its granule, its address 2 mod 4 (pieces of 2). -/
theorem cmr_split_8_1 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 4 = 2) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Load .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (v : BitVec 64) (hv : Equiv.memRead s.mem 8 a.toNat = some v) :
    (interpretSailEffects (checked_mem_read (.Load .Data) .PBMT_PMA .Machine (.Physaddr a) 8 false false false false)).run s
      = .ok (.Ok (v, default_meta)) s := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inl rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 1 := by
    have hb : (a &&& 3#64).toNat = a.toNat % 4 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 2
    have h' : a &&& 3#64 = 2#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 1#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (1 : Int) (3 : Int) = 2 := by decide
  have htd : Int.tdiv 8 2 = 4 := by decide
  simp (config := { decide := true }) [checked_mem_read, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, read_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_readable, within_clint, within_sig, within_htif_readable,
    within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [read_piece s a 8 v hv hwr 2 0 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 2 1 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 2 2 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 2 3 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  simp only [rt1, rt2, rt4]
  congr 3
  simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', zeros]
  bv_decide

set_option maxHeartbeats 0 in
/-- A load of 8 bytes across its granule, its address 4 mod 8 (pieces of 4). -/
theorem cmr_split_8_2 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (hlow : a.toNat % 8 = 4) (hwr : a.toNat + 8 < 2 ^ 64)
    (hna : Int.tmod (a.toNat : Int) 8 ≠ 0) (hnw : allowed_misaligned a 8 4 = false)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : permits r.attributes (.Load .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (v : BitVec 64) (hv : Equiv.memRead s.mem 8 a.toNat = some v) :
    (interpretSailEffects (checked_mem_read (.Load .Data) .PBMT_PMA .Machine (.Physaddr a) 8 false false false false)).run s
      = .ok (.Ok (v, default_meta)) s := by
  have hp := pmaCheck_split s rs hr a 8 (by omega) _ (.inl rfl) r hm hperm hg hex hna hnw
  have hc : BitVec.countTrailingZeros a = 2 := by
    have hb : (a &&& 7#64).toNat = a.toNat % 8 := by
      rw [BitVec.toNat_and]; exact Nat.and_two_pow_sub_one_eq_mod a.toNat 3
    have h' : a &&& 7#64 = 4#64 := by apply BitVec.eq_of_toNat_eq; rw [hb]; simpa using hlow
    have : a.reverse.clz = 2#64 := by revert h'; bv_decide
    simp [BitVec.countTrailingZeros, Sail.BitVec.countTrailingZeros, Sail.BitVec.countLeadingZeros, this]
  have hc2 : BitVec.countTrailingZeros (to_bits (l := 13) 8) = 3 := by decide
  have hpw : (2 : Int) ^ min (2 : Int) (3 : Int) = 4 := by decide
  have htd : Int.tdiv 8 4 = 2 := by decide
  simp (config := { decide := true }) [checked_mem_read, check_pma_with_pmp_priority, hp, split_misaligned,
    misaligned_order, read_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
    pmpCheck_run, within_mmio_readable, within_clint, within_sig, within_htif_readable,
    within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert, split_access, BitVec.toNatInt, hna, extract_full, hnw, hc, hc2, hpw, htd]
  rw [read_piece s a 8 v hv hwr 4 0 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  rw [read_piece s a 8 v hv hwr 4 1 (by decide) _ (by bv_decide)]; dsimp only
  simp (config := { decide := true }) [readReg_run _ _ _ hh, interp_bind, run_bind', monadLift_self, liftM,
    interp_pure, run_pure', me_run_lift_bind, me_run_map_lift, bind_assoc, pmpCheck_run, within_mmio_readable,
    within_clint, within_sig, within_htif_readable, within_htif_writable, LeanZiskRV.assert,
    Sail.ArchSem.PreSail.assert]
  simp only [rt1, rt2, rt4]
  congr 3
  simp only [Sail.BitVec.updateSubrange, Sail.BitVec.updateSubrange', zeros]
  bv_decide


set_option maxHeartbeats 4000000 in
/-- A load inside a region that permits it, of 1, 2, 4 or 8 bytes, at any
    alignment: its bytes, little-endian. Aligned or within its 16-byte
    granule, sail-riscv reads it whole; across the granule, in pieces
    (`cmr_split_*`), which make the same value. -/
theorem mem_read_any (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (w : Nat) (hw : w = 1 ∨ w = 2 ∨ w = 4 ∨ w = 8) (hwr : a.toNat + w < 2 ^ 64)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : permits r.attributes (.Load .Data) = true) (hg : r.attributes.misaligned_atomicity_granule_size_exp = 4)
    (hex : r.attributes.misaligned_exceptions.load_store = none)
    (v : BitVec (8 * w)) (hv : Equiv.memRead s.mem w a.toNat = some v) :
    (interpretSailEffects (mem_read (.Load .Data) .PBMT_PMA .Machine (.Physaddr a) w false false false)).run s
      = .ok (.Ok (from_bytes_le (n := w) (bitvec_to_vecbytes v))) s := by
  by_cases hal : AlignOk r (.Load .Data) a w
  · exact mem_read_run s rs hr hh a w (by omega) _ (Or.inr (Or.inl rfl)) r hm hperm hal v
      (Equiv.riscv_readBytes s w _ v hv)
  have hna : Int.tmod (a.toNat : Int) w ≠ 0 := fun h => hal (.inl h)
  have hnw : allowed_misaligned a w 4 = false := by
    cases h : allowed_misaligned a w 4
    · rfl
    · exact absurd (.inr ⟨by simp, hg, by omega, h⟩) hal
  have hmod : a.toNat % w ≠ 0 := by
    intro e; apply hna; rw [show (w : Int) = ((w : Nat) : Int) from rfl, tmod_nat, e]; rfl
  rcases hw with rfl | rfl | rfl | rfl
  · exact absurd (Nat.mod_one _) hmod
  · have h := cmr_split_2_0 s rs hr hh a (by omega) hwr hna hnw r hm hperm hg hex v hv
    simp [mem_read, mem_read_priv_meta, h, MemoryOpResult_drop_meta, rt2]
  · by_cases ho : a.toNat % 2 = 1
    · have h := cmr_split_4_0 s rs hr hh a ho hwr hna hnw r hm hperm hg hex v hv
      simp [mem_read, mem_read_priv_meta, h, MemoryOpResult_drop_meta, rt4]
    · have h := cmr_split_4_1 s rs hr hh a (by omega) hwr hna hnw r hm hperm hg hex v hv
      simp [mem_read, mem_read_priv_meta, h, MemoryOpResult_drop_meta, rt4]
  · by_cases ho : a.toNat % 2 = 1
    · have h := cmr_split_8_0 s rs hr hh a ho hwr hna hnw r hm hperm hg hex v hv
      simp [mem_read, mem_read_priv_meta, h, MemoryOpResult_drop_meta, rt8]
    · by_cases h4 : a.toNat % 4 = 2
      · have h := cmr_split_8_1 s rs hr hh a h4 hwr hna hnw r hm hperm hg hex v hv
        simp [mem_read, mem_read_priv_meta, h, MemoryOpResult_drop_meta, rt8]
      · have h := cmr_split_8_2 s rs hr hh a (by omega) hwr hna hnw r hm hperm hg hex v hv
        simp [mem_read, mem_read_priv_meta, h, MemoryOpResult_drop_meta, rt8]

end Equiv.Riscv
