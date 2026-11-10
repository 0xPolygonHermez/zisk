//! Chunk events: the Rust side of emulator-asm/src/par_events.hpp, which is the contract.
//!
//! A chunked trace service (MT, MO) appends one event per published chunk and per fork-join
//! region change to its events object, and posts its `chunk_done` semaphore once per event. The
//! reader handles one event per post, in event order:
//!
//! - `Main`: the next chunk of the main trace is complete, at the reader's cursor.
//! - `Region`: a region runs apart; its tasks write their chunks to their own trace objects.
//! - `TaskChunk`: a task's next chunk is complete, at an offset of its trace object.
//! - `TaskDone`: a task reached its next pass, after so many steps and chunks.
//! - `RegionEnd`: every task is done; the main trace continues at the reader's cursor.
//!
//! The chunks of a region are read as the tasks produce them, but handed on in the sequential
//! order: [`RegionSequencer`] releases a task's chunks once every task before it is done, which is
//! when the steps they start at are known.

#![cfg_attr(not(all(target_os = "linux", target_arch = "x86_64")), allow(dead_code))]

use std::collections::VecDeque;
use std::fmt;

use anyhow::{anyhow, bail, Result};

/// The exit codes (`MEM_ERROR`) of a run a fork-join region aborted, `PAR_ABORT_*` in
/// emulator-asm/src/par.hpp: the region failed in the service, or the guest ended or faulted
/// inside a task. `AsmRunError::from_exit_code` names them.
pub(crate) const PAR_ABORT_ERROR: u64 = 0x100;
pub(crate) const PAR_ABORT_TASK_ENDED: u64 = 0x101;
pub(crate) const PAR_ABORT_TASK_FAULTED: u64 = 0x102;

/// The layout version of the events object.
pub(crate) const PAR_EVENTS_VERSION: u64 = 3;
/// Events one run can publish: 2^18 chunks (the executor's 2^36 steps at the 2^18 steps a chunk
/// the driver pads to), and 2^11 region events (those of the 512 tasks a run runs apart at most).
pub(crate) const PAR_EVENTS_CAPACITY: usize = (1 << 18) + (1 << 11);
/// The size of a task's trace object: the whole trace address range, mostly sparse.
pub(crate) const PAR_TASK_TRACE_SIZE: usize = 1 << 35;

pub(crate) const PAR_EVENT_MAIN: u64 = 1;
pub(crate) const PAR_EVENT_REGION: u64 = 2;
pub(crate) const PAR_EVENT_TASK_CHUNK: u64 = 3;
pub(crate) const PAR_EVENT_TASK_DONE: u64 = 4;
pub(crate) const PAR_EVENT_REGION_END: u64 = 5;

/// One event slot, as the service writes it: `kind` last.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParEvent {
    pub kind: u64,
    pub a: u64,
    pub b: u64,
    pub c: u64,
}

/// The events object's header; the event slots follow it. Besides the slots' state, it reports the
/// run's regions (see [`RegionReport`]), which the reader reads once the service has answered.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ParEventsHeader {
    pub version: u64,
    pub tail: u64,
    pub regions_sequential: u64,
    pub sequential_reasons: u64,
    pub failure: u64,
    pub failure_region: u64,
    pub failure_task: u64,
    pub failure_detail: u64,
}

/// Why a region ran one task after another: the bits `1 << PAR_SEQUENTIAL_*` of
/// `sequential_reasons`.
pub(crate) const PAR_SEQUENTIAL_CHUNK_SIZE: u64 = 1;
pub(crate) const PAR_SEQUENTIAL_HINTS: u64 = 2;
pub(crate) const PAR_SEQUENTIAL_WORKERS: u64 = 3;
pub(crate) const PAR_SEQUENTIAL_FEW_TASKS: u64 = 4;
pub(crate) const PAR_SEQUENTIAL_BUDGET: u64 = 5;
pub(crate) const PAR_SEQUENTIAL_TASK_LIST: u64 = 6;
pub(crate) const PAR_SEQUENTIAL_FIRST_PASS: u64 = 7;
pub(crate) const PAR_SEQUENTIAL_HOST: u64 = 8;

/// How a region run apart failed (`failure`), and what `failure_detail` is then.
pub(crate) const PAR_FAILURE_TASK_EXIT: u64 = 1;
pub(crate) const PAR_FAILURE_TASK_SIGNAL: u64 = 2;
pub(crate) const PAR_FAILURE_TASK_RECORD: u64 = 3;
pub(crate) const PAR_FAILURE_MAX_STEPS: u64 = 4;
pub(crate) const PAR_FAILURE_HOST: u64 = 5;
pub(crate) const PAR_FAILURE_EVENTS: u64 = 6;
pub(crate) const PAR_FAILURE_PROTOCOL: u64 = 7;

/// No region, or no task.
pub(crate) const PAR_NONE: u64 = u64::MAX;

/// The exit codes of a task's process that fail it (`PAR_EXIT_*` in emulator-asm/src/par.hpp).
pub(crate) const PAR_EXIT_SETUP: u64 = 2;
pub(crate) const PAR_EXIT_ENDED: u64 = 3;
pub(crate) const PAR_EXIT_ABORTED: u64 = 4;
pub(crate) const PAR_EXIT_STATE: u64 = 5;
pub(crate) const PAR_EXIT_PUBLISH: u64 = 6;
pub(crate) const PAR_EXIT_SHM_FULL: u64 = 7;
pub(crate) const PAR_EXIT_MAX_STEPS: u64 = 8;
pub(crate) const PAR_EXIT_RESET: u64 = 9;

/// Why the assembly emulator ran a fork-join region one task after another, publishing ordinary
/// chunks: the execution is right, and proves the same, but the region gains nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequentialReason {
    /// The chunk size is not the one the fork-join driver pads to.
    ChunkSize,
    /// The execution takes precompile results as hints, which a task would need its place in.
    Hints,
    /// One worker: `ZISK_ASM_PAR_WORKERS` is 1 or less, or not a number, or it is unset and the
    /// service may run on one CPU only.
    Workers,
    /// The region has fewer than two tasks.
    FewTasks,
    /// The region would take the run past its budget of 512 tasks run apart.
    Budget,
    /// The task list is not what the driver lays out: not in the RAM, a task without page-aligned
    /// RAM that holds its stack, two tasks owning the same memory, or a task owning the list.
    TaskList,
    /// The region's first pass does not start a chunk.
    FirstPass,
    /// The service could not copy the task list to check it.
    Host,
}

impl SequentialReason {
    const ALL: [(u64, Self); 8] = [
        (PAR_SEQUENTIAL_CHUNK_SIZE, Self::ChunkSize),
        (PAR_SEQUENTIAL_HINTS, Self::Hints),
        (PAR_SEQUENTIAL_WORKERS, Self::Workers),
        (PAR_SEQUENTIAL_FEW_TASKS, Self::FewTasks),
        (PAR_SEQUENTIAL_BUDGET, Self::Budget),
        (PAR_SEQUENTIAL_TASK_LIST, Self::TaskList),
        (PAR_SEQUENTIAL_FIRST_PASS, Self::FirstPass),
        (PAR_SEQUENTIAL_HOST, Self::Host),
    ];

    /// Whether it follows from how the execution is set up, as expected, rather than from the
    /// guest, the run's budget or the host.
    pub fn is_configuration(self) -> bool {
        matches!(self, Self::ChunkSize | Self::Hints | Self::Workers | Self::FewTasks)
    }
}

