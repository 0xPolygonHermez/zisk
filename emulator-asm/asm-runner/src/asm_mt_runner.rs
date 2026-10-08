use named_sem::NamedSemaphore;
use zisk_common::{stats_begin, stats_end, stats_mark, AsmExecutionInfo};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zisk_common::{ChunkId, EmuTrace, ExecutorStatsHandle};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::atomic::{fence, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tracing::{error, info, warn};

use zisk_common::PrecompileLogs;

use crate::{
    sem_chunk_done_name, shmem_output_name, shmem_prec_log_name, AsmMTChunk, AsmMTHeader,
    AsmMultiShmem, AsmRunError, AsmService, AsmServices, AsmShmem, PrecLogHeader,
    MAX_TRACE_CHUNK_INFO, SEM_CHUNK_DONE_WAIT_DURATION, TRACE_DELTA_SIZE, TRACE_INITIAL_SIZE,
    TRACE_MAX_SIZE,
};

use anyhow::{Context, Result};

/// This struct manages the shared memory and synchronization primitives for reading memory operation traces from the C++ side.
pub struct MTShmemReader {
    pub(crate) output_shmem: AsmMultiShmem<AsmMTHeader>,
    prec_log: PrecLogShmem,
}

/// The precompile log, as read in the current run.
struct PrecLogShmem {
    shmem: AsmShmem<PrecLogHeader>,
    /// Writable, to free the pages read.
    fd: std::os::fd::OwnedFd,
    /// Words read.
    read: usize,
    /// Bytes freed.
    freed: usize,
    /// Inconsistent: no longer parsed.
    broken: bool,
    overflowed: bool,
}

/// The first page holds the header.
const PAGE: usize = 4096;

impl MTShmemReader {
    /// Creates a new `MTShmemReader` by opening and mapping the shared memory for the MT trace output.
    pub fn new(shm_prefix: &str, unlock_mapped_memory: bool) -> Result<Self> {
        let output_name = shmem_output_name(shm_prefix, AsmService::MT, None);

        let output_shmem = AsmMultiShmem::<AsmMTHeader>::open_and_map(
            &output_name,
            TRACE_INITIAL_SIZE,
            TRACE_DELTA_SIZE,
            TRACE_MAX_SIZE,
            unlock_mapped_memory,
            false,
        )?;

        // Sparse, so not locked. Required: every process must plan the same instances.
        let prec_log_name = shmem_prec_log_name(shm_prefix);
        let shmem =
            AsmShmem::<PrecLogHeader>::open_and_map(&prec_log_name, true).with_context(|| {
                format!("No precompile input log {prec_log_name}: rebuild the ROM's assembly")
            })?;
        let fd = crate::shmem_sys::open(&prec_log_name, libc::O_RDWR)?;
        // SAFETY: just opened, owned by nothing else.
        let fd = unsafe { <std::os::fd::OwnedFd as std::os::fd::FromRawFd>::from_raw_fd(fd) };
        let prec_log =
            PrecLogShmem { shmem, fd, read: 0, freed: PAGE, broken: false, overflowed: false };

        Ok(Self { output_shmem, prec_log })
    }

    /// Starts reading a new run's log.
    fn reset_precompile_log(&mut self) {
        let log = &mut self.prec_log;
        (log.read, log.freed, log.broken, log.overflowed) = (0, PAGE, false, false);
    }

    /// Appends the new records to `logs` and frees the pages read.
    fn append_precompile_logs(&mut self, logs: &mut PrecompileLogs) {
        let log = &mut self.prec_log;
        let header_ptr = log.shmem.mapped_ptr() as *const PrecLogHeader;
        // SAFETY: aligned header word, stored with release.
        let used = unsafe {
            (*(std::ptr::addr_of!((*header_ptr).used_words) as *const AtomicU64))
                .load(Ordering::Acquire)
        } as usize;
        if !log.broken {
            if let Err(e) = Self::parse(log, used, logs) {
                error!("Precompile input log: {e}; collected by replay");
                log.broken = true;
                *logs = PrecompileLogs::default();
            }
        }

        // Also when broken: the producer keeps writing.
        let free_to = (std::mem::size_of::<PrecLogHeader>() + used * 8) / PAGE * PAGE;
        if free_to > log.freed {
            use std::os::fd::AsRawFd;
            // SAFETY: pages already read.
            let result = unsafe {
                libc::fallocate(
                    log.fd.as_raw_fd(),
                    libc::FALLOC_FL_PUNCH_HOLE | libc::FALLOC_FL_KEEP_SIZE,
                    log.freed as libc::off_t,
                    (free_to - log.freed) as libc::off_t,
                )
            };
            if result != 0 {
                warn!("Precompile input log pages not freed: {}", std::io::Error::last_os_error());
            }
            log.freed = free_to;
        }
    }

    /// Parses the records up to `used` words.
    fn parse(log: &mut PrecLogShmem, used: usize, logs: &mut PrecompileLogs) -> Result<()> {
        // A stale chunk post can show the next run's count.
        anyhow::ensure!(used >= log.read, "reset under its reader ({used} < {})", log.read);
        // SAFETY: written before the count was published.
        let words = unsafe {
            std::slice::from_raw_parts(
                (log.shmem.data_ptr() as *const u64).add(log.read),
                used - log.read,
            )
        };
        let mut at = 0;
        while at < words.len() {
            let len = words[at] as usize;
            anyhow::ensure!(
                len > zisk_common::STEP && at + 1 + len <= words.len(),
                "bad record of {len} words at word {}",
                log.read + at
            );
            logs.push(&words[at + 1..at + 1 + len]);
            at += 1 + len;
        }
        log.read = used;
        if !log.overflowed && log.shmem.map_header().capacity_words as usize == used {
            log.overflowed = true;
            warn!("Precompile input log full at {used} words; the rest is collected by replay");
        }
        Ok(())
    }
}

/// This struct is used to run the assembly code in a separate process and generate minimal traces.
pub struct AsmRunnerMT {
    /// The generated trace chunks from the MT trace.
    pub vec_chunks: Vec<EmuTrace>,
}

impl AsmRunnerMT {
    /// Creates a new `AsmRunnerMT` with the given trace chunks.
    pub fn new(vec_chunks: Vec<EmuTrace>) -> Self {
        Self { vec_chunks }
    }

    /// Runs the assembly code in a separate process, collects the MT trace chunks, and generates execution info.
    ///
    /// `on_chunk` is called on this thread once per published chunk, in chunk
    /// order, with every chunk published so far (`idx` is the last) and whether
    /// `idx` ends the execution. Returning `Err` aborts the run: the ASM
    /// children are signalled and the error is propagated to the caller.
    #[allow(clippy::too_many_arguments)]
    pub fn run_and_count<F, R>(
        preloaded: &mut MTShmemReader,
        max_steps: u64,
        chunk_size: u64,
        mut on_chunk: F,
        on_runner_failure: R,
        asm_services: AsmServices,
        _stats: ExecutorStatsHandle,
    ) -> Result<(Vec<Arc<EmuTrace>>, AsmExecutionInfo, PrecompileLogs)>
    where
        F: FnMut(usize, &[Arc<EmuTrace>], bool, &mut PrecompileLogs) -> Result<()>,
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
        //   MAX_TRACE_CHUNK_INFO        = chunk header + 3 words + 32 (asm_mt.rs)
        const MAX_MTRACE_REGS_ACCESS_SIZE: usize = (2 + 2 + 3) * 8; // 56 bytes
        const MAX_BYTES_DIRECT_MTRACE: usize = 256;
        const MAX_BYTES_MTRACE_STEP: usize = MAX_BYTES_DIRECT_MTRACE + MAX_MTRACE_REGS_ACCESS_SIZE;

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
        let mut precompile_logs = PrecompileLogs::default();
        preloaded.reset_precompile_log();

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

                    emu_traces.push(emu_trace);
                    preloaded.append_precompile_logs(&mut precompile_logs);
                    if let Err(e) =
                        on_chunk(chunk_id.0, &emu_traces, should_exit, &mut precompile_logs)
                    {
                        signal_runner_failure();
                        break Err(e.context("MT chunk callback failed"));
                    }

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

                    // The loop only ends cleanly on the chunk flagged `end`, so
                    // reaching here means the chunk stream was truncated: never
                    // report success. The header's code is preferred when it says
                    // the child failed, but an unpopulated (or "fine") header must
                    // not turn a truncated run into `Ok`.
                    break Ok(match preloaded.output_shmem.map_header().exit_code {
                        0 => 1,
                        exit_code => exit_code,
                    });
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

        // Cross-run chunk-stream desync detector: the `chunk_done` semaphore
        // carries no run epoch, so a stale post left by a prior aborted run can
        // shift THIS run's chunk stream (consumer reads leftover bytes), silently
        // producing a wrong minimal trace -> wrong instance plan ->
        // VerifyGlobalConstraints. The header's `num_chunks` is authoritative for
        // this run; if populated and it disagrees with what we consumed, a desync
        // occurred. Log-only and guarded by `!= 0` so an unpopulated header can
        // never false-fire.
        {
            let header_chunks = preloaded.output_shmem.map_header().num_chunks;
            let consumed = chunk_id.0 as u64 + 1;
            if header_chunks != 0 && header_chunks != consumed {
                error!(
                    "[CHUNK-DESYNC] MT consumed {consumed} chunks but header reports {header_chunks} — likely a stale chunk_done post from a prior run shifted the stream (wrong minimal trace)"
                );
            }
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
        Ok((emu_traces, asm_execution_info, precompile_logs))
    }
}
