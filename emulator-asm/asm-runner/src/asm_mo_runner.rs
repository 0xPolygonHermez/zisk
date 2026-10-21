#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use named_sem::NamedSemaphore;
use zisk_common::{stats_begin, stats_end, stats_mark, ExecutorStatsHandle, Plan};

use std::ffi::c_void;
use std::sync::atomic::{fence, Ordering};
use tracing::{error, warn};

use crate::MAX_TRACE_CHUNK_INFO;
use crate::SEM_CHUNK_DONE_WAIT_DURATION;
use crate::TRACE_DELTA_SIZE;
use crate::TRACE_INITIAL_SIZE;
use crate::TRACE_MAX_SIZE;
use crate::{
    sem_chunk_done_name, shmem_output_name, AsmMOChunk, AsmMOHeader, AsmMultiShmem, AsmRunError,
    AsmService, AsmServices, GpuBufferSource, OwnedMemInstances,
};
#[cfg(gpu)]
use proofman_util::{timer_start_info, timer_stop_and_log_info};
#[cfg(gpu)]
use zisk_sm_mem_planner::GpuCountAndPlan;
use zisk_sm_mem_planner::MemPlanner;

use anyhow::{Context, Result};

#[cfg(feature = "save_mem_plans")]
use zisk_sm_mem_common::save_plans;

// ASYNC-DMA SAFETY INVARIANT: the GPU issues async H2D copies straight from this
// shmem region, so the source must stay immutable until `run()` drains the streams.
// This holds because (1) MultiShMem reserves its address range once and only
// MAP_FIXEDs new files — existing mappings never move (see multi_shmem.rs
// `check_size_changed`), and (2) the MO trace is strictly append-only: bytes are
// never rewritten once `chunk_done` is signaled. In-place rewrites of already-emitted
// regions (or ring-buffer reuse of offsets within a run) would reintroduce a race.
#[cfg(gpu)]
fn register_mo_shmem_pinned(
    gpu_count_and_plan: &GpuCountAndPlan,
    shmem: &AsmMultiShmem<AsmMOHeader>,
    registered: &mut usize,
) {
    if *registered == usize::MAX {
        return; // registration unsupported on this device
    }
    let total = shmem.total_mapped_size();
    if total <= *registered {
        return;
    }
    let new_ptr = unsafe { (shmem.mapped_ptr() as *const c_void).add(*registered) };
    if gpu_count_and_plan.register_input_pinned(new_ptr, total - *registered) {
        *registered = total;
    } else {
        *registered = usize::MAX; // give up; do not retry
    }
}

/// The planner, and whether the setup gave the device memory witness its room (the planner then
/// retains every block's accesses); without it the run keeps the CPU witness, logged once here.
#[cfg(gpu)]
fn setup_gpu_count_and_plan(gpu_buffer: GpuBufferSource) -> Option<(GpuCountAndPlan, bool)> {
    // gpu_id: device to bind; negative = keep current device (self-allocated).
    let (d_buf, bytes, gpu_id): (*mut c_void, usize, i32) = match gpu_buffer {
        GpuBufferSource::Cpu => {
            tracing::info!("[gpu] no GPU buffer requested; using CPU mem_planner path");
            return None;
        }
        GpuBufferSource::Borrowed { ptr, size, .. } if ptr == 0 || size == 0 => {
            tracing::info!(
                "[gpu] borrowed buffer is empty (--gpu not set at runtime); using CPU mem_planner path"
            );
            return None;
        }
        GpuBufferSource::Borrowed { ptr, size, gpu_id } => {
            let Ok(gpu_id) = i32::try_from(gpu_id) else {
                tracing::error!(
                    "[gpu] gpu_id {gpu_id} exceeds i32::MAX; using CPU mem_planner path"
                );
                return None;
            };
            (ptr as *mut c_void, size, gpu_id)
        }
        GpuBufferSource::SelfAllocated => (std::ptr::null_mut(), 0, -1),
    };

    let gpu_count_and_plan = GpuCountAndPlan::new();
    // SAFETY: `d_buf` is either null (self-allocated) or a device buffer of
    // `bytes` bytes borrowed from the prover, which outlives this planner.
    if !unsafe {
        gpu_count_and_plan.setup(d_buf, bytes, 1, 0, gpu_id, device_mem_witness_requested())
    } {
        tracing::error!("[gpu] GpuCountAndPlan::setup returned false; falling back to CPU");
        return None;
    }
    match gpu_buffer {
        GpuBufferSource::SelfAllocated => {
            tracing::info!("[gpu] GpuCountAndPlan set up (self-allocated device buffer)");
        }
        _ => {
            tracing::info!(
                "[gpu] GpuCountAndPlan set up (borrowed {:.3} GB)",
                bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            );
        }
    }
    let device_witness = device_mem_witness_requested() && gpu_count_and_plan.retains_rows();
    if device_mem_witness_requested() && !device_witness {
        tracing::warn!(
            "[gpu] memory witness on the device unavailable for this run (see above); the memory instances take the CPU witness"
        );
    }
    Some((gpu_count_and_plan, device_witness))
}

