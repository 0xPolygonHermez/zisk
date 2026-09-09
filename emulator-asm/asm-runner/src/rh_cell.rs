//! [`RhCell`] — handoff point for the ASM ROM-histogram runner.
//!
//! The RH runner thread is spawned at the start of an ASM execution and finishes
//! some time after the minimal-trace run does. The executor used to join it inside
//! `execute()`, which put the runner's residual on the critical path of every phase
//! that follows. This cell moves the join to the point of use: the executor *parks*
//! the [`RhJoinHandle`] here as soon as the emulation returns, and the ROM instance
//! reads the histogram through [`RhCell::with_histogram`] when it computes its
//! witness — joining then, if the runner has not finished yet.
//!
//! The `JoinHandle` is the whole synchronisation mechanism: it carries the value,
//! carries the error, and blocks a reader that arrives early. The cell adds only
//! caching (so the value survives a second read) and shared ownership.
//!
//! Shared ownership is the reason this type exists. Two owners need the same handle,
//! with different lifetimes:
//!
//! * the ROM instance, which reads the histogram at witness time and is rebuilt on
//!   every job;
//! * the executor, which must reach the handle at the start of the *next* job to
//!   guarantee the RH child has answered before the input shared memory is reset —
//!   draining that memory's semaphores mid-emulation strands the child, which then
//!   holds the RH reader lock and hangs the next job's runner.
//!
//! So the cell lives in the long-lived `RomSM` and is shared with the instance
//! behind an `Arc`.
//!
//! # Lifecycle
//!
//! The order the five methods happen in — each one's own docs say who drives it:
//! [`park`](RhCell::park) → [`is_armed`](RhCell::is_armed) →
//! [`resolve`](RhCell::resolve) → [`with_histogram`](RhCell::with_histogram) →
//! [`drain`](RhCell::drain).
//!
//! `resolve` is an optimisation, not a precondition: skipping it leaves the join to the
//! first reader, which is correct but blocks a thread that holds scheduler budget.
//! `drain` is *not* optional — see the shared-memory note above.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use thiserror::Error;

use crate::{AsmRHData, AsmRunnerRH};

/// Handle to a spawned ROM-histogram runner thread, as parked by the executor.
pub type RhJoinHandle = JoinHandle<anyhow::Result<AsmRunnerRH>>;

/// Why a histogram read could not be served.
#[derive(Debug, Clone, Error)]
pub enum RhCellError {
    /// The runner thread ran to completion but reported an error.
    #[error("ROM histogram runner failed: {0}")]
    RunnerFailed(String),

    /// The runner thread panicked; its payload is lost by the time we join.
    #[error("ROM histogram runner thread panicked")]
    ThreadPanicked,

    /// No runner was parked for this execution. On the ASM path this means the
    /// handoff was skipped — never that the histogram is empty.
    #[error("no ROM histogram runner was parked for this execution")]
    NotArmed,
}

/// Parked runner handle plus the joined histogram, shared between the ROM state
/// machine and its instance. See the module docs for the ownership rationale.
#[derive(Default)]
pub struct RhCell {
    inner: Mutex<RhState>,
}

/// One execution's runner, in whichever stage it has reached. The type is the
/// invariant: a handle that has been joined is gone, and its outcome — histogram or
/// failure — is what replaces it.
#[derive(Default)]
enum RhState {
    /// Nothing parked: a fresh cell, or one drained at a job boundary.
    #[default]
    Empty,
    /// Runner still to be joined.
    Parked(RhJoinHandle),
    /// Joined histogram, kept (not taken) so a recomputed witness reads it again.
    /// `asm_rowh_output` aliases the shared-memory mapping.
    Ready(AsmRunnerRH),
    /// A join that produced no histogram, replayed to every later reader.
    Failed(RhCellError),
}

impl RhState {
    /// Joins any parked handle and drops everything this execution left behind.
    /// Joining a thread that has already returned does not block.
    fn clear(&mut self) {
        // The value is discarded: a handle still parked here belongs to an execution
        // nobody consumed (typically a cancelled proof).
        if let Self::Parked(handle) = std::mem::take(self) {
            let _ = handle.join();
        }
    }

