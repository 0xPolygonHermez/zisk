use std::ffi::c_void;
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

impl AsmMOHeader {
    pub fn from_ptr(mapped_ptr: *mut c_void) -> AsmMOHeader {
        let output_header;
        unsafe {
            output_header = std::ptr::read(mapped_ptr as *const AsmMOHeader);
        }

        assert!(output_header.shmem_allocated_size > 0);
        assert!(output_header.shmem_used_size > 0);

        output_header
    }
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

#[repr(C)]
#[derive(Debug)]
pub struct MemOpsTrace {
    pub mem_ops: Vec<u64>,
}

impl AsmMOChunk {
    pub fn to_mem_ops(mapped_ptr: &mut *const AsmMOChunk) -> MemOpsTrace {
        // Read chunk data
        let chunk = unsafe { std::ptr::read(*mapped_ptr) };
        *mapped_ptr = unsafe { mapped_ptr.add(1) };

        // Convert mem_reads into a Vec<u64> without copying
        let mem_ops_ptr = *mapped_ptr as *mut u64;
        let mem_ops_len = chunk.mem_ops_size as usize;
        let mem_ops = unsafe { Vec::from_raw_parts(mem_ops_ptr, mem_ops_len, mem_ops_len) };

        // Advance the pointer after reading memory reads
        *mapped_ptr = unsafe { (*mapped_ptr as *mut u64).add(mem_ops_len) as *const AsmMOChunk };

        // Return the parsed OutputChunk
        MemOpsTrace { mem_ops }
    }
}