/// The block's input region as the guest sees it: the input shared memory up to the bytes written.
#[cfg(gpu)]
fn input_image(shm_prefix: &str) -> std::result::Result<crate::ShmemReader, String> {
    let size = crate::ControlShmem::inputs_size_of(shm_prefix).map_err(|e| e.to_string())?;
    if size > zisk_core::MAX_INPUT_SIZE {
        return Err(format!("the input region reports {size} bytes, more than the region holds"));
    }
    let name = crate::shmem_input_name(shm_prefix);
    crate::ShmemReader::new_unlocked(&name, 8 + size as usize).map_err(|e| e.to_string())
}

/// This struct manages the shared memory and synchronization primitives for reading memory operation traces from the C++ side.
pub struct MOShmemReader {
    pub(crate) output_shmem: AsmMultiShmem<AsmMOHeader>,
    mem_planner: Option<MemPlanner>,
    handle_mo: Option<std::thread::JoinHandle<MemPlanner>>,
    #[cfg(gpu)]
    gpu_count_and_plan: Option<GpuCountAndPlan>,
    /// The planner serves the memory witness (its setup gave it the room).
    #[cfg(gpu)]
    device_witness: bool,
    #[cfg(gpu)]
    registered_bytes: usize,
}

impl MOShmemReader {
    /// Creates a new `MOShmemReader` by opening and mapping the shared memory for the MO trace output.
    pub fn new(
        shm_prefix: &str,
        unlock_mapped_memory: bool,
        buffer_source: GpuBufferSource,
    ) -> Result<Self> {
        let output_name = shmem_output_name(shm_prefix, AsmService::MO, None);

        let output_shared_memory = AsmMultiShmem::<AsmMOHeader>::open_and_map(
            &output_name,
            TRACE_INITIAL_SIZE,
            TRACE_DELTA_SIZE,
            TRACE_MAX_SIZE,
            unlock_mapped_memory,
            cfg!(gpu),
        )?;

        #[cfg(gpu)]
        let (gpu_count_and_plan, device_witness) = match setup_gpu_count_and_plan(buffer_source) {
            Some((planner, device_witness)) => (Some(planner), device_witness),
            None => (None, false),
        };
        #[cfg(not(gpu))]
        let _ = buffer_source;

        Ok(Self {
            output_shmem: output_shared_memory,
            mem_planner: Some(MemPlanner::new()),
            handle_mo: None,
            #[cfg(gpu)]
            gpu_count_and_plan,
            #[cfg(gpu)]
            device_witness,
            // Pinned pages disabled: no measured benefit and not supported on all devices.
            // `usize::MAX` is the "give up" sentinel in `register_mo_shmem_pinned`.
            #[cfg(gpu)]
            registered_bytes: usize::MAX,
        })
    }
}

impl Drop for MOShmemReader {
    fn drop(&mut self) {
        // The block's device witness, if still armed, and its registration end before the
        // planner (a field of this reader) is destroyed.
        #[cfg(gpu)]
        {
            zisk_sm_mem_planner::gpu_mem_witness_release_now();
            zisk_sm_mem_planner::clear_gpu_mem_witness();
        }
        if let Some(handle_mo) = self.handle_mo.take() {
            match handle_mo.join() {
                Ok(mem_planner) => {
                    // If the thread completed successfully, we can safely drop the MemPlanner.
                    drop(mem_planner);
                }
                Err(e) => {
                    eprintln!("Warning: background thread panicked in MOShmemReader: {e:?}");
                }
            }
        }
    }
}

/// The block's device memory witness: the GPU planner retained the block's accesses and builds
/// the memory instances' rows while its arena is still borrowed; each owned instance copies its
/// rows into its host trace buffer. The executor calls `fill_owned` once the memory instances
/// are registered and placed, before it releases the arena.
#[derive(Debug)]
pub struct DeviceMemWitness {
    #[cfg(gpu)]
    shm_prefix: String,
}

