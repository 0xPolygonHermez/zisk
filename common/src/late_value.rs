//! [`LateValue`] — a value computed by a worker thread and read where it is used.
//!
//! Some producers are started early and finish late: a thread is spawned at the
//! beginning of a phase and its result is not needed until the end of the next
//! one. Joining it at the point it was spawned puts the producer's whole tail on
//! the critical path; joining it at the point of use puts only the part that had
//! not finished yet — usually nothing.
//!
//! `LateValue` is the handoff point between the two. The party that spawns the
//! producer [`park`](LateValue::park)s its handle here; the party that needs the
//! value reads it through [`with`](LateValue::with), which joins the producer
//! first if it is still running. Both hold the same `LateValue` (behind an
//! `Arc`), because they are usually different components with different
//! lifetimes — the motivating case is the ASM ROM-histogram runner, parked by
//! the executor and read by the ROM state machine's instance, which is rebuilt
//! on every job while the state machine is not.
//!
//! # Why the `JoinHandle` is the whole mechanism
//!
//! There is no condition variable and no ready flag: the handle already carries
//! the value, carries the error, and blocks a reader that arrives early. This
//! type adds only what the handle lacks — caching, so a second reader (or a
//! recomputation) sees the same value, and shared ownership.
//!
//! # The value is borrowed, never handed over
//!
//! [`with`](LateValue::with) lends the value to a closure and there is no
//! `take`. Producers whose result borrows foreign memory — a shared-memory
//! mapping, say — stay valid exactly as long as this cell holds them, which is
//! until [`drain`](LateValue::drain). That is a contract the API can keep;
//! handing the value out would make it a convention.
//!
//! # Lifecycle
//!
//! [`park`](LateValue::park) → [`is_armed`](LateValue::is_armed) →
//! [`resolve`](LateValue::resolve) → [`with`](LateValue::with) →
//! [`drain`](LateValue::drain).
//!
//! `resolve` is an optimisation, not a precondition: skipping it leaves the join
//! to the first reader, which is correct but blocks whichever thread that is.
//! Whether `drain` is optional depends on the producer — one that owns an
//! external resource has to be retired before that resource is reused.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use thiserror::Error;

/// How long a join may take before it is reported as a wait the consumer paid.
const WAIT_LOG_THRESHOLD: Duration = Duration::from_millis(1);

/// Handle to the thread producing a [`LateValue`]'s value.
///
/// The producer maps its own failures to [`LateValueError`] before returning, so
/// the cell needs to know nothing about where the value comes from.
pub type LateJoinHandle<T> = JoinHandle<Result<T, LateValueError>>;

/// Why a [`LateValue`] could not be served.
#[derive(Debug, Clone, Error)]
pub enum LateValueError {
    /// The producer ran to completion but reported an error.
    #[error("late value producer failed: {0}")]
    Failed(String),

    /// The producer thread panicked; its payload is lost by the time we join.
    #[error("late value producer thread panicked")]
    Panicked,

    /// Nothing was parked, so there is no value and no producer to wait for.
    /// Distinct from a producer that failed — see [`LateValue::is_armed`].
    #[error("no producer was parked for this late value")]
    NotArmed,
}

impl LateValueError {
    /// Names a producer-side failure, from whatever error type the producer uses.
    pub fn failed(cause: impl std::fmt::Display) -> Self {
        Self::Failed(cause.to_string())
    }
}

/// A value produced by a worker thread, joined at the point of use.
///
/// See the module docs for the ownership and lifetime rationale.
pub struct LateValue<T> {
    /// Names the value in wait logs; the cell is generic, its diagnostics are not.
    label: &'static str,
    inner: Mutex<State<T>>,
}

/// One round's producer, in whichever stage it has reached. The type is the
/// invariant: a handle that has been joined is gone, and its outcome — value or
/// failure — is what replaces it.
enum State<T> {
    /// Nothing parked: a fresh cell, or one that has been drained.
    Empty,
    /// Producer still to be joined.
    Parked(LateJoinHandle<T>),
    /// Joined value, kept (not taken) so a second read sees it again.
    Ready(T),
    /// A join that produced no value, replayed to every later reader.
    Failed(LateValueError),
}

// Hand-written rather than derived: `Empty` carries no `T`, so the cell must not
// inherit a `T: Default` bound it has no use for.
impl<T> Default for State<T> {
    fn default() -> Self {
        Self::Empty
    }
}

impl<T> State<T> {
    /// Joins any parked producer and drops everything this round left behind.
    /// Joining a thread that has already returned does not block.
    fn clear(&mut self) {
        // The value is discarded: a handle still parked here belongs to a round
        // nobody consumed.
        if let Self::Parked(handle) = std::mem::take(self) {
            let _ = handle.join();
        }
    }

