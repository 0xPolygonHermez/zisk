/-
  sail-riscv's semantics of `csrr rd, marchid` (`csrrs rd, marchid, x0`), in
  machine mode: a read, no write, so none of the CSR's side effects; rd
  gets the register marchid, which ZisK's profile sets to ZisK's ID
  (contract § 6).
-/
import Equiv.RiscvStep

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem

set_option maxHeartbeats 0 in
/-- `csrrs rd, 0xF12, x0` in machine mode: rd gets marchid. -/
theorem execute_marchid (s : RVState) (h : WF s) (hp : s.regs.get? Register.cur_privilege = some .Machine)
    (rd : BitVec 5) (v : BitVec 64) (hm : s.regs.get? Register.marchid = some v) :
    (interpretSailEffects (execute_CSRReg 0xF12#12 (.Regidx 0) (.Regidx rd) .CSRRS)).run s =
      .ok RETIRE_SUCCESS (setX s rd.toNat v) := by
  have ha : csr_access_type csrop.CSRRS (regidx.Regidx rd == zreg) (regidx.Regidx 0 == zreg) = .CSRRead := by
    cases (regidx.Regidx rd == zreg) <;> rfl
  -- the checks on marchid, in machine mode, a read: each a constant (`rv_step`'s `simp [*]` uses them)
  have e1 : ∀ t : RVState, (interpretSailEffects (privLevel_to_CSR_privbits Privilege.Machine)).run t = .ok 3#2 t :=
    fun _ => rfl
  have e2 : ∀ t : RVState, (interpretSailEffects (is_CSR_accessible 0xF12#12 Privilege.Machine
      CSRAccessType.CSRRead)).run t = .ok true t := fun _ => rfl
  have e3 : ∀ t : RVState, (interpretSailEffects (stateen_allows_CSR_access 0xF12#12 Privilege.Machine
      CSRAccessType.CSRRead)).run t = .ok true t := fun _ => rfl
  have e4 : ∀ t : RVState, (interpretSailEffects (csr_id_read_callback 0xF12#12 v)).run t = .ok () t := fun _ => rfl
  have e5 : read_CSR 0xF12#12 = readReg Register.marchid := rfl
  simp only [execute_CSRReg, doCSR, rX_bits, toNatInt_toNat, interp_bind, run_bind', ha]
  erw [rX_run s 0 (by omega) 0 (by simp [getX])]; dsimp only
  simp only [check_CSR_result, check_CSR, check_CSR_priv, interp_bind, run_bind']
  repeat rv_step h
  rw [e4]; dsimp only
  simp only [wX_bits, toNatInt_toNat, interp_bind, run_bind']
  erw [wX_run s rd.toNat rd.isLt]; rfl

end Equiv.Riscv
