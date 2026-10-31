/- `Frame` of sail-zisk's Hash operations (Equiv/ZFrame.lean), a module so that they check in parallel. -/
import Equiv.ZFrame

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Frame.exec_keccak (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_KECCAK ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_blake2b (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLAKE2B ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_blake2s (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLAKE2S ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_blake3 (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLAKE3 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_poseidon2 (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_POSEIDON2 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_poseidon1 (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_POSEIDON1 ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_poseidon2_koalabear (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_POSEIDON2_KOALABEAR ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

end Equiv.Zisk