    /// Joins the runner if that has not happened yet, and returns the histogram.
    ///
    /// The returned reference is what makes the join-once rule checkable: only the
    /// `Ready` arm can produce one.
    fn resolve(&mut self) -> Result<&AsmRHData, RhCellError> {
        *self = match std::mem::take(self) {
            Self::Parked(handle) => join_runner(handle).map_or_else(Self::Failed, Self::Ready),
            settled => settled,
        };
        match self {
            Self::Ready(runner) => Ok(&runner.asm_rowh_output),
            Self::Failed(failure) => Err(failure.clone()),
            Self::Empty => Err(RhCellError::NotArmed),
            Self::Parked(_) => unreachable!("just resolved"),
        }
    }
}

/// Joins a ROM-histogram runner thread and names what went wrong if it did not
/// deliver. The sole decode of a runner join, so every reader sees one vocabulary.
///
/// The whole point of parking the handle is that this join finds the runner already
/// finished. It reports when it does not: that residual used to be the `WAIT_ASM_RH`
/// timer, and this log is now the only place it is visible.
fn join_runner(handle: RhJoinHandle) -> Result<AsmRunnerRH, RhCellError> {
    let started = std::time::Instant::now();
    let outcome = handle.join();
    let waited = started.elapsed();
    if waited > std::time::Duration::from_millis(1) {
        tracing::info!("ROM histogram was not ready; witness waited {waited:?} for it");
    }

    match outcome {
        Ok(Ok(runner)) => Ok(runner),
        Ok(Err(e)) => Err(RhCellError::RunnerFailed(format!("{e:#}"))),
        Err(_) => Err(RhCellError::ThreadPanicked),
    }
}

impl RhCell {
    /// Creates an empty cell. A cell stays empty until an ASM execution parks its
    /// runner, which is what makes [`Self::is_armed`] a sound ASM-mode test.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parks this execution's runner handle, discarding anything a previous
    /// execution left behind.
    ///
    /// A handle still parked here means the previous job was never drained (a
    /// cancelled proof); it is joined and dropped rather than leaked into this job,
    /// which does not block in practice because it finished long ago.
    pub fn park(&self, handle: RhJoinHandle) {
        let mut state = self.lock();
        state.clear();
        *state = RhState::Parked(handle);
    }

    /// Returns `true` once this execution has parked a runner, until it is drained.
    ///
    /// The ROM state machine uses this to pick the ASM path when building its instance.
    /// It is `true` from the moment the handle is parked — before the runner has
    /// finished — so the choice does not depend on runner timing, and it stays `true`
    /// for a runner that *failed*.
    ///
    /// The failure case is the subtle one, and the reason the predicate is "not empty"
    /// rather than "has a histogram": answering `false` there would send the ROM state
    /// machine down its Rust backend, whose collectors an ASM execution never fills, so
    /// the proof would carry an all-zero ROM trace instead of reporting the failure.
    pub fn is_armed(&self) -> bool {
        !matches!(*self.lock(), RhState::Empty)
    }

    /// Calls `f` with this execution's histogram, joining the runner first if it is
    /// still in flight.
    ///
    /// The histogram is borrowed rather than handed over: it aliases a shared-memory
    /// mapping that must not be moved out or dropped here, and a witness may be
    /// recomputed later in the same proof.
    ///
    /// `f` runs while the cell's lock is held — which costs nothing today, since the
    /// readers are sequential and on one thread, but a second concurrent reader would
    /// serialise against whatever `f` does.
    ///
    /// # Errors
    ///
    /// Returns [`RhCellError`] if the runner failed or panicked, or if no runner was
    /// parked for this execution. A failure is remembered, so every later reader
    /// sees the same error instead of `NotArmed`.
    pub fn with_histogram<T>(&self, f: impl FnOnce(&AsmRHData) -> T) -> Result<T, RhCellError> {
        Ok(f(self.lock().resolve()?))
    }

