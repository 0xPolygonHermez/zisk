use std::{
    ffi::c_void,
    fmt::{Debug, Formatter},
};

#[repr(C)]
#[derive(Debug)]
pub struct OutputHeaderC {
    pub version: u64,
    pub exit_code: u64,
    pub mt_allocated_size: u64,
    pub mt_used_size: u64,
}

impl From<[u64; 5]> for OutputHeaderC {
    fn from(arr: [u64; 5]) -> Self {
        Self { version: arr[0], exit_code: arr[1], mt_allocated_size: arr[2], mt_used_size: arr[3] }
    }
}

impl OutputHeaderC {
    pub fn map_output_header(mapped_ptr: *mut c_void) -> OutputHeaderC {
        let output_header;
        unsafe {
            output_header = std::ptr::read(mapped_ptr as *const OutputHeaderC);
        }

        assert!(output_header.mt_allocated_size > 0);
        assert!(output_header.mt_used_size > 0);

        output_header
    }

    // pub fn map_output(mut mapped_ptr: *mut c_void) {
    //     let num_chunks = unsafe { std::ptr::read(mapped_ptr as *const u64) };
    //     mapped_ptr = unsafe { mapped_ptr.add(8) };

    //     for chunk_id in 0..num_chunks {
    //         unsafe {
    //             let chunk = std::ptr::read(mapped_ptr as *const OutputChunkC);
    //             mapped_ptr =
    //                 (mapped_ptr as *mut u8).add(std::mem::size_of::<OutputChunkC>()) as *mut c_void;

    //             // Read `mem_reads_size` as u64 slice
    //             let mem_reads_ptr = mapped_ptr as *const u64;
    //             let mem_reads_len = chunk.mem_reads_size as usize;

    //             // Create a Vec<u64> referencing this memory
    //             let mem_reads_vec =
    //                 Vec::from_raw_parts(mem_reads_ptr as *mut u64, mem_reads_len, mem_reads_len);

    //             println!("Chunk {}: {:#x?} {}", chunk_id, chunk, chunk.mem_reads_size);

    //             if chunk_id == 1 {
    //                 println!(
    //                     "Memory Reads: {:#x?}  {:#x?}",
    //                     mem_reads_vec[0],
    //                     mem_reads_vec.last().unwrap()
    //                 );
    //             }
    //             // println!("Memory Reads: {:#x?}", mem_reads_vec);

    //             // Advance the pointer to the next chunk
    //             mapped_ptr = (mapped_ptr as *mut u64).add(mem_reads_len) as *mut c_void;

    //             // Prevent Vec from freeing memory (we don't own it)
    //             std::mem::forget(mem_reads_vec);
    //         }
    //     }
    // }

    pub unsafe fn map_output2<'a>(mapped_ptr: &mut *mut c_void) -> OutputChunk<'a> {
        let chunk = std::ptr::read(*mapped_ptr as *const OutputChunkC);
        *mapped_ptr =
            (*mapped_ptr as *mut u8).add(std::mem::size_of::<OutputChunkC>()) as *mut c_void;

        // Create a slice over the memory without copying
        let mem_reads_ptr = *mapped_ptr as *const u64;
        let mem_reads_len = chunk.mem_reads_size as usize;
        let mem_reads_slice = std::slice::from_raw_parts(mem_reads_ptr, mem_reads_len);

        // Advance the pointer
        *mapped_ptr = (*mapped_ptr as *mut u64).add(mem_reads_len) as *mut c_void;

        let mut output_chunk = OutputChunk::from(chunk);
        output_chunk.mem_reads = mem_reads_slice;

        output_chunk
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct OutputChunkC {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
    pub registers: [u64; 33],
    pub last_pc: u64,
    pub last_sp: u64,
    pub last_c: u64,
    pub end: u64,
    pub steps: u64,
    pub mem_reads_size: u64,
}

#[repr(C)]
pub struct OutputChunk<'a> {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,

    pub registers: [u64; 33],

    pub last_pc: u64,
    pub last_sp: u64,
    pub last_c: u64,
    pub end: u64,
    pub steps: u64,
    pub mem_reads: &'a [u64],
}

impl From<OutputChunkC> for OutputChunk<'_> {
    fn from(chunk: OutputChunkC) -> Self {
        Self {
            pc: chunk.pc,
            sp: chunk.sp,
            c: chunk.c,
            step: chunk.step,
            registers: chunk.registers,
            last_pc: chunk.last_pc,
            last_sp: chunk.last_sp,
            last_c: chunk.last_c,
            end: chunk.end,
            steps: chunk.steps,
            mem_reads: &[],
        }
    }
}

impl Debug for OutputChunk<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OutputChunk")
            .field("pc", &format_args!("{:#x}", self.pc))
            .field("sp", &format_args!("{:#x}", self.sp))
            .field("c", &format_args!("{:#x}", self.c))
            .field("step", &self.step)
            .field("registers", &self.registers)
            .field("last_pc", &format_args!("{:#x}", self.last_pc))
            .field("last_sp", &format_args!("{:#x}", self.last_sp))
            .field("last_c", &format_args!("{:#x}", self.last_c))
            .field("end", &format_args!("{:#x}", self.end))
            .field("steps", &format_args!("{:}", self.steps))
            .field("mem reads size", &format_args!("{:}", self.mem_reads.len()))
            .finish()
    }
}
