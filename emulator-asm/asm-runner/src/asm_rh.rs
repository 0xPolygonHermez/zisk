use std::fmt::Debug;

use crate::{AsmSharedMemory, AsmShmemHeader};

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHHeader {
    pub version: u64,
    pub exit_code: u64,
    pub shmem_allocated_size: u64,
    pub steps: u64,
}

impl AsmShmemHeader for AsmRHHeader {
    fn allocated_size(&self) -> u64 {
        self.shmem_allocated_size
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct AsmRHData {
    pub steps: u64,
    pub inst_count: Vec<u64>,
}

impl AsmRHData {
    pub fn new(steps: u64, inst_count: Vec<u64>) -> Self {
        AsmRHData { steps, inst_count }
    }
}

impl AsmRHData {
    /// Create an `OutputChunk` from a pointer.
    ///
    /// # Safety
    /// This function is unsafe because it reads from a raw pointer in shared memory.
    pub fn from_shared_memory(asm_shared_memory: &AsmSharedMemory<AsmRHHeader>) -> AsmRHData {
        unsafe {
            let data_ptr = asm_shared_memory.data_ptr() as *mut u64;
            // chunk data
            let len = std::ptr::read(data_ptr) as usize;
            let data_ptr = data_ptr.add(1);
            let inst_count = Vec::from_raw_parts(data_ptr, len, len);

            AsmRHData { steps: asm_shared_memory.map_header().steps, inst_count }
        }
    }
}
