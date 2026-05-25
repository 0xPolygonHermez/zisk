#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use named_sem::NamedSemaphore;
use zisk_common::{stats_begin, stats_end, stats_mark, ExecutorStatsHandle, Plan};

use std::ffi::c_void;
use std::sync::atomic::{fence, Ordering};
use std::time::{Duration, Instant}; // TEMP-MOPROF
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
#[cfg(gpu)]
use mem_planner_cpp::{GpuCountAndPlan, GpuMemOp};
use proofman_util::{timer_start_info, timer_stop_and_log_info};

use anyhow::{Context, Result};

#[cfg(feature = "save_mem_plans")]
use mem_common::save_plans;

pub struct MOShMemReader {
    pub output_shmem: AsmMultiSharedMemory<AsmMOHeader>,
    mem_planner: Option<MemPlanner>,
    handle_mo: Option<std::thread::JoinHandle<MemPlanner>>,
    #[cfg(gpu)]
    gpu_count_and_plan: Option<GpuCountAndPlan>,
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

        #[cfg(gpu)]
        let gpu_count_and_plan = setup_gpu_count_and_plan(gpu_buf_ptr, gpu_buf_size);
        #[cfg(not(gpu))]
        let _ = (gpu_buf_ptr, gpu_buf_size);

        Ok(Self {
            output_shmem: output_shared_memory,
            mem_planner: Some(MemPlanner::new()),
            handle_mo: None,
            #[cfg(gpu)]
            gpu_count_and_plan,
        })
    }
}

