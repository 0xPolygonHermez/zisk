use std::fmt::Debug;

use crate::AsmShmemHeader;

#[repr(C)]
#[derive(Debug)]
pub struct AsmMOHeader {
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
pub struct AsmMOChunk {
    pub end: u64,
    pub mem_ops_size: u64,
}
