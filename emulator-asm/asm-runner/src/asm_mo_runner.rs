#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use named_sem::NamedSemaphore;
use zisk_common::Plan;

use std::ffi::c_void;
use std::sync::atomic::{fence, Ordering};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::error;

use crate::{AsmMOChunk, AsmMOHeader, AsmRunError, AsmService, AsmServices, AsmSharedMemory};
use mem_planner_cpp::MemPlanner;

use anyhow::{Context, Result};

// This struct is used to run the assembly code in a separate process and generate minimal traces.
pub struct AsmRunnerMO {
    pub plans: Vec<Plan>,
}

impl AsmRunnerMO {
    pub fn new(plans: Vec<Plan>) -> Self {
        Self { plans }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run(
        asm_shared_memory: Arc<Mutex<Option<AsmSharedMemory<AsmMOHeader>>>>,
        max_steps: u64,
        chunk_size: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let port = if let Some(base_port) = base_port {
            AsmServices::port_for(&AsmService::MO, base_port, local_rank)
        } else {
            AsmServices::default_port(&AsmService::MO, local_rank)
        };

        let sem_chunk_done_name =
            AsmSharedMemory::<AsmMOHeader>::shmem_chunk_done_name(port, AsmService::MO, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

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
                AsmSharedMemory::create_shmem(
                    port,
                    AsmService::MO,
                    local_rank,
                    unlock_mapped_memory,
                )
                .expect("Error creating MO assembly shared memory"),
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
}
