use crate::{EmuTrace, EmuTraceStart};
use libc::shm_unlink;
use std::ffi::{c_void, CString};
use std::fmt::Debug;

#[derive(Debug)]
pub struct AsmMinimalTraces {
    shmem_output_name: String,
    mapped_ptr: *mut c_void,
    pub vec_chunks: Vec<EmuTrace>,
}

unsafe impl Send for AsmMinimalTraces {}
unsafe impl Sync for AsmMinimalTraces {}

impl Drop for AsmMinimalTraces {
    fn drop(&mut self) {
        unsafe {
            // Forget all mem_reads Vec<u64> before unmapping
            for chunk in &mut self.vec_chunks {
                std::mem::forget(std::mem::take(&mut chunk.mem_reads));
            }

            // Unmap shared memory
            libc::munmap(self.mapped_ptr, self.total_size());

            let shmem_output_name =
                CString::new(self.shmem_output_name.clone()).expect("CString::new failed");
            let shmem_output_name_ptr = shmem_output_name.as_ptr();

            shm_unlink(shmem_output_name_ptr);
        }
    }
}

impl AsmMinimalTraces {
    pub fn new(
        shmem_output_name: String,
        mapped_ptr: *mut c_void,
        vec_chunks: Vec<EmuTrace>,
    ) -> Self {
        AsmMinimalTraces { shmem_output_name, mapped_ptr, vec_chunks }
    }

    fn total_size(&self) -> usize {
        self.vec_chunks.iter().map(|chunk| std::mem::size_of_val(&chunk.mem_reads)).sum::<usize>()
            + std::mem::size_of::<AsmOutputHeader>()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AsmOutputHeader {
    pub version: u64,
    pub exit_code: u64,
    pub mt_allocated_size: u64,
    pub mt_used_size: u64,
}

impl AsmOutputHeader {
    pub fn from_ptr(mapped_ptr: *mut c_void) -> AsmOutputHeader {
        let output_header;
        unsafe {
            output_header = std::ptr::read(mapped_ptr as *const AsmOutputHeader);
        }

        assert!(output_header.mt_allocated_size > 0);
        assert!(output_header.mt_used_size > 0);

        output_header
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AsmOutputChunkC {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
    pub registers: [u64; 33],
    pub last_c: u64,
    pub end: u64,
    pub steps: u64,
    pub mem_reads_size: u64,
}

impl AsmOutputChunkC {
    /// Create an `OutputChunk` from a pointer.
    ///
    /// # Safety
    /// This function is unsafe because it reads from a raw pointer in shared memory.
    pub unsafe fn to_emu_trace(mapped_ptr: &mut *mut c_void) -> EmuTrace {
        // Read chunk data
        let chunk = unsafe { std::ptr::read(*mapped_ptr as *const AsmOutputChunkC) };
        *mapped_ptr = unsafe {
            (*mapped_ptr as *mut u8).add(std::mem::size_of::<AsmOutputChunkC>()) as *mut c_void
        };

        // Convert mem_reads into a Vec<u64> without copying
        let mem_reads_ptr = *mapped_ptr as *mut u64;
        let mem_reads_len = chunk.mem_reads_size as usize;
        let mem_reads = unsafe { Vec::from_raw_parts(mem_reads_ptr, mem_reads_len, mem_reads_len) };

        // Advance the pointer after reading memory reads
        *mapped_ptr = unsafe { (*mapped_ptr as *mut u64).add(mem_reads_len) as *mut c_void };

        let mut registers = [0u64; 32];
        registers[1..].copy_from_slice(&chunk.registers[..31]);

        // Return the parsed OutputChunk
        EmuTrace {
            start_state: EmuTraceStart {
                pc: chunk.pc,
                sp: chunk.sp,
                c: chunk.c,
                step: chunk.step,
                regs: registers,
            },
            last_c: chunk.last_c,
            end: chunk.end == 1,
            steps: chunk.steps,
            mem_reads,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AsmInputC {
    pub chunk_size: u64,
    pub max_steps: u64,
    pub initial_trace_size: u64,
    pub input_data_size: u64,
}

impl AsmInputC {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&self.chunk_size.to_le_bytes());
        bytes.extend_from_slice(&self.max_steps.to_le_bytes());
        bytes.extend_from_slice(&self.initial_trace_size.to_le_bytes());
        bytes.extend_from_slice(&self.input_data_size.to_le_bytes());
        bytes
    }
}