impl DeviceMemWitness {
    /// Starts the block's preparation for the owned instances, on its own thread, and arms the
    /// arena's release (`d_buffers` is the prover's device buffers) for the last owned instance's
    /// fill. Returns whether the release is deferred that way; otherwise the block takes the CPU
    /// witness and the caller releases the arena now.
    pub fn fill_owned(&self, owned: &OwnedMemInstances<'_>, d_buffers: *mut c_void) -> bool {
        #[cfg(not(gpu))]
        {
            let _ = (owned, d_buffers);
            false
        }
        #[cfg(gpu)]
        {
            let n_owned = owned.ram.len() + owned.rom.len() + owned.input.len() + owned.align.len();
            if n_owned == 0 {
                tracing::debug!("[gpu] memory witness on the device: no owned memory instance");
                return false;
            }
            timer_start_info!(GPU_MEM_WITNESS_PREPARE);
            let prepared = input_image(&self.shm_prefix).and_then(|image| {
                zisk_sm_mem_planner::gpu_mem_witness_prepare_async(
                    image.bytes().to_vec(),
                    owned.ram.clone(),
                    owned.rom.clone(),
                    owned.input.clone(),
                    &owned.align,
                )
            });
            timer_stop_and_log_info!(GPU_MEM_WITNESS_PREPARE);
            match prepared {
                Ok(()) => {
                    tracing::info!(
                        "[gpu] memory witness on the device: {n_owned} owned instances, preparing"
                    );
                    let d_buffers = d_buffers as usize;
                    zisk_sm_mem_planner::gpu_mem_witness_arm(
                        n_owned,
                        Box::new(move || {
                            // The last owned memory instance has its rows in its trace buffer: the
                            // arena goes back and the memory airs leave the device for the rest
                            // of the block.
                            zisk_common::MEM_ROWS_ON_DEVICE.store(false, Ordering::Release);
                            proofman_starks_lib_c::release_first_gpu_buffer_c(
                                d_buffers as *mut c_void,
                            );
                        }),
                    );
                    zisk_common::MEM_ROWS_ON_DEVICE.store(true, Ordering::Release);
                    true
                }
                Err(e) => {
                    // The planner already reported why on stderr when it detected the condition.
                    tracing::info!("[gpu] memory witness on the CPU for this block ({e})");
                    false
                }
            }
        }
    }
}

/// A memory instance the device could not serve after all (its preparation failed, or its fill):
/// every memory instance of the block takes the CPU witness from here on, and the arena goes back
/// now. The caller collects the instances it still has to compute.
pub fn device_mem_witness_failed(reason: &str) {
    tracing::warn!(
        "[gpu] memory witness on the device failed ({reason}); the memory instances fall back to the CPU witness"
    );
    zisk_common::MEM_ROWS_ON_DEVICE.store(false, Ordering::Release);
    #[cfg(gpu)]
    zisk_sm_mem_planner::gpu_mem_witness_release_now();
}

/// Whether the memory witness is built on the device (`ZISK_MEM_GPU_FILL`): the planner then
/// retains the block's accesses and the executor registers the secondaries in two rounds.
pub fn device_mem_witness_requested() -> bool {
    zisk_common::device_mem_witness_active()
}

/// Whether proofman's GPU layout has streaming-commit slots below the memory-ops window. The device
/// witness keeps the window borrowed until the last owned memory instance is filled, and the
/// other instances' contributions commit on the slots meanwhile; a layout without them (verify
/// constraints, debug info) runs every contribution on the basic streams the window covers and
/// would wait for the release that waits for them.
pub fn gpu_layout_has_commit_slots(d_buffers: *mut std::ffi::c_void) -> bool {
    #[cfg(gpu)]
    {
        use proofman_starks_lib_c::{get_layout_offset_c, LayoutBoundary};
        if d_buffers.is_null() {
            return false;
        }
        get_layout_offset_c(d_buffers, LayoutBoundary::MopsBase)
            > get_layout_offset_c(d_buffers, LayoutBoundary::AuxBase)
    }
    #[cfg(not(gpu))]
    {
        let _ = d_buffers;
        false
    }
}

