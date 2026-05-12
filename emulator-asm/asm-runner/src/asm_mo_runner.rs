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
#[cfg(feature = "gpu")]
use mem_planner_cpp::{GpuMemOp, GpuMemPlanner};

use anyhow::{Context, Result};

#[cfg(feature = "save_mem_plans")]
use mem_common::save_plans;

pub struct MOShMemReader {
    pub output_shmem: AsmMultiSharedMemory<AsmMOHeader>,
    mem_planner: Option<MemPlanner>,
    handle_mo: Option<std::thread::JoinHandle<MemPlanner>>,
    /// Set up once at worker startup against proofman's unified GPU buffer,
    /// then reused block-after-block via `reset()`. Constructing/destroying
    /// per call would cost ~240 ms in CUDA cleanup.
    #[cfg(feature = "gpu")]
    gpu_planner: Option<GpuMemPlanner>,
}

impl MOShMemReader {
    pub fn new(
        shm_prefix: &str,
        unlock_mapped_memory: bool,
        gpu_buf_ptr: usize,
        gpu_buf_size: u64,
    ) -> Result<Self> {
        let output_name = shmem_output_name(shm_prefix, AsmService::MO, None);

        let output_shared_memory = AsmMultiSharedMemory::<AsmMOHeader>::open_and_map(
            &output_name,
            TRACE_INITIAL_SIZE,
            TRACE_DELTA_SIZE,
            TRACE_MAX_SIZE,
            unlock_mapped_memory,
        )?;

        #[cfg(feature = "gpu")]
        let gpu_planner = setup_gpu_planner(gpu_buf_ptr, gpu_buf_size);
        #[cfg(not(feature = "gpu"))]
        let _ = (gpu_buf_ptr, gpu_buf_size);

        Ok(Self {
            output_shmem: output_shared_memory,
            mem_planner: Some(MemPlanner::new()),
            handle_mo: None,
            #[cfg(feature = "gpu")]
            gpu_planner,
        })
    }
}

#[cfg(feature = "gpu")]
fn setup_gpu_planner(gpu_buf_ptr: usize, gpu_buf_size: u64) -> Option<GpuMemPlanner> {
    if gpu_buf_ptr == 0 || gpu_buf_size == 0 {
        tracing::warn!(
            "[gpu] no borrowed buffer (ptr=0x{:x}, size={}); planner not constructed",
            gpu_buf_ptr,
            gpu_buf_size,
        );
        return None;
    }
    let gp = GpuMemPlanner::new();
    if !gp.setup(gpu_buf_ptr as *mut c_void, gpu_buf_size as usize, 1, 0) {
        tracing::error!(
            "[gpu] GpuMemPlanner::setup returned false (size={} bytes); planner disabled",
            gpu_buf_size,
        );
        return None;
    }
    tracing::info!(
        "[gpu] GpuMemPlanner set up (borrowed {:.3} GB)",
        gpu_buf_size as f64 / (1024.0 * 1024.0 * 1024.0),
    );
    Some(gp)
}

