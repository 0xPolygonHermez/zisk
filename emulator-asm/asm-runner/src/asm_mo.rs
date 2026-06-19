use std::fmt::Debug;

use crate::AsmShmemHeader;

#[repr(C)]
#[derive(Debug)]
pub(crate) struct AsmMOHeader {
    pub version: u64,
    pub exit_code: u64,
    pub shmem_allocated_size: u64,
    pub shmem_used_size: u64,
    pub num_chunks: u64,
}

impl AsmShmemHeader for AsmMOHeader {
    fn allocated_size(&self) -> u64 {
        self.shmem_allocated_size
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct AsmMOChunk {
    pub end: u64,
    pub mem_ops_size: u64,
}

/// Type of buffer used on GPU MO count and plan.
#[derive(Clone, Copy, Debug, Default)]
pub enum GpuBufferSource {
    /// Execution is finally delegated to the CPU path.
    #[default]
    Cpu,
    /// Uses the provided pil2-proofman prover buffer.
    Borrowed {
        /// Raw pointer to the buffer.
        ptr: usize,
        /// Buffer size in bytes.
        size: usize,
    },
    /// The buffer is allocated and owned by the runner.
    SelfAllocated,
}