impl fmt::Display for SequentialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ChunkSize => "the chunk size is not the one the fork-join driver pads to",
            Self::Hints => "the execution takes precompile results as hints",
            Self::Workers => {
                "one worker: ZISK_ASM_PAR_WORKERS is 1 or less, or the service may run on one CPU only"
            }
            Self::FewTasks => "a region has fewer than 2 tasks",
            Self::Budget => "the run reached its budget of 512 tasks run apart",
            Self::TaskList => {
                "a task list is not what the driver lays out (the service's log says how)"
            }
            Self::FirstPass => "a region's first pass does not start a chunk",
            Self::Host => "the service could not copy a task list to check it",
        })
    }
}

/// Where a fork-join region run apart failed: the region, counted from 0 among those the run ran
/// apart, and the task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskAt {
    /// The region, counted from 0 among those the run ran apart.
    pub region: u64,
    /// The task, counted from 0 in its region.
    pub task: u64,
}

impl fmt::Display for TaskAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "task {} of region {}", self.task, self.region)
    }
}

/// How a fork-join region run apart failed, as the assembly service reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionFailure {
    /// The region it failed in, if it failed in one.
    pub region: Option<u64>,
    /// The task it failed in, if it failed in one.
    pub task: Option<u64>,
    /// What failed.
    pub cause: FailureCause,
}

/// What failed in a fork-join region run apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCause {
    /// The guest ended the program (it exited or panicked) inside the task.
    GuestEnded,
    /// The guest faulted inside the task.
    GuestFaulted,
    /// The task's process could not set up its trace or memory.
    TaskSetup,
    /// The task reached another pass than the next one, or in another state.
    TaskState,
    /// The task's process could not publish a chunk or its end.
    TaskPublish,
    /// Writing the task's trace, an event or the guest memory faulted: `/dev/shm`, which backs
    /// them, is full.
    ShmFull,
    /// The emulation reached `max_steps` inside the region.
    MaxSteps,
    /// A reset stopped the emulation inside the task: the run is being cancelled.
    Reset,
    /// The task's process was killed by a signal.
    TaskKilled {
        /// The signal's number.
        signal: u64,
    },
    /// The task's process exited without saying how long it ran, or said it wrongly.
    TaskRecord,
    /// The service could not set the region up.
    Host {
        /// The system's error number, 0 if none.
        errno: u64,
    },
    /// The service could not publish an event.
    Events {
        /// The system's error number, 0 if the run's events were full.
        errno: u64,
    },
    /// The guest broke the driver's protocol: a region inside another, or a pass outside one.
    Protocol,
    /// A failure this reader does not know.
    Unknown {
        /// The service's code for it (`PAR_FAILURE_*`).
        failure: u64,
        /// Its detail.
        detail: u64,
    },
}

impl RegionFailure {
    /// The failure reported by the header's words, if any.
    fn decode(failure: u64, region: u64, task: u64, detail: u64) -> Option<Self> {
        let cause = match (failure, detail) {
            (0, _) => return None,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_ENDED) => FailureCause::GuestEnded,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_ABORTED) => FailureCause::GuestFaulted,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_SETUP) => FailureCause::TaskSetup,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_STATE) => FailureCause::TaskState,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_PUBLISH) => FailureCause::TaskPublish,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_SHM_FULL) => FailureCause::ShmFull,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_MAX_STEPS) => FailureCause::MaxSteps,
            (PAR_FAILURE_TASK_EXIT, PAR_EXIT_RESET) => FailureCause::Reset,
            (PAR_FAILURE_TASK_SIGNAL, signal) => FailureCause::TaskKilled { signal },
            (PAR_FAILURE_TASK_RECORD, _) => FailureCause::TaskRecord,
            (PAR_FAILURE_MAX_STEPS, _) => FailureCause::MaxSteps,
            (PAR_FAILURE_HOST, errno) => FailureCause::Host { errno },
            (PAR_FAILURE_EVENTS, errno) => FailureCause::Events { errno },
            (PAR_FAILURE_PROTOCOL, _) => FailureCause::Protocol,
            (failure, detail) => FailureCause::Unknown { failure, detail },
        };
        let some = |word| (word != PAR_NONE).then_some(word);
        Some(Self { region: some(region), task: some(task), cause })
    }

    /// The task it failed in, if both it and its region are known.
    pub fn task_at(&self) -> Option<TaskAt> {
        Some(TaskAt { region: self.region?, task: self.task? })
    }
}

impl fmt::Display for RegionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.region, self.task) {
            (Some(region), Some(task)) => write!(f, "{}: ", TaskAt { region, task })?,
            (Some(region), None) => write!(f, "region {region}: ")?,
            (None, Some(task)) => write!(f, "task {task}: ")?,
            (None, None) => {}
        }
        let errno = |errno: u64| {
            if errno == 0 {
                String::new()
            } else {
                format!(" ({})", std::io::Error::from_raw_os_error(errno as i32))
            }
        };
        match self.cause {
            FailureCause::GuestEnded => write!(f, "the guest ended (it exited or panicked)"),
            FailureCause::GuestFaulted => write!(f, "the guest faulted"),
            FailureCause::TaskSetup => {
                write!(f, "the task's process could not set up its trace or memory")
            }
            FailureCause::TaskState => write!(
                f,
                "the task reached another pass than the next one, or in another state than the \
                 region's"
            ),
            FailureCause::TaskPublish => {
                write!(f, "the task's process could not publish a chunk or its end")
            }
            FailureCause::ShmFull => write!(
                f,
                "the task's process could not write its trace, an event or the guest memory: \
                 /dev/shm, which backs them, is full"
            ),
            FailureCause::MaxSteps => write!(f, "the emulation reached max_steps"),
            FailureCause::Reset => write!(f, "a reset stopped the emulation: the run is cancelled"),
            FailureCause::TaskKilled { signal } => {
                write!(f, "the task's process was killed by signal {signal}")
            }
            FailureCause::TaskRecord => {
                write!(f, "the task's process ended without saying how long it ran")
            }
            FailureCause::Host { errno: e } => {
                write!(f, "the service could not set the region up{}", errno(e))
            }
            FailureCause::Events { errno: 0 } => {
                write!(f, "the run published more chunk events than the events object holds")
            }
            FailureCause::Events { errno: e } => {
                write!(f, "the service could not publish a chunk event{}", errno(e))
            }
            FailureCause::Protocol => write!(
                f,
                "the guest broke the fork-join driver's protocol (a region inside another, or a \
                 pass outside one)"
            ),
            FailureCause::Unknown { failure, detail } => {
                write!(f, "failure {failure} ({detail}), which this reader does not know")
            }
        }
    }
}

/// What a service reports about the fork-join regions of its last run, besides their events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RegionReport {
    /// Regions it ran one task after another.
    pub regions_sequential: u64,
    /// Why: a bit `1 << PAR_SEQUENTIAL_*` per reason seen.
    sequential_reasons: u64,
    /// How a region run apart failed, if one did.
    pub failure: Option<RegionFailure>,
}

impl RegionReport {
    pub(crate) fn decode(header: &ParEventsHeader) -> Self {
        Self {
            regions_sequential: header.regions_sequential,
            sequential_reasons: header.sequential_reasons,
            failure: RegionFailure::decode(
                header.failure,
                header.failure_region,
                header.failure_task,
                header.failure_detail,
            ),
        }
    }

    /// Why regions ran one task after another, each reason once.
    pub(crate) fn sequential_reasons(&self) -> Vec<SequentialReason> {
        SequentialReason::ALL
            .iter()
            .filter(|(code, _)| self.sequential_reasons & (1 << code) != 0)
            .map(|&(_, reason)| reason)
            .collect()
    }

