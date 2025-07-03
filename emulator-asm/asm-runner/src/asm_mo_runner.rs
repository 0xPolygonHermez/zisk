use libc::{close, PROT_READ, PROT_WRITE, S_IRUSR, S_IWUSR};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use named_sem::NamedSemaphore;
use zisk_common::Plan;

use std::ffi::c_void;
use std::path::Path;
use std::sync::atomic::{fence, Ordering};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs, ptr};
use tracing::error;

use crate::{
    shmem_utils, AsmInputC2, AsmMOChunk, AsmMOHeader, AsmRunError, AsmServices, AsmSharedMemory,
    AsmSharedMemoryMode,
};
use mem_planner_cpp::MemPlanner;

use anyhow::{Context, Result};

// This struct is used to run the assembly code in a separate process and generate minimal traces.
pub struct AsmRunnerMO {
    pub plans: Vec<Plan>,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl AsmRunnerMO {
    pub fn new(plans: Vec<Plan>) -> Self {
        Self { plans }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run(
        asm_shared_memory: Arc<Mutex<Option<AsmSharedMemory<AsmMOHeader>>>>,
        inputs_path: &Path,
        max_steps: u64,
        chunk_size: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        const MEM_READS_SIZE_DUMMY: u64 = 0xFFFFFFFFFFFFFFFF;

        let prefix = AsmServices::shmem_prefix(&crate::AsmService::MO, base_port, local_rank);

        let shmem_input_name = format!("{prefix}_MO_input");
        let shmem_output_name = format!("{prefix}_MO_output");
        let sem_chunk_done_name = format!("/{prefix}_MO_chunk_done");

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        Self::write_input(inputs_path, &shmem_input_name, unlock_mapped_memory);

        let handle = std::thread::spawn(move || {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            asm_services.send_memory_ops_request(max_steps, chunk_size)
        });

        // Open and map the shared memory where the assembly emulator writes its output.
        // The shared memory is created by the C++ assembly emulator.

        // Initialize the assembly shared memory if necessary
        let mut asm_shared_memory = asm_shared_memory.lock().unwrap();

        if asm_shared_memory.is_none() {
            *asm_shared_memory = Some(
                AsmSharedMemory::<AsmMOHeader>::open_and_map(
                    &shmem_output_name,
                    AsmSharedMemoryMode::ReadOnly,
                    unlock_mapped_memory,
                )
                .expect("Error creating assembly shared memory"),
            );
        }

        // Get the pointer to the data in the shared memory.
        let mut data_ptr = asm_shared_memory.as_ref().unwrap().data_ptr() as *const AsmMOChunk;

        // Initialize C++ memory operations trace
        let mem_planner = MemPlanner::new();
        mem_planner.execute();

        let exit_code = loop {
            match sem_chunk_done.timed_wait(Duration::from_secs(10)) {
                Ok(()) => {
                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    let chunk = unsafe { std::ptr::read(data_ptr) };

                    // TODO! Remove this check in the near future
                    if chunk.mem_ops_size == MEM_READS_SIZE_DUMMY {
                        panic!("Unexpected state: invalid data received from C++");
                    }

                    data_ptr = unsafe { data_ptr.add(1) };

                    mem_planner.add_chunk(chunk.mem_ops_size, data_ptr as *const c_void);

                    if chunk.end == 1 {
                        break 0;
                    }

                    data_ptr = unsafe {
                        (data_ptr as *mut u64).add(chunk.mem_ops_size as usize) as *const AsmMOChunk
                    };
                }
                Err(e) => {
                    error!("Semaphore '{}' error: {:?}", sem_chunk_done_name, e);

                    break asm_shared_memory.as_ref().unwrap().header().exit_code;
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

        mem_planner.set_completed();
        mem_planner.wait();
        let plans = mem_planner.collect_plans();

        Ok(AsmRunnerMO::new(plans))
    }

    fn write_input(inputs_path: &Path, shmem_input_name: &str, unlock_mapped_memory: bool) {
        let inputs = fs::read(inputs_path).expect("Failed to read input file");
        let asm_input = AsmInputC2 { zero: 0, input_data_size: inputs.len() as u64 };
        let shmem_input_size = (inputs.len() + size_of::<AsmInputC2>() + 7) & !7;

        let mut full_input = Vec::with_capacity(shmem_input_size);
        full_input.extend_from_slice(&asm_input.to_bytes());
        full_input.extend_from_slice(&inputs);
        while full_input.len() < shmem_input_size {
            full_input.push(0);
        }

        let fd = shmem_utils::open_shmem(shmem_input_name, libc::O_RDWR, S_IRUSR | S_IWUSR);

        let ptr = shmem_utils::map(
            fd,
            shmem_input_size,
            PROT_READ | PROT_WRITE,
            unlock_mapped_memory,
            "MO input mmap",
        );
        unsafe {
            ptr::copy_nonoverlapping(full_input.as_ptr(), ptr as *mut u8, shmem_input_size);
            shmem_utils::unmap(ptr, shmem_input_size);
            close(fd);
        }
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