impl Drop for MOShMemReader {
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
        preloaded: &mut MOShMemReader,
        max_steps: u64,
        chunk_size: u64,
        asm_services: AsmServices,
        _stats: ExecutorStatsHandle,
    ) -> Result<Self> {
        stats_begin!(_stats, 0, _runner_scope, "ASM_MO_RUNNER", 0);

        let sem_chunk_done_name = sem_chunk_done_name(asm_services.sem_prefix(), AsmService::MO);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        // Capture parent id for thread
        let _parent_id = _runner_scope.id();
        let _thread_stats = _stats.clone();

        let handle = std::thread::spawn(move || {
            stats_begin!(_thread_stats, _parent_id, _mo_scope, "ASM_MO", 0);

            #[allow(clippy::let_and_return)]
            let result = asm_services.send_memory_ops_request(max_steps, chunk_size);

            stats_end!(_thread_stats, &_mo_scope);

            result
        });

        let mem_planner = match preloaded.mem_planner.take() {
            Some(p) => p,
            None => preloaded
                .handle_mo
                .take()
                .ok_or_else(|| {
                    anyhow::anyhow!("MOShMemReader: both mem_planner and handle_mo are None")
                })?
                .join()
                .map_err(|_| anyhow::anyhow!("MO preload background thread panicked"))?,
        };

        // Get the pointer to the data in the shared memory.
        let mut data_ptr = preloaded.output_shmem.data_ptr() as *const AsmMOChunk;

        // In GPU mode, the in-process GpuMemPlanner provides the segment
        // table (injected after `wait()` below); the CPU side only needs to
        // run the mem-align worker.
        #[cfg(feature = "gpu")]
        mem_planner.execute_align_only();
        #[cfg(not(feature = "gpu"))]
        mem_planner.execute();

        #[cfg(feature = "gpu")]
        let gpu_planner: Option<GpuMemPlanner> = preloaded.gpu_planner.take();

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

                    // `MemCountersBusData` and `GpuMemOp` share an identical
                    // {u32 addr; u32 flags;} __packed layout, so the cast is
                    // a pointer reinterpret — no copy.
                    #[cfg(feature = "gpu")]
                    if let Some(ref gp) = gpu_planner {
                        let memops = unsafe {
                            std::slice::from_raw_parts(
                                data_ptr as *const GpuMemOp,
                                chunk.mem_ops_size as usize,
                            )
                        };
                        if !gp.add_chunk(memops) {
                            tracing::error!(
                                "[gpu] add_chunk failed (n={})",
                                chunk.mem_ops_size,
                            );
                        }
                    }

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

        mem_planner.set_completed();

        // Drain the GPU pipeline. The returned slice borrows from
        // `gpu_planner`'s host-pinned buffers — kept alive for the inject below.
        #[cfg(feature = "gpu")]
        let gpu_metas_view: Option<(*const c_void, u32)> = gpu_planner
            .as_ref()
            .and_then(|gp| gp.run())
            .map(|metas| (metas.as_ptr() as *const c_void, metas.len() as u32));

        mem_planner.wait();

        // Populate `mcp->segments[]`. Two paths:
        //   * GPU feature on  → inject the in-memory metas produced this run.
        //   * GPU feature off → fall back to the legacy `tmp/metas.bin` file
        //                       written by the standalone GPU runner.
        #[cfg(feature = "gpu")]
        {
            if let Some((ptr, n)) = gpu_metas_view {
                let ok = unsafe { mem_planner.inject_gpu_metas_from_pointers(ptr, n) };
                if !ok {
                    tracing::error!("[gpu] inject_gpu_metas_from_pointers failed");
                }
            } else if !mem_planner.load_mem_metas_from_disk() {
                tracing::warn!(
                    "[gpu] no GPU planner and tmp/metas.bin missing; segments will be empty"
                );
            }

            // Reset and stash for the next block. Cheap — keeps CUDA resources alive.
            if let Some(gp) = gpu_planner {
                gp.reset();
                preloaded.gpu_planner = Some(gp);
            }
        }
        #[cfg(not(feature = "gpu"))]
        if !mem_planner.load_mem_metas_from_disk() {
            tracing::warn!("tmp/metas.bin missing; segments will be empty (no GPU build)");
        }

        let result: Result<Vec<Plan>> = (|| -> Result<Vec<Plan>> {
            if exit_code != 0 {
                return Err(AsmRunError::ExitCode(exit_code as u32))
                    .context("Child process returned error");
            }

            let response = handle
                .join()
                .map_err(|_| AsmRunError::JoinPanic)?
                .map_err(AsmRunError::ServiceError)?;

            if response.result != 0 {
                return Err(anyhow::anyhow!(
                    "ASM MO service returned non-zero result: {}",
                    response.result
                ));
            }
            if response.trace_len == 0 {
                return Err(anyhow::anyhow!("ASM MO service returned empty trace"));
            }
            if response.trace_len > response.allocated_len {
                return Err(anyhow::anyhow!(
                    "ASM MO service trace_len ({}) exceeds allocated_len ({})",
                    response.trace_len,
                    response.allocated_len
                ));
            }

            let mut mem_align_plans = mem_planner.wait_mem_align_plans();
            stats_end!(_stats, &_process_scope);
            stats_begin!(_stats, &_runner_scope, _collect_scope, "MO_COLLECT_PLANS", 0);
            let plans = mem_planner.collect_plans(&mut mem_align_plans);
            stats_end!(_stats, &_collect_scope);
            Ok(plans)
        })();

        // Always re-stash the CPU planner so the next call has one to take.
        preloaded.handle_mo = Some(std::thread::spawn(move || {
            drop(mem_planner);
            MemPlanner::new()
        }));

        let plans = result?;

        #[cfg(feature = "save_mem_plans")]
        save_plans(&plans, "mem_plans_cpp.txt");

        stats_end!(_stats, &_runner_scope);
        Ok(AsmRunnerMO::new(plans))
    }
}