/// Whether the memory-ops emulator runs in its light form (no values, no steps): whenever the
/// device witness, which needs them, is off.
pub fn mops_light() -> bool {
    !device_mem_witness_requested()
}

/// Releases the arena a block still holds (its memory instances were not all computed here):
/// the proof's end.
pub fn device_mem_witness_end() {
    #[cfg(gpu)]
    zisk_sm_mem_planner::gpu_mem_witness_release_now();
}

/// This struct is used to run the assembly code in a separate process and generate minimal traces.
pub struct AsmRunnerMO {
    /// The generated plans from the MO trace.
    pub plans: Vec<Plan>,
    /// Bytes of the proofman-owned GPU buffer consumed by the GPU mem-ops
    /// planner; `None` on the CPU planner path.
    pub gpu_mops_used_bytes: Option<u64>,
    /// The device memory witness of this block, when the GPU planner retained the accesses and
    /// `ZISK_MEM_GPU_FILL` asks for it.
    pub device_witness: Option<DeviceMemWitness>,
}

impl AsmRunnerMO {
    /// Creates a new `AsmRunnerMO` with the given plans (no GPU planner usage).
    pub fn new(plans: Vec<Plan>) -> Self {
        Self { plans, gpu_mops_used_bytes: None, device_witness: None }
    }

