/-
  sail-riscv's decoder on the vector opcodes (OP-V 0x57, OP-VE 0x77): the
  profile has no vector extension (`vector_support_level` Disabled), but
  their checks read mstatus (VS) and misa before finding it off, so they
  take those registers' values. `known` learns them.
-/
import Equiv.RiscvDecodeRejBase
import Equiv.Vtype

set_option maxRecDepth 1000000

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

theorem ce_zve32x (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zve32x)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zve32f (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zve32f)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zve64x (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zve64x)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zve64f (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zve64f)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zve64d (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zve64d)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_v (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_V)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvfh (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvfh)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvfhmin (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvfhmin)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvbb (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvbb)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvkb (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvkb)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvbc (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvbc)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvkg (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvkg)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvkned (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvkned)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvknha (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvknha)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvknhb (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvknhb)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvksed (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvksed)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvksh (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvksh)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvabd (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvabd)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvfbfmin (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvfbfmin)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvfbfwma (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvfbfwma)).run s = .ok false s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvl32b (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvl32b)).run s = .ok true s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvl64b (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvl64b)).run s = .ok true s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

theorem ce_zvl128b (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv) :
    (interpretSailEffects (currentlyEnabled .Ext_Zvl128b)).run s = .ok true s := by
  simp (config := { decide := true }) [num_of_vector_support, currentlyEnabled, hartSupports, inVirtualPrivilege, privLevel_is_virtual, readReg_run _ _ _ hp, readReg_run _ _ _ hms,
    readReg_run _ _ _ hmv, vector_support_ge, vector_support_level]

/-- The vector extensions' checks, off. -/
macro_rules | `(tactic| known) => `(tactic| first
  | rw [ce_zvl32b _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvl64b _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvl128b _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zve32x _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zve32f _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zve64x _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zve64f _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zve64d _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_v _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvfh _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvfhmin _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvbb _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvkb _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvbc _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvkg _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvkned _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvknha _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvknhb _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvksed _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvksh _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvabd _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvfbfmin _ (by assumption) _ (by assumption) _ (by assumption)]
  | rw [ce_zvfbfwma _ (by assumption) _ (by assumption) _ (by assumption)])

theorem get_sew_run (s : RVState) (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) :
    (interpretSailEffects (get_sew ())).run s =
      .ok (2 ^ max (BitVec.toNatInt (_get_Vtype_vsew v) + 3) 0) s := by
  have h4 : BitVec.toNatInt (_get_Vtype_vsew v) < 4 := by
    have := hok.1; simp only [BitVec.toNatInt, Int.ofNat_eq_natCast]; omega
  simp [get_sew, get_sew_pow, readReg_run _ _ _ hv, h4, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]

theorem get_lmul_pow_run (s : RVState) (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) :
    (interpretSailEffects (get_lmul_pow ())).run s = .ok (_get_Vtype_vlmul v).toInt s := by
  have h4 := hok.2
  simp [get_lmul_pow, readReg_run _ _ _ hv, h4, LeanZiskRV.assert, Sail.ArchSem.PreSail.assert]

theorem zvk_check_encdec_ret (s : RVState) (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v)
    (l : RegisterType vl) (hl : s.regs.get? vl = some l) (st : RegisterType vstart) (hst : s.regs.get? vstart = some st)
    (a b : Nat) : ∃ r, (interpretSailEffects (zvk_check_encdec a b)).run s = .ok r s := by
  simp [zvk_check_encdec, get_lmul_pow_run _ _ hv hok, get_sew_run _ _ hv hok, readReg_run _ _ _ hl,
    readReg_run _ _ _ hst, vstart_reserved_behavior]

theorem valid_widening_fp_conversion_ret (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : vfwunary0) :
    ∃ r, (interpretSailEffects (valid_widening_fp_conversion c)).run s = .ok r s := by
  cases c <;> simp [valid_widening_fp_conversion, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, num_of_vector_support, vector_support_ge, vector_support_level, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports]

theorem valid_narrowing_fp_conversion_ret (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : vfnunary0) :
    ∃ r, (interpretSailEffects (valid_narrowing_fp_conversion c)).run s = .ok r s := by
  cases c <;> simp [valid_narrowing_fp_conversion, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, num_of_vector_support, vector_support_ge, vector_support_level, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports]

theorem valid_wide_vvtype_ret (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : vvfunct6) :
    ∃ r, (interpretSailEffects (valid_wide_vvtype c)).run s = .ok r s := by
  cases c <;> simp [valid_wide_vvtype, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, num_of_vector_support, vector_support_ge, vector_support_level, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports]