#[cfg(gpu)]
fn setup_gpu_count_and_plan(gpu_buf_ptr: usize, gpu_buf_size: u64) -> Option<GpuCountAndPlan> {
    if gpu_buf_ptr == 0 || gpu_buf_size == 0 {
        tracing::info!(
            "[gpu] no borrowed buffer (--gpu not set at runtime); using CPU mem_planner path"
        );
        return None;
    }
    let gp = GpuCountAndPlan::new();
    if !gp.setup(gpu_buf_ptr as *mut c_void, gpu_buf_size as usize, 1, 0) {
        tracing::error!(
            "[gpu] GpuCountAndPlan::setup returned false (size={} bytes); falling back to CPU",
            gpu_buf_size,
        );
        return None;
    }
    tracing::info!(
        "[gpu] GpuCountAndPlan set up (borrowed {:.3} GB)",
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

        let mut data_ptr = preloaded.output_shmem.data_ptr() as *const AsmMOChunk;

        // Take the optional GPU planner for this block.
        #[cfg(gpu)]
        let gpu_count_and_plan: Option<GpuCountAndPlan> = preloaded.gpu_count_and_plan.take();

        #[cfg(gpu)]
        if let Some(ref gp) = gpu_count_and_plan {
            gp.reset();
        }

        // CPU workers are only spawned when no GPU planner is active.
        #[cfg(gpu)]
        if gpu_count_and_plan.is_none() {
            mem_planner.execute();
        }
        #[cfg(not(gpu))]
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

        // TEMP-MOPROF: split the chunk-consume loop into producer-wait
        // (blocked in timed_wait) vs the add_chunk copy, with chunk/memop
        // counts and first-chunk latency. Instant overhead is ~once per chunk
        // (hundreds/block), not per memop — negligible.
        let _mo_loop_start = Instant::now();
        let mut _mo_t_wait = Duration::ZERO;
        let mut _mo_t_add = Duration::ZERO;
        let mut _mo_n_chunks: u64 = 0;
        let mut _mo_n_memops: u64 = 0;
        let mut _mo_t_first = Duration::ZERO;
        let exit_code = loop {
            let _mo_w = Instant::now(); // TEMP-MOPROF
            let _mo_wait_res = sem_chunk_done.timed_wait(SEM_CHUNK_DONE_WAIT_DURATION); // TEMP-MOPROF
            _mo_t_wait += _mo_w.elapsed(); // TEMP-MOPROF
            match _mo_wait_res {
                Ok(()) => {
                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);
                    if _mo_t_first.is_zero() {
                        _mo_t_first = _mo_loop_start.elapsed(); // TEMP-MOPROF
                    }

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

                    // Feed this chunk to whichever planner is active
                    let _mo_a = Instant::now(); // TEMP-MOPROF
                    #[cfg(gpu)]
                    if let Some(ref gp) = gpu_count_and_plan {
                        let memops = unsafe {
                            std::slice::from_raw_parts(
                                data_ptr as *const GpuMemOp,
                                chunk.mem_ops_size as usize,
                            )
                        };
                        if !gp.add_chunk(memops) {
                            tracing::error!("[gpu] add_chunk failed (n={})", chunk.mem_ops_size);
                        }
                    } else {
                        mem_planner.add_chunk(chunk.mem_ops_size, data_ptr as *const c_void);
                    }
                    #[cfg(not(gpu))]
                    mem_planner.add_chunk(chunk.mem_ops_size, data_ptr as *const c_void);
                    _mo_t_add += _mo_a.elapsed(); // TEMP-MOPROF
                    _mo_n_chunks += 1; // TEMP-MOPROF
                    _mo_n_memops += chunk.mem_ops_size; // TEMP-MOPROF

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

        // TEMP-MOPROF: per-block MO chunk-consume breakdown. wait = blocked on
        // the ASM MO producer; add_chunk = the shmem->pinned copy; first_chunk =
        // latency to first chunk. Pair with GPU_MOPS_TIME + WAIT_PLAN_MEM_CPP.
        tracing::info!(
            "[mo-prof] chunks={} memops={} first_chunk={}ms wait={}ms add_chunk={}ms loop_total={}ms",
            _mo_n_chunks,
            _mo_n_memops,
            _mo_t_first.as_millis(),
            _mo_t_wait.as_millis(),
            _mo_t_add.as_millis(),
            _mo_loop_start.elapsed().as_millis(),
        );

        // Wind the C++ planner down before any further work. Without this,
        // its background threads stay parked waiting for more chunks, and
        // the C++ destructor blocks on `Drop`, holding the `MOShMemReader`
        // Mutex and hanging the next job's MO thread on lock acquisition.
        // In the GPU case no-op since the GPU planner has no background threads
        mem_planner.set_completed();

        // GPU path: drain the pipeline. Pointer into pinned host memory owned
        // by the planner; valid until the planner is reset below.
        timer_start_info!(GPU_MOPS_TIME);
        #[cfg(gpu)]
        let gpu_metas_view: Option<(*const c_void, u32)> = gpu_count_and_plan
            .as_ref()
            .and_then(|gp| gp.run())
            .map(|metas| (metas.as_ptr() as *const c_void, metas.len() as u32));
        #[cfg(not(gpu))]
        let gpu_metas_view: Option<(*const c_void, u32)> = None;
        timer_stop_and_log_info!(GPU_MOPS_TIME);

        // owner: join CPU workers; no-op in GPU mode (null-guarded)
        mem_planner.wait();

        // GPU path: build align plans
        #[cfg(gpu)]
        let gpu_align_plans: Option<Vec<Plan>> =
            gpu_count_and_plan.as_ref().map(|gp| gp.build_align_plans());

        // inject GPU-produced segments to the C++ segment table
        // (`mcp->segments[]`). No-op on the CPU path (gpu_metas_view None).
        if let Some((ptr, n)) = gpu_metas_view {
            let ok = unsafe { mem_planner.inject_gpu_metas_from_pointers(ptr, n) };
            if !ok {
                tracing::error!("[gpu] inject_gpu_metas_from_pointers failed");
            }
        }

        // Stash the GPU planner for the next block
        #[cfg(gpu)]
        if let Some(gp) = gpu_count_and_plan {
            preloaded.gpu_count_and_plan = Some(gp);
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

            // Use GPU-built align plans if available, otherwise wait on the
            // CPU mem-align worker.
            #[cfg(gpu)]
            let mut mem_align_plans =
                gpu_align_plans.unwrap_or_else(|| mem_planner.wait_mem_align_plans());
            #[cfg(not(gpu))]
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