    /// Joins the runner now and caches its histogram, so a later [`Self::with_histogram`]
    /// cannot block.
    ///
    /// Called once the work the runner was overlapping is done, from a thread that holds
    /// no scheduler budget — the readers that come later run on proofman witness threads,
    /// which hold core permits while they wait. Errors as [`Self::with_histogram`] does.
    pub fn resolve(&self) -> Result<(), RhCellError> {
        self.with_histogram(|_| ())
    }

    /// Joins a still-parked runner and drops this execution's histogram.
    ///
    /// Called at the job boundary, before the input shared memory is rewound: the RH
    /// child must have answered by then, and it releases the previous histogram
    /// before the next run overwrites the segment it points into.
    pub fn drain(&self) {
        self.lock().clear();
    }

    /// A poisoned lock carries no broken invariant here — the histogram is either
    /// present or it is not — so the guard is recovered rather than propagated.
    fn lock(&self) -> MutexGuard<'_, RhState> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn histogram(steps: u64, counts: Vec<u64>) -> AsmRunnerRH {
        AsmRunnerRH::new(AsmRHData::new(steps, counts, Vec::new()))
    }

    /// Parks a handle whose thread has already returned `value`.
    fn parked(cell: &RhCell, value: anyhow::Result<AsmRunnerRH>) {
        cell.park(std::thread::spawn(move || value));
    }

    /// Parks a runner that panics, the shape of a runner that died mid-histogram.
    fn parked_panicking(cell: &RhCell) {
        cell.park(std::thread::spawn(|| panic!("RH runner died")));
    }

    /// How long [`parked_slow`]'s runner takes to deliver. Shared so the assertions that
    /// depend on it cannot drift apart from the sleep.
    const SLOW_RUNNER: std::time::Duration = std::time::Duration::from_millis(50);

    /// Parks a runner that has *not* finished yet, delivering `histogram(11, vec![9])`.
    fn parked_slow(cell: &RhCell) {
        cell.park(std::thread::spawn(|| {
            std::thread::sleep(SLOW_RUNNER);
            Ok(histogram(11, vec![9]))
        }));
    }

    #[test]
    fn fresh_cell_is_not_armed_and_serves_nothing() {
        let cell = RhCell::new();
        assert!(!cell.is_armed());
        let err = cell.with_histogram(|_| ()).expect_err("empty cell must not serve a histogram");
        assert!(matches!(err, RhCellError::NotArmed), "got {err:?}");
    }

    #[test]
    fn armed_as_soon_as_a_handle_is_parked() {
        let cell = RhCell::new();
        parked(&cell, Ok(histogram(7, vec![1, 0, 2])));
        assert!(cell.is_armed(), "parking must arm the cell before the runner is joined");
    }

    #[test]
    fn histogram_is_readable_twice() {
        let cell = RhCell::new();
        parked(&cell, Ok(histogram(50, vec![3, 0, 1])));

        let first = cell.with_histogram(|h| (h.steps, h.inst_count.to_vec())).expect("first read");
        assert_eq!(first, (50, vec![3, 0, 1]));

        // The joined value is cached, not taken: a recomputed witness reads it again.
        let second = cell.with_histogram(|h| h.steps).expect("second read");
        assert_eq!(second, 50);
        assert!(cell.is_armed());
    }

    #[test]
    fn read_waits_for_a_runner_still_in_flight() {
        let cell = RhCell::new();
        parked_slow(&cell);

        let steps = cell.with_histogram(|h| h.steps).expect("read must join the runner");
        assert_eq!(steps, 11);
    }

    #[test]
    fn runner_error_is_reported_and_replayed() {
        let cell = RhCell::new();
        parked(&cell, Err(anyhow::anyhow!("child process exited with code: 11")));

        let err = cell.with_histogram(|_| ()).expect_err("a failed runner must not serve data");
        assert!(
            matches!(&err, RhCellError::RunnerFailed(m) if m.contains("code: 11")),
            "got {err:?}"
        );

        // Later readers see the same failure, never `NotArmed`.
        let again = cell.with_histogram(|_| ()).expect_err("failure must be remembered");
        assert!(matches!(again, RhCellError::RunnerFailed(_)), "got {again:?}");
    }

    #[test]
    fn panicked_runner_is_reported() {
        let cell = RhCell::new();
        parked_panicking(&cell);

        let err = cell.with_histogram(|_| ()).expect_err("a panicked runner must surface an error");
        assert!(matches!(err, RhCellError::ThreadPanicked), "got {err:?}");
    }

    #[test]
    fn resolve_caches_so_a_later_read_cannot_block() {
        let cell = RhCell::new();
        parked_slow(&cell);

        cell.resolve().expect("resolve joins the runner");

        // The handle is gone, so this read has nothing left to wait for.
        let started = std::time::Instant::now();
        let steps = cell.with_histogram(|h| h.steps).expect("read after resolve");
        assert_eq!(steps, 11);
        assert!(started.elapsed() < SLOW_RUNNER / 2, "read must not join");
    }

    #[test]
    fn a_failed_runner_leaves_the_cell_armed() {
        // Armed-ness answers "does this execution's ROM witness come from the assembly",
        // which stays true when the runner failed. Reporting `false` here would send
        // `RomSM::build_instance` down the Rust path, whose collectors the ASM run never
        // fills — an all-zero ROM trace instead of an error.
        let cell = RhCell::new();
        parked_panicking(&cell);
        cell.resolve().expect_err("the runner failed");

        assert!(cell.is_armed(), "a failed execution is still an ASM execution");
    }

    #[test]
    fn parking_again_clears_a_previous_failure() {
        let cell = RhCell::new();
        parked_panicking(&cell);
        cell.with_histogram(|_| ()).expect_err("first execution failed");

        parked(&cell, Ok(histogram(9, vec![4])));

        let steps = cell.with_histogram(|h| h.steps).expect("the new execution serves its own");
        assert_eq!(steps, 9, "a remembered failure must not outlive its execution");
    }

    #[test]
    fn drain_clears_a_remembered_failure() {
        let cell = RhCell::new();
        parked_panicking(&cell);
        cell.resolve().expect_err("the runner failed");

        cell.drain();

        assert!(!cell.is_armed());
        let err = cell.with_histogram(|_| ()).expect_err("drained cell serves nothing");
        assert!(matches!(err, RhCellError::NotArmed), "the failure is gone, not replayed: {err:?}");
    }

    /// The cell is shared behind an `Arc` between the ROM instance and the executor, so it
    /// has to stay `Send + Sync` even though today's readers are sequential.
    #[test]
    fn the_cell_is_shareable_across_threads() {
        fn assert_shareable<T: Send + Sync>() {}
        assert_shareable::<RhCell>();
    }

    #[test]
    fn drain_disarms_the_cell() {
        let cell = RhCell::new();
        parked(&cell, Ok(histogram(1, vec![1])));
        cell.with_histogram(|_| ()).expect("read before drain");

        cell.drain();

        assert!(!cell.is_armed(), "a drained cell must not claim ASM mode for the next job");
        let err = cell.with_histogram(|_| ()).expect_err("drained cell serves nothing");
        assert!(matches!(err, RhCellError::NotArmed), "got {err:?}");
    }

    #[test]
    fn drain_joins_a_handle_nobody_consumed() {
        let cell = RhCell::new();
        parked(&cell, Ok(histogram(2, vec![2])));

        // No read happened — the cancelled-proof path. Draining must still clear it.
        cell.drain();
        assert!(!cell.is_armed());
    }

    #[test]
    fn parking_again_discards_the_previous_execution() {
        let cell = RhCell::new();
        parked(&cell, Ok(histogram(100, vec![5])));
        cell.with_histogram(|_| ()).expect("first execution read");

        parked(&cell, Ok(histogram(200, vec![6])));

        let steps = cell.with_histogram(|h| h.steps).expect("second execution read");
        assert_eq!(steps, 200, "a new execution must not serve the previous histogram");
    }
}