theorem valid_wide_vxtype_ret (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : vxfunct6) :
    ∃ r, (interpretSailEffects (valid_wide_vxtype c)).run s = .ok r s := by
  cases c <;> simp [valid_wide_vxtype, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, num_of_vector_support, vector_support_ge, vector_support_level, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports]

theorem valid_wide_mvvtype_ret (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : mvvfunct6) :
    ∃ r, (interpretSailEffects (valid_wide_mvvtype c)).run s = .ok r s := by
  cases c <;> simp [valid_wide_mvvtype, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, num_of_vector_support, vector_support_ge, vector_support_level, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports]

theorem valid_wide_mvxtype_ret (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : mvxfunct6) :
    ∃ r, (interpretSailEffects (valid_wide_mvxtype c)).run s = .ok r s := by
  cases c <;> simp [valid_wide_mvxtype, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, num_of_vector_support, vector_support_ge, vector_support_level, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports]

theorem zvknhab_check_encdec_ret (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v)
    (l : RegisterType vl) (hl : s.regs.get? vl = some l) (st : RegisterType vstart) (hst : s.regs.get? vstart = some st)
    (a b c : vregidx) : ∃ r, (interpretSailEffects (zvknhab_check_encdec a b c)).run s = .ok r s := by
  simp [zvknhab_check_encdec, zvk_check_encdec, get_lmul_pow_run _ _ hv hok, get_sew_run _ _ hv hok,
    readReg_run _ _ _ hl, readReg_run _ _ _ hst, vstart_reserved_behavior]


