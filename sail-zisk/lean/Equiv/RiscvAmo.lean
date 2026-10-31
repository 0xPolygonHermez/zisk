/-
  sail-riscv's AMOs, in ZisK's profile: `amo<op>.d`/`.w rd, rs2, (rs1)` at
  an address the PMA check passes unsplit, in a region readable, writable
  and allowing the op, in machine mode with no pointer masking: the bytes
  there become the op of rs2 and what was there, and rd gets what was there
  (sign-extended, for .w) — through sail-riscv's whole AMO path.
-/
import Equiv.RiscvAmoMem

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- An AMO's result: the op of rs2's value and the loaded value, as
    sail-riscv's execute_AMO computes it. -/
def amoVal {n : Nat} (op : amoop) (loaded r2 : BitVec n) : BitVec n :=
  match op with
  | .AMOSWAP => r2
  | .AMOADD => r2 + loaded
  | .AMOXOR => r2 ^^^ loaded
  | .AMOAND => r2 &&& loaded
  | .AMOOR => r2 ||| loaded
  | .AMOMIN => if zopz0zI_s r2 loaded then r2 else loaded
  | .AMOMAX => if zopz0zK_s r2 loaded then r2 else loaded
  | .AMOMINU => if zopz0zI_u r2 loaded then r2 else loaded
  | .AMOMAXU => if zopz0zK_u r2 loaded then r2 else loaded
  | .AMOCAS => r2

theorem amo_ne_fetch (op : amoop) (aq rl : Bool) :
    (MemoryAccessType.Atomic (op, aq, rl, mem_payload.Data, mem_payload.Data) != .InstructionFetch ()) = true := by
  cases op <;> cases aq <;> cases rl <;> rfl
theorem amo_ne_lx (op : amoop) (aq rl : Bool) :
    (MemoryAccessType.Atomic (op, aq, rl, mem_payload.Data, mem_payload.Data) != .LoadExecute .Data) = true := by
  cases op <;> cases aq <;> cases rl <;> rfl
theorem amo_ne_lpte (op : amoop) (aq rl : Bool) :
    (MemoryAccessType.Atomic (op, aq, rl, mem_payload.Data, mem_payload.Data) != .Load .PageTableEntry) = true := by
  cases op <;> cases aq <;> cases rl <;> rfl
theorem amo_ne_spte (op : amoop) (aq rl : Bool) :
    (MemoryAccessType.Atomic (op, aq, rl, mem_payload.Data, mem_payload.Data) != .Store .PageTableEntry) = true := by
  cases op <;> cases aq <;> cases rl <;> rfl

theorem xlen64 : (Functions.xlen == 64) = true := rfl

theorem plat_amo (op : amoop) (aq rl : Bool) : plat_misaligned_exception (amoAcc op aq rl) false = none := by
  simp [plat_misaligned_exception, is_amo_access, plat_misaligned_access]

