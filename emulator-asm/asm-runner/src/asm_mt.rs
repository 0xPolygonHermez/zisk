use std::fmt::Debug;
use zisk_common::EmuTrace;
use zisk_common::EmuTraceStart;
use zisk_core::{REGS_IN_MAIN_FROM, REGS_IN_MAIN_TO, REGS_IN_MAIN_TOTAL_NUMBER};

use crate::AsmShmemHeader;

#[repr(C)]
#[derive(Debug)]
pub struct AsmMTHeader {
    pub version: u64,
    pub exit_code: u64,
    pub shmem_allocated_size: u64,
    pub shmem_used_size: u64,
    pub num_chunks: u64,
}

impl AsmShmemHeader for AsmMTHeader {
    fn allocated_size(&self) -> u64 {
        self.shmem_allocated_size
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct AsmMTChunk {
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

impl AsmMTChunk {
    /// Create an `OutputChunk` from a pointer.
    ///
    /// # Safety
    /// This function is unsafe because it reads from a raw pointer in shared memory.
    pub fn to_emu_trace(mapped_ptr: &mut *const AsmMTChunk) -> EmuTrace {
        // Read chunk data
        let chunk = unsafe { std::ptr::read(*mapped_ptr) };
        *mapped_ptr = unsafe { mapped_ptr.add(1) };

        // Convert mem_reads into a Vec<u64> without copying
        let mem_reads_ptr = *mapped_ptr as *mut u64;
        let mem_reads_len = chunk.mem_reads_size as usize;
        let mem_reads =
            unsafe { std::slice::from_raw_parts(mem_reads_ptr, mem_reads_len).to_vec() };

        // Advance the pointer after reading memory reads
        *mapped_ptr = unsafe { (*mapped_ptr as *mut u64).add(mem_reads_len) as *const AsmMTChunk };

        let mut registers = [0u64; REGS_IN_MAIN_TOTAL_NUMBER];
        registers[REGS_IN_MAIN_FROM..].copy_from_slice(&chunk.registers[..REGS_IN_MAIN_TO]);

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

#[repr(C)]
#[derive(Debug)]
pub struct AsmInputC2 {
    pub zero: u64, // Not used
    pub input_data_size: u64,
}

impl AsmInputC2 {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&self.input_data_size.to_le_bytes());
        bytes
    }
}
