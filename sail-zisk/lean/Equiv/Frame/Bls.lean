/- `Frame` of sail-zisk's Bls operations (Equiv/ZFrame.lean), a module so that they check in parallel. -/
import Equiv.ZFrame

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Frame.exec_bls12_381_curve_add (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLS12_381_CURVE_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_bls12_381_curve_dbl (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLS12_381_CURVE_DBL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_bls12_381_complex_add (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLS12_381_COMPLEX_ADD ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_bls12_381_complex_sub (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLS12_381_COMPLEX_SUB ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_bls12_381_complex_mul (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_BLS12_381_COMPLEX_MUL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

end Equiv.Zisk