    /// Joins the producer if that has not happened yet, and returns the value.
    ///
    /// The returned reference is what makes the join-once rule checkable: only
    /// the `Ready` arm can produce one.
    fn resolve(&mut self, label: &'static str) -> Result<&T, LateValueError> {
        *self = match std::mem::take(self) {
            Self::Parked(handle) => {
                join_producer(label, handle).map_or_else(Self::Failed, Self::Ready)
            }
            settled => settled,
        };
        match self {
            Self::Ready(value) => Ok(value),
            Self::Failed(failure) => Err(failure.clone()),
            Self::Empty => Err(LateValueError::NotArmed),
            Self::Parked(_) => unreachable!("just resolved"),
        }
    }
}

/// Joins a producer thread and names what went wrong if it did not deliver. The
/// sole decode of a join, so every reader sees one vocabulary.
///
/// Parking the handle is worthwhile precisely when this join finds the producer
/// already finished; the log reports when it does not, which is the only place
/// that residual is visible.
fn join_producer<T>(label: &'static str, handle: LateJoinHandle<T>) -> Result<T, LateValueError> {
    let started = Instant::now();
    let outcome = handle.join();
    let waited = started.elapsed();
    if waited > WAIT_LOG_THRESHOLD {
        tracing::info!("{label} was not ready; its consumer waited {waited:?} for it");
    }

    match outcome {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(failure)) => Err(failure),
        Err(_) => Err(LateValueError::Panicked),
    }
}

impl<T> LateValue<T> {
    /// Creates an empty cell, labelled for wait logs.
    ///
    /// A cell stays empty until a producer is parked, which is what makes
    /// [`is_armed`](Self::is_armed) a sound "this round has one" test.
    pub fn new(label: &'static str) -> Self {
        Self { label, inner: Mutex::new(State::Empty) }
    }

    /// Creates a cell already holding `value`, with no producer behind it.
    ///
    /// For callers that have the value up front, and for tests that want a cell
    /// to read from without spawning anything.
    pub fn ready(label: &'static str, value: T) -> Self {
        Self { label, inner: Mutex::new(State::Ready(value)) }
    }

    /// Parks this round's producer, discarding anything a previous round left.
    ///
    /// A handle still parked here belongs to a round that was never drained; it
    /// is joined and dropped rather than leaked into this one, which does not
    /// block in practice because it finished long ago.
    pub fn park(&self, handle: LateJoinHandle<T>) {
        let mut state = self.lock();
        state.clear();
        *state = State::Parked(handle);
    }

    /// Returns `true` once a producer has been parked, until the cell is drained.
    ///
    /// It is `true` from the moment the handle is parked — before the producer
    /// has finished — so callers that branch on it do not depend on timing, and
    /// it stays `true` for a producer that *failed*. That last case is the
    /// subtle one, and the reason the predicate is "not empty" rather than "has
    /// a value": answering `false` there would make a failure indistinguishable
    /// from a round that never had a producer, and a caller choosing a fallback
    /// path would silently take it.
    pub fn is_armed(&self) -> bool {
        !matches!(*self.lock(), State::Empty)
    }

    /// Calls `f` with this round's value, joining the producer first if it is
    /// still running.
    ///
    /// The value is borrowed, not handed over: it may be read again later, and a
    /// producer whose result borrows foreign memory needs this cell to keep
    /// owning it. `f` runs while the cell's lock is held, so a second concurrent
    /// reader serialises against whatever `f` does.
    ///
    /// # Errors
    ///
    /// [`LateValueError`] if the producer failed or panicked, or if none was
    /// parked. A failure is remembered, so every later reader sees the same
    /// error rather than [`LateValueError::NotArmed`].
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> Result<R, LateValueError> {
        Ok(f(self.lock().resolve(self.label)?))
    }

    /// Joins the producer now and caches its value, so a later [`with`](Self::with)
    /// cannot block.
    ///
    /// Call it once the work the producer was overlapping is done, from a thread
    /// that can afford to wait — the readers that come afterwards may not be.
    /// Errors as [`with`](Self::with) does.
    pub fn resolve(&self) -> Result<(), LateValueError> {
        self.with(|_| ())
    }

    /// Joins a still-parked producer and drops this round's value.
    ///
    /// Call it before whatever the producer's result borrows is reused, and
    /// before a new round begins: it guarantees the producer has finished, and
    /// releases the value it delivered.
    pub fn drain(&self) {
        self.lock().clear();
    }

    /// A poisoned lock carries no broken invariant here — the value is either
    /// present or it is not — so the guard is recovered rather than propagated.
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    const LABEL: &str = "test value";

    /// How long [`parked_slow`]'s producer takes to deliver. Shared so the
    /// assertions that depend on it cannot drift apart from the sleep.
    const SLOW: Duration = Duration::from_millis(50);

    /// Parks a producer that returns `outcome` without delay.
    fn parked(cell: &LateValue<u32>, outcome: Result<u32, LateValueError>) {
        cell.park(std::thread::spawn(move || outcome));
    }

    /// Parks a producer that has *not* finished yet, delivering `7`.
    fn parked_slow(cell: &LateValue<u32>) {
        cell.park(std::thread::spawn(|| {
            std::thread::sleep(SLOW);
            Ok(7)
        }));
    }