    /// Logs the regions the service ran one task after another, and why, once per run: at info
    /// when every reason is the execution's set-up, as expected, and at warn when one is the
    /// guest's, the run's budget or the host's.
    pub(crate) fn log_sequential(&self) {
        if self.regions_sequential == 0 {
            return;
        }
        let reasons = self.sequential_reasons();
        let why = reasons.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ");
        let message = format!(
            "fork-join: {} region(s) ran one task after another: {why}",
            self.regions_sequential
        );
        if reasons.iter().all(|reason| reason.is_configuration()) {
            tracing::info!("{message}");
        } else {
            tracing::warn!("{message}");
        }
    }
}

/// The size of the events object.
pub(crate) const PAR_EVENTS_BYTES: usize =
    std::mem::size_of::<ParEventsHeader>() + PAR_EVENTS_CAPACITY * std::mem::size_of::<ParEvent>();

/// An event, decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChunkEvent {
    Main,
    /// The region's number and its tasks; the step its first pass starts at, which the event also
    /// carries, is not needed: each task's first chunk says it
    Region {
        region: u64,
        tasks: u64,
    },
    TaskChunk {
        task: u64,
        index: u64,
        offset: u64,
    },
    TaskDone {
        task: u64,
        steps: u64,
        chunks: u64,
    },
    RegionEnd {
        region: u64,
    },
}

impl ChunkEvent {
    pub(crate) fn decode(event: ParEvent) -> Result<Self> {
        let ParEvent { kind, a, b, c } = event;
        Ok(match kind {
            PAR_EVENT_MAIN => Self::Main,
            PAR_EVENT_REGION => Self::Region { region: a, tasks: b },
            PAR_EVENT_TASK_CHUNK => Self::TaskChunk { task: a, index: b, offset: c },
            PAR_EVENT_TASK_DONE => Self::TaskDone { task: a, steps: b, chunks: c },
            PAR_EVENT_REGION_END => Self::RegionEnd { region: a },
            other => bail!("unknown chunk event kind {other} ({a}, {b}, {c})"),
        })
    }
}

/// Hands the chunks of one region on in the sequential order.
///
/// Chunks arrive per task, in order within each task, interleaved between tasks. Task `i`'s chunks
/// start, in the sequential execution, after every chunk of the tasks before it, and at the sum of
/// their steps: so they are released only once every task before `i` is done, each with that sum,
/// its `shift`. The task at the frontier (the first one not done) is released as its chunks come.
pub(crate) struct RegionSequencer<T> {
    tasks: Vec<TaskChunks<T>>,
    /// Steps per chunk: a task runs a whole number of chunks
    chunk_size: u64,
    /// The first task with chunks not yet released, or not done
    frontier: usize,
    /// The steps of the tasks before the frontier
    shift: u64,
}

struct TaskChunks<T> {
    pending: VecDeque<T>,
    received: u64,
    /// Its steps, once the task is done (every chunk of it received by then)
    done: Option<u64>,
}

impl<T> RegionSequencer<T> {
    pub(crate) fn new(tasks: usize, chunk_size: u64) -> Self {
        let tasks = (0..tasks)
            .map(|_| TaskChunks { pending: VecDeque::new(), received: 0, done: None })
            .collect();
        Self { tasks, chunk_size, frontier: 0, shift: 0 }
    }

    fn task(&mut self, task: u64) -> Result<&mut TaskChunks<T>> {
        let n = self.tasks.len();
        self.tasks
            .get_mut(task as usize)
            .ok_or_else(|| anyhow!("fork-join task {task} is not one of the region's {n}"))
    }

    /// Task `task`'s chunk `index` arrived.
    pub(crate) fn chunk(&mut self, task: u64, index: u64, item: T) -> Result<()> {
        let chunks = self.task(task)?;
        if chunks.done.is_some() {
            bail!("fork-join task {task} published chunk {index} after it was done");
        }
        if index != chunks.received {
            bail!(
                "fork-join task {task} published chunk {index}, expected chunk {}",
                chunks.received
            );
        }
        chunks.received += 1;
        chunks.pending.push_back(item);
        Ok(())
    }

    /// Task `task` is done, after `steps` steps in `chunks` chunks, all of them published.
    pub(crate) fn done(&mut self, task: u64, steps: u64, chunks: u64) -> Result<()> {
        let chunk_size = self.chunk_size;
        if chunks.checked_mul(chunk_size) != Some(steps) {
            bail!("fork-join task {task} ran {steps} steps in {chunks} chunks of {chunk_size}");
        }
        let state = self.task(task)?;
        if state.done.is_some() {
            bail!("fork-join task {task} is done twice");
        }
        if state.received != chunks {
            bail!(
                "fork-join task {task} is done after {chunks} chunks, but published {}",
                state.received
            );
        }
        state.done = Some(steps);
        Ok(())
    }

    /// Hands on, in the sequential order, every chunk that can be: `out(shift, item)`, where
    /// `shift` is the steps of the tasks before the chunk's task.
    pub(crate) fn release(&mut self, mut out: impl FnMut(u64, T) -> Result<()>) -> Result<()> {
        while self.frontier < self.tasks.len() {
            let shift = self.shift;
            let task = &mut self.tasks[self.frontier];
            while let Some(item) = task.pending.pop_front() {
                out(shift, item)?;
            }
            // Done means every chunk was received, so every one is released by now
            match task.done {
                Some(steps) => {
                    self.shift += steps;
                    self.frontier += 1;
                }
                None => break,
            }
        }
        Ok(())
    }

    /// Every task is done and every chunk released.
    pub(crate) fn is_complete(&self) -> bool {
        self.frontier == self.tasks.len()
    }

