//! The `asm-runner` crate provides the core logic for managing the assembly runner process,
//! including shared memory management, synchronization primitives,
//! and communication with the C++ side of the emulator.
//! It defines the main types and functions used by the assembly runner to execute assembly code
//! and interact with the rest of the emulator.

#![warn(missing_docs)]
#![warn(rustdoc::all)]
#![deny(rustdoc::missing_crate_level_docs)]
// On non-Linux-x86_64 the ASM runner is a stub shell: the real runners and the
// shared-memory machinery they drive are `cfg`-gated out, so most of the crate's
// types/fns have no caller there. Silence the resulting dead-code/unused-import
// noise off the supported platform; Linux-x86_64 keeps full lint enforcement.
#![cfg_attr(
    not(all(target_os = "linux", target_arch = "x86_64")),
    allow(dead_code, unused_imports)
)]

extern crate libc;

mod asm_mo;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_mo_runner;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_mo_runner_stub;
mod asm_mt;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_mt_runner;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_mt_runner_stub;
mod asm_rh;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod asm_rh_runner;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod asm_rh_runner_stub;
mod asm_runner;
mod asm_services;
mod control_shmem;
mod hints_file;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod hints_shmem;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod hints_shmem_stub;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod inputs_shmem;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod inputs_shmem_stub;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod multi_shmem;
mod naming;
mod shmem_reader;
mod shmem_sys;
mod shmem_utils;
mod shmem_writer;

// Internal layout/header structs — not part of the public API.
pub(crate) use asm_mo::*;
pub use asm_mo::{GpuBufferSource, OwnedMemInstances};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_mo_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_mo_runner_stub::*;
pub(crate) use asm_mt::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_mt_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_mt_runner_stub::*;
// `AsmRHData` is public API (read by sm-rom); `AsmRHHeader` is an internal layout struct.
pub use asm_rh::AsmRHData;
pub(crate) use asm_rh::AsmRHHeader;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use asm_rh_runner::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use asm_rh_runner_stub::*;
pub use asm_runner::*;
pub use asm_services::*;
pub use control_shmem::*;
// `HintsFile` is a public file-based StreamSink alternative to HintsShmem.
pub use hints_file::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use hints_shmem::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use hints_shmem_stub::*;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub use inputs_shmem::*;
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub use inputs_shmem_stub::*;
// Low-level shmem primitives + naming — crate-internal, not part of the public API.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use multi_shmem::*;
pub(crate) use naming::*;
pub(crate) use shmem_reader::*;
pub(crate) use shmem_utils::*;
pub(crate) use shmem_writer::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_INITIAL_SIZE: usize = 0x180000000; // 6GB
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_DELTA_SIZE: usize = 0x080000000; // 2GB
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) const TRACE_MAX_SIZE: usize = 0x1000000000; // 64GB

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const SEM_CHUNK_DONE_WAIT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

/// How long a runner waits for a chunk before it looks whether the service has answered
/// the request; see [`wait_for_chunk`].
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const SEM_CHUNK_DONE_POLL: std::time::Duration = std::time::Duration::from_millis(50);

/// Why a service's chunk stream stopped before its last chunk; see [`wait_for_chunk`].
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[derive(Debug)]
pub(crate) enum ChunkStop {
    /// The service answered the request with no chunk left to read: the run failed (a
    /// caught fault posts nothing) or the service died.
    Answered,
    /// Neither a chunk nor an answer came in time, or the wait itself failed.
    Stalled(named_sem::Error),
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl std::fmt::Display for ChunkStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkStop::Answered => write!(f, "the service answered without its last chunk"),
            ChunkStop::Stalled(e) => write!(f, "no chunk and no answer: {e}"),
        }
    }
}

