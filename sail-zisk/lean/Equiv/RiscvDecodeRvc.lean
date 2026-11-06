/-
  sail-riscv's compressed decoder, in ZisK's profile: every clause needs
  Zca (or Zcb, Zcd, Zcf, Zcmop, all built on it), which the profile lacks,
  so any 16-bit parcel decodes to `C_ILLEGAL`. The clauses are skipped as
  the 32-bit decoder's are (`skip_p`), the extension checks run to their
  values (`peel_c`).
-/
import Equiv.RiscvDecodeRejBase

set_option maxRecDepth 1000000
namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

theorem ce_zca (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zca)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_zcb (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zcb)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_zcd (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (hp : s.regs.get? cur_privilege = some .Machine) :
    (interpretSailEffects (currentlyEnabled .Ext_Zcd)).run s = .ok false s := by
  simp (config := { decide := true }) [currentlyEnabled, hartSupports, readReg_run _ _ _ hv,
    readReg_run _ _ _ hms, readReg_run _ _ _ hp, inVirtualPrivilege, privLevel_is_virtual]

theorem ce_zcf (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (hp : s.regs.get? cur_privilege = some .Machine) :
    (interpretSailEffects (currentlyEnabled .Ext_Zcf)).run s = .ok false s := by
  simp (config := { decide := true }) [currentlyEnabled, hartSupports, readReg_run _ _ _ hv,
    readReg_run _ _ _ hms, readReg_run _ _ _ hp, inVirtualPrivilege, privLevel_is_virtual]

theorem ce_zbb_run (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zbb)).run s = .ok (_get_Misa_B v == 1#1) s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_zba_run (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zba)).run s = .ok (_get_Misa_B v == 1#1) s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_m_run (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_M)).run s = .ok (_get_Misa_M v == 1#1) s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_zmmul_run (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zmmul)).run s = .ok (_get_Misa_M v == 1#1) s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

theorem ce_zcmop (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_Zcmop)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

/-- `peel`, with the compressed decoder's extension checks and register
    mappings. -/
macro "peel_c" : tactic => `(tactic| first
  | (refine bind_none_run _ _ _ ?hm (fun _ => ?_)
     case hm => first
       | (unfold encdec_ntl_backwards; split <;> first
           | exact ⟨_, rfl⟩ | (exfalso; simp_all [encdec_ntl_backwards_matches]))
       | (simp only [encdec_reg_backwards, Functions.base_E_enabled, LeanZiskRV.not, Bool.not_false,
           Bool.true_or, if_true]; exact ⟨_, rfl⟩))
  | (rw [if_neg]; rotate_left; (focus (simp; done)); try dsimp only)
  | peel
  | (rw [ce_zca _ _ (by assumption)]; try dsimp only)
  | (rw [ce_zcb _ _ (by assumption)]; try dsimp only)
  | (rw [ce_zcd _ _ (by assumption) _ (by assumption) (by assumption)]; try dsimp only)
  | (rw [ce_zcf _ _ (by assumption) _ (by assumption) (by assumption)]; try dsimp only)
  | (rw [ce_zicfiss _ _ (by assumption)]; try dsimp only)
  | (rw [ce_zbb_run _ _ (by assumption)]; try dsimp only)
  | (rw [ce_zba_run _ _ (by assumption)]; try dsimp only)
  | (rw [ce_m_run _ _ (by assumption)]; try dsimp only)
  | (rw [ce_zmmul_run _ _ (by assumption)]; try dsimp only)
  | (rw [ce_zcmop _ _ (by assumption)]; try dsimp only)
  | (refine ite_none_run _ _ _ ?_; intro))

/-- `skip_p` over the compressed decoder's clauses. -/
macro "skip_c" : tactic => `(tactic| first
  | (refine (skip_ite _ _ _ _ ?h).trans ?_
     case h => intro hc; exfalso; revert hc; guard_false)
  | (refine (skip_none _ _ _ ?h).trans ?_
     case h => (try dsimp only); (repeat peel_c); first | done | rfl)
  | dsimp only)

set_option maxHeartbeats 0 in
/-- Any 16-bit parcel decodes to `C_ILLEGAL`, the state as it was. -/
theorem decode_rvc (s : RVState) (h : BitVec 16)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (hp : s.regs.get? cur_privilege = some .Machine)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (ext_decode_compressed h)).run s = .ok (.C_ILLEGAL h) s := by
  unfold ext_decode_compressed
  delta encdec_compressed_backwards
  dsimp only
  repeat skip_c
  simp (config := { decide := true }) [currentlyEnabled, hartSupports, zicfiss_xSSE, interp_bind,
    readReg_run _ _ _ hp, readReg_run _ _ _ hmv]

end Equiv.Riscv