    /// Runs the assembly code in a separate process, collects the MO trace, and generates plans.
    #[allow(clippy::too_many_arguments)]
    pub fn run<R>(
        preloaded: &mut MOShmemReader,
        max_steps: u64,
        chunk_size: u64,
        on_runner_failure: R,
        asm_services: AsmServices,
        _stats: ExecutorStatsHandle,
    ) -> Result<Self>
    where
        R: FnOnce() -> Result<()>,
    {
        stats_begin!(_stats, 0, _runner_scope, "ASM_MO_RUNNER", 0);

        let sem_chunk_done_name = sem_chunk_done_name(asm_services.sem_prefix(), AsmService::MO);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let stale = crate::drain_semaphore(&mut sem_chunk_done);
        if stale > 0 {
            warn!(
                "MO semaphore '{sem_chunk_done_name}' had {stale} stale chunk_done post(s) at run start; a prior run skipped its end-side cleanup"
            );
        }

        #[cfg(gpu)]
        let shm_prefix = asm_services.shm_prefix().to_string();
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
                    anyhow::anyhow!("MOShmemReader: both mem_planner and handle_mo are None")
                })?
                .join()
                .map_err(|_| anyhow::anyhow!("MO preload background thread panicked"))?,
        };
        // A new block: the previous block's RAM witness source is gone.
        #[cfg(gpu)]
        zisk_sm_mem_planner::clear_gpu_mem_witness();
        zisk_common::MEM_ROWS_ON_DEVICE.store(false, Ordering::Release);
        // Take the optional GPU planner for this block.
        #[cfg(gpu)]
        let gpu_count_and_plan_opt: Option<GpuCountAndPlan> = preloaded.gpu_count_and_plan.take();
        #[cfg(gpu)]
        let device_witness_available = preloaded.device_witness;

        let mut data_ptr = preloaded.output_shmem.data_ptr() as *const AsmMOChunk;
        // The chunks handed to the GPU planner, for the CPU planner should the device plan fail.
        #[cfg(gpu)]
        let mut fed_chunks: Vec<(*const c_void, u64)> = Vec::new();

        #[cfg(gpu)]
        if let Some(ref gpu_count_and_plan) = gpu_count_and_plan_opt {
            gpu_count_and_plan.reset();
            register_mo_shmem_pinned(
                gpu_count_and_plan,
                &preloaded.output_shmem,
                &mut preloaded.registered_bytes,
            );
        } else {
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
        //   MAX_TRACE_CHUNK_INFO = chunk header + 3 words + 32 (asm_mt.rs)
        //   MAX_CHUNK_TRACE_SIZE = (chunk_size * MAX_BYTES_MTRACE_STEP) + MAX_TRACE_CHUNK_INFO
        // The most memory-ops bytes one step can write (constants.hpp TRACE_TARGET_MO mirrors it).
        const MAX_BYTES_MO_STEP: usize = zisk_core::MAX_MO_BYTES_PER_STEP as usize;
        let threshold_bytes = (chunk_size as usize * MAX_BYTES_MO_STEP)
            + MAX_TRACE_CHUNK_INFO
            + zisk_core::MAX_MO_INIT_DATA_MARGIN as usize;
        let mut threshold = unsafe {
            preloaded
                .output_shmem
                .mapped_ptr()
                .add(preloaded.output_shmem.total_mapped_size() - threshold_bytes)
                as *const AsmMOChunk
        };

        let mut on_runner_failure = Some(on_runner_failure);
        let mut signal_runner_failure = || {
            if let Some(on_failure) = on_runner_failure.take() {
                if let Err(reset_err) = on_failure() {
                    error!("MO on_runner_failure failed: {reset_err:#}");
                }
            }
        };

        let loop_result: Result<u64> = loop {
            match sem_chunk_done.timed_wait(SEM_CHUNK_DONE_WAIT_DURATION) {
                Ok(()) => {
                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    // Check if we need to map additional shared memory files.
                    if data_ptr >= threshold {
                        match preloaded.output_shmem.check_size_changed() {
                            Ok(true) => {
                                // Update threshold based on new total mapped size
                                threshold = unsafe {
                                    preloaded.output_shmem.mapped_ptr().add(
                                        preloaded.output_shmem.total_mapped_size()
                                            - threshold_bytes,
                                    ) as *const AsmMOChunk
                                };
                                #[cfg(gpu)]
                                if let Some(ref gpu_count_and_plan) = gpu_count_and_plan_opt {
                                    register_mo_shmem_pinned(
                                        gpu_count_and_plan,
                                        &preloaded.output_shmem,
                                        &mut preloaded.registered_bytes,
                                    );
                                }
                            }
                            Ok(false) => {}
                            Err(e) => {
                                signal_runner_failure();
                                break Err(e).context(
                                    "Failed to check and map new shared memory files for MO trace",
                                );
                            }
                        }
                    }

                    let chunk = unsafe { std::ptr::read(data_ptr) };

                    data_ptr = unsafe { data_ptr.add(1) };

                    stats_mark!(_stats, &_runner_scope, "MO_CHUNK_DONE", 0);

                    // Feed this chunk to whichever planner is active
                    #[cfg(gpu)]
                    if let Some(ref gpu_count_and_plan) = gpu_count_and_plan_opt {
                        fed_chunks.push((data_ptr as *const c_void, chunk.mem_ops_size));
                        if !gpu_count_and_plan
                            .add_chunk(chunk.mem_ops_size, data_ptr as *const c_void)
                        {
                            tracing::warn!("[gpu] the planner rejected chunk {} (n={}); the block is planned on the CPU", fed_chunks.len() - 1, chunk.mem_ops_size);
                        }
                    } else {
                        mem_planner.add_chunk(chunk.mem_ops_size, data_ptr as *const c_void);
                    }
                    #[cfg(not(gpu))]
                    mem_planner.add_chunk(chunk.mem_ops_size, data_ptr as *const c_void);

                    if chunk.end == 1 {
                        break Ok(0);
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

                    signal_runner_failure();

                    break Ok(preloaded.output_shmem.map_header().exit_code);
                }
            }
        };

        // Wind the C++ planner down before any further work. Without this,
        // its background threads stay parked waiting for more chunks, and
        // the C++ destructor blocks on `Drop`, holding the `MOShmemReader`
        // Mutex and hanging the next job's MO thread on lock acquisition.
        // In the GPU case no-op since the GPU planner has no background threads
        #[cfg(gpu)]
        let gpu_planner = gpu_count_and_plan_opt.is_some();
        #[cfg(not(gpu))]
        let gpu_planner = false;
        if !gpu_planner {
            mem_planner.set_completed();
        }

        // With the CPU witness, the streaming commits pause while the device plans: the plan is
        // what the witness waits for. With the device witness the commits run through.
        #[cfg(gpu)]
        let pause = gpu_planner && !device_mem_witness_requested();
        #[cfg(gpu)]
        if pause {
            proofman_starks_lib_c::stream_commit_pause_c();
        }

        #[cfg(gpu)]
        timer_start_info!(GPU_MOPS_TIME);
        #[cfg(gpu)]
        let mut gpu_metas_view: Option<(*const c_void, u32)> = gpu_count_and_plan_opt
            .as_ref()
            .and_then(|gpu_count_and_plan| gpu_count_and_plan.run())
            .map(|metas| (metas.as_ptr() as *const c_void, metas.len() as u32));
        #[cfg(gpu)]
        timer_stop_and_log_info!(GPU_MOPS_TIME);
        // The device could not plan the block (its reason went to stderr): the CPU planner takes
        // the same chunks, still in the shared memory, and the block keeps the CPU witness.
        #[cfg(gpu)]
        let cpu_fallback = gpu_planner && gpu_metas_view.is_none();
        #[cfg(gpu)]
        if cpu_fallback {
            tracing::warn!("[gpu] no device plan for this block; planning it on the CPU");
            timer_start_info!(CPU_MOPS_FALLBACK);
            mem_planner.execute();
            for (ptr, n) in &fed_chunks {
                mem_planner.add_chunk(*n, *ptr);
            }
            mem_planner.set_completed();
            timer_stop_and_log_info!(CPU_MOPS_FALLBACK);
            gpu_metas_view = None;
        }

        #[cfg(gpu)]
        let gpu_mops_used_bytes: Option<u64> =
            gpu_count_and_plan_opt.as_ref().map(|gcp| gcp.max_used_bytes() as u64);
        #[cfg(not(gpu))]
        let gpu_mops_used_bytes: Option<u64> = None;

        // owner: join CPU workers; no-op in GPU mode (null-guarded)
        mem_planner.wait();

        let joined = handle.join();

        crate::drain_semaphore(&mut sem_chunk_done);

        // The plan is closed: the planner can serve the block's memory witness for the instances
        // the executor will own.
        #[cfg(not(gpu))]
        let device_witness: Option<DeviceMemWitness> = None;
        #[cfg(gpu)]
        let device_witness = gpu_count_and_plan_opt.as_ref().and_then(|gpu_count_and_plan| {
            if gpu_metas_view.is_none() || !device_witness_available {
                return None;
            }
            zisk_sm_mem_planner::register_gpu_mem_witness(gpu_count_and_plan, chunk_size)
                .then(|| DeviceMemWitness { shm_prefix: shm_prefix.clone() })
        });
        #[cfg(gpu)]
        let inject_ok = match gpu_metas_view {
            Some((metas_ptr, n)) => unsafe {
                mem_planner.inject_gpu_metas_from_pointers(metas_ptr, n)
            },
            None => !gpu_planner || cpu_fallback,
        };

        let result: Result<Vec<Plan>> = (|| -> Result<Vec<Plan>> {
            let exit_code = loop_result?;
            if exit_code != 0 {
                return Err(AsmRunError::ExitCode(exit_code as u32))
                    .context("Child process returned error");
            }

            let response =
                joined.map_err(|_| AsmRunError::JoinPanic)?.map_err(AsmRunError::ServiceError)?;

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

            // The GPU planner produced no metas or they failed validation in
            // inject; the segment table is unpopulated, so fail the run rather
            // than collect a wrong plan.
            #[cfg(gpu)]
            if !inject_ok {
                return Err(anyhow::anyhow!(
                    "[gpu] GPU count-and-plan produced no metas or they were rejected; \
                     segment table is unpopulated, aborting MO run"
                ));
            }

            // The device's MemAlign plans, or the CPU worker's.
            #[cfg(gpu)]
            let mut mem_align_plans = match gpu_count_and_plan_opt.as_ref() {
                Some(gpu_count_and_plan) if !cpu_fallback => gpu_count_and_plan.build_align_plans(),
                _ => mem_planner.wait_mem_align_plans(),
            };
            #[cfg(not(gpu))]
            let mut mem_align_plans = mem_planner.wait_mem_align_plans();

            stats_end!(_stats, &_process_scope);
            stats_begin!(_stats, &_runner_scope, _collect_scope, "MO_COLLECT_PLANS", 0);
            let plans = mem_planner.collect_plans(&mut mem_align_plans);
            stats_end!(_stats, &_collect_scope);
            Ok(plans)
        })();

        // Always re-stash the planner so the next call has one to take.
        preloaded.handle_mo = Some(std::thread::spawn(move || {
            drop(mem_planner);
            MemPlanner::new()
        }));
        // Always re-stash the gpu_count_and_plan so the next call has one to take.
        #[cfg(gpu)]
        if let Some(gpu_count_and_plan) = gpu_count_and_plan_opt {
            preloaded.gpu_count_and_plan = Some(gpu_count_and_plan);
        }

        let plans = result?;

        #[cfg(feature = "save_mem_plans")]
        save_plans(&plans, "mem_plans_cpp.txt");

        stats_end!(_stats, &_runner_scope);
        Ok(AsmRunnerMO { plans, gpu_mops_used_bytes, device_witness })
    }
}
