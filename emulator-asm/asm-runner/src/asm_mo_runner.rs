#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use named_sem::NamedSemaphore;
use zisk_common::ExecutorStatsHandle;
use zisk_common::Plan;

use std::ffi::c_void;
use std::sync::atomic::{fence, Ordering};
use std::time::Duration;
use tracing::error;

use crate::{AsmMOChunk, AsmMOHeader, AsmRunError, AsmService, AsmServices, AsmSharedMemory};
use mem_planner_cpp::MemPlanner;

use anyhow::{Context, Result};

#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;

#[cfg(feature = "save_mem_plans")]
use mem_common::save_plans;

pub struct PreloadedMO {
    pub output_shmem: AsmSharedMemory<AsmMOHeader>,
    mem_planner: Option<MemPlanner>,
    handle_mo: Option<std::thread::JoinHandle<MemPlanner>>,
}

impl PreloadedMO {
    pub fn new(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let port = if let Some(base_port) = base_port {
            AsmServices::port_for(&AsmService::MO, base_port, local_rank)
        } else {
            AsmServices::default_port(&AsmService::MO, local_rank)
        };

        let output_name =
            AsmSharedMemory::<AsmMOHeader>::shmem_output_name(port, AsmService::MO, local_rank);

        let output_shared_memory =
            AsmSharedMemory::<AsmMOHeader>::open_and_map(&output_name, unlock_mapped_memory)?;

        Ok(Self {
            output_shmem: output_shared_memory,
            mem_planner: Some(MemPlanner::new()),
            handle_mo: None,
        })
    }
}

impl Drop for PreloadedMO {
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
        preloaded: &mut PreloadedMO,
        max_steps: u64,
        chunk_size: u64,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        _stats: ExecutorStatsHandle,
    ) -> Result<Self> {
        #[cfg(feature = "stats")]
        let parent_stats_id = _stats.next_id();
        #[cfg(feature = "stats")]
        _stats.add_stat(0, parent_stats_id, "ASM_MO_RUNNER", 0, ExecutorStatsEvent::Begin);

        let port = if let Some(base_port) = base_port {
            AsmServices::port_for(&AsmService::MO, base_port, local_rank)
        } else {
            AsmServices::default_port(&AsmService::MO, local_rank)
        };

        let sem_chunk_done_name =
            AsmSharedMemory::<AsmMOHeader>::shmem_chunk_done_name(port, AsmService::MO, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let __stats = _stats.clone();

        let handle = std::thread::spawn(move || {
            #[cfg(feature = "stats")]
            let stats_id = __stats.next_id();
            #[cfg(feature = "stats")]
            __stats.add_stat(parent_stats_id, stats_id, "ASM_MO", 0, ExecutorStatsEvent::Begin);

            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            let result = asm_services.send_memory_ops_request(max_steps, chunk_size);

            // Add to executor stats
            #[cfg(feature = "stats")]
            __stats.add_stat(parent_stats_id, stats_id, "ASM_MO", 0, ExecutorStatsEvent::End);

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

        #[cfg(feature = "stats")]
        let stats_id = _stats.next_id();
        #[cfg(feature = "stats")]
        _stats.add_stat(
            parent_stats_id,
            stats_id,
            "MO_PROCESS_CHUNKS",
            0,
            ExecutorStatsEvent::Begin,
        );

        // Threshold (in bytes) used to detect when the shared memory region size has changed.
        // Computed to optimize the common case where minor size fluctuations are ignored.
        // It is based on the worst-case scenario of memory usage.
        let threshold_bytes = (chunk_size as usize * 200) + (44 * 8) + 32;
        let mut threshold = unsafe {
            preloaded.output_shmem.mapped_ptr().add(threshold_bytes) as *const AsmMOChunk
        };

        let exit_code = loop {
            match sem_chunk_done.timed_wait(Duration::from_secs(10)) {
                Ok(()) => {
                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    // Check if we need to remap the shared memory
                    if data_ptr >= threshold
                        && preloaded
                            .output_shmem
                            .check_size_changed(&mut data_ptr)
                            .context("Failed to check and remap shared memory for MO trace")?
                    {
                        threshold = unsafe {
                            preloaded.output_shmem.mapped_ptr().add(threshold_bytes)
                                as *const AsmMOChunk
                        };
                    }

                    let chunk = unsafe { std::ptr::read(data_ptr) };

                    data_ptr = unsafe { data_ptr.add(1) };

                    // Add to executor stats
                    #[cfg(feature = "stats")]
                    {
                        let stats_id = _stats.next_id();
                        _stats.add_stat(
                            parent_stats_id,
                            stats_id,
                            "MO_CHUNK_DONE",
                            0,
                            ExecutorStatsEvent::Mark,
                        );
                    }

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

        // Add to executor stats
        #[cfg(feature = "stats")]
        _stats.add_stat(parent_stats_id, stats_id, "MO_PROCESS_CHUNKS", 0, ExecutorStatsEvent::End);

        #[cfg(feature = "stats")]
        let stats_id = _stats.next_id();
        #[cfg(feature = "stats")]
        _stats.add_stat(
            parent_stats_id,
            stats_id,
            "MO_COLLECT_PLANS",
            0,
            ExecutorStatsEvent::Begin,
        );

        let plans = mem_planner.collect_plans(&mut mem_align_plans);

        #[cfg(feature = "save_mem_plans")]
        save_plans(&plans, "mem_plans_cpp.txt");

        // Add to executor stats
        #[cfg(feature = "stats")]
        _stats.add_stat(parent_stats_id, stats_id, "MO_COLLECT_PLANS", 0, ExecutorStatsEvent::End);

        // #[cfg(feature = "stats")]
        // {
        //     let mem_stats = mem_planner.get_mem_stats();
        //     for i in mem_stats {
        //         _stats.add_stat(i);
        //     }
        // }

        preloaded.handle_mo = Some(std::thread::spawn(move || {
            drop(mem_planner);
            MemPlanner::new()
        }));

        #[cfg(feature = "stats")]
        _stats.add_stat(0, parent_stats_id, "ASM_MO_RUNNER", 0, ExecutorStatsEvent::End);

        Ok(AsmRunnerMO::new(plans))
    }
}
