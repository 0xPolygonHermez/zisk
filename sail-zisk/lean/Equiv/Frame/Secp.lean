/- `Frame` of sail-zisk's Secp operations (Equiv/ZFrame.lean), a module so that they check in parallel. -/
import Equiv.ZFrame

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Frame.exec_secp256k1_add (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_SECP256K1_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_secp256k1_dbl (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_SECP256K1_DBL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_secp256r1_add (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_SECP256R1_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_secp256r1_dbl (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_SECP256R1_DBL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

end Equiv.Zisk
