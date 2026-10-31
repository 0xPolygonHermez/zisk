/- vtype's fields, as sail-riscv's decoder needs them on a vector word. -/
import LeanZiskRV

namespace Equiv.Riscv

open LeanZiskRV LeanZiskRV.Functions Sail
open Register

/-- vtype's SEW and LMUL fields, neither reserved. -/
def VtypeOk (v : RegisterType vtype) : Prop :=
  (_get_Vtype_vsew v).toNat < 4 ∧ (_get_Vtype_vlmul v).toInt > -4

end Equiv.Riscv
