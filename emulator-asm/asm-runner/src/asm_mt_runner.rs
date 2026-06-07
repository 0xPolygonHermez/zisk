use named_sem::NamedSemaphore;
use zisk_common::{stats_begin, stats_end, stats_mark, AsmExecutionInfo};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zisk_common::{ChunkId, EmuTrace, ExecutorStatsHandle};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::atomic::{fence, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tracing::{error, info, warn};

use crate::{
    sem_chunk_done_name, shmem_output_name, AsmMTChunk, AsmMTHeader, AsmMultiSharedMemory,
    AsmRunError, AsmService, AsmServices, SEM_CHUNK_DONE_WAIT_DURATION, TRACE_DELTA_SIZE,
    TRACE_INITIAL_SIZE, TRACE_MAX_SIZE,
};

use anyhow::{Context, Result};

pub struct MTShMemReader {
    pub(crate) output_shmem: AsmMultiSharedMemory<AsmMTHeader>,
}

impl MTShMemReader {
    pub fn new(shm_prefix: &str, unlock_mapped_memory: bool) -> Result<Self> {
        let output_name = shmem_output_name(shm_prefix, AsmService::MT, None);

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
    pub fn run_and_count<F, R>(
        preloaded: &mut MTShMemReader,
        max_steps: u64,
        chunk_size: u64,
        mut on_chunk: F,
        on_runner_failure: R,
        asm_services: AsmServices,
        _stats: ExecutorStatsHandle,
    ) -> Result<(Vec<Arc<EmuTrace>>, AsmExecutionInfo)>
    where
        F: FnMut(usize, Arc<EmuTrace>),
        R: FnOnce() -> Result<()>,
    {
        stats_begin!(_stats, 0, _runner_scope, "ASM_MT_RUNNER", 0);

        let sem_chunk_done_name = sem_chunk_done_name(asm_services.sem_prefix(), AsmService::MT);

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let stale = crate::drain_chunk_done(&mut sem_chunk_done);
        if stale > 0 {
            warn!(
                "MT semaphore '{sem_chunk_done_name}' had {stale} stale chunk_done post(s) at run start; a prior run skipped its end-side cleanup"
            );
        }

        // Capture parent id for thread
        let _parent_id = _runner_scope.id();
        let _thread_stats = _stats.clone();
        let handle = std::thread::spawn(move || {
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

        let mut on_runner_failure = Some(on_runner_failure);
        let mut signal_runner_failure = || {
            if let Some(on_failure) = on_runner_failure.take() {
                if let Err(reset_err) = on_failure() {
                    error!("MT on_runner_failure failed: {reset_err:#}");
                }
            }
        };

        let loop_result: Result<u64> = loop {
            match sem_chunk_done.timed_wait(SEM_CHUNK_DONE_WAIT_DURATION) {
                Ok(()) => {
                    stats_mark!(_stats, &_runner_scope, "MT_CHUNK_DONE", 0);

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
                                    ) as *const AsmMTChunk
                                };
                            }
                            Ok(false) => {}
                            Err(e) => {
                                signal_runner_failure();
                                break Err(e).context(
                                    "Failed to check and map new shared memory files for MT trace",
                                );
                            }
                        }
                    }

                    let emu_trace = Arc::new(AsmMTChunk::to_emu_trace(&mut data_ptr));
                    let should_exit = emu_trace.end;

                    on_chunk(chunk_id.0, emu_trace.clone());
                    emu_traces.push(emu_trace);

                    if should_exit {
                        break Ok(0);
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

                    signal_runner_failure();

                    if chunk_id.0 == 0 {
                        break Ok(1);
                    }

                    break Ok(preloaded.output_shmem.map_header().exit_code);
                }
            }
        };

        let join_outcome = handle.join();

        crate::drain_chunk_done(&mut sem_chunk_done);

        let (handle, elapsed) = join_outcome.map_err(|_| AsmRunError::JoinPanic)?;
        let exit_code = loop_result?;

        if exit_code != 0 {
            return Err(AsmRunError::ExitCode(exit_code as u32))
                .context("Child process returned error");
        }

        let total_steps = emu_traces.iter().map(|x| x.steps).sum::<u64>();
        let mhz = (total_steps as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
        let asm_execution_info = AsmExecutionInfo { time: elapsed.as_secs_f32(), mhz: mhz as f32 };
        info!("··· Assembly execution speed: {}MHz ({:2?})", mhz.round(), elapsed);

        let response = handle.map_err(AsmRunError::ServiceError)?;

        if response.result != 0 {
            return Err(anyhow::anyhow!(
                "ASM MT service returned non-zero result: {}",
                response.result
            ));
        }
        if response.trace_len == 0 {
            return Err(anyhow::anyhow!("ASM MT service returned empty trace"));
        }
        if response.trace_len > response.allocated_len {
            return Err(anyhow::anyhow!(
                "ASM MT service trace_len ({}) exceeds allocated_len ({})",
                response.trace_len,
                response.allocated_len
            ));
        }

        stats_end!(_stats, &_runner_scope);
        Ok((emu_traces, asm_execution_info))
    }
}
