/- `Frame` of sail-zisk's Dma operations (Equiv/ZFrame.lean), a module so that they check in parallel. -/
import Equiv.ZFrame

set_option maxRecDepth 1000000

namespace Equiv.Zisk

open _root_.Zisk

set_option maxHeartbeats 0 in
theorem Frame.exec_dma_memcpy (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_DMA_MEMCPY ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_dma_xmemcpy (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_DMA_XMEMCPY ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_dma_memcmp (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_DMA_MEMCMP ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_dma_xmemcmp (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_DMA_XMEMCMP ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_dma_xmemset (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_DMA_XMEMSET ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

set_option maxHeartbeats 0 in
theorem Frame.exec_jump_dest (a b ext : BitVec 64) : Frame (Functions.zisk_exec_mem (.ZOP_JUMP_DEST ()) a b ext) := by
  simp only [Functions.zisk_exec_mem]
  frame

end Equiv.Zisk
