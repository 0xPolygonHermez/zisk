/-
  sail-riscv on a 16-bit parcel at pc (its low two bits not 11), the
  compressed instructions off in ZisK's profile: the fetch reads the parcel
  alone (`F_RVC`), the decoder finds nothing in it (`decode_rvc`), and the
  step, Zca off, finds the instruction illegal: the trap vector.
-/
import Equiv.RiscvTrap
import Equiv.RiscvDecodeRvc

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- sail-riscv's fetch of a 16-bit parcel at a 4-byte aligned pc in an
    executable region: the parcel alone. -/
theorem fetch_rvc (s : RVState) (rs : List PMA_Region) (hs : MemReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc) (hal : pc.toNat % 4 = 0)
    (r1 : PMA_Region) (hm1 : matching_pma_region rs (.Physaddr pc) 2 = some r1)
    (hx1 : r1.attributes.executable = true) (lo : BitVec 16)
    (hlo : (Sail.ArchSem.readBytes 2 pc.toNat : Sail.ArchSem.ArchSemSeqM exception _).run s = .ok lo s)
    (hrvc : isRVC (from_bytes_le (n := 2) (bitvec_to_vecbytes lo)) = true) :
    (interpretSailEffects (fetch ())).run s = .ok (.F_RVC (from_bytes_le (n := 2) (bitvec_to_vecbytes lo))) s := by
  have ha1 : Int.tmod (pc.toNat : Int) 2 = 0 := by
    rw [show (2 : Int) = ((2 : Nat) : Int) from rfl, tmod_nat]; omega
  obtain ⟨b0, b1⟩ := access_low pc hal
  have f1 := fetch_bytes_run s rs hs pc pc 2 (by omega) r1 hm1 hx1 ha1 lo hlo
  obtain ⟨hp, _, ⟨mi, hmi⟩, -, -, -, -⟩ := hs
  simp (config := { decide := true }) [fetch, ext_fetch_check_pc, me_run_lift_bind,
    readReg_run _ _ _ hpc, readReg_run _ _ _ hmi, currentlyEnabled, hartSupports,
    is_aligned_vaddr, BitVec.addInt, f1, hrvc, b0, b1]
  rfl

/-- The fetch in terms of memory: the parcel's two bytes, when the low two
    bits of the first are not 11. -/
theorem fetch_half (s : RVState) (rs : List PMA_Region) (hs : MemReady s rs)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc) (hal : pc.toNat % 4 = 0)
    (r1 : PMA_Region) (hm1 : matching_pma_region rs (.Physaddr pc) 2 = some r1)
    (hx1 : r1.attributes.executable = true) (b0 b1 : BitVec 8)
    (h0 : s.mem.get? pc.toNat = some b0) (h1 : s.mem.get? (pc.toNat + 1) = some b1)
    (hc : b0.extractLsb 1 0 ≠ 3#2) :
    (interpretSailEffects (fetch ())).run s = .ok (.F_RVC (b1 ++ b0)) s := by
  have lo := readBytes2_run s pc.toNat b0 b1 h0 h1
  have hr : isRVC (from_bytes_le (n := 2) (bitvec_to_vecbytes (b1 ++ b0))) = true := by
    rw [rt2]; simp [isRVC, Sail.BitVec.extractLsb, LeanZiskRV.not]
    have : (b1 ++ b0).extractLsb 1 0 = b0.extractLsb 1 0 := by bv_decide
    rw [this]; exact hc
  have := fetch_rvc s rs hs pc hpc hal r1 hm1 hx1 _ lo hr
  rwa [rt2] at this

set_option maxHeartbeats 4000000 in
/-- One step of sail-riscv on a 16-bit parcel at pc, in machine mode, mtvec
    at 4: an illegal instruction, Zca off; the step ends at the trap
    vector. -/
theorem try_step_rvc (s : RVState) (rs : List PMA_Region) (hr : StepReady s rs)
    (hmt : s.regs.get? mtvec = some 0x4#64)
    (pc : BitVec 64) (hpc : s.regs.get? PC = some pc) (hal : pc.toNat % 4 = 0) (hx : ExecAt rs pc)
    (b0 b1 : BitVec 8) (h0 : s.mem.get? pc.toNat = some b0) (h1 : s.mem.get? (pc.toNat + 1) = some b1)
    (hc : b0.extractLsb 1 0 ≠ 3#2) (n : Nat) :
    ∃ s', (interpretSailEffects (try_step n false)).run s = .ok false s' ∧ s'.regs.get? PC = some 0x4#64 := by
  obtain ⟨h, hmip, hmie, hhs, help, hs, -, ⟨mv, hmv, hmM⟩, ⟨bv, hbv, hbB⟩, -⟩ := hr
  obtain ⟨r1, hm1, hx1⟩ := hx
  have hp := hs.priv
  simp only [try_step, run_hart_active, should_inc_minstret, counter_priv_filter_bit, dispatchInterrupt,
    getPendingSet, read_mip, is_landing_pad_expected, hart_is_active, hart_is_waiting, tick_pc,
    get_config_rvfi, get_config_print_instr, currentlyEnabled, hartSupports, external_interrupts_pending,
    supervisor_software_interrupt_pending, plat_have_sig, handle_exception]
  rw [interp_bind, run_bind', update_mip_run s h hmip]; dsimp only
  repeat rv_step h
  rw [fetch_half _ rs ?hs1 pc ?hpc1 hal r1 hm1 hx1 b0 b1 ?h0 ?h1 hc]
  rotate_left
  · exact hs.congr (by
      intro r hr
      simp only [memRegs, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl | rfl | rfl | rfl | rfl | rfl <;> simp [Std.ExtDHashMap.get?_insert])
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hpc
  · exact h0
  · exact h1
  dsimp only [ext_fetch_hook]
  repeat rv_step h
  rw [decode_rvc _ _ (regVal h mstatus) ?hms1 ?hp1 mv ?hmv1]
  rotate_left
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using h.get? mstatus
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hp
  · simp only [Std.ExtDHashMap.get?_insert]; simpa using hmv
  repeat rv_step h
  rw [(exception_handler_eq _ ?hw ?hp2 ?hmt2 _ _).1]
  rotate_left
  · exact ⟨by (repeat apply isSome_insert); exact h.all⟩
  · simp [Std.ExtDHashMap.get?_insert, hp]
  · simp [Std.ExtDHashMap.get?_insert, hmt]
  generalize hE : excState _ _ _ = E
  have hw2 : WF E := by
    rw [← hE]
    exact (exception_handler_eq _ ⟨by (repeat apply isSome_insert); exact h.all⟩
      (by simp [Std.ExtDHashMap.get?_insert, hp]) (by simp [Std.ExtDHashMap.get?_insert, hmt]) _ _).2.1
  have hhs2 : E.regs.get? hart_state = some (.HART_ACTIVE ()) := by
    rw [← hE, (exception_handler_eq _ ⟨by (repeat apply isSome_insert); exact h.all⟩
      (by simp [Std.ExtDHashMap.get?_insert, hp]) (by simp [Std.ExtDHashMap.get?_insert, hmt]) _ _).2.2]
    simp [Std.ExtDHashMap.get?_insert, hhs]
  simp only [set_next_pc]
  repeat (first | rv_step hw2 | (simp only [WF.get? hw2]; try dsimp only))
  exact ⟨_, rfl, by simp⟩

end Equiv.Riscv
