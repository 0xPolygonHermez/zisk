#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use named_sem::NamedSemaphore;
use zisk_common::{stats_begin, stats_end, stats_mark, ExecutorStatsHandle, Plan};

use std::ffi::c_void;
use std::sync::atomic::{fence, Ordering};
use tracing::error;

use crate::SEM_CHUNK_DONE_WAIT_DURATION;
use crate::TRACE_DELTA_SIZE;
use crate::TRACE_INITIAL_SIZE;
use crate::TRACE_MAX_SIZE;
use crate::{
    sem_chunk_done_name, shmem_output_name, AsmMOChunk, AsmMOHeader, AsmMultiSharedMemory,
    AsmRunError, AsmService, AsmServices,
};
use mem_planner_cpp::MemPlanner;

use anyhow::{Context, Result};

#[cfg(feature = "save_mem_plans")]
use mem_common::save_plans;

pub struct MOOutputShmem {
    pub output_shmem: AsmMultiSharedMemory<AsmMOHeader>,
    mem_planner: Option<MemPlanner>,
    handle_mo: Option<std::thread::JoinHandle<MemPlanner>>,
}

impl MOOutputShmem {
    pub fn new(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let port = AsmServices::port_base_for(base_port, local_rank);

        let output_name = shmem_output_name(port, AsmService::MO, local_rank, None);

        let output_shared_memory = AsmMultiSharedMemory::<AsmMOHeader>::open_and_map(
            &output_name,
            TRACE_INITIAL_SIZE,
            TRACE_DELTA_SIZE,
            TRACE_MAX_SIZE,
            unlock_mapped_memory,
        )?;

        Ok(Self {
            output_shmem: output_shared_memory,
            mem_planner: Some(MemPlanner::new()),
            handle_mo: None,
        })
    }
}

impl Drop for MOOutputShmem {
    fn drop(&mut self) {
        if let Some(handle_mo) = self.handle_mo.take() {
            match handle_mo.join() {
                Ok(mem_planner) => {
                    // If the thread completed successfully, we can safely drop the MemPlanner.
                    drop(mem_planner);
                }
                Err(e) => {
                    eprintln!("Warning: background thread panicked in PreloadedMO: {e:?}");
                }
            }
        }
    }
}

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
        preloaded: &mut MOOutputShmem,
        max_steps: u64,
        chunk_size: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        _stats: ExecutorStatsHandle,
    ) -> Result<Self> {
        stats_begin!(_stats, 0, _runner_scope, "ASM_MO_RUNNER", 0);

        let port = AsmServices::port_base_for(base_port, local_rank);

        let sem_chunk_done_name = sem_chunk_done_name(port, AsmService::MO, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        // Capture parent id for thread
        let _parent_id = _runner_scope.id();
        let _thread_stats = _stats.clone();

        let handle = std::thread::spawn(move || {
            stats_begin!(_thread_stats, _parent_id, _mo_scope, "ASM_MO", 0);

            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            #[allow(clippy::let_and_return)]
            let result = asm_services.send_memory_ops_request(max_steps, chunk_size);

            stats_end!(_thread_stats, &_mo_scope);

            result
        });

        let mem_planner = preloaded
            .mem_planner
            .take()
            .unwrap_or_else(|| preloaded.handle_mo.take().unwrap().join().unwrap());

        // Get the pointer to the data in the shared memory.
        let mut data_ptr = preloaded.output_shmem.data_ptr() as *const AsmMOChunk;

        // Initialize C++ memory operations trace
        mem_planner.execute();

        stats_begin!(_stats, &_runner_scope, _process_scope, "MO_PROCESS_CHUNKS", 0);

        // Threshold (in bytes) used to detect when we need to check for new shared memory files.
        // Must match MAX_CHUNK_TRACE_SIZE from main.c to ensure we check before the producer
        // reallocates. Constants from main.c:
        //   MAX_MTRACE_REGS_ACCESS_SIZE = (2 + 2 + 3) * 8 = 56
        //   MAX_BYTES_DIRECT_MTRACE = 256
        //   MAX_BYTES_MTRACE_STEP = 256 + 56 = 312
        //   MAX_TRACE_CHUNK_INFO = (44 * 8) + 32 = 384
        //   MAX_CHUNK_TRACE_SIZE = (chunk_size * MAX_BYTES_MTRACE_STEP) + MAX_TRACE_CHUNK_INFO
        const MAX_MTRACE_REGS_ACCESS_SIZE: usize = (2 + 2 + 3) * 8;
        const MAX_BYTES_DIRECT_MTRACE: usize = 256;
        const MAX_BYTES_MTRACE_STEP: usize = MAX_BYTES_DIRECT_MTRACE + MAX_MTRACE_REGS_ACCESS_SIZE;
        const MAX_TRACE_CHUNK_INFO: usize = (44 * 8) + 32;

        let threshold_bytes = (chunk_size as usize * MAX_BYTES_MTRACE_STEP) + MAX_TRACE_CHUNK_INFO;
        let mut threshold = unsafe {
            preloaded
                .output_shmem
                .mapped_ptr()
                .add(preloaded.output_shmem.total_mapped_size() - threshold_bytes)
                as *const AsmMOChunk
        };

        let exit_code = loop {
            match sem_chunk_done.timed_wait(SEM_CHUNK_DONE_WAIT_DURATION) {
                Ok(()) => {
                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    // Check if we need to map additional shared memory files.
                    if data_ptr >= threshold
                        && preloaded.output_shmem.check_size_changed().context(
                            "Failed to check and map new shared memory files for MO trace",
                        )?
                    {
                        // Update threshold based on new total mapped size
                        threshold =
                            unsafe {
                                preloaded.output_shmem.mapped_ptr().add(
                                    preloaded.output_shmem.total_mapped_size() - threshold_bytes,
                                ) as *const AsmMOChunk
                            };
                    }

                    let chunk = unsafe { std::ptr::read(data_ptr) };

                    data_ptr = unsafe { data_ptr.add(1) };

                    stats_mark!(_stats, &_runner_scope, "MO_CHUNK_DONE", 0);

                    mem_planner.add_chunk(chunk.mem_ops_size, data_ptr as *const c_void);

                    if chunk.end == 1 {
                        break 0;
                    }

                    data_ptr = unsafe {
                        (data_ptr as *mut u64).add(chunk.mem_ops_size as usize) as *const AsmMOChunk
                    };
                }
                Err(named_sem::Error::WaitFailed(e))
                    if e.kind() == std::io::ErrorKind::Interrupted =>
                {
                    continue
                }
                Err(e) => {
                    error!("Semaphore '{}' error: {:?}", sem_chunk_done_name, e);

                    break preloaded.output_shmem.map_header().exit_code;
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
        // Wait for mem_align_plans, this mem_align_plans are calculated in rust from
        // counters calculated in C++
        let mut mem_align_plans = mem_planner.wait_mem_align_plans();
        mem_planner.wait();

        stats_end!(_stats, &_process_scope);
        stats_begin!(_stats, &_runner_scope, _collect_scope, "MO_COLLECT_PLANS", 0);

        let plans = mem_planner.collect_plans(&mut mem_align_plans);

        #[cfg(feature = "save_mem_plans")]
        save_plans(&plans, "mem_plans_cpp.txt");

        stats_end!(_stats, &_collect_scope);

        preloaded.handle_mo = Some(std::thread::spawn(move || {
            drop(mem_planner);
            MemPlanner::new()
        }));

        stats_end!(_stats, &_runner_scope);
        Ok(AsmRunnerMO::new(plans))
    }
}