    #[test]
    fn fresh_cell_is_not_armed_and_serves_nothing() {
        let cell = LateValue::<u32>::new(LABEL);
        assert!(!cell.is_armed());
        let err = cell.with(|_| ()).expect_err("an empty cell must not serve a value");
        assert!(matches!(err, LateValueError::NotArmed), "got {err:?}");
    }

    #[test]
    fn parking_arms_the_cell() {
        let cell = LateValue::new(LABEL);
        parked(&cell, Ok(1));
        assert!(cell.is_armed());
    }

    #[test]
    fn ready_serves_without_a_producer() {
        let cell = LateValue::ready(LABEL, 42u32);
        assert!(cell.is_armed());
        assert_eq!(cell.with(|v| *v).expect("a materialized cell serves its value"), 42);
    }

    #[test]
    fn with_delivers_what_the_producer_returned() {
        let cell = LateValue::new(LABEL);
        parked(&cell, Ok(9));
        assert_eq!(cell.with(|v| *v).expect("producer succeeded"), 9);
    }

    #[test]
    fn with_waits_for_a_producer_still_running() {
        let cell = LateValue::new(LABEL);
        let started = Instant::now();
        parked_slow(&cell);
        assert_eq!(cell.with(|v| *v).expect("producer succeeded"), 7);
        assert!(started.elapsed() >= SLOW, "the reader must have waited for the producer");
    }

    #[test]
    fn the_value_survives_a_second_read() {
        let cell = LateValue::new(LABEL);
        parked(&cell, Ok(5));
        assert_eq!(cell.with(|v| *v).expect("first read"), 5);
        assert_eq!(cell.with(|v| *v).expect("second read"), 5, "the value is kept, not taken");
    }

    #[test]
    fn resolve_caches_the_value_for_later_readers() {
        let cell = LateValue::new(LABEL);
        parked_slow(&cell);
        cell.resolve().expect("producer succeeded");
        assert_eq!(cell.with(|v| *v).expect("read after resolve"), 7);
    }

    #[test]
    fn resolve_is_idempotent() {
        let cell = LateValue::new(LABEL);
        parked(&cell, Ok(3));
        cell.resolve().expect("first resolve");
        cell.resolve().expect("second resolve must not try to join again");
    }

    #[test]
    fn a_failed_producer_is_replayed_to_every_reader() {
        let cell = LateValue::new(LABEL);
        parked(&cell, Err(LateValueError::failed("boom")));

        let first = cell.with(|_| ()).expect_err("the producer failed");
        assert!(matches!(&first, LateValueError::Failed(m) if m.contains("boom")), "got {first:?}");

        let second = cell.with(|_| ()).expect_err("the failure must outlive the first read");
        assert!(
            matches!(&second, LateValueError::Failed(m) if m.contains("boom")),
            "a replayed failure must not degrade to NotArmed: got {second:?}"
        );
    }

    #[test]
    fn a_failed_producer_leaves_the_cell_armed() {
        let cell = LateValue::new(LABEL);
        parked(&cell, Err(LateValueError::failed("boom")));
        let _ = cell.with(|_| ());
        assert!(
            cell.is_armed(),
            "a failure must stay distinguishable from a round that had no producer"
        );
    }

    #[test]
    fn a_panicking_producer_reports_panicked() {
        let cell = LateValue::<u32>::new(LABEL);
        cell.park(std::thread::spawn(|| panic!("producer died")));
        let err = cell.with(|_| ()).expect_err("a panicking producer delivers nothing");
        assert!(matches!(err, LateValueError::Panicked), "got {err:?}");
    }

    #[test]
    fn draining_disarms_the_cell() {
        let cell = LateValue::new(LABEL);
        parked(&cell, Ok(1));
        cell.drain();
        assert!(!cell.is_armed(), "the next round must not inherit this producer");
        assert!(matches!(cell.with(|_| ()).unwrap_err(), LateValueError::NotArmed));
    }

    #[test]
    fn draining_releases_a_value_already_read() {
        let cell = LateValue::ready(LABEL, 1u32);
        assert_eq!(cell.with(|v| *v).expect("read"), 1);
        cell.drain();
        assert!(!cell.is_armed(), "drain releases the value, not just a pending producer");
    }

    #[test]
    fn parking_over_a_parked_producer_joins_the_old_one() {
        let cell = LateValue::new(LABEL);
        let finished = Arc::new(AtomicBool::new(false));

        let flag = Arc::clone(&finished);
        cell.park(std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            flag.store(true, Ordering::SeqCst);
            Ok(1)
        }));
        parked(&cell, Ok(2));

        assert!(
            finished.load(Ordering::SeqCst),
            "the replaced producer must be joined, not leaked"
        );
        assert_eq!(cell.with(|v| *v).expect("the new producer's value"), 2);
    }

    #[test]
    fn a_reader_panic_does_not_take_the_cell_with_it() {
        let cell = LateValue::ready(LABEL, 3u32);

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = cell.with(|_| panic!("reader died holding the lock"));
        }));
        assert!(panicked.is_err(), "the reader's panic must propagate");

        assert_eq!(cell.with(|v| *v).expect("a poisoned lock must still serve"), 3);
    }
}
