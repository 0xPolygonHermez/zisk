/- sail-zisk's ctz, Lean's clz of the reverse. -/
import Equiv.Count

namespace Equiv

open LeanZiskRV LeanZiskRV.Functions Sail

set_option maxRecDepth 200000 in
set_option maxHeartbeats 0 in
theorem zctz64 (x : BitVec 64) :
    _root_.Zisk.Functions.to_bits (l := 64) (_root_.Zisk.Functions.ctz_bits x) = x.reverse.clz := by
  simp only [_root_.Zisk.Functions.ctz_bits, Sail.BitVec.length, forIn, ForIn'.forIn', IntRange.forIn', id_bind,
    id_pure, Id.run]
  count_loop
  bv_decide

end Equiv
