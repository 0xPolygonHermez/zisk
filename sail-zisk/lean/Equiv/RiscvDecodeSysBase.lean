/-
  sail-riscv's decoder on SYSTEM's funct3 4, the extensions there off: H
  (its loads and stores), Zimop, Zicfiss. `known` learns H's check.
-/
import Equiv.RiscvDecodeVecBase

namespace Equiv.Riscv
open LeanZiskRV LeanZiskRV.Functions Sail Sail.ArchSem
open Register

/-- H is off: the profile has no hypervisor. Its check reads misa, and S's
    and the page tables' checks, before it finds that. -/
theorem ce_h (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (currentlyEnabled .Ext_H)).run s = .ok false s := by
  simp [currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

macro_rules | `(tactic| known) => `(tactic| rw [ce_h _ _ (by assumption)])

/-- Zicfiss's shadow stack in machine mode: off, after S's check. -/
theorem xsse_machine (s : RVState) (v : RegisterType misa) (hv : s.regs.get? misa = some v) :
    (interpretSailEffects (zicfiss_xSSE .Machine)).run s = .ok false s := by
  simp [zicfiss_xSSE, currentlyEnabled, hartSupports, readReg_run _ _ _ hv]

macro_rules | `(tactic| known) => `(tactic| first
  | rw [readReg_run _ cur_privilege _ (by assumption)]
  | rw [xsse_machine _ _ (by assumption)])

/-- The CSR ops RISC-V has, as bits: funct3's low two bits, 0 none. -/
theorem csrop_matches_eq (b : BitVec 2) : encdec_csrop_backwards_matches b = (b != 0#2) := by
  revert b; decide

macro_rules | `(tactic| funct_false) => `(tactic| (
  intro h
  simp only [csrop_matches_eq, Sail.BitVec.extractLsb, Bool.and_eq_true, Bool.or_eq_true,
    Bool.not_eq_true', beq_iff_eq, bne_iff_ne, ne_eq] at *
  bv_decide))

end Equiv.Riscv
