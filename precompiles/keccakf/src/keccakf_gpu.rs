//! FFI to the CUDA Keccakf witness kernel (`cu/keccakf_witness.cu`).
//!
//! The kernel's entry points are generated C-side by `ZISK_GPU_WITNESS_ENTRIES`
//! and declared here by [`zisk_gpu_witness::declare_kernel!`], so all this file
//! carries is what is genuinely Keccakf's: the operation layout.

use super::KeccakfInput;

/// One operation as the kernel consumes it. Mirrors `KeccakfGpuOp` in the `.cu`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GpuOp {
    pub state: [u64; 25],
    pub step: u64,
    pub addr: u64,
}

// SAFETY: repr(C), 27 u64 fields mirroring KeccakfGpuOp field for field: no padding, no pointers.
unsafe impl ::proofman_common::GpuWitnessOp for GpuOp {}

impl From<&KeccakfInput> for GpuOp {
    fn from(input: &KeccakfInput) -> Self {
        Self { state: input.state, step: input.step_main, addr: input.addr_main as u64 }
    }
}

zisk_gpu_witness::declare_kernel! {
    /// Keccakf packed-witness kernel: one permutation per warp lane.
    pub KeccakfKernel, prefix = keccakf, op = GpuOp,
}