set_option maxHeartbeats 8000000 in
theorem execute_amo8 (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (op : amoop) (hcas : op ≠ .AMOCAS) (aq rl : Bool)
    (rs1 rs2 rd : RegBits) (v1 v2 vd : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (h2 : getX t rs2.toNat = some v2) (hd : getX t rd.toNat = some vd)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr v1) 8 = some r)
    (hperm : AmoPerm r.attributes op 8) (hal : AlignOk r (amoAcc op aq rl) v1 8)
    (v : BitVec 64)
    (hv : (Sail.ArchSem.readBytes 8 v1.toNat : Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_AMO op aq rl (.Regidx rs2) (.Regidx rs1) 8 (.Regidx rd))).run t
      = .ok RETIRE_SUCCESS
          (setX { t with mem := Equiv.memWrite (n := 8) t.mem v1.toNat (amoVal op v v2) } rd.toNat v) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have rdd := rX_run t rd.toNat rd.lt vd hd
  have hc : (op == .AMOCAS) = false := by cases op <;> first | rfl | exact absurd rfl hcas
  have hwv := mem_write_value_amo8 t rs hr hh v1 op aq rl r hm hperm hal
  have ht := translateAddr_amo_run t v1 op aq rl mi hmi hp se hse me hme
  have hea := mem_write_ea_amo t rs hr hh v1 8 op aq rl r hm hperm hal
  have hrd := mem_read_amo t rs hr hh v1 8 (by omega) op aq rl r hm hperm hal v hv
  simp only [execute_AMO, default_access_info, get_pmlen, is_aligned_vaddr, get_transformed_data_addr, ext_data_get_addr,
    transform_effective_address, rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable,
    pm_transform_PA, Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode,
    currentlyEnabled, hartSupports, LeanZiskRV.not, wX_bits]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_amo, bits_of_virtaddr, ite_self]; try dsimp only)
    | (simp only [amo_ne_fetch, amo_ne_lx, amo_ne_lpte, amo_ne_spte, Bool.true_and, Bool.not_false,
        xlen64, if_true]; try dsimp only)
    | (simp only [BitVec.add_zero, pm_id]; try dsimp only)
    | (rw [readReg_run _ _ _ hsec]; try dsimp only)
    | (simp only [hpm]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [hrd]; try dsimp only)
    | (rw [r2]; try dsimp only)
    | (rw [rdd]; try dsimp only)
    | (simp only [hc, Bool.false_and, Bool.false_eq_true, if_false]; try dsimp only)
    | (erw [hwv]; try dsimp only))
  have e1 : BitVec.setWidth (8 * 8) (from_bytes_le (n := 8) (bitvec_to_vecbytes v)) = v := by rw [rt8]; simp
  have e2 : (trunc (m := 64) v2 : BitVec 64) = v2 := by
    apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [trunc, Sail.BitVec.truncate, hi]
  have e3 : ∀ x : BitVec 64, sign_extend (m := 64) x = x := by
    intro x; apply BitVec.eq_of_getLsbD_eq; intro i hi; simp [sign_extend, Sail.BitVec.signExtend, hi]
  simp only [e1, e2, e3]
  erw [wX_run _ rd.toNat rd.lt]
  simp only [interp_pure, run_pure']
  simp only [trunc, Sail.BitVec.truncate, sign_extend, Sail.BitVec.signExtend, Nat.reduceMul, Int.cast_ofNat_Int,
    Int.reduceMul, Int.reduceToNat, BitVec.truncate_eq_setWidth, BitVec.setWidth_eq, BitVec.signExtend_eq]
  cases op <;> rfl

set_option maxHeartbeats 8000000 in
theorem execute_amo4 (t : RVState) (_h : WF t) (rs : List PMA_Region) (hs : MemReady t rs)
    (sec : RegisterType mseccfg) (hsec : t.regs.get? mseccfg = some sec) (hpm : _get_Seccfg_PMM sec = 0#2)
    (op : amoop) (hcas : op ≠ .AMOCAS) (aq rl : Bool)
    (rs1 rs2 rd : RegBits) (v1 v2 vd : BitVec 64) (h1 : getX t rs1.toNat = some v1)
    (h2 : getX t rs2.toNat = some v2) (hd : getX t rd.toNat = some vd)
    (r : PMA_Region) (hm : matching_pma_region rs (.Physaddr v1) 4 = some r)
    (hperm : AmoPerm r.attributes op 4) (hal : AlignOk r (amoAcc op aq rl) v1 4)
    (v : BitVec 32)
    (hv : (Sail.ArchSem.readBytes 4 v1.toNat : Sail.ArchSem.ArchSemSeqM exception _).run t = .ok v t) :
    (interpretSailEffects (execute_AMO op aq rl (.Regidx rs2) (.Regidx rs1) 4 (.Regidx rd))).run t
      = .ok RETIRE_SUCCESS
          (setX { t with mem := Equiv.memWrite (n := 4) t.mem v1.toNat (amoVal op v (v2.setWidth 32)) } rd.toNat
            (v.signExtend 64)) := by
  have hs' := hs
  obtain ⟨hp, ⟨ms, hms, hmprv⟩, ⟨mi, hmi⟩, ⟨se, hse⟩, ⟨me, hme⟩, hh, hr⟩ := hs
  have r1 := rX_run t rs1.toNat rs1.lt v1 h1
  have r2 := rX_run t rs2.toNat rs2.lt v2 h2
  have rdd := rX_run t rd.toNat rd.lt vd hd
  have hc : (op == .AMOCAS) = false := by cases op <;> first | rfl | exact absurd rfl hcas
  have hwv := mem_write_value_amo4 t rs hr hh v1 op aq rl r hm hperm hal
  have ht := translateAddr_amo_run t v1 op aq rl mi hmi hp se hse me hme
  have hea := mem_write_ea_amo t rs hr hh v1 4 op aq rl r hm hperm hal
  have hrd := mem_read_amo t rs hr hh v1 4 (by omega) op aq rl r hm hperm hal v hv
  simp only [execute_AMO, default_access_info, get_pmlen, is_aligned_vaddr, get_transformed_data_addr, ext_data_get_addr,
    transform_effective_address, rX_bits, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert, is_pmm_applicable,
    pm_transform_PA, Functions.xlen_bytes, get_pmm, pmlen_of_mode, toNatInt_toNat, Functions.satp_mode,
    currentlyEnabled, hartSupports, LeanZiskRV.not, wX_bits]
  repeat (first
    | rv_step _h
    | (rw [effectivePrivilege_run _ _ _ _ hmprv]; try dsimp only)
    | (rw [r1]; try dsimp only)
    | (simp only [pmm_mode_backwards, pm_id, plat_amo, bits_of_virtaddr, ite_self]; try dsimp only)
    | (simp only [amo_ne_fetch, amo_ne_lx, amo_ne_lpte, amo_ne_spte, Bool.true_and, Bool.not_false,
        xlen64, if_true]; try dsimp only)
    | (simp only [BitVec.add_zero, pm_id]; try dsimp only)
    | (rw [readReg_run _ _ _ hsec]; try dsimp only)
    | (simp only [hpm]; try dsimp only)
    | (rw [ht]; try dsimp only)
    | (erw [hea]; try dsimp only)
    | (erw [hrd]; try dsimp only)
    | (rw [r2]; try dsimp only)
    | (rw [rdd]; try dsimp only)
    | (simp only [hc, Bool.false_and, Bool.false_eq_true, if_false]; try dsimp only)
    | (erw [hwv]; try dsimp only))
  have e1 : BitVec.setWidth (8 * 4) (from_bytes_le (n := 4) (bitvec_to_vecbytes v)) = v := by rw [rt4]; simp
  simp only [e1]
  erw [wX_run _ rd.toNat rd.lt]
  simp only [interp_pure, run_pure']
  simp only [trunc, Sail.BitVec.truncate, sign_extend, Sail.BitVec.signExtend, Nat.reduceMul, Int.cast_ofNat_Int,
    Int.reduceMul, Int.reduceToNat, BitVec.truncate_eq_setWidth, BitVec.setWidth_eq, BitVec.signExtend_eq]
  cases op <;> rfl

end Equiv.Riscv