    /// The steps of every task released so far.
    #[cfg(test)]
    pub(crate) fn steps_released(&self) -> u64 {
        self.shift
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) use linux::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod linux {
    use std::os::raw::c_void;
    use std::sync::mpsc::{self, SendError, SyncSender};
    use std::time::{Duration, Instant};

    use anyhow::{anyhow, bail, Context, Result};

    use super::*;
    use crate::{
        shmem_events_name, shmem_sys, shmem_task_trace_name, AsmRunError, AsmService, TaskFailure,
    };

    /// How long a reader waits for an event slot its writer has reserved but not finished, while
    /// the service has not answered: as long as it waits for a chunk.
    const EVENT_WRITE_LIMIT: Duration = crate::SEM_CHUNK_DONE_WAIT_DURATION;

    /// A read-only mapping of a shared memory object, unmapped on drop.
    struct Mapping {
        ptr: *mut c_void,
        size: usize,
    }

    // SAFETY: the mapping's address is fixed for the handle's lifetime and only read.
    unsafe impl Send for Mapping {}
    unsafe impl Sync for Mapping {}

    impl Mapping {
        fn open(name: &str, size: usize) -> Result<Self> {
            let fd = shmem_sys::open(name, libc::O_RDONLY)?;
            // Never locked: a task's trace object is mostly sparse
            let mapped = shmem_sys::map(fd, size, libc::PROT_READ, false, name);
            // The mapping keeps the object, not the descriptor: a reader holds up to 512 task
            // trace objects per service until its next run, which would take as many descriptors
            // (1024 per process by default)
            unsafe { libc::close(fd) };
            Ok(Self { ptr: mapped?, size })
        }
    }

    impl Drop for Mapping {
        fn drop(&mut self) {
            unsafe { libc::munmap(self.ptr, self.size) };
        }
    }

    /// A service's events object, as its reader maps it.
    pub(crate) struct EventsShmem {
        mapping: Mapping,
    }

    impl EventsShmem {
        /// Maps the events object of `service`, which the service creates at its setup.
        pub(crate) fn open(shm_prefix: &str, service: AsmService) -> Result<Self> {
            let name = shmem_events_name(shm_prefix, service);
            let mapping = Mapping::open(&name, PAR_EVENTS_BYTES)?;
            let header = unsafe { (mapping.ptr as *const ParEventsHeader).read_volatile() };
            if header.version != PAR_EVENTS_VERSION {
                bail!(
                    "events object '{name}' has layout version {}, expected {PAR_EVENTS_VERSION}",
                    header.version
                );
            }
            Ok(Self { mapping })
        }

        /// What the service reports about the regions of its last run. Read it once the service
        /// has answered the run: it writes the report before.
        pub(crate) fn report(&self) -> RegionReport {
            std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
            let header = unsafe { (self.mapping.ptr as *const ParEventsHeader).read_volatile() };
            RegionReport::decode(&header)
        }

        /// The event in slot `index`, once its writer has finished it; `None` if the service
        /// answers (`answered`) first: the run is over, and a writer that reserved the slot
        /// without filling it (a task's process killed in between) never will. The run failed
        /// then, and the header of the service's output says how.
        pub(crate) fn read(
            &self,
            index: usize,
            answered: impl Fn() -> bool,
        ) -> Result<Option<ChunkEvent>> {
            if index >= PAR_EVENTS_CAPACITY {
                bail!("more than {PAR_EVENTS_CAPACITY} chunk events in one run");
            }
            let slot = unsafe {
                (self.mapping.ptr as *const u8)
                    .add(std::mem::size_of::<ParEventsHeader>())
                    .cast::<ParEvent>()
                    .add(index)
            };
            let kind_ptr = unsafe { std::ptr::addr_of!((*slot).kind) };
            let mut spins = 0u32;
            let mut started: Option<Instant> = None;
            loop {
                // The writer stores `kind` last, with release ordering
                let kind = unsafe { kind_ptr.read_volatile() };
                if kind != 0 {
                    std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);
                    let event = unsafe { slot.read_volatile() };
                    return ChunkEvent::decode(ParEvent { kind, ..event }).map(Some);
                }
                if spins < 1000 {
                    spins += 1;
                    std::hint::spin_loop();
                } else {
                    if answered() {
                        return Ok(None);
                    }
                    let since = *started.get_or_insert_with(Instant::now);
                    if since.elapsed() > EVENT_WRITE_LIMIT {
                        bail!("chunk event {index} was posted but never written");
                    }
                    std::thread::sleep(Duration::from_micros(20));
                }
            }
        }
    }

    /// The trace objects of the regions one run ran apart, as its reader maps them. The chunks
    /// read from them borrow the mappings, so they are kept until the next run starts.
    #[derive(Default)]
    pub(crate) struct TaskTraces {
        /// Per region of the run (by its number), per task
        regions: Vec<Vec<Mapping>>,
        /// The thread that unmaps past runs' mappings, once a run had any (see `clear`)
        reaper: Option<SyncSender<Vec<Vec<Mapping>>>>,
    }

    impl TaskTraces {
        /// Drops the previous run's mappings: its chunks are gone by the time the next run starts,
        /// so nothing borrows them any more, now or later.
        ///
        /// They are unmapped on a thread of their own, off the reader, which is about to wait for
        /// the run's first chunk: a run maps up to 512 task traces, and the last mapping of an
        /// object frees the memory its trace took in /dev/shm, so unmapping them takes longer the
        /// larger the traces were. The thread starts with the first run that had any and lives as
        /// long as the reader, so that a run costs the reader a hand-over, not a thread's start.
        /// The hand-over goes through a channel without a buffer, and completes once the thread is
        /// done with the mappings before: while a run writes its traces, those of one past run at
        /// most are still in /dev/shm, and only for as long as unmapping them takes. Each reader
        /// has its own thread, since the MT and MO readers clear theirs at the same time.
        pub(crate) fn clear(&mut self) {
            let regions = std::mem::take(&mut self.regions);
            if regions.is_empty() {
                return;
            }
            if let Some(reaper) = &self.reaper {
                // Its thread hangs up only with this sender, at the reader's drop; were it gone,
                // the mappings would come back in the error, and be unmapped here
                if let Err(SendError(regions)) = reaper.send(regions) {
                    drop(regions);
                }
                return;
            }
            // The first run's mappings go to the thread as it starts: the reader does not wait
            // for it to be ready
            let (reaper, batches) = mpsc::sync_channel::<Vec<Vec<Mapping>>>(0);
            let spawned = std::thread::Builder::new()
                .name("asm-trace-unmap".into())
                .spawn(move || std::iter::once(regions).chain(batches).for_each(drop));
            match spawned {
                Ok(_) => self.reaper = Some(reaper),
                // The mappings went with the thread's closure, unmapped here; the next run with
                // regions tries again
                Err(e) => tracing::warn!(
                    "could not start the thread that unmaps fork-join task traces ({e}); the \
                     reader unmapped them"
                ),
            }
        }

        /// Maps the trace objects of `region`'s `tasks` tasks, which carry the prefix of the
        /// program that ran (`AsmServices::sem_prefix`).
        pub(crate) fn map_region(
            &mut self,
            program_prefix: &str,
            service: AsmService,
            region: u64,
            tasks: u64,
        ) -> Result<()> {
            if region as usize != self.regions.len() {
                bail!(
                    "fork-join region {region} announced after {} regions in this run",
                    self.regions.len()
                );
            }
            let mappings = (0..tasks)
                .map(|task| {
                    let name = shmem_task_trace_name(program_prefix, service, region, task);
                    let mapping = Mapping::open(&name, PAR_TASK_TRACE_SIZE)
                        .with_context(|| format!("mapping fork-join task trace '{name}'"))?;
                    // Nobody opens it by name after this reader: the service hands each object
                    // to its task's process by descriptor. Unlinked now, its memory goes with the
                    // last mapping, even when a process dies before the service's cleanup; left
                    // behind, a crashed run's traces would fill the RAM that backs /dev/shm.
                    if let Ok(c_name) = std::ffi::CString::new(name.as_str()) {
                        unsafe { libc::shm_unlink(c_name.as_ptr()) };
                    }
                    Ok(mapping)
                })
                .collect::<Result<Vec<_>>>()?;
            self.regions.push(mappings);
            Ok(())
        }

        /// Where `offset` of task `task`'s trace object of `region` is mapped.
        pub(crate) fn at(&self, region: u64, task: u64, offset: u64) -> Result<*const u8> {
            let mapping = self
                .regions
                .get(region as usize)
                .and_then(|tasks| tasks.get(task as usize))
                .ok_or_else(|| {
                    anyhow!("no trace object for fork-join region {region} task {task}")
                })?;
            if offset as usize >= mapping.size {
                bail!("fork-join task {task} chunk at offset {offset}, past its trace object");
            }
            Ok(unsafe { (mapping.ptr as *const u8).add(offset as usize) })
        }
    }

    /// A task's chunk as its reader reads it: what it hands on, and whether the guest ended in it.
    pub(crate) struct TaskChunkRead<T> {
        pub item: T,
        pub end: bool,
    }

    /// The fork-join side of a chunk reader (MT, MO): follows the region events of a run, maps
    /// each region's task traces, and hands the tasks' chunks on in the sequential order. What a
    /// chunk is, `T`, and what becomes of it are the reader's; so are the main chunks.
    pub(crate) struct RegionReader<T> {
        service: AsmService,
        program_prefix: String,
        chunk_size: u64,
        /// The region the service is running apart, if any: its number, and its chunks
        region: Option<(u64, RegionSequencer<T>)>,
        /// Regions read run apart in this run
        regions_apart: u64,
    }

