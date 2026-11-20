/-
  sail-riscv's memory layer under an AMO, in ZisK's profile: an access of
  kind `Atomic` (op, aq, rl, data, data). As for a plain access (RiscvMem,
  RiscvStore*): no translation, no PMP, and a PMA check an access inside a
  region that is readable, writable and allows the op passes unsplit,
  aligned or within its granule; the read gives the interpreter's bytes,
  the write puts them there.
-/
import Equiv.RiscvStoreW

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- An AMO's access kind. -/
abbrev amoAcc (op : amoop) (aq rl : Bool) : MemoryAccessType mem_payload := .Atomic (op, aq, rl, .Data, .Data)

theorem translateAddr_amo_run (s : RVState) (va : BitVec 64) (op : amoop) (aq rl : Bool)
    (m : RegisterType misa) (hm : s.regs.get? misa = some m)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (se : RegisterType senvcfg) (hse : s.regs.get? senvcfg = some se)
    (me : RegisterType menvcfg) (hme : s.regs.get? menvcfg = some me) :
    (interpretSailEffects (translateAddr (.Virtaddr va) (amoAcc op aq rl) .Machine)).run s
      = .ok (.Ok (.Physaddr va, .PBMT_PMA, ())) s := by
  simp (config := { decide := true }) [translateAddr, translate_vs_stage, Functions.satp_mode, is_svukte_qualified,
    is_shadow_stack_access, bits_of_virtaddr, zero_extend, Sail.BitVec.zeroExtend, currentlyEnabled,
    read_senvcfg, inVirtualPrivilege, me_run_lift_bind, me_run_pure,
    readReg_run _ _ _ hm, readReg_run _ _ _ hp, readReg_run _ _ _ hse, readReg_run _ _ _ hme]

/-- What an AMO needs of its region: readable, writable, the op allowed at
    the width. -/
def AmoPerm (a : PMA) (op : amoop) (w : Nat) : Prop :=
  a.readable = true ∧ a.writable = true ∧ pma_allows_atomic_op a.atomic_support op w = true

theorem pmaCheck_amo_run (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (a : BitVec 64) (w : Nat) (op : amoop) (aq rl : Bool)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : AmoPerm r.attributes op w) (hal : AlignOk r (amoAcc op aq rl) a w) :
    (interpretSailEffects (pmaCheck (.Physaddr a) w (amoAcc op aq rl) .PBMT_PMA true)).run s
      = .ok (.Ok { splittable := .CannotSplit, granule_size_exp := 0 }) s := by
  obtain ⟨base, size, attrs, dt⟩ := r
  obtain ⟨h1, h2, h3⟩ := hperm
  simp only at h1 h2 h3
  rcases hal with hal | ⟨-, hg, hw8, hall⟩
  · simp (config := { decide := true }) [pmaCheck, hm, override_PMA, mag_pma_check, is_aligned_paddr,
      BitVec.toNatInt, hal, h1, h2, h3, me_run_lift_bind, me_run_pure, readReg_run _ _ _ hr,
      LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_mag_applicable_access, bind_assoc]
    all_goals rfl
  · simp only at hg
    simp (config := { decide := true }) [pmaCheck, hm, override_PMA, mag_pma_check, within_pma_mag,
      mag_of_pma, hg, hall, h1, h2, h3, Functions.xlen_bytes, me_run_lift_bind, me_run_pure,
      readReg_run _ _ _ hr, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_mag_applicable_access,
      bind_assoc, is_vector_access, extract_full]
    all_goals rfl

/-! ## Reads and writes of any kind -/

