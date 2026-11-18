use named_sem::NamedSemaphore;
use zisk_common::{stats_begin, stats_end, stats_mark, AsmExecutionInfo};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use zisk_common::{EmuTrace, ExecutorStatsHandle};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::atomic::{fence, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tracing::{error, info, warn};

use crate::{
    sem_chunk_done_name, shmem_output_name, AsmMTChunk, AsmMTHeader, AsmMultiShmem, AsmRunError,
    AsmService, AsmServices, ChunkEvent, EventsShmem, RegionReader, TaskChunkRead, TaskTraces,
    MAX_TRACE_CHUNK_INFO, SEM_CHUNK_DONE_WAIT_DURATION, TRACE_DELTA_SIZE, TRACE_INITIAL_SIZE,
    TRACE_MAX_SIZE,
};

use anyhow::{Context, Result};

/// Reads the minimal traces the MT service writes to its shared output segment.
pub struct MTShmemReader {
    pub(crate) output_shmem: AsmMultiShmem<AsmMTHeader>,
    /// The service's chunk events: one per post of its `chunk_done` semaphore, which says what
    /// the post is for. The service creates them with its output, at its setup
    pub(crate) events: EventsShmem,
    /// The trace objects of the fork-join regions the last run ran apart: its chunks borrow them
    pub(crate) task_traces: TaskTraces,
}

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

        // Without them the reader could not tell a fork-join task's chunk from a main one
        let events = EventsShmem::open(shm_prefix, AsmService::MT)
            .context("Failed to open the MT chunk events")?;

        Ok(Self { output_shmem, events, task_traces: TaskTraces::default() })
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
    ///
    /// The chunks of a fork-join region the service runs apart come out of order, as its tasks
    /// produce them. They are held until every task before theirs is done, and then handed on in
    /// chunk order with the steps of those tasks added to their step: `on_chunk` sees exactly the
    /// chunks a sequential run publishes.
    #[allow(clippy::too_many_arguments)]
    pub fn run_and_count<F, R>(
        preloaded: &mut MTShmemReader,
        max_steps: u64,
        chunk_size: u64,
        mut on_chunk: F,
        on_runner_failure: R,
        asm_services: AsmServices,
        _stats: ExecutorStatsHandle,
    ) -> Result<(Vec<Arc<EmuTrace>>, AsmExecutionInfo)>
    where
        F: FnMut(usize, &[Arc<EmuTrace>], bool) -> Result<()>,
        R: FnOnce() -> Result<()>,
    {
        stats_begin!(_stats, 0, _runner_scope, "ASM_MT_RUNNER", 0);

        // The previous run's chunks are gone (the executor clears them before it executes), and
        // with them the borrows of its regions' trace objects
        preloaded.task_traces.clear();

        let sem_chunk_done_name = sem_chunk_done_name(asm_services.sem_prefix(), AsmService::MT);
        // The program's prefix, which its fork-join task trace objects carry
        let program_prefix = asm_services.sem_prefix().to_string();

        let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

        let stale = crate::drain_semaphore(&mut sem_chunk_done);
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
        let mut next_event = 0usize;
        // The fork-join regions the service runs apart, whose chunks it hands on in chunk order
        let mut regions =
            RegionReader::<EmuTrace>::new(AsmService::MT, &program_prefix, chunk_size);

        let mut on_runner_failure = Some(on_runner_failure);
        let mut signal_runner_failure = || {
            if let Some(on_failure) = on_runner_failure.take() {
                if let Err(reset_err) = on_failure() {
                    error!("MT on_runner_failure failed: {reset_err:#}");
                }
            }
        };

        let loop_result: Result<u64> = loop {
            match crate::wait_for_chunk(
                &mut sem_chunk_done,
                || handle.is_finished(),
                SEM_CHUNK_DONE_WAIT_DURATION,
            ) {
                Ok(()) => {
                    stats_mark!(_stats, &_runner_scope, "MT_CHUNK_DONE", 0);

                    // Synchronize with memory changes from the C++ side
                    fence(Ordering::Acquire);

                    let event = match preloaded.events.read(next_event, || handle.is_finished()) {
                        Ok(Some(event)) => event,
                        Ok(None) => {
                            // The service ended the run with this event's slot reserved but never
                            // written (a task's process stopped while publishing it): the stream
                            // stopped, and the header says how the run failed
                            error!(
                                "MT chunk stream on '{sem_chunk_done_name}' stopped: the service ended the run with chunk event {next_event} never written"
                            );
                            signal_runner_failure();
                            break Ok(crate::stopped_stream_exit_code(
                                preloaded.output_shmem.map_header().exit_code,
                            ));
                        }
                        Err(e) => {
                            signal_runner_failure();
                            break Err(e.context("Failed to read the MT chunk events"));
                        }
                    };
                    next_event += 1;

                    let handled: Result<bool> = (|| {
                        match event {
                            ChunkEvent::Main => {
                                if regions.in_region() {
                                    anyhow::bail!("a main chunk inside a fork-join region");
                                }
                                // Check if we need to map additional shared memory files.
                                if data_ptr >= threshold
                                    && preloaded.output_shmem.check_size_changed().context(
                                        "Failed to check and map new shared memory files for MT trace",
                                    )?
                                {
                                    // Update threshold based on new total mapped size
                                    threshold = unsafe {
                                        preloaded.output_shmem.mapped_ptr().add(
                                            preloaded.output_shmem.total_mapped_size()
                                                - threshold_bytes,
                                        ) as *const AsmMTChunk
                                    };
                                }

                                let emu_trace = Arc::new(AsmMTChunk::to_emu_trace(&mut data_ptr));
                                let should_exit = emu_trace.end;
                                emu_traces.push(emu_trace);
                                on_chunk(emu_traces.len() - 1, &emu_traces, should_exit)
                                    .context("MT chunk callback failed")?;
                                Ok(should_exit)
                            }
                            // A region's chunk, handed on in chunk order with the steps of the
                            // tasks before its task added to its step, and the next `ChunkId`
                            region_event => {
                                regions.follow(
                                    region_event,
                                    &mut preloaded.task_traces,
                                    |at| {
                                        let mut chunk = at as *const AsmMTChunk;
                                        let emu_trace = AsmMTChunk::to_emu_trace(&mut chunk);
                                        let end = emu_trace.end;
                                        TaskChunkRead { item: emu_trace, end }
                                    },
                                    |shift, mut emu_trace| {
                                        emu_trace.start_state.step += shift;
                                        emu_traces.push(Arc::new(emu_trace));
                                        on_chunk(emu_traces.len() - 1, &emu_traces, false)
                                            .context("MT chunk callback failed")
                                    },
                                )?;
                                Ok(false)
                            }
                        }
                    })();

                    match handled {
                        Ok(true) => break Ok(0),
                        Ok(false) => {}
                        Err(e) => {
                            signal_runner_failure();
                            break Err(e);
                        }
                    }
                }
                Err(stop) => {
                    error!("MT chunk stream on '{sem_chunk_done_name}' stopped: {stop}");

                    signal_runner_failure();

                    // The loop only ends cleanly on the chunk flagged `end`, so
                    // reaching here means the chunk stream was truncated: never
                    // report success. The header's code is preferred when it says
                    // the child failed, but an unpopulated (or "fine") header must
                    // not turn a truncated run into `Ok`.
                    break Ok(crate::stopped_stream_exit_code(
                        preloaded.output_shmem.map_header().exit_code,
                    ));
                }
            }
        };

        let join_outcome = handle.join();

        crate::drain_semaphore(&mut sem_chunk_done);

        let (handle, elapsed) = join_outcome.map_err(|_| AsmRunError::JoinPanic)?;
        let exit_code = loop_result?;

        if exit_code != 0 {
            // A service that died says how in its answer: that is the cause to report.
            if let Err(e) = handle {
                return Err(AsmRunError::ServiceError(e)).with_context(|| {
                    format!("MT stream ended before its last chunk (exit code {exit_code})")
                });
            }
            return Err(AsmRunError::from_exit_code(exit_code, preloaded.events.report().failure))
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
            let consumed = emu_traces.len() as u64;
            if header_chunks != 0 && header_chunks != consumed {
                error!(
                    "[CHUNK-DESYNC] MT consumed {consumed} chunks but header reports {header_chunks} — likely a stale chunk_done post from a prior run shifted the stream (wrong minimal trace)"
                );
            }
        }

        let total_steps = emu_traces.iter().map(|x| x.steps).sum::<u64>();
        let mhz = (total_steps as f64 / elapsed.as_secs_f64()) / 1_000_000.0;
        // What the service says of the regions it did not run apart
        let report = preloaded.events.report();
        report.log_sequential();
        let asm_execution_info = AsmExecutionInfo {
            time: elapsed.as_secs_f32(),
            mhz: mhz as f32,
            fork_join_regions_apart: regions.regions_apart(),
            fork_join_regions_sequential: report.regions_sequential,
        };
        let regions_apart = regions.regions_apart();
        if regions_apart > 0 {
            info!(
                "··· Assembly execution speed: {}MHz ({:2?}), {regions_apart} fork-join region(s) run apart",
                mhz.round(),
                elapsed
            );
        } else {
            info!("··· Assembly execution speed: {}MHz ({:2?})", mhz.round(), elapsed);
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AsmRunnerOptions, AsmServices};

    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    /// A program whose services are stand-ins: shell scripts that answer pings, run
    /// `on_minimal_trace` on a minimal-trace request, and exit on anything else. Its
    /// directory goes when it is dropped.
    struct FakeProgram {
        dir: PathBuf,
    }

    impl FakeProgram {
        fn new(name: &str, on_minimal_trace: &str) -> Self {
            let script = format!(
                r#"#!/bin/sh
for arg in "$@"; do case "$arg" in --just_create*) exit 0;; esac; done
while :; do
  id=$(head -c 40 | od -An -tu8 -N8 | tr -d ' ')
  [ -n "$id" ] || exit 0
  case "$id" in
    1) printf '\002'; head -c 39 /dev/zero ;;
    3) {on_minimal_trace} ;;
    *) exit 0 ;;
  esac
done
"#
            );
            let dir = std::env::temp_dir()
                .join(format!("zisk_unittest_mt_runner_{}_{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            // Written by a child process, so that no child another test forks meanwhile
            // inherits a descriptor open for writing on it, which would fail its exec.
            for service in AsmServices::SERVICES {
                let path = service.command_path_for(&dir.join("prog").to_string_lossy());
                let mut writer = Command::new("sh")
                    .args(["-c", r#"cat > "$0" && chmod 755 "$0""#])
                    .arg(&path)
                    .stdin(Stdio::piped())
                    .spawn()
                    .unwrap();
                writer.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
                assert!(writer.wait().unwrap().success(), "write {path}");
            }
            Self { dir }
        }
    }

    impl Drop for FakeProgram {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Runs a minimal trace of `program` on rank `rank`'s segments, as a job does, and
    /// returns its error and how long the run took to fail.
    fn failed_run(program: &FakeProgram, rank: i32) -> (anyhow::Error, Duration) {
        failed_run_with(program, rank, 0, None)
    }

    /// As [`failed_run`], with `exit_code` in the output's header, where the service leaves
    /// it before it answers, and, `post` after the run starts, a post of the `chunk_done`
    /// semaphore whose event is never written, as a task's process killed while it publishes
    /// one leaves it.
    fn failed_run_with(
        program: &FakeProgram,
        rank: i32,
        exit_code: u64,
        post: Option<Duration>,
    ) -> (anyhow::Error, Duration) {
        let (services, _claim) = AsmServices::new(
            0,
            rank,
            "f".repeat(64),
            &program.dir.join("prog-mt.bin"),
            false,
            AsmRunnerOptions::new(),
        )
        .expect("the stand-in services start");

        // The output the MT service would write, with no chunk in it. The segments' last
        // owner, `services`, unlinks it.
        let size = 1 << 20;
        let name = shmem_output_name(services.shm_prefix(), AsmService::MT, Some(0));
        let c = std::ffi::CString::new(name.as_str()).unwrap();
        unsafe {
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0);
            assert_eq!(libc::ftruncate(fd, size as libc::off_t), 0);
            libc::close(fd);
        }
        let header = crate::ShmemWriter::new(&name, size, true).unwrap();
        header
            .write_u64_at(std::mem::offset_of!(AsmMTHeader, shmem_allocated_size), size as u64)
            .unwrap();
        header.write_u64_at(std::mem::offset_of!(AsmMTHeader, exit_code), exit_code).unwrap();
        drop(header);
        // The chunk events the MT service creates at its setup; this run publishes none
        let events = crate::shmem_events_name(services.shm_prefix(), AsmService::MT);
        let events_c = std::ffi::CString::new(events.as_str()).unwrap();
        unsafe {
            let fd = libc::shm_open(events_c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0);
            assert_eq!(libc::ftruncate(fd, crate::PAR_EVENTS_BYTES as libc::off_t), 0);
            libc::close(fd);
        }
        crate::ShmemWriter::new(&events, crate::PAR_EVENTS_BYTES, true)
            .unwrap()
            .write_u64_at(0, crate::PAR_EVENTS_VERSION)
            .unwrap();
        let output = shmem_output_name(services.shm_prefix(), AsmService::MT, None);
        let mut reader = MTShmemReader {
            output_shmem: AsmMultiShmem::open_and_map(&output, size, size, size * 4, true, false)
                .unwrap(),
            events: EventsShmem::open(services.shm_prefix(), AsmService::MT).unwrap(),
            task_traces: TaskTraces::default(),
        };

        let poster = post.map(|after| {
            let name = sem_chunk_done_name(services.sem_prefix(), AsmService::MT);
            std::thread::spawn(move || {
                std::thread::sleep(after);
                NamedSemaphore::create(name, 0).unwrap().post().unwrap();
            })
        });
        let start = Instant::now();
        let Err(error) = AsmRunnerMT::run_and_count(
            &mut reader,
            1 << 20,
            1,
            |_, _, _| Ok(()),
            || Ok(()),
            services,
            ExecutorStatsHandle::new(),
        ) else {
            panic!("a run that publishes no chunk must fail");
        };
        if let Some(poster) = poster {
            poster.join().unwrap();
        }
        unsafe { libc::shm_unlink(events_c.as_ptr()) };
        (error, start.elapsed())
    }

    /// A run whose emulation faults is answered with a failed result and no last chunk,
    /// which is what a caught fault does: it fails as soon as it is answered, not after
    /// the chunk wait's stall limit.
    #[test]
    fn a_run_answered_without_its_last_chunk_fails_at_once() {
        let program = FakeProgram::new(
            "fault",
            r"printf '\004'; head -c 7 /dev/zero; printf '\001'; head -c 31 /dev/zero",
        );
        let (error, took) = failed_run(&program, 111);
        assert!(took < SEM_CHUNK_DONE_WAIT_DURATION / 5, "failed after {took:?}: {error:#}");
        assert!(
            error
                .chain()
                .any(|e| matches!(e.downcast_ref::<AsmRunError>(), Some(AsmRunError::ExitCode(1)))),
            "{error:#}"
        );
    }

    /// A run the service answers with an event it posted never written, as when a task's
    /// process is killed between reserving its slot and filling it, fails with what the
    /// service's header says, here the guest ending inside a fork-join task, and as soon as it
    /// is answered.
    #[test]
    fn a_run_answered_with_an_event_never_written_keeps_its_failure() {
        let program = FakeProgram::new(
            "unwritten",
            r"sleep 0.6; printf '\004'; head -c 7 /dev/zero; printf '\001'; head -c 31 /dev/zero",
        );
        let post = Some(Duration::from_millis(200));
        let (error, took) = failed_run_with(&program, 113, crate::PAR_ABORT_TASK_ENDED, post);
        assert!(took < SEM_CHUNK_DONE_WAIT_DURATION / 5, "failed after {took:?}: {error:#}");
        assert!(
            error.chain().any(|e| matches!(
                e.downcast_ref::<AsmRunError>(),
                Some(AsmRunError::ForkJoinTask { failure: crate::TaskFailure::Ended, .. })
            )),
            "{error:#}"
        );
    }

    /// A service killed during a run fails it as soon as its pipe says so, and the error
    /// says it died.
    #[test]
    fn a_run_whose_service_dies_fails_at_once_and_says_why() {
        let program = FakeProgram::new("crash", "kill -KILL $$");
        let (error, took) = failed_run(&program, 112);
        assert!(took < SEM_CHUNK_DONE_WAIT_DURATION / 5, "failed after {took:?}: {error:#}");
        assert!(
            error.chain().any(|e| matches!(
                e.downcast_ref::<AsmRunError>(),
                Some(AsmRunError::ServiceDied { .. })
            )),
            "{error:#}"
        );
    }
}
