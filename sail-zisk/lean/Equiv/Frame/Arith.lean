/- `Frame` of sail-zisk's Arith operations (Equiv/ZFrame.lean), a module so that they check in parallel. -/
import Equiv.ZFrame

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Frame.exec_add256 (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_ADD256 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_arith256 (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_ARITH256 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_arith256_mod (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_ARITH256_MOD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_arith384_mod (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_ARITH384_MOD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

end Equiv.Zisk