/// The marks a service's emulation leaves in its control output segment while it blocks on
/// data the job has not delivered yet: the C side's `waiting_for_precompile_address` (hints)
/// and `waiting_for_input_address` (input). Each holds `counter << 1` while the emulation
/// waits and `(counter << 1) + 1` once it wakes, and `server_reset_fast` zeroes both after
/// every run.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) struct DataWaits {
    control: ShmemReader,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl DataWaits {
    /// The size of a service's control output segment (`CONTROL_OUTPUT_SIZE`).
    const CONTROL_OUTPUT_SIZE: usize = 0x1000;
    /// Byte offsets of the marks (`shmem_control_output_address[1]` and `[2]`).
    const WAITING_FOR_HINTS: usize = 8;
    const WAITING_FOR_INPUT: usize = 16;

    /// Map `service`'s control output segment. Without it a runner cannot tell a service that
    /// waits for its data from a stuck one, and takes every silence past the stall limit for
    /// a stall, so a failure to map it is only warned about.
    pub(crate) fn open(shm_prefix: &str, service: AsmService) -> Option<Self> {
        let name = shmem_control_output_name(shm_prefix, service);
        match ShmemReader::new(&name, Self::CONTROL_OUTPUT_SIZE) {
            Ok(control) => Some(Self { control }),
            Err(e) => {
                tracing::warn!("Cannot map '{name}' to see {service} wait for data: {e}");
                None
            }
        }
    }

    /// Whether the emulation is blocked now on input or hints that have not arrived.
    pub(crate) fn waiting(&self) -> bool {
        [Self::WAITING_FOR_HINTS, Self::WAITING_FOR_INPUT].into_iter().any(|offset| {
            let mark = self.control.read_u64_at(offset);
            mark != 0 && mark % 2 == 0
        })
    }
}

/// Wait for the next `chunk_done` post of a request, which `answered` says whether the
/// service has answered.
///
/// A service posts each chunk of a request before it answers it: the emulation that posts
/// them returns before the response is written, on the same thread, in every version of
/// the services. So once the request is answered, every post it made is already counted,
/// and a wait that finds none left means the stream will never end. A failed run or a dead
/// service then surfaces within [`SEM_CHUNK_DONE_POLL`] of its answer; `stall` stays the
/// limit for a service that neither posts nor answers.
///
/// Time the service spends waiting for data, which `waiting` says, does not count towards
/// `stall`: a job's input or hints arrive at the sender's pace, and a guest that reads its
/// whole input first produces no chunk until all of it has arrived. A job whose data stops
/// arriving is ended by whatever ends the job, a cancel or a failure, whose reset answers
/// the request.
///
/// The same order keeps a request's posts from reaching the next one: a runner stops
/// reading only once its request is answered or the wait has failed, and in both cases
/// joins the request, which ends with the answer, before it drains the semaphore.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn wait_for_chunk(
    sem: &mut named_sem::NamedSemaphore,
    answered: impl Fn() -> bool,
    waiting: impl Fn() -> bool,
    stall: std::time::Duration,
) -> Result<(), ChunkStop> {
    let mut deadline = std::time::Instant::now() + stall;
    loop {
        match sem.timed_wait(SEM_CHUNK_DONE_POLL.min(stall)) {
            Ok(()) => return Ok(()),
            Err(named_sem::Error::WaitTimeout) => {}
            Err(named_sem::Error::WaitFailed(e)) if e.kind() == std::io::ErrorKind::Interrupted => {
                // A signal, not a timeout: the checks below apply all the same.
            }
            Err(e) => return Err(ChunkStop::Stalled(e)),
        }
        if answered() {
            // Every post the request made is counted by now: take one if any is left.
            return match sem.try_wait() {
                Ok(()) => Ok(()),
                Err(named_sem::Error::WouldBlock) => Err(ChunkStop::Answered),
                Err(e) => Err(ChunkStop::Stalled(e)),
            };
        }
        if waiting() {
            deadline = std::time::Instant::now() + stall;
        } else if std::time::Instant::now() >= deadline {
            return Err(ChunkStop::Stalled(named_sem::Error::WaitTimeout));
        }
    }
}

