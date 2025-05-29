use libc::{close, shm_unlink, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR, S_IXUSR};

use mem_planner_cpp::MemPlanner;
use named_sem::NamedSemaphore;
use zisk_common::ChunkId;

use std::ffi::c_void;
use std::fmt::Debug;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, ptr};

use tracing::error;

use crate::{
    shmem_utils, AsmInputC2, AsmMOChunk, AsmMOHeader, AsmRunError, AsmServices, MemOpsTrace,
};

use anyhow::{Context, Result};

// This struct is used to run the assembly code in a separate process and generate minimal traces.
#[derive(Debug)]
pub struct AsmRunnerMO {
    shmem_output_name: String,
    mapped_ptr: *mut c_void,
    pub vec_chunks: Vec<MemOpsTrace>,
}

unsafe impl Send for AsmRunnerMO {}
unsafe impl Sync for AsmRunnerMO {}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for AsmRunnerMO {
    fn drop(&mut self) {
        for chunk in &mut self.vec_chunks {
            std::mem::forget(std::mem::take(&mut chunk.mem_ops));
        }
        let total_size = self.total_size();
        unsafe {
            shmem_utils::unmap(self.mapped_ptr, total_size);
        }
        let c_name =
            std::ffi::CString::new(self.shmem_output_name.clone()).expect("CString::new failed");
        unsafe {
            if shm_unlink(c_name.as_ptr()) != 0 {
                error!("shm_unlink failed: {:?}", std::io::Error::last_os_error());
            }
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmRunnerMO {
    pub fn new(
        shmem_output_name: String,
        mapped_ptr: *mut std::ffi::c_void,
        vec_chunks: Vec<MemOpsTrace>,
    ) -> Self {
        Self { shmem_output_name, mapped_ptr, vec_chunks }
    }

    pub fn total_size(&self) -> usize {
        self.vec_chunks.iter().map(|chunk| chunk.mem_ops.len() * size_of::<u64>()).sum::<usize>()
            + size_of::<AsmMOHeader>()
    }

    pub fn run(inputs_path: &Path, max_steps: u64, chunk_size: u64) -> Result<AsmRunnerMO> {
        const SHMEM_INPUT_NAME: &str = "ZISKMO_input";
        const SHMEM_OUTPUT_NAME: &str = "ZISKMO_output";
        const SEM_CHUNK_DONE_NAME: &str = "/ZISKMO_chunk_done";
        const MEM_READS_SIZE_DUMMY: u64 = 0xFFFFFFFFFFFFFFFF;

        let mut sem_chunk_done = NamedSemaphore::create(SEM_CHUNK_DONE_NAME, 0)
            .map_err(|e| AsmRunError::SemaphoreError(SEM_CHUNK_DONE_NAME, e))?;

        Self::write_input(inputs_path, SHMEM_INPUT_NAME);

        let should_exit = Arc::new(AtomicBool::new(false));
        let should_exit_cloned = Arc::clone(&should_exit);
        let handle = std::thread::spawn(move || {
            let mut sem_chunk_done = NamedSemaphore::create(SEM_CHUNK_DONE_NAME, 0)
                .expect("Failed to create named semaphore");
            let response = AsmServices::send_memory_ops_request(max_steps, chunk_size);

            should_exit_cloned.store(true, std::sync::atomic::Ordering::SeqCst);
            sem_chunk_done.post().expect("Failed to post semaphore sem_chunk_done");
            response
        });

        let mut chunk_id = ChunkId(0);

        // Read the header data
        let header_ptr = Self::get_output_ptr(SHMEM_OUTPUT_NAME) as *const AsmMOHeader;
        let header = unsafe { std::ptr::read(header_ptr) };
        println!("Header: {:?}", header);

        // From header, skips the header size and 8 bytes more to get the data pointer.
        // The 8 bytes are for the number of chunks.
        let mut data_ptr = unsafe { header_ptr.add(1) } as *const AsmMOChunk;

        // TODO! REMOVE !!!
        std::thread::sleep(Duration::from_millis(50));

        // Initialize C++ memory operations trace
        let mem_planner = MemPlanner::new();
        mem_planner.execute();

        let mut mo_traces = Vec::new();
        let exit_code = loop {
            match sem_chunk_done.timed_wait(Duration::from_secs(10)) {
                Ok(()) => {
                    let header = unsafe { std::ptr::read(header_ptr) };
                    println!("Header after wait: {:?}", header);
                    if should_exit.load(std::sync::atomic::Ordering::SeqCst)
                        && chunk_id.0 == header.num_chunks as usize
                    {
                        break 0;
                    }

                    let mo_trace = loop {
                        // Read only memory reads size
                        let chunk = unsafe { std::ptr::read(data_ptr) };
                        println!(
                            "Pre-reading to check if it is  a 0xFFFFFFFFFFFFFFFF ? {:?}",
                            chunk.mem_ops_size
                        );

                        if chunk.mem_ops_size == MEM_READS_SIZE_DUMMY {
                            std::thread::sleep(Duration::from_nanos(1));
                        } else {
                            println!("{:?}", chunk);
                            mem_planner.add_chunk(chunk.mem_ops_size, unsafe {
                                (data_ptr as *const u8).add(8) as *const c_void
                            });
                            break AsmMOChunk::to_mem_ops(&mut data_ptr);
                        }
                    };
                    mo_traces.push(mo_trace);

                    chunk_id.0 += 1;
                    println!("Memory operation chunk ID: {}", chunk_id.0);
                }
                Err(e) => {
                    error!("Semaphore sem_chunk_done error: {:?}", e);

                    if chunk_id.0 == 0 {
                        break 1;
                    }

                    let header = unsafe { std::ptr::read(header_ptr) };
                    break header.exit_code;
                }
            }
        };

        if exit_code != 0 {
            return Err(AsmRunError::ExitCode(exit_code as u32))
                .context("Child process returned error");
        }

        // Wait for the assembly emulator to complete writing the trace
        let response = handle
            .join()
            .map_err(|_| AsmRunError::JoinPanic)?
            .map_err(AsmRunError::ServiceError)?;

        assert_eq!(response.result, 0);
        assert!(response.trace_len > 0);
        assert!(response.trace_len <= response.allocated_len);

        println!("Memory operations trace length: {}", mo_traces.len());
        mem_planner.set_completed();
        mem_planner.wait();

        Ok(AsmRunnerMO::new(SHMEM_OUTPUT_NAME.to_string(), header_ptr as *mut c_void, mo_traces))
    }

    pub fn write_input(inputs_path: &Path, shmem_input_name: &str) {
        let inputs = fs::read(inputs_path).expect("Failed to read input file");
        let asm_input = AsmInputC2 { zero: 0, input_data_size: inputs.len() as u64 };
        let shmem_input_size = (inputs.len() + size_of::<AsmInputC2>() + 7) & !7;

        let mut full_input = Vec::with_capacity(shmem_input_size);
        full_input.extend_from_slice(&asm_input.to_bytes());
        full_input.extend_from_slice(&inputs);

        let fd =
            shmem_utils::open_shmem(shmem_input_name, libc::O_RDWR, S_IRUSR | S_IWUSR | S_IXUSR);
        let ptr = shmem_utils::map(fd, shmem_input_size, PROT_READ | PROT_WRITE, "input mmap");
        unsafe {
            ptr::copy_nonoverlapping(full_input.as_ptr(), ptr as *mut u8, shmem_input_size);
            shmem_utils::unmap(ptr, shmem_input_size);
            close(fd);
        }
    }

    pub fn get_output_ptr(shmem_output_name: &str) -> *mut std::ffi::c_void {
        let fd =
            shmem_utils::open_shmem(shmem_output_name, libc::O_RDONLY, S_IRUSR | S_IWUSR | S_IXUSR);
        let header_size = size_of::<AsmMOHeader>();
        let temp = shmem_utils::map(fd, header_size, PROT_READ, "header temp map");
        let header = unsafe { (temp as *const AsmMOHeader).read() };
        unsafe {
            shmem_utils::unmap(temp, header_size);
        }
        shmem_utils::map(fd, header.mt_allocated_size as usize, PROT_READ, "output full map")
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
impl AsmRunnerMO {
    pub fn new(_: String, _: *mut c_void, _: Vec<EmuTrace>) -> Self {
        panic!(
            "AsmRunnerMO::new() is not supported on this platform. Only Linux x86_64 is supported."
        )
    }

    pub fn run_and_count<T: Task>(
        _: &Path,
        _: &Path,
        _: u64,
        _: u64,
        _: AsmRunnerOptions,
        _: TaskFactory<T>,
    ) -> (AsmRunnerMO, Vec<T::Output>) {
        panic!("AsmRunnerMO::run_and_count() is not supported on this platform. Only Linux x86_64 is supported.")
    }
}
