/- `Frame` of sail-zisk's Edwards operations (Equiv/ZFrame.lean), a module so that they check in parallel. -/
import Equiv.ZFrame

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Frame.exec_ed25519_add (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_ED25519_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_babyjubjub_add (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BABYJUBJUB_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_sha256 (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_SHA256 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

end Equiv.Zisk