/// Consume every pending post on `sem` and return how many there were.
///
/// A named semaphore outlives the run that posted to it, so a run that was
/// aborted between a post and its matching wait leaves a count behind that would
/// be miscounted as a fresh signal. `sem_open` does not reset an existing
/// semaphore's value — the initial value is ignored once the name exists — so
/// re-opening is not enough and the count has to be swept explicitly.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) fn drain_semaphore(sem: &mut named_sem::NamedSemaphore) -> u64 {
    let mut swept = 0;
    while sem.try_wait().is_ok() {
        swept += 1;
    }
    swept
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use super::*;
    use named_sem::NamedSemaphore;

    #[test]
    fn drain_semaphore_sweeps_all_pending_posts() {
        let name = format!("/ZISK_unittest_drain_{}", std::process::id());
        let mut sem = NamedSemaphore::create(&name, 0).unwrap();
        for _ in 0..3 {
            sem.post().unwrap();
        }
        assert_eq!(drain_semaphore(&mut sem), 3, "should sweep the 3 pending posts");
        assert_eq!(drain_semaphore(&mut sem), 0, "nothing left to sweep");

        let c = std::ffi::CString::new(name).unwrap();
        unsafe { libc::sem_unlink(c.as_ptr()) };
    }

    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    /// A `chunk_done` semaphore, unlinked when dropped.
    struct ChunkDone {
        name: String,
        sem: NamedSemaphore,
    }

    impl ChunkDone {
        fn new(tag: &str) -> Self {
            let name = format!("/ZISK_unittest_chunk_{}_{tag}", std::process::id());
            Self { sem: NamedSemaphore::create(&name, 0).unwrap(), name }
        }

        /// A service's request on its own thread: it posts `chunks` chunks, `gap` apart, and
        /// then answers, which is the thread's end.
        fn request(&self, chunks: usize, gap: Duration) -> JoinHandle<()> {
            let name = self.name.clone();
            thread::spawn(move || {
                let mut sem = NamedSemaphore::create(&name, 0).unwrap();
                for _ in 0..chunks {
                    thread::sleep(gap);
                    sem.post().unwrap();
                }
            })
        }

        /// Reads the request's chunks as a runner does: how many it read, and why it stopped.
        fn read(&mut self, request: &JoinHandle<()>, stall: Duration) -> (usize, ChunkStop) {
            self.read_waiting(request, || false, stall)
        }

        /// As [`read`](Self::read), for a service that `waiting` says waits for data.
        fn read_waiting(
            &mut self,
            request: &JoinHandle<()>,
            waiting: impl Fn() -> bool,
            stall: Duration,
        ) -> (usize, ChunkStop) {
            let mut read = 0;
            loop {
                match wait_for_chunk(&mut self.sem, || request.is_finished(), &waiting, stall) {
                    Ok(()) => read += 1,
                    Err(stop) => return (read, stop),
                }
            }
        }
    }

    impl Drop for ChunkDone {
        fn drop(&mut self) {
            let c = std::ffi::CString::new(self.name.as_str()).unwrap();
            unsafe { libc::sem_unlink(c.as_ptr()) };
        }
    }

    /// A request answered without its last chunk, as on a caught fault or a dead service,
    /// stops the stream about when it is answered, not at the stall limit, and only after
    /// every chunk it posted has been read, slow ones included.
    #[test]
    fn an_answered_request_stops_its_stream_once_its_chunks_are_read() {
        let mut chunk_done = ChunkDone::new("answered");
        let request = chunk_done.request(3, SEM_CHUNK_DONE_POLL * 3);

        let start = Instant::now();
        let (read, stop) = chunk_done.read(&request, Duration::from_secs(10));
        assert_eq!(read, 3, "every chunk posted before the answer is read");
        assert!(matches!(stop, ChunkStop::Answered), "{stop}");
        assert!(start.elapsed() < Duration::from_secs(2), "stopped after {:?}", start.elapsed());
    }

    /// A service that neither posts nor answers still stops the stream at the stall limit.
    #[test]
    fn a_request_neither_posting_nor_answering_stalls() {
        let mut chunk_done = ChunkDone::new("stalled");
        let (answer, unanswered) = std::sync::mpsc::channel::<()>();
        let request = thread::spawn(move || {
            let _ = unanswered.recv();
        });

        let start = Instant::now();
        let (read, stop) = chunk_done.read(&request, Duration::from_millis(300));
        assert_eq!(read, 0);
        assert!(matches!(stop, ChunkStop::Stalled(named_sem::Error::WaitTimeout)), "{stop}");
        assert!(start.elapsed() >= Duration::from_millis(300), "{:?}", start.elapsed());
        drop(answer);
        request.join().unwrap();
    }

    /// A service that waits for its data between chunks for longer than the stall limit, as
    /// with an input that arrives slowly, is not taken for a stalled one.
    #[test]
    fn a_service_waiting_for_data_does_not_stall() {
        let mut chunk_done = ChunkDone::new("waiting");
        let stall = Duration::from_millis(200);
        let request = chunk_done.request(2, stall * 3);

        let (read, stop) = chunk_done.read_waiting(&request, || true, stall);
        assert_eq!(read, 2, "every chunk is read, however long the service waited for data");
        assert!(matches!(stop, ChunkStop::Answered), "{stop}");
    }

    /// The stall limit counts again from when the service was last seen waiting for data, at
    /// most one poll before it stopped.
    #[test]
    fn a_service_stalls_a_stall_limit_after_it_stops_waiting() {
        let mut chunk_done = ChunkDone::new("waited");
        let (answer, unanswered) = std::sync::mpsc::channel::<()>();
        let request = thread::spawn(move || {
            let _ = unanswered.recv();
        });

        let start = Instant::now();
        let waited = Duration::from_millis(500);
        let stall = Duration::from_millis(300);
        let (read, stop) = chunk_done.read_waiting(&request, || start.elapsed() < waited, stall);
        assert_eq!(read, 0);
        assert!(matches!(stop, ChunkStop::Stalled(named_sem::Error::WaitTimeout)), "{stop}");
        let took = start.elapsed();
        assert!(took >= waited + stall - SEM_CHUNK_DONE_POLL, "{took:?}");
        drop(answer);
        request.join().unwrap();
    }

    /// The marks read as the C service writes them: waiting while even and not zero.
    #[test]
    fn data_waits_reads_the_service_marks() {
        let prefix = format!("ZISK_unittest_waits_{}", std::process::id());
        let name = shmem_control_output_name(&prefix, AsmService::MT);
        let c = std::ffi::CString::new(name.as_str()).unwrap();
        unsafe {
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0);
            assert_eq!(libc::ftruncate(fd, DataWaits::CONTROL_OUTPUT_SIZE as libc::off_t), 0);
            libc::close(fd);
        }
        let control = ShmemWriter::new(&name, DataWaits::CONTROL_OUTPUT_SIZE, true).unwrap();
        let waits = DataWaits::open(&prefix, AsmService::MT).expect("the segment maps");

        assert!(!waits.waiting(), "a reset service waits for nothing");
        for (offset, what) in
            [(DataWaits::WAITING_FOR_INPUT, "input"), (DataWaits::WAITING_FOR_HINTS, "hints")]
        {
            control.write_u64_at(offset, 3 << 1).unwrap();
            assert!(waits.waiting(), "the third wait for {what} is on");
            control.write_u64_at(offset, (3 << 1) + 1).unwrap();
            assert!(!waits.waiting(), "the third wait for {what} is over");
        }
        assert!(DataWaits::open(&format!("{prefix}_missing"), AsmService::MT).is_none());

        drop(waits);
        drop(control);
        unsafe { libc::shm_unlink(c.as_ptr()) };
    }

    /// No post of a request is taken by the next one. Each request's runner reads until its
    /// request is answered, then joins it and drains the semaphore, as the runners do; so
    /// a request that failed midway hands the next one a semaphore at zero, and the next
    /// one reads exactly its own chunks.
    #[test]
    fn a_failed_request_leaves_no_post_for_the_next() {
        let mut chunk_done = ChunkDone::new("handoff");
        for (chunks, gap) in [(2, Duration::ZERO), (5, SEM_CHUNK_DONE_POLL), (4, Duration::ZERO)] {
            let request = chunk_done.request(chunks, gap);
            let (read, stop) = chunk_done.read(&request, Duration::from_secs(10));
            request.join().unwrap();
            assert_eq!(drain_semaphore(&mut chunk_done.sem), 0, "nothing is left to drain");
            assert_eq!(read, chunks, "a request reads exactly its own chunks");
            assert!(matches!(stop, ChunkStop::Answered), "{stop}");
        }
    }
}
