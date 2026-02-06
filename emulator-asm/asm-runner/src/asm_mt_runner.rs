use named_sem::NamedSemaphore;
use zisk_common::{stats_begin, stats_end, stats_mark};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zisk_common::{ChunkId, EmuTrace, ExecutorStatsHandle};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::atomic::{fence, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tracing::{error, info};

use crate::{
    sem_chunk_done_name, shmem_output_name, AsmMTChunk, AsmMTHeader, AsmMultiSharedMemory,
    AsmRunError, AsmService, AsmServices, SEM_CHUNK_DONE_WAIT_DURATION, TRACE_DELTA_SIZE,
    TRACE_INITIAL_SIZE, TRACE_MAX_SIZE,
};

use anyhow::{Context, Result};

pub struct MTOutputShmem {
    pub output_shmem: AsmMultiSharedMemory<AsmMTHeader>,
}

impl MTOutputShmem {
    pub fn new(
        local_rank: i32,
        base_port: Option<u16>,
        unlock_mapped_memory: bool,
    ) -> Result<Self> {
        let port = AsmServices::port_base_for(base_port, local_rank);

        let output_name = shmem_output_name(port, AsmService::MT, local_rank, None);

        let output_shmem = AsmMultiSharedMemory::<AsmMTHeader>::open_and_map(
            &output_name,
            TRACE_INITIAL_SIZE,
            TRACE_DELTA_SIZE,
            TRACE_MAX_SIZE,
            unlock_mapped_memory,
        )?;

        Ok(Self { output_shmem })
    }
}

// This struct is used to run the assembly code in a separate process and generate minimal traces.
pub struct AsmRunnerMT {
    pub vec_chunks: Vec<EmuTrace>,
}

impl AsmRunnerMT {
    pub fn new(vec_chunks: Vec<EmuTrace>) -> Self {
        Self { vec_chunks }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_and_count<F: FnMut(usize, Arc<EmuTrace>)>(
        preloaded: &mut MTOutputShmem,
        max_steps: u64,
        chunk_size: u64,
        mut on_chunk: F,
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        _stats: ExecutorStatsHandle,
    ) -> Result<Vec<Arc<EmuTrace>>> {
        stats_begin!(_stats, 0, _runner_scope, "ASM_MT_RUNNER", 0);

        let port = AsmServices::port_base_for(base_port, local_rank);

        let sem_chunk_done_name = sem_chunk_done_name(port, AsmService::MT, local_rank);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        // Capture parent id for thread
        let _parent_id = _runner_scope.id();
        let _thread_stats = _stats.clone();
        let handle = std::thread::spawn(move || {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);

            stats_begin!(_thread_stats, _parent_id, _mt_scope, "ASM_MT", 0);
            let start = Instant::now();
            let result = asm_services.send_minimal_trace_request(max_steps, chunk_size);

            stats_end!(_thread_stats, &_mt_scope);

            (result, start.elapsed())
        });

        let mut chunk_id = ChunkId(0);

        // Get the pointer to the data in the shared memory.
        let mut data_ptr = preloaded.output_shmem.data_ptr() as *const AsmMTChunk;

        // Calculate threshold for detecting when to map additional shared memory files.
        // CRITICAL: These constants must match main.c to ensure we check for new files BEFORE
        // the C++ producer needs to allocate beyond current mapped region. Mismatch will cause
        // the producer to map new files while we still hold Cow::Borrowed references to old
        // mappings, creating dangling pointers.
        //
        // Constants from main.c:
        //   MAX_MTRACE_REGS_ACCESS_SIZE = (2 + 2 + 3) * 8    // Register access overhead per step
        //   MAX_BYTES_DIRECT_MTRACE     = 256                // Direct memory trace data per step
        //   MAX_BYTES_MTRACE_STEP       = 256 + 56 = 312     // Total per-step overhead
        //   MAX_TRACE_CHUNK_INFO        = (44 * 8) + 32      // Chunk metadata size
        const MAX_MTRACE_REGS_ACCESS_SIZE: usize = (2 + 2 + 3) * 8; // 56 bytes
        const MAX_BYTES_DIRECT_MTRACE: usize = 256;
        const MAX_BYTES_MTRACE_STEP: usize = MAX_BYTES_DIRECT_MTRACE + MAX_MTRACE_REGS_ACCESS_SIZE;
        const MAX_TRACE_CHUNK_INFO: usize = (44 * 8) + 32; // 384 bytes

        let threshold_bytes = (chunk_size as usize * MAX_BYTES_MTRACE_STEP) + MAX_TRACE_CHUNK_INFO;
        let mut threshold = unsafe {
            preloaded
                .output_shmem
                .mapped_ptr()
                .add(preloaded.output_shmem.total_mapped_size() - threshold_bytes)
                as *const AsmMTChunk
        };

        // Pre-allocate reasonable initial capacity to avoid early reallocations
        let mut emu_traces: Vec<Arc<EmuTrace>> = Vec::with_capacity(1024);

        let exit_code = loop {
            match sem_chunk_done.timed_wait(SEM_CHUNK_DONE_WAIT_DURATION) {
                Ok(()) => {
                    stats_mark!(_stats, &_runner_scope, "MT_CHUNK_DONE", 0);

                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    // Check if we need to map additional shared memory files.
                    if data_ptr >= threshold
                        && preloaded.output_shmem.check_size_changed().context(
                            "Failed to check and map new shared memory files for MT trace",
                        )?
                    {
                        // Update threshold based on new total mapped size
                        threshold =
                            unsafe {
                                preloaded.output_shmem.mapped_ptr().add(
                                    preloaded.output_shmem.total_mapped_size() - threshold_bytes,
                                ) as *const AsmMTChunk
                            };
                    }

                    let emu_trace = Arc::new(AsmMTChunk::to_emu_trace(&mut data_ptr));
                    let should_exit = emu_trace.end;

                    on_chunk(chunk_id.0, emu_trace.clone());
                    emu_traces.push(emu_trace);

                    if should_exit {
                        break 0;
                    }
                    chunk_id.0 += 1;
                }
                Err(named_sem::Error::WaitFailed(e))
                    if e.kind() == std::io::ErrorKind::Interrupted =>
                {
                    continue
                }
                Err(e) => {
                    error!("Semaphore '{}' error: {:?}", sem_chunk_done_name, e);

                    if chunk_id.0 == 0 {
                        break 1;
                    }

                    break preloaded.output_shmem.map_header().exit_code;
                }
            }
        };

        if exit_code != 0 {
            return Err(AsmRunError::ExitCode(exit_code as u32))
                .context("Child process returned error");
        }

        // Wait for the assembly emulator to complete writing the trace
        let (handle, elapsed) = handle.join().map_err(|_| AsmRunError::JoinPanic)?;

        let total_steps = emu_traces.iter().map(|x| x.steps).sum::<u64>();
        let mhz = (total_steps as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
        info!("··· Assembly execution speed: {}MHz ({:2?})", mhz.round(), elapsed);

        let response = handle.map_err(AsmRunError::ServiceError)?;

        assert_eq!(response.result, 0);
        assert!(response.trace_len > 0);
        assert!(response.trace_len <= response.allocated_len);

        stats_end!(_stats, &_runner_scope);
        Ok(emu_traces)
    }
}