theorem read_ram_run' (s : RVState) (rk : read_kind) (hk : rk ≠ .Read_RISCV_acquire ∧ rk ≠ .Read_RISCV_strong_acquire)
    (acc : MemoryAccessType mem_payload) (a : BitVec 64) (w : Nat) (v : BitVec (8 * w)) (hv : (Sail.ArchSem.readBytes w a.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok v s) :
    (interpretSailEffects (read_ram rk acc (.Physaddr a) w false)).run s
      = .ok (from_bytes_le (n := w) (bitvec_to_vecbytes v), default_meta) s := by
  cases rk <;> simp at hk <;>
  · simp [read_ram, Sail.ArchSem.PreSail.sail_mem_read, interpretSailEffects, FreeM.liftM,
      interpretInstructionEffect, Mem_request.toArchSem]
    erw [hv]; rfl

theorem write_ram_run8' (s : RVState) (wk : write_kind)
    (hk : wk ≠ .Write_RISCV_release ∧ wk ≠ .Write_RISCV_strong_release) (acc : MemoryAccessType mem_payload) (a : BitVec 64)
    (v : BitVec 64) :
    (interpretSailEffects (write_ram wk acc (.Physaddr a) 8 v ())).run s
      = .ok true { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hw := Equiv.riscv_writeBytes (n := 8) s a.toNat v
  cases wk <;> simp at hk <;>
  · simp [write_ram, LeanZiskRV.ArchSem.sail_mem_write, Sail.ArchSem.PreSail.sail_mem_write, interpretSailEffects,
      FreeM.liftM, interpretInstructionEffect, Mem_request.toArchSem, wt8]
    erw [hw]

theorem write_ram_run4' (s : RVState) (wk : write_kind)
    (hk : wk ≠ .Write_RISCV_release ∧ wk ≠ .Write_RISCV_strong_release) (acc : MemoryAccessType mem_payload) (a : BitVec 64)
    (v : BitVec 32) :
    (interpretSailEffects (write_ram wk acc (.Physaddr a) 4 v ())).run s
      = .ok true { s with mem := Equiv.memWrite (n := 4) s.mem a.toNat v } := by
  have hw := Equiv.riscv_writeBytes (n := 4) s a.toNat v
  cases wk <;> simp at hk <;>
  · simp [write_ram, LeanZiskRV.ArchSem.sail_mem_write, Sail.ArchSem.PreSail.sail_mem_write, interpretSailEffects,
      FreeM.liftM, interpretInstructionEffect, Mem_request.toArchSem, wt4]
    erw [hw]

theorem write_ram_ea_run' (s : RVState) (wk : write_kind)
    (hk : wk ≠ .Write_RISCV_release ∧ wk ≠ .Write_RISCV_strong_release) (acc : MemoryAccessType mem_payload) (a : BitVec 64)
    (w : Nat) :
    (interpretSailEffects (write_ram_ea wk acc (.Physaddr a) w)).run s = .ok () s := by
  cases wk <;> simp at hk <;>
  simp [write_ram_ea, LeanZiskRV.ArchSem.sail_mem_address_announce, Sail.ArchSem.PreSail.sail_mem_address_announce,
    interpretSailEffects, FreeM.liftM, interpretInstructionEffect]

/-! ## The AMO's three accesses -/

set_option maxHeartbeats 4000000 in
/-- The AMO's write announcement (`mem_write_ea`, conditional): nothing changes. -/
theorem mem_write_ea_amo (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (w : Nat) (op : amoop) (aq rl : Bool)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : AmoPerm r.attributes op w) (hal : AlignOk r (amoAcc op aq rl) a w) :
    (interpretSailEffects (mem_write_ea (.Physaddr a) w .Machine (amoAcc op aq rl) .PBMT_PMA (aq && rl) rl true)).run s
      = .ok (.Ok ()) s := by
  have hp := pmaCheck_amo_run s rs hr a w op aq rl r hm hperm hal
  cases aq <;> cases rl <;>
  · simp (config := { decide := true }) [mem_write_ea, check_pma_with_pmp_priority, hp, split_misaligned,
      misaligned_order, write_kind_of_flags, me_run_lift_bind, bind_assoc, untilFuelM, untilFuelM.go,
      pmpCheck_run, within_mmio_writable, within_clint, within_sig, within_htif_writable,
      readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
    have h0 : a + 0#64 = a := by simp
    erw [h0, write_ram_ea_run' _ _ (by simp)]

set_option maxHeartbeats 4000000 in
/-- The AMO's read (reserved): the interpreter's bytes, little-endian. -/
theorem mem_read_amo (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (w : Nat) (hw : 0 < w) (op : amoop) (aq rl : Bool)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) w = some r)
    (hperm : AmoPerm r.attributes op w) (hal : AlignOk r (amoAcc op aq rl) a w)
    (v : BitVec (8 * w)) (hv : (Sail.ArchSem.readBytes w a.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok v s) :
    (interpretSailEffects (mem_read (amoAcc op aq rl) .PBMT_PMA .Machine (.Physaddr a) w aq (aq && rl) true)).run s
      = .ok (.Ok (from_bytes_le (n := w) (bitvec_to_vecbytes v))) s := by
  have hp := pmaCheck_amo_run s rs hr a w op aq rl r hm hperm hal
  cases aq <;> cases rl <;>
  · simp (config := { decide := true }) [mem_read, mem_read_priv_meta, MemoryOpResult_drop_meta, checked_mem_read,
      check_pma_with_pmp_priority, hp, split_misaligned, misaligned_order, read_kind_of_flags, me_run_lift_bind,
      bind_assoc, untilFuelM, untilFuelM.go, pmpCheck_run, within_mmio_readable, within_clint, within_sig,
      within_htif_readable, within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt,
      LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]
    have h0 : a + 0#64 = a := by simp
    erw [h0, read_ram_run' s _ (by simp) _ a w v hv]
    simp only [full_update w hw]

set_option maxHeartbeats 4000000 in
/-- The AMO's write (conditional) of 8 bytes: the bytes, little-endian. -/
theorem mem_write_value_amo8 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (op : amoop) (aq rl : Bool)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 8 = some r)
    (hperm : AmoPerm r.attributes op 8) (hal : AlignOk r (amoAcc op aq rl) a 8) (v : BitVec 64) :
    (interpretSailEffects (mem_write_value (.Physaddr a) 8 v .Machine (amoAcc op aq rl) .PBMT_PMA (aq && rl) rl
      true)).run s = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 8) s.mem a.toNat v } := by
  have hp := pmaCheck_amo_run s rs hr a 8 op aq rl r hm hperm hal
  have hx : Sail.BitVec.extractLsb v 63 0 = v := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [Sail.BitVec.extractLsb]; omega
  cases aq <;> cases rl <;>
  · simp (config := { decide := true }) [mem_write_value, mem_write_value_priv_meta, checked_mem_write,
      check_pma_with_pmp_priority, hp, split_misaligned, misaligned_order, write_kind_of_flags, me_run_lift_bind,
      bind_assoc, untilFuelM, untilFuelM.go, pmpCheck_run, within_mmio_writable, within_clint, within_sig,
      within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
      Sail.ArchSem.PreSail.assert]
    have h0 : a + 0#64 = a := by simp
    erw [h0, write_ram_run8' s _ (by simp) _ a (Sail.BitVec.extractLsb v 63 0)]
    simp only [hx]

set_option maxHeartbeats 4000000 in
/-- The AMO's write (conditional) of 4 bytes: the bytes, little-endian. -/
theorem mem_write_value_amo4 (s : RVState) (rs : List PMA_Region) (hr : s.regs.get? pma_regions = some rs)
    (hh : s.regs.get? htif_tohost_base = some none)
    (a : BitVec 64) (op : amoop) (aq rl : Bool)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr a) 4 = some r)
    (hperm : AmoPerm r.attributes op 4) (hal : AlignOk r (amoAcc op aq rl) a 4) (v : BitVec 32) :
    (interpretSailEffects (mem_write_value (.Physaddr a) 4 v .Machine (amoAcc op aq rl) .PBMT_PMA (aq && rl) rl
      true)).run s = .ok (.Ok true) { s with mem := Equiv.memWrite (n := 4) s.mem a.toNat v } := by
  have hp := pmaCheck_amo_run s rs hr a 4 op aq rl r hm hperm hal
  have hx : Sail.BitVec.extractLsb v 31 0 = v := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [Sail.BitVec.extractLsb]; omega
  cases aq <;> cases rl <;>
  · simp (config := { decide := true }) [mem_write_value, mem_write_value_priv_meta, checked_mem_write,
      check_pma_with_pmp_priority, hp, split_misaligned, misaligned_order, write_kind_of_flags, me_run_lift_bind,
      bind_assoc, untilFuelM, untilFuelM.go, pmpCheck_run, within_mmio_writable, within_clint, within_sig,
      within_htif_writable, readReg_run _ _ _ hh, bits_of_physaddr, BitVec.addInt, LeanZiskRV.assert,
      Sail.ArchSem.PreSail.assert]
    have h0 : a + 0#64 = a := by simp
    erw [h0, write_ram_run4' s _ (by simp) _ a (Sail.BitVec.extractLsb v 31 0)]
    simp only [hx]

end Equiv.Riscv