/-- A bind whose parts return, the state as it was, returns. -/
theorem bind_ret {α β : Type} (m : SailM α) (f : α → SailM β) (s : RVState)
    (hm : ∃ a, (interpretSailEffects m).run s = .ok a s) (hf : ∀ a, ∃ b, (interpretSailEffects (f a)).run s = .ok b s) :
    ∃ b, (interpretSailEffects (m >>= f)).run s = .ok b s := by
  obtain ⟨a, ha⟩ := hm
  obtain ⟨b, hb⟩ := hf a
  exact ⟨b, by rw [interp_bind, run_bind', ha]; exact hb⟩

/-- A block whose value does not depend on what its first part returns. -/
theorem bind_pure_const {α β : Type} (m : SailM α) (h : α → β) (c : β) (s : RVState)
    (hm : ∃ a, (interpretSailEffects m).run s = .ok a s) (hc : ∀ a, h a = c) :
    (interpretSailEffects (m >>= fun a => pure (h a))).run s = .ok c s := by
  obtain ⟨a, ha⟩ := hm
  rw [interp_bind, run_bind', ha]; simp only [hc]; rfl

/-- Off, whatever the conversion: no vector float. -/
theorem valid_widening_fp_conversion_false (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : vfwunary0) :
    (interpretSailEffects (valid_widening_fp_conversion c)).run s = .ok false s := by
  cases c <;> simp [valid_widening_fp_conversion, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports,
    num_of_vector_support, vector_support_ge, vector_support_level]

/-- Off, whatever the conversion: no vector float. -/
theorem valid_narrowing_fp_conversion_false (s : RVState) (hp : s.regs.get? cur_privilege = some .Machine)
    (ms : RegisterType mstatus) (hms : s.regs.get? mstatus = some ms)
    (mv : RegisterType misa) (hmv : s.regs.get? misa = some mv)
    (v : RegisterType vtype) (hv : s.regs.get? vtype = some v) (hok : VtypeOk v) (c : vfnunary0) :
    (interpretSailEffects (valid_narrowing_fp_conversion c)).run s = .ok false s := by
  cases c <;> simp [valid_narrowing_fp_conversion, ce_zve32x _ hp _ hms _ hmv, ce_zve32f _ hp _ hms _ hmv, ce_zve64x _ hp _ hms _ hmv, ce_zve64f _ hp _ hms _ hmv, ce_zve64d _ hp _ hms _ hmv, ce_v _ hp _ hms _ hmv, ce_zvfh _ hp _ hms _ hmv, ce_zvfhmin _ hp _ hms _ hmv, ce_zvbb _ hp _ hms _ hmv, ce_zvkb _ hp _ hms _ hmv, ce_zvbc _ hp _ hms _ hmv, ce_zvkg _ hp _ hms _ hmv, ce_zvkned _ hp _ hms _ hmv, ce_zvknha _ hp _ hms _ hmv, ce_zvknhb _ hp _ hms _ hmv, ce_zvksed _ hp _ hms _ hmv, ce_zvksh _ hp _ hms _ hmv, ce_zvabd _ hp _ hms _ hmv, ce_zvfbfmin _ hp _ hms _ hmv, ce_zvfbfwma _ hp _ hms _ hmv, ce_zvl128b _ hp _ hms _ hmv, get_sew_run _ _ hv hok, readReg_run _ _ _ hmv, hartSupports,
    num_of_vector_support, vector_support_ge, vector_support_level]

/-- An extension check, off, and-ed with anything that returns: off. -/
theorem ce_and_block (s : RVState) (e : extension) (B : SailM Bool)
    (hce : (interpretSailEffects (currentlyEnabled e)).run s = .ok false s)
    (hB : ∃ y, (interpretSailEffects B).run s = .ok y s) :
    (interpretSailEffects (currentlyEnabled e >>= fun x => B >>= fun y => pure (x && y))).run s = .ok false s := by
  obtain ⟨y, hy⟩ := hB
  rw [interp_bind, run_bind', hce]
  simp only [interp_bind, run_bind', hy]
  rfl

/-- vtype's fields, read. -/
macro_rules | `(tactic| known) => `(tactic| first
  | rw [get_sew_run _ _ (by assumption) (by assumption)]
  | rw [get_lmul_pow_run _ _ (by assumption) (by assumption)]
  | rw [valid_widening_fp_conversion_false _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption)]
  | rw [valid_narrowing_fp_conversion_false _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption)]
  | (rw [bind_pure_const]
     rotate_left
     · returns
     · exact fun _ => rfl)
  | (rw [interp_bind, run_bind']
     known
     try dsimp only
     known))

/-- The vector checks' values, and the vector fields' decoders. -/
macro_rules | `(tactic| returns) => `(tactic| first
  | exact ⟨_, get_sew_run _ _ (by assumption) (by assumption)⟩
  | exact ⟨_, get_lmul_pow_run _ _ (by assumption) (by assumption)⟩
  | exact ⟨_, ce_zve32x _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zve32f _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zve64x _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zve64f _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zve64d _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_v _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvfh _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvfhmin _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvbb _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvkb _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvbc _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvkg _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvkned _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvknha _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvknhb _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvksed _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvksh _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvabd _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvfbfmin _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvfbfwma _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvl32b _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvl64b _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | exact ⟨_, ce_zvl128b _ (by assumption) _ (by assumption) _ (by assumption)⟩
  | (apply bind_ret
     · returns
     · intro
       returns)
  | exact zvk_check_encdec_ret _ _ (by assumption) (by assumption) _ (by assumption) _ (by assumption) _ _
  | exact zvknhab_check_encdec_ret _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption) _ (by assumption) _ (by assumption) _ _ _
  | exact valid_widening_fp_conversion_ret _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption) _
  | exact valid_narrowing_fp_conversion_ret _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption) _
  | exact valid_wide_vvtype_ret _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption) _
  | exact valid_wide_vxtype_ret _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption) _
  | exact valid_wide_mvvtype_ret _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption) _
  | exact valid_wide_mvxtype_ret _ (by assumption) _ (by assumption) _ (by assumption) _ (by assumption) (by assumption) _
  | (simp only [encdec_vvfunct6_backwards, encdec_zvabd_vabd_func6_backwards, encdec_zvabd_vwabda_func6_backwards, encdec_nvsfunct6_backwards, encdec_nvfunct6_backwards, encdec_vxfunct6_backwards, encdec_nxsfunct6_backwards, encdec_nxfunct6_backwards, encdec_vxsgfunct6_backwards, encdec_vifunct6_backwards, encdec_nisfunct6_backwards, encdec_nifunct6_backwards, encdec_visgfunct6_backwards, encdec_nreg_forwards, encdec_mvvfunct6_backwards, encdec_mvvmafunct6_backwards, encdec_wvvfunct6_backwards, encdec_wvfunct6_backwards, encdec_wmvvfunct6_backwards, vext_vs1_backwards, encdec_mvxfunct6_backwards, encdec_mvxmafunct6_backwards, encdec_wvxfunct6_backwards, encdec_wxfunct6_backwards, encdec_wmvxfunct6_backwards, encdec_fvvfunct6_backwards, encdec_fvvmafunct6_backwards, encdec_fwvvfunct6_backwards, encdec_fwvvmafunct6_backwards, encdec_fwvfunct6_backwards, encdec_vfunary0_vs1_backwards, encdec_vfwunary0_vs1_backwards, encdec_vfnunary0_vs1_backwards, encdec_vfunary1_vs1_backwards, encdec_fvffunct6_backwards, encdec_fvfmafunct6_backwards, encdec_fwvffunct6_backwards, encdec_fwvfmafunct6_backwards, encdec_fwffunct6_backwards, encdec_mmfunct6_backwards, encdec_vvmfunct6_backwards, encdec_vvmcfunct6_backwards, encdec_vvmsfunct6_backwards, encdec_vvcmpfunct6_backwards, encdec_vxmfunct6_backwards, encdec_vxmcfunct6_backwards, encdec_vxmsfunct6_backwards, encdec_vxcmpfunct6_backwards, encdec_vimfunct6_backwards, encdec_vimcfunct6_backwards, encdec_vimsfunct6_backwards, encdec_vicmpfunct6_backwards, encdec_fvvmfunct6_backwards, encdec_fvfmfunct6_backwards, encdec_rivvfunct6_backwards, encdec_rmvvfunct6_backwards, encdec_rfvvfunct6_backwards, encdec_rfwvvfunct6_backwards, encdec_vaesdf_backwards, encdec_vaesdm_backwards, encdec_vaesef_backwards, encdec_vaesem_backwards, encdec_vsha2_backwards]; split <;>
      first | exact ⟨_, rfl⟩ | (exfalso; simp_all [encdec_vvfunct6_backwards_matches, encdec_zvabd_vabd_func6_backwards_matches, encdec_zvabd_vwabda_func6_backwards_matches, encdec_nvsfunct6_backwards_matches, encdec_nvfunct6_backwards_matches, encdec_vxfunct6_backwards_matches, encdec_nxsfunct6_backwards_matches, encdec_nxfunct6_backwards_matches, encdec_vxsgfunct6_backwards_matches, encdec_vifunct6_backwards_matches, encdec_nisfunct6_backwards_matches, encdec_nifunct6_backwards_matches, encdec_visgfunct6_backwards_matches, encdec_nreg_forwards_matches, encdec_mvvfunct6_backwards_matches, encdec_mvvmafunct6_backwards_matches, encdec_wvvfunct6_backwards_matches, encdec_wvfunct6_backwards_matches, encdec_wmvvfunct6_backwards_matches, vext_vs1_backwards_matches, encdec_mvxfunct6_backwards_matches, encdec_mvxmafunct6_backwards_matches, encdec_wvxfunct6_backwards_matches, encdec_wxfunct6_backwards_matches, encdec_wmvxfunct6_backwards_matches, encdec_fvvfunct6_backwards_matches, encdec_fvvmafunct6_backwards_matches, encdec_fwvvfunct6_backwards_matches, encdec_fwvvmafunct6_backwards_matches, encdec_fwvfunct6_backwards_matches, encdec_vfunary0_vs1_backwards_matches, encdec_vfwunary0_vs1_backwards_matches, encdec_vfnunary0_vs1_backwards_matches, encdec_vfunary1_vs1_backwards_matches, encdec_fvffunct6_backwards_matches, encdec_fvfmafunct6_backwards_matches, encdec_fwvffunct6_backwards_matches, encdec_fwvfmafunct6_backwards_matches, encdec_fwffunct6_backwards_matches, encdec_mmfunct6_backwards_matches, encdec_vvmfunct6_backwards_matches, encdec_vvmcfunct6_backwards_matches, encdec_vvmsfunct6_backwards_matches, encdec_vvcmpfunct6_backwards_matches, encdec_vxmfunct6_backwards_matches, encdec_vxmcfunct6_backwards_matches, encdec_vxmsfunct6_backwards_matches, encdec_vxcmpfunct6_backwards_matches, encdec_vimfunct6_backwards_matches, encdec_vimcfunct6_backwards_matches, encdec_vimsfunct6_backwards_matches, encdec_vicmpfunct6_backwards_matches, encdec_fvvmfunct6_backwards_matches, encdec_fvfmfunct6_backwards_matches, encdec_rivvfunct6_backwards_matches, encdec_rmvvfunct6_backwards_matches, encdec_rfvvfunct6_backwards_matches, encdec_rfwvvfunct6_backwards_matches, encdec_vaesdf_backwards_matches, encdec_vaesdm_backwards_matches, encdec_vaesef_backwards_matches, encdec_vaesem_backwards_matches, encdec_vsha2_backwards_matches])))

/-- A block an extension check, off, decides. -/
macro_rules | `(tactic| known) => `(tactic| (rw [ce_and_block]; rotate_left; (· known); (· returns)))

end Equiv.Riscv