    impl<T> RegionReader<T> {
        /// The reader of one run of `service`, whose task trace objects carry `program_prefix`
        /// (`AsmServices::sem_prefix`).
        pub(crate) fn new(service: AsmService, program_prefix: &str, chunk_size: u64) -> Self {
            Self {
                service,
                program_prefix: program_prefix.to_string(),
                chunk_size,
                region: None,
                regions_apart: 0,
            }
        }

        /// Whether a region runs apart: no main chunk can come until it ends.
        pub(crate) fn in_region(&self) -> bool {
            self.region.is_some()
        }

        /// The regions read run apart so far in this run.
        pub(crate) fn regions_apart(&self) -> u64 {
            self.regions_apart
        }

        /// Follows a region event: any but `Main`, which the reader reads itself. `read` reads a
        /// task's chunk at its address in the task's trace object; `release` takes, in chunk
        /// order, every chunk the event releases, with the steps of the tasks before its task.
        /// A chunk in which the guest ended fails the run: a task must not end the program.
        pub(crate) fn follow(
            &mut self,
            event: ChunkEvent,
            traces: &mut TaskTraces,
            read: impl FnOnce(*const u8) -> TaskChunkRead<T>,
            release: impl FnMut(u64, T) -> Result<()>,
        ) -> Result<()> {
            match event {
                ChunkEvent::Main => bail!("a main chunk is not a region event"),
                ChunkEvent::Region { region: number, tasks } => {
                    if let Some((current, _)) = &self.region {
                        bail!("fork-join region {number} starts inside region {current}");
                    }
                    traces.map_region(&self.program_prefix, self.service, number, tasks)?;
                    self.region =
                        Some((number, RegionSequencer::new(tasks as usize, self.chunk_size)));
                    self.regions_apart += 1;
                }
                ChunkEvent::TaskChunk { task, index, offset } => {
                    let Some((number, sequencer)) = self.region.as_mut() else {
                        bail!("a fork-join task chunk outside a region");
                    };
                    let chunk = read(traces.at(*number, task, offset)?);
                    if chunk.end {
                        let at = TaskAt { region: *number, task };
                        return Err(anyhow::Error::new(AsmRunError::ForkJoinTask {
                            failure: TaskFailure::Ended,
                            at: Some(at),
                        })
                        .context(format!("fork-join {at}, chunk {index}")));
                    }
                    sequencer.chunk(task, index, chunk.item)?;
                    sequencer.release(release)?;
                }
                ChunkEvent::TaskDone { task, steps, chunks } => {
                    let Some((_, sequencer)) = self.region.as_mut() else {
                        bail!("a fork-join task end outside a region");
                    };
                    sequencer.done(task, steps, chunks)?;
                    sequencer.release(release)?;
                }
                ChunkEvent::RegionEnd { region: number } => match self.region.take() {
                    Some((current, sequencer)) if current == number && sequencer.is_complete() => {}
                    Some((current, _)) => {
                        bail!("fork-join region {number} ended with region {current} not complete")
                    }
                    None => bail!("fork-join region {number} ended without starting"),
                },
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::ffi::CString;

        /// A shared memory object this test creates, as the service would, unlinked on drop.
        struct Object {
            name: String,
        }

        impl Object {
            fn new(name: String, size: usize) -> Self {
                let c = CString::new(name.as_str()).unwrap();
                unsafe {
                    let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
                    assert!(fd >= 0, "shm_open({name})");
                    assert_eq!(libc::ftruncate(fd, size as libc::off_t), 0);
                    libc::close(fd);
                }
                Self { name }
            }
        }

        impl Drop for Object {
            fn drop(&mut self) {
                let c = CString::new(self.name.as_str()).unwrap();
                unsafe { libc::shm_unlink(c.as_ptr()) };
            }
        }

        /// The descriptors this process holds open on the object `name`.
        fn descriptors_on(name: &str) -> usize {
            let path = format!("/dev/shm/{name}");
            std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .flatten()
                .filter_map(|entry| std::fs::read_link(entry.path()).ok())
                .filter(|target| target.to_string_lossy().trim_end_matches(" (deleted)") == path)
                .count()
        }

        /// A reader keeps the objects it maps, not their descriptors: with up to 512 task trace
        /// objects mapped per service until the next run, descriptors would run out.
        #[test]
        fn mappings_hold_no_descriptor() {
            let object = Object::new(format!("ZISK_unittest_mapping_{}", std::process::id()), 4096);
            let mappings: Vec<Mapping> =
                (0..8).map(|_| Mapping::open(&object.name, 4096).unwrap()).collect();
            assert_eq!(descriptors_on(&object.name), 0);
            let maps = std::fs::read_to_string("/proc/self/maps").unwrap();
            let path = format!("/dev/shm/{}", object.name);
            assert_eq!(maps.lines().filter(|line| line.ends_with(&path)).count(), 8);
            assert!(mappings.iter().all(|mapping| unsafe { *(mapping.ptr as *const u64) } == 0));
        }

        /// A slot whose writer reserved it but never filled it reads as nothing once the service
        /// has answered, so that the reader reports how the run failed; a filled slot reads as
        /// its event.
        #[test]
        fn an_unwritten_event_reads_as_none_once_the_service_answers() {
            let prefix = format!("ZISK_unittest_unwritten_{}", std::process::id());
            let object = Object::new(shmem_events_name(&prefix, AsmService::MO), PAR_EVENTS_BYTES);
            let slot = |index: usize| {
                std::mem::size_of::<ParEventsHeader>() + index * std::mem::size_of::<ParEvent>()
            };
            let writer = crate::ShmemWriter::new(&object.name, PAR_EVENTS_BYTES, true).unwrap();
            writer.write_u64_at(0, PAR_EVENTS_VERSION).unwrap();
            writer.write_u64_at(slot(1), PAR_EVENT_MAIN).unwrap();

            let events = EventsShmem::open(&prefix, AsmService::MO).unwrap();
            assert_eq!(events.read(1, || false).unwrap(), Some(ChunkEvent::Main));
            assert_eq!(events.read(0, || true).unwrap(), None);
        }

        /// The events object too, which a reader maps for as long as it lives.
        #[test]
        fn the_events_object_holds_no_descriptor() {
            let prefix = format!("ZISK_unittest_events_{}", std::process::id());
            let object = Object::new(shmem_events_name(&prefix, AsmService::MT), PAR_EVENTS_BYTES);
            crate::ShmemWriter::new(&object.name, PAR_EVENTS_BYTES, true)
                .unwrap()
                .write_u64_at(0, PAR_EVENTS_VERSION)
                .unwrap();
            let _events = EventsShmem::open(&prefix, AsmService::MT).unwrap();
            assert_eq!(descriptors_on(&object.name), 0);
        }

        /// Writes `value` at `offset` of the shared memory object `name`, as a task would.
        fn put(name: &str, offset: u64, value: u64) {
            let c = CString::new(name).unwrap();
            unsafe {
                let fd = libc::shm_open(c.as_ptr(), libc::O_RDWR, 0);
                assert!(fd >= 0, "shm_open({name})");
                let bytes = value.to_le_bytes();
                let written =
                    libc::pwrite(fd, bytes.as_ptr().cast(), bytes.len(), offset as libc::off_t);
                assert_eq!(written, 8);
                libc::close(fd);
            }
        }

        /// The task trace objects of `region`'s `tasks` tasks, as the service creates them.
        fn task_objects(prefix: &str, region: u64, tasks: u64) -> Vec<Object> {
            (0..tasks)
                .map(|task| {
                    let name = shmem_task_trace_name(prefix, AsmService::MT, region, task);
                    Object::new(name, PAR_TASK_TRACE_SIZE)
                })
                .collect()
        }

        /// A region's chunks, read as its tasks publish them, are released in chunk order with
        /// the steps of the tasks before theirs; a chunk the guest ended in fails the run, naming
        /// its task; and events out of the protocol's order are errors.
        #[test]
        fn a_region_reader_releases_in_chunk_order() {
            const CHUNK: u64 = 1 << 18;
            const ENDED: u64 = u64::MAX;
            let prefix = format!("ZISK_unittest_regions_{}", std::process::id());
            // A chunk reads as the word its task wrote at its offset
            let read = |at: *const u8| {
                let word = unsafe { (at as *const u64).read_unaligned() };
                TaskChunkRead { item: word, end: word == ENDED }
            };
            let mut traces = TaskTraces::default();

            let region_0 = task_objects(&prefix, 0, 2);
            put(&region_0[0].name, 8192, 40);
            put(&region_0[1].name, 4096, 41);
            let mut reader = RegionReader::<u64>::new(AsmService::MT, &prefix, CHUNK);
            let mut released = Vec::new();
            let mut follow = |reader: &mut RegionReader<u64>, event| {
                reader.follow(event, &mut traces, read, |shift, word| {
                    released.push((word, shift));
                    Ok(())
                })
            };
            assert!(follow(&mut reader, ChunkEvent::TaskDone { task: 0, steps: CHUNK, chunks: 1 })
                .is_err());
            follow(&mut reader, ChunkEvent::Region { region: 0, tasks: 2 }).unwrap();
            assert!(reader.in_region());
            assert!(follow(&mut reader, ChunkEvent::Region { region: 1, tasks: 2 }).is_err());
            // Task 1's chunk comes first, and waits for task 0 to be done
            follow(&mut reader, ChunkEvent::TaskChunk { task: 1, index: 0, offset: 4096 }).unwrap();
            follow(&mut reader, ChunkEvent::TaskChunk { task: 0, index: 0, offset: 8192 }).unwrap();
            follow(&mut reader, ChunkEvent::TaskDone { task: 0, steps: CHUNK, chunks: 1 }).unwrap();
            follow(&mut reader, ChunkEvent::TaskDone { task: 1, steps: CHUNK, chunks: 1 }).unwrap();
            assert!(follow(&mut reader, ChunkEvent::RegionEnd { region: 1 }).is_err());
            assert!(!reader.in_region());
            assert_eq!(reader.regions_apart(), 1);
            assert_eq!(released, [(40, 0), (41, CHUNK)]);

            // The next region of the run: the guest ends in task 1's chunk
            let region_1 = task_objects(&prefix, 1, 2);
            put(&region_1[1].name, 0, ENDED);
            let mut nothing = |_: u64, _: u64| -> Result<()> { Ok(()) };
            let region = ChunkEvent::Region { region: 1, tasks: 2 };
            reader.follow(region, &mut traces, read, &mut nothing).unwrap();
            let chunk = ChunkEvent::TaskChunk { task: 1, index: 0, offset: 0 };
            let error = reader.follow(chunk, &mut traces, read, &mut nothing).unwrap_err();
            assert!(
                matches!(
                    error.downcast_ref::<AsmRunError>(),
                    Some(AsmRunError::ForkJoinTask {
                        failure: TaskFailure::Ended,
                        at: Some(TaskAt { region: 1, task: 1 })
                    })
                ),
                "{error:#}"
            );
        }

        /// The mappings this process holds of objects whose names start with `prefix`.
        fn mappings_of(prefix: &str) -> usize {
            let path = format!("/dev/shm/{prefix}");
            std::fs::read_to_string("/proc/self/maps")
                .unwrap()
                .lines()
                .filter(|line| line.contains(&path))
                .count()
        }

        /// Whether the mappings of objects whose names start with `prefix` are all gone within
        /// 10 s.
        fn unmapped_soon(prefix: &str) -> bool {
            let deadline = Instant::now() + Duration::from_secs(10);
            while mappings_of(prefix) > 0 {
                if Instant::now() > deadline {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            true
        }

        /// A run's task traces are unmapped once the next run clears them, by the reader's own
        /// thread, which a run without regions never starts; and a run's hand-over waits for that
        /// thread to be done with the run before, so one past run's at most are left mapped.
        #[test]
        fn cleared_task_traces_are_unmapped_off_the_reader() {
            let pid = std::process::id();
            let mut traces = TaskTraces::default();
            traces.clear();
            assert!(traces.reaper.is_none(), "a run without regions starts no thread");

            // The first run with regions: its traces go to the thread as it starts
            let first = format!("ZISK_unittest_unmap_a_{pid}");
            let _first = [task_objects(&first, 0, 3), task_objects(&first, 1, 2)];
            traces.map_region(&first, AsmService::MT, 0, 3).unwrap();
            traces.map_region(&first, AsmService::MT, 1, 2).unwrap();
            assert_eq!(mappings_of(&first), 5);
            traces.clear();
            assert!(traces.reaper.is_some());
            assert!(traces.at(0, 0, 0).is_err(), "the next run starts with no region");
            assert!(unmapped_soon(&first), "{} left mapped", mappings_of(&first));

            // Two more runs: the third's hand-over returns with the second's traces unmapped
            let second = format!("ZISK_unittest_unmap_b_{pid}");
            let _second = task_objects(&second, 0, 2);
            traces.map_region(&second, AsmService::MT, 0, 2).unwrap();
            traces.clear();
            let third = format!("ZISK_unittest_unmap_c_{pid}");
            let _third = task_objects(&third, 0, 4);
            traces.map_region(&third, AsmService::MT, 0, 4).unwrap();
            assert_eq!(mappings_of(&third), 4);
            traces.clear();
            assert_eq!(mappings_of(&second), 0);
            assert!(unmapped_soon(&third), "{} left mapped", mappings_of(&third));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value of `#define NAME <value>` in `header`, the text of the C header `file`: an
    /// integer, decimal or hexadecimal, or `(1ULL << N)`, before any comment.
    fn define_in(header: &str, file: &str, name: &str) -> u64 {
        let prefix = format!("#define {name} ");
        let line = header
            .lines()
            .find(|l| l.starts_with(&prefix))
            .unwrap_or_else(|| panic!("{file} has no `{prefix}...`"));
        let value = line[prefix.len()..].split("//").next().unwrap().trim();
        if let Some(shift) = value.strip_prefix("(1ULL << ").and_then(|rest| rest.strip_suffix(')'))
        {
            return 1u64 << shift.trim().parse::<u32>().unwrap();
        }
        let parsed = match value.strip_prefix("0x") {
            Some(hex) => u64::from_str_radix(hex, 16).ok(),
            None => value.parse().ok(),
        };
        parsed.unwrap_or_else(|| panic!("{file}: `{line}` is not a known form"))
    }

    /// The value of `#define NAME` in emulator-asm/src/par_events.hpp.
    fn c_define(name: &str) -> u64 {
        define_in(include_str!("../../src/par_events.hpp"), "par_events.hpp", name)
    }

    /// The value of `#define NAME` in emulator-asm/src/par.hpp.
    fn par_define(name: &str) -> u64 {
        define_in(include_str!("../../src/par.hpp"), "par.hpp", name)
    }

    #[test]
    fn layout_matches_the_c_header() {
        assert_eq!(c_define("PAR_EVENTS_VERSION"), PAR_EVENTS_VERSION);
        assert_eq!(c_define("PAR_EVENTS_CAPACITY"), PAR_EVENTS_CAPACITY as u64);
        assert_eq!(c_define("PAR_TASK_TRACE_SIZE"), PAR_TASK_TRACE_SIZE as u64);
        assert_eq!(c_define("PAR_EVENT_MAIN"), PAR_EVENT_MAIN);
        assert_eq!(c_define("PAR_EVENT_REGION"), PAR_EVENT_REGION);
        assert_eq!(c_define("PAR_EVENT_TASK_CHUNK"), PAR_EVENT_TASK_CHUNK);
        assert_eq!(c_define("PAR_EVENT_TASK_DONE"), PAR_EVENT_TASK_DONE);
        assert_eq!(c_define("PAR_EVENT_REGION_END"), PAR_EVENT_REGION_END);
        // version, tail, the six words of the report; then the slots, four words each
        assert_eq!(std::mem::size_of::<ParEventsHeader>(), 8 * 8);
        assert_eq!(std::mem::size_of::<ParEvent>(), 4 * 8);
        assert_eq!(PAR_EVENTS_BYTES, 64 + 32 * ((1 << 18) + (1 << 11)));
    }

    /// The codes the readers name a fork-join abort by (`AsmRunError::from_exit_code`).
    #[test]
    fn abort_codes_match_the_c_header() {
        assert_eq!(par_define("PAR_ABORT_ERROR"), PAR_ABORT_ERROR);
        assert_eq!(par_define("PAR_ABORT_TASK_ENDED"), PAR_ABORT_TASK_ENDED);
        assert_eq!(par_define("PAR_ABORT_TASK_FAULTED"), PAR_ABORT_TASK_FAULTED);
        for (name, code) in [
            ("PAR_EXIT_SETUP", PAR_EXIT_SETUP),
            ("PAR_EXIT_ENDED", PAR_EXIT_ENDED),
            ("PAR_EXIT_ABORTED", PAR_EXIT_ABORTED),
            ("PAR_EXIT_STATE", PAR_EXIT_STATE),
            ("PAR_EXIT_PUBLISH", PAR_EXIT_PUBLISH),
            ("PAR_EXIT_SHM_FULL", PAR_EXIT_SHM_FULL),
            ("PAR_EXIT_MAX_STEPS", PAR_EXIT_MAX_STEPS),
            ("PAR_EXIT_RESET", PAR_EXIT_RESET),
        ] {
            assert_eq!(par_define(name), code, "{name}");
        }
    }

    /// The report's words and codes (`RegionReport`), in the order the header lays them out.
    #[test]
    fn report_matches_the_c_header() {
        let header = include_str!("../../src/par_events.hpp");
        let fields: Vec<&str> = header
            .lines()
            .skip_while(|l| !l.contains("uint64_t version;"))
            .take_while(|l| !l.contains("ParEvent events["))
            .filter_map(|l| l.trim().strip_prefix("uint64_t "))
            .map(|l| l.split(';').next().unwrap())
            .collect();
        assert_eq!(
            fields,
            [
                "version",
                "tail",
                "regions_sequential",
                "sequential_reasons",
                "failure",
                "failure_region",
                "failure_task",
                "failure_detail"
            ]
        );
        for (name, code) in [
            ("PAR_SEQUENTIAL_CHUNK_SIZE", PAR_SEQUENTIAL_CHUNK_SIZE),
            ("PAR_SEQUENTIAL_HINTS", PAR_SEQUENTIAL_HINTS),
            ("PAR_SEQUENTIAL_WORKERS", PAR_SEQUENTIAL_WORKERS),
            ("PAR_SEQUENTIAL_FEW_TASKS", PAR_SEQUENTIAL_FEW_TASKS),
            ("PAR_SEQUENTIAL_BUDGET", PAR_SEQUENTIAL_BUDGET),
            ("PAR_SEQUENTIAL_TASK_LIST", PAR_SEQUENTIAL_TASK_LIST),
            ("PAR_SEQUENTIAL_FIRST_PASS", PAR_SEQUENTIAL_FIRST_PASS),
            ("PAR_SEQUENTIAL_HOST", PAR_SEQUENTIAL_HOST),
            ("PAR_FAILURE_TASK_EXIT", PAR_FAILURE_TASK_EXIT),
            ("PAR_FAILURE_TASK_SIGNAL", PAR_FAILURE_TASK_SIGNAL),
            ("PAR_FAILURE_TASK_RECORD", PAR_FAILURE_TASK_RECORD),
            ("PAR_FAILURE_MAX_STEPS", PAR_FAILURE_MAX_STEPS),
            ("PAR_FAILURE_HOST", PAR_FAILURE_HOST),
            ("PAR_FAILURE_EVENTS", PAR_FAILURE_EVENTS),
            ("PAR_FAILURE_PROTOCOL", PAR_FAILURE_PROTOCOL),
        ] {
            assert_eq!(c_define(name), code, "{name}");
        }
        assert!(header.contains("#define PAR_NONE UINT64_MAX") && PAR_NONE == u64::MAX);
    }

    /// A report decodes to the reasons it names, each once, and to the failure it describes.
    #[test]
    fn reports_decode_to_their_reasons_and_failure() {
        let header = |reasons: u64, failure, region, task, detail| ParEventsHeader {
            version: PAR_EVENTS_VERSION,
            tail: 0,
            regions_sequential: 3,
            sequential_reasons: reasons,
            failure,
            failure_region: region,
            failure_task: task,
            failure_detail: detail,
        };

        let quiet = RegionReport::decode(&header(0, 0, PAR_NONE, PAR_NONE, 0));
        assert_eq!((quiet.sequential_reasons(), quiet.failure), (vec![], None));

        let report = RegionReport::decode(&header(
            (1 << PAR_SEQUENTIAL_HINTS) | (1 << PAR_SEQUENTIAL_BUDGET),
            PAR_FAILURE_TASK_EXIT,
            1,
            3,
            PAR_EXIT_SHM_FULL,
        ));
        assert_eq!(report.regions_sequential, 3);
        assert_eq!(
            report.sequential_reasons(),
            [SequentialReason::Hints, SequentialReason::Budget]
        );
        assert!(SequentialReason::Hints.is_configuration());
        assert!(!SequentialReason::Budget.is_configuration());
        let failure = report.failure.unwrap();
        assert_eq!(failure.task_at(), Some(TaskAt { region: 1, task: 3 }));
        assert_eq!(failure.cause, FailureCause::ShmFull);
        assert!(failure.to_string().starts_with("task 3 of region 1: "), "{failure}");
        assert!(failure.to_string().contains("/dev/shm"), "{failure}");

        let decode = |failure, region, task, detail| {
            RegionReport::decode(&header(0, failure, region, task, detail)).failure.unwrap()
        };
        let ended = decode(PAR_FAILURE_TASK_EXIT, 0, 2, PAR_EXIT_ENDED);
        assert_eq!(ended.cause, FailureCause::GuestEnded);
        let killed = decode(PAR_FAILURE_TASK_SIGNAL, 0, 5, 9);
        assert_eq!(killed.cause, FailureCause::TaskKilled { signal: 9 });
        let host = decode(PAR_FAILURE_HOST, 2, PAR_NONE, 12);
        assert_eq!(
            (host.region, host.task, host.cause),
            (Some(2), None, FailureCause::Host { errno: 12 })
        );
        assert!(host
            .to_string()
            .starts_with("region 2: the service could not set the region up ("));
        let protocol = decode(PAR_FAILURE_PROTOCOL, PAR_NONE, PAR_NONE, 0);
        assert_eq!((protocol.task_at(), protocol.cause), (None, FailureCause::Protocol));
        assert!(protocol.to_string().starts_with("the guest broke"), "{protocol}");
        let full = decode(PAR_FAILURE_EVENTS, PAR_NONE, PAR_NONE, 0);
        assert!(full.to_string().contains("more chunk events"), "{full}");
        let unknown = decode(99, PAR_NONE, PAR_NONE, 7);
        assert_eq!(unknown.cause, FailureCause::Unknown { failure: 99, detail: 7 });
    }

    #[test]
    fn events_decode_by_kind() {
        let event = |kind, a, b, c| ChunkEvent::decode(ParEvent { kind, a, b, c }).unwrap();
        assert_eq!(event(PAR_EVENT_MAIN, 0, 0, 0), ChunkEvent::Main);
        assert_eq!(
            event(PAR_EVENT_REGION, 1, 8, 1 << 18),
            ChunkEvent::Region { region: 1, tasks: 8 }
        );
        assert_eq!(
            event(PAR_EVENT_TASK_CHUNK, 3, 2, 4096),
            ChunkEvent::TaskChunk { task: 3, index: 2, offset: 4096 }
        );
        assert_eq!(
            event(PAR_EVENT_TASK_DONE, 3, 3 << 18, 3),
            ChunkEvent::TaskDone { task: 3, steps: 3 << 18, chunks: 3 }
        );
        assert_eq!(event(PAR_EVENT_REGION_END, 1, 0, 0), ChunkEvent::RegionEnd { region: 1 });
        assert!(ChunkEvent::decode(ParEvent { kind: 9, a: 0, b: 0, c: 0 }).is_err());
    }

    const CHUNK: u64 = 1 << 18;

    /// What a reader does with a region's events: releases (task, index, shift) in order.
    fn release_all(seq: &mut RegionSequencer<(u64, u64)>, released: &mut Vec<(u64, u64, u64)>) {
        seq.release(|shift, (task, index)| {
            released.push((task, index, shift));
            Ok(())
        })
        .unwrap();
    }

    /// Three tasks of 2, 1 and 3 chunks, their chunks interleaved and their ends out of order,
    /// are released in the sequential order, each task shifted by the steps before it, and
    /// task 0 as its chunks come.
    #[test]
    fn interleaved_tasks_are_released_in_order_with_their_shifts() {
        let mut seq = RegionSequencer::new(3, CHUNK);
        let mut released = Vec::new();

        seq.chunk(2, 0, (2, 0)).unwrap();
        release_all(&mut seq, &mut released);
        assert!(released.is_empty(), "task 2 waits for tasks 0 and 1");

        seq.chunk(0, 0, (0, 0)).unwrap();
        release_all(&mut seq, &mut released);
        assert_eq!(released, [(0, 0, 0)], "the frontier task goes as its chunks come");

        seq.chunk(1, 0, (1, 0)).unwrap();
        seq.done(1, CHUNK, 1).unwrap();
        seq.chunk(2, 1, (2, 1)).unwrap();
        release_all(&mut seq, &mut released);
        assert_eq!(released.len(), 1, "task 1 is done but task 0 is not");

        seq.chunk(0, 1, (0, 1)).unwrap();
        seq.done(0, 2 * CHUNK, 2).unwrap();
        release_all(&mut seq, &mut released);
        assert_eq!(
            released,
            [(0, 0, 0), (0, 1, 0), (1, 0, 2 * CHUNK), (2, 0, 3 * CHUNK), (2, 1, 3 * CHUNK)],
            "task 0 done releases task 1, and task 2's backlog after it"
        );
        assert!(!seq.is_complete());

        seq.chunk(2, 2, (2, 2)).unwrap();
        release_all(&mut seq, &mut released);
        assert_eq!(released.last(), Some(&(2, 2, 3 * CHUNK)), "task 2 is the frontier now");
        seq.done(2, 3 * CHUNK, 3).unwrap();
        release_all(&mut seq, &mut released);
        assert!(seq.is_complete());
        assert_eq!(seq.steps_released(), 6 * CHUNK);
        assert_eq!(released.len(), 6);
    }

    #[test]
    fn a_region_of_one_task_of_one_chunk() {
        let mut seq = RegionSequencer::new(1, CHUNK);
        let mut released = Vec::new();
        seq.chunk(0, 0, (0, 0)).unwrap();
        seq.done(0, CHUNK, 1).unwrap();
        release_all(&mut seq, &mut released);
        assert_eq!(released, [(0, 0, 0)]);
        assert!(seq.is_complete());
    }

    #[test]
    fn a_task_done_with_chunks_missing_or_out_of_order_is_rejected() {
        let mut seq = RegionSequencer::<(u64, u64)>::new(2, CHUNK);
        assert!(seq.chunk(0, 1, (0, 1)).is_err(), "chunk 1 before chunk 0");
        seq.chunk(0, 0, (0, 0)).unwrap();
        assert!(seq.done(0, 2 * CHUNK, 2).is_err(), "done after 1 of 2 chunks");
        assert!(seq.done(0, CHUNK + 1, 1).is_err(), "steps that are not whole chunks");
        assert!(seq.chunk(5, 0, (5, 0)).is_err(), "no task 5");
        seq.done(0, CHUNK, 1).unwrap();
        assert!(seq.chunk(0, 1, (0, 1)).is_err(), "a chunk after done");
        assert!(seq.done(0, CHUNK, 1).is_err(), "done twice");
    }

    /// A reader's whole sequence: main chunks, a region whose tasks finish in reverse order,
    /// main chunks again; the chunk ids follow the sequential order.
    #[test]
    fn main_chunks_around_a_region_keep_the_sequential_ids() {
        #[derive(Debug, PartialEq)]
        enum Item {
            Main(u64),
            Task(u64, u64, u64),
        }
        let events = [
            ChunkEvent::Main,
            ChunkEvent::Main,
            ChunkEvent::Region { region: 0, tasks: 2 },
            ChunkEvent::TaskChunk { task: 1, index: 0, offset: 0 },
            ChunkEvent::TaskChunk { task: 1, index: 1, offset: 0 },
            ChunkEvent::TaskDone { task: 1, steps: 2 * CHUNK, chunks: 2 },
            ChunkEvent::TaskChunk { task: 0, index: 0, offset: 0 },
            ChunkEvent::TaskDone { task: 0, steps: CHUNK, chunks: 1 },
            ChunkEvent::RegionEnd { region: 0 },
            ChunkEvent::Main,
        ];
        let mut ids: Vec<Item> = Vec::new();
        let mut main = 0;
        let mut seq: Option<RegionSequencer<(u64, u64)>> = None;
        for event in events {
            match event {
                ChunkEvent::Main => {
                    ids.push(Item::Main(main));
                    main += 1;
                }
                ChunkEvent::Region { tasks, .. } => {
                    seq = Some(RegionSequencer::new(tasks as usize, CHUNK))
                }
                ChunkEvent::TaskChunk { task, index, .. } => {
                    let s = seq.as_mut().unwrap();
                    s.chunk(task, index, (task, index)).unwrap();
                    s.release(|shift, (t, k)| {
                        ids.push(Item::Task(t, k, shift));
                        Ok(())
                    })
                    .unwrap();
                }
                ChunkEvent::TaskDone { task, steps, chunks } => {
                    let s = seq.as_mut().unwrap();
                    s.done(task, steps, chunks).unwrap();
                    s.release(|shift, (t, k)| {
                        ids.push(Item::Task(t, k, shift));
                        Ok(())
                    })
                    .unwrap();
                }
                ChunkEvent::RegionEnd { .. } => {
                    assert!(seq.take().unwrap().is_complete());
                }
            }
        }
        assert_eq!(
            ids,
            [
                Item::Main(0),
                Item::Main(1),
                Item::Task(0, 0, 0),
                Item::Task(1, 0, CHUNK),
                Item::Task(1, 1, CHUNK),
                Item::Main(2),
            ]
        );
    }
}
