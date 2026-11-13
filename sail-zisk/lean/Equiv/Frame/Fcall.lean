/- `Frame` of sail-zisk's Fcall operations (Equiv/ZFrame.lean), a module so that they check in parallel. -/
import Equiv.ZFrame

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Frame.exec_fcall_param (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_FCALL_PARAM ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_fcall (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_FCALL ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_fcall_get (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_FCALL_GET ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_dma_inputcpy (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_DMA_INPUTCPY ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

end Equiv.Zisk
