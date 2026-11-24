//! Bounds the bytes the witness phase's input collectors hold at once.
//!
//! [`WitnessPhase::pre_calculate`](crate::WitnessPhase::pre_calculate) collects every pending
//! secondary instance in one replay of the chunks. When the capacities their collectors reserve
//! exceed the memory available, it collects them in waves instead: a wave starts only once
//! proofman has computed enough of the instances the earlier waves announced, which frees their
//! collectors. This module holds the pieces that decide that, free of `ProofCtx` so they can be
//! tested on their own.

use std::time::{Duration, Instant};

use proofman_common::WitnessState;
use zisk_common::CollectorBytes;

use crate::error::{ExecutorError, ExecutorResult};

/// Environment variable that sets the budget: `off` collects everything in one pass, as before the
/// budget existed; a size (`24G`, `512M`, plain bytes) fixes it; unset or `auto` derives it from the
/// memory available.
pub const BUDGET_ENV: &str = "ZISK_COLLECT_BUDGET";

/// Memory the automatic budget also leaves, per process, while the first registration round
/// collects inside the execution: the memory-operations runner and its plans are still being built
/// alongside it.
pub const FIRST_ROUND_RESERVE: u64 = 4 << 30;

/// How often a wave waiting for room checks the instances it waits on.
const POLL: Duration = Duration::from_millis(10);

/// How long a wait may go without any instance being computed before it says so in the log.
const STALL_REPORT: Duration = Duration::from_secs(30);

/// What the collectors may hold at once: `heap` bytes of the host's memory, and `total` bytes with
/// a lent range's free bytes counted in.
///
/// Collectors fit when their [`CollectorBytes`] keep both: `heap ≤ budget.heap` and
/// `total ≤ budget.total`. That is exactly "what lands in the heap fits in it": heap buffers always
/// do, and lendable ones too once the lent range is full, so the heap holds
/// `heap + max(0, lendable − lent)`, which is at most `budget.heap` if and only if both hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub heap: u64,
    pub total: u64,
}

impl Budget {
    /// The same bytes either way: no lent range to count.
    pub fn heap_only(bytes: u64) -> Self {
        Self { heap: bytes, total: bytes }
    }

    /// Whether collectors reserving `bytes` fit.
    pub fn fits(self, bytes: CollectorBytes) -> bool {
        bytes.heap <= self.heap && bytes.total() <= self.total
    }

    /// This budget, raised so that every instance of `pending` fits in it alone.
    ///
    /// A wave is never empty: an instance that exceeds the budget is collected anyway, alone, and
    /// takes from the heap what its heap buffers need plus what the lent range cannot hold of the
    /// others. The waves may as well hold that much at all times. Without this floor, a heap budget
    /// of zero sends every instance with a single heap byte alone, however much of the lent range is
    /// free, and the collect replays the chunks once per instance.
    pub fn holding_any(self, pending: &[Pending]) -> Self {
        let lent = self.total.saturating_sub(self.heap);
        let floor = pending
            .iter()
            .map(|p| p.bytes.heap + p.bytes.lendable.saturating_sub(lent))
            .max()
            .unwrap_or(0);
        let heap = self.heap.max(floor);
        Self { heap, total: heap + lent }
    }

    /// The room left beside collectors that hold `held`, when `lent_free` bytes of the lent range
    /// are free now.
    ///
    /// Where the held buffers are is measured, not taken from their kind: a lendable buffer that
    /// found the range full went to the heap, and stays there after the ones in the range are
    /// freed. Whatever the range does not hold of `held` is in the heap, and its heap buffers at
    /// least.
    pub fn room(self, held: CollectorBytes, lent_free: u64) -> Self {
        let lent = self.total.saturating_sub(self.heap);
        let lent_free = lent_free.min(lent);
        let in_heap = held.total().saturating_sub(lent - lent_free).max(held.heap);
        let heap = self.heap.saturating_sub(in_heap);
        Self { heap, total: heap + lent_free }
    }
}

/// `bytes` against `budget`, for the log. When a lent range counts, it says what the budget is made
/// of, and how much of `bytes` only the heap can hold.
pub fn describe(bytes: CollectorBytes, budget: Budget) -> String {
    let mut out = format!("{:.1} GiB of a {:.1} GiB budget", gib(bytes.total()), gib(budget.total));
    if budget.total > budget.heap {
        out += &format!(
            " ({:.1} GiB of heap + {:.1} GiB lent; {:.1} GiB in buffers that only go to the heap)",
            gib(budget.heap),
            gib(budget.total - budget.heap),
            gib(bytes.heap)
        );
    }
    out
}

/// [`describe`] for a collect in waves, which ran within `used`: when that is the measured `budget`
/// raised by [`Budget::holding_any`], the log says to what.
pub fn describe_waves(bytes: CollectorBytes, budget: Budget, used: Budget) -> String {
    let mut out = describe(bytes, budget);
    if used != budget {
        out += &format!(", raised to {:.1} GiB", gib(used.total));
        if used.total > used.heap {
            out += &format!(" ({:.1} GiB of heap)", gib(used.heap));
        }
        out += " to hold the largest instance";
    }
    out
}

/// What the collectors may hold at once, or `None` to collect everything in one pass.
///
/// `host_share` measures this process's share of the host memory available, after the margin
/// (`host_memory::host_memory_share`); the budget leaves `reserve` of it for work of its
/// own that runs alongside the collect. An explicit size is taken as is, without measuring, and
/// without counting a lent range.
pub fn collect_budget(reserve: u64, host_share: impl FnOnce() -> Option<u64>) -> Option<Budget> {
    match std::env::var(BUDGET_ENV) {
        Ok(value) if value.eq_ignore_ascii_case("off") => None,
        Ok(value) if !value.is_empty() && !value.eq_ignore_ascii_case("auto") => {
            match parse_size(&value) {
                Some(bytes) => Some(Budget::heap_only(bytes)),
                None => {
                    tracing::warn!("{BUDGET_ENV}={value} is not off, auto or a size; using auto");
                    available_budget(reserve, host_share)
                }
            }
        }
        _ => available_budget(reserve, host_share),
    }
}

/// What the host can give this process's collectors: its share of the memory available, less
/// `reserve`, and that plus the lent range still free. `None` when the platform reports no memory
/// figures, so nothing is held back on a guess.
fn available_budget(reserve: u64, host_share: impl FnOnce() -> Option<u64>) -> Option<Budget> {
    let Some(share) = host_share() else {
        tracing::warn!("no memory figures from the system; collecting without a budget");
        return None;
    };
    let heap = share.saturating_sub(reserve);
    // A lent range is this process's alone and already resident: the OS counts it as used.
    Some(Budget { heap, total: heap + zisk_common::lent_memory::free_bytes() })
}

/// Parses `24G`, `512M`, `64K` (powers of 1024) or plain bytes.
pub(super) fn parse_size(value: &str) -> Option<u64> {
    let value = value.trim();
    let (digits, shift) = match value.char_indices().last()? {
        (i, 'k' | 'K') => (&value[..i], 10),
        (i, 'm' | 'M') => (&value[..i], 20),
        (i, 'g' | 'G') => (&value[..i], 30),
        (i, 't' | 'T') => (&value[..i], 40),
        _ => (value, 0),
    };
    digits.trim().parse::<u64>().ok()?.checked_mul(1 << shift)
}

/// Environment variable: `off` (or `0`) keeps the memory-operations output from being lent to the
/// collectors once a run has consumed it ([`zisk_common::lent_memory`]). Lent by default, in GPU
/// builds, which are the ones that map the output for writing; the switch is there to rule the
/// lending out, as it rests on jemalloc's experimental arena API.
pub const LEND_ENV: &str = "ZISK_LEND_MO";

/// Whether to lend the memory-operations output to the collectors ([`LEND_ENV`]).
pub fn lending_enabled() -> bool {
    !matches!(std::env::var(LEND_ENV).as_deref(), Ok("off") | Ok("0"))
}

/// Takes back the memory-operations output lent to the collectors, before the next run writes it.
///
/// By then every collector is gone, so a buffer still alive in it is a bug, and the next run
/// would overwrite it. Nothing can make that run safe, so the process stops here; its supervisor
/// starts it again.
pub fn take_back_lent_memory() {
    if let Err(e) = zisk_common::lent_memory::take_back() {
        tracing::error!(
            "{e} when the memory-operations output must be taken back; stopping the process, as \
             the next run would overwrite them"
        );
        std::process::abort();
    }
}

/// Bytes in GiB, for the log.
pub fn gib(bytes: u64) -> f64 {
    bytes as f64 / (1u64 << 30) as f64
}

/// An instance to collect, with the bytes its collectors reserve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    pub global_id: usize,
    pub bytes: CollectorBytes,
}

/// The instances of one call that exceed the budget, in the order to collect them.
#[derive(Debug)]
pub struct WavePlan {
    pub budget: Budget,
    /// Bytes all of them reserve together.
    pub total: CollectorBytes,
    pub pending: Vec<Pending>,
}

/// How many instances from the front of `pending` fit in `room` together; possibly none.
pub fn fitting_prefix(pending: &[Pending], room: Budget) -> usize {
    let mut used = CollectorBytes::default();
    pending
        .iter()
        .take_while(|p| {
            used = used + p.bytes;
            room.fits(used)
        })
        .count()
}

/// How many instances to take from the front of `pending` so their bytes fit in `room`.
///
/// At least one, so an instance larger than the room is collected rather than never
/// ([`Budget::holding_any`] sizes the waves' budget so that this takes no more than the budget).
pub fn next_wave(pending: &[Pending], room: Budget) -> usize {
    fitting_prefix(pending, room).max(1).min(pending.len())
}

/// Whether the next wave may start: once nothing collected is still held, or once what is held is
/// at most half the budget and the next instance fits beside it, with `lent_free` bytes of the lent
/// range free now ([`Budget::room`]).
///
/// The half keeps the waves large. A wave started at the first freed byte would hold one instance
/// and replay its chunks on its own, which is what collecting one instance at a time costs.
pub fn may_start(
    held: CollectorBytes,
    next: CollectorBytes,
    budget: Budget,
    lent_free: u64,
) -> bool {
    held.total() == 0
        || (held.total() <= budget.total / 2 && budget.room(held, lent_free).fits(next))
}

/// The instances a call has collected whose collectors are still alive: announced, and not yet
/// computed by proofman.
#[derive(Debug, Default)]
pub struct Held {
    instances: Vec<Pending>,
}

impl Held {
    /// Adds a wave just collected; every instance in it has been announced.
    pub fn add(&mut self, wave: &[Pending]) {
        self.instances.extend_from_slice(wave);
    }

    /// Bytes still held.
    pub fn bytes(&self) -> CollectorBytes {
        self.instances.iter().map(|p| p.bytes).sum()
    }

    /// Instances still held.
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    /// Drops the instances whose collectors are gone: computed (`Done`), or reclaimed after that
    /// (`Evicted`). An instance stays held while it waits for a witness thread (`Queued`) or has one
    /// (`Running`); `Absent` would mean proofman forgot the announce, so it no longer holds either.
    pub fn refresh(&mut self, state: impl Fn(usize) -> WitnessState) {
        self.instances
            .retain(|p| matches!(state(p.global_id), WitnessState::Queued | WitnessState::Running));
    }

    fn report(&self, state: impl Fn(usize) -> WitnessState) -> String {
        const SHOWN: usize = 16;
        let mut out: Vec<String> = self
            .instances
            .iter()
            .take(SHOWN)
            .map(|p| format!("{}:{:?}", p.global_id, state(p.global_id)))
            .collect();
        if self.instances.len() > SHOWN {
            out.push(format!("+{} more", self.instances.len() - SHOWN));
        }
        out.join(", ")
    }
}

/// Waits until the next wave, whose first instance reserves `next`, may start.
///
/// Only the instances in `held` can make room, and proofman computes them on its own threads. The
/// wait ends with an error when the proof is cancelled, since they would then never be computed.
/// Without progress it only reports, every [`STALL_REPORT`], which instances it waits on.
/// `lent_free` measures the lent range's free bytes, read on every check.
pub fn wait_for_room(
    held: &mut Held,
    next: CollectorBytes,
    budget: Budget,
    lent_free: impl Fn() -> u64,
    state: impl Fn(usize) -> WitnessState,
    cancelled: impl Fn() -> bool,
) -> ExecutorResult<()> {
    let started = Instant::now();
    let mut last_progress = started;
    held.refresh(&state);
    while !may_start(held.bytes(), next, budget, lent_free()) {
        if cancelled() {
            return Err(ExecutorError::CollectCancelled);
        }
        std::thread::sleep(POLL);
        let before = held.len();
        held.refresh(&state);
        if held.len() < before {
            last_progress = Instant::now();
        } else if last_progress.elapsed() >= STALL_REPORT {
            tracing::warn!(
                "collect: {:.0} s waiting for room, none of {} instances computed in {:.0} s: {}",
                started.elapsed().as_secs_f64(),
                held.len(),
                last_progress.elapsed().as_secs_f64(),
                held.report(&state)
            );
            last_progress = Instant::now();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn heap(bytes: u64) -> CollectorBytes {
        CollectorBytes::heap(bytes)
    }

    fn lendable(bytes: u64) -> CollectorBytes {
        CollectorBytes { lendable: bytes, heap: 0 }
    }

    fn pending_of(bytes: &[CollectorBytes]) -> Vec<Pending> {
        bytes.iter().enumerate().map(|(i, &bytes)| Pending { global_id: i, bytes }).collect()
    }

    /// Heap buffers only: the budget is then a single figure.
    fn pending(sizes: &[u64]) -> Vec<Pending> {
        pending_of(&sizes.iter().map(|&s| heap(s)).collect::<Vec<_>>())
    }

    fn budget(bytes: u64) -> Budget {
        Budget::heap_only(bytes)
    }

    #[test]
    fn sizes_parse_in_powers_of_1024() {
        assert_eq!(parse_size("24G"), Some(24 << 30));
        assert_eq!(parse_size("512m"), Some(512 << 20));
        assert_eq!(parse_size("64K"), Some(64 << 10));
        assert_eq!(parse_size("1T"), Some(1 << 40));
        assert_eq!(parse_size(" 1000 "), Some(1000));
        assert_eq!(parse_size("G"), None);
        assert_eq!(parse_size("lots"), None);
        assert_eq!(parse_size("99999999999T"), None, "an overflow is not a size");
    }

    #[test]
    fn the_budget_is_the_host_share_less_the_reserve_plus_the_lent_range() {
        const GIB: u64 = 1 << 30;
        let lent = zisk_common::lent_memory::free_bytes();
        assert_eq!(
            available_budget(4 * GIB, || Some(10 * GIB)),
            Some(Budget { heap: 6 * GIB, total: 6 * GIB + lent })
        );
        assert_eq!(
            available_budget(12 * GIB, || Some(10 * GIB)),
            Some(Budget { heap: 0, total: lent }),
            "a reserve above the share"
        );
        assert_eq!(available_budget(0, || None), None, "no memory figures, no budget");
    }

    #[test]
    fn the_two_limits_are_what_lands_in_the_heap() {
        // Lendable bytes beyond the lent range fall back to the heap: the heap then holds
        // `heap + max(0, lendable - lent)`. Both limits together say exactly that it fits.
        for heap_room in 0..6u64 {
            for lent in 0..6u64 {
                let room = Budget { heap: heap_room, total: heap_room + lent };
                for h in 0..8u64 {
                    for l in 0..8u64 {
                        let lands_in_heap = h + l.saturating_sub(lent);
                        assert_eq!(
                            room.fits(CollectorBytes { lendable: l, heap: h }),
                            lands_in_heap <= heap_room,
                            "heap {h} + lendable {l} in {room:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn heap_buffers_never_use_the_lent_range() {
        let room = Budget { heap: 4, total: 10 };
        assert_eq!(fitting_prefix(&pending(&[3, 3]), room), 1, "6 heap bytes in a 4-byte heap");
        assert_eq!(fitting_prefix(&pending_of(&[lendable(5), lendable(4)]), room), 2);
        assert_eq!(
            fitting_prefix(&pending_of(&[lendable(5), lendable(6)]), room),
            1,
            "11 bytes: 6 in the lent range, 5 spilling into a 4-byte heap"
        );
        assert_eq!(fitting_prefix(&pending_of(&[lendable(6), heap(4)]), room), 2);
    }

    #[test]
    fn the_waves_budget_holds_any_single_instance() {
        // Nothing to raise: every instance already fits.
        let room = Budget { heap: 4, total: 10 };
        assert_eq!(room.holding_any(&pending_of(&[lendable(6) + heap(4), heap(1)])), room);
        assert_eq!(room.holding_any(&[]), room);

        // A heap of zero: raised to the instance that lands most in the heap, on its heap bytes
        // (3) or on what the lent range (6) cannot hold of its lendable ones (9 - 6 + 1).
        let none = Budget { heap: 0, total: 6 };
        let p = pending_of(&[lendable(5) + heap(3), lendable(9) + heap(1), lendable(2)]);
        let raised = none.holding_any(&p);
        assert_eq!(raised, Budget { heap: 4, total: 10 });
        for one in &p {
            assert!(raised.fits(one.bytes), "{one:?} fits alone");
        }
        // Without a lent range it is the largest instance.
        assert_eq!(budget(0).holding_any(&pending(&[3, 7, 5])), budget(7));
    }

    #[test]
    fn a_heap_of_zero_does_not_send_every_instance_alone() {
        // 12 bytes lent and free, no heap: small heap parts ride along up to the largest one.
        let none = Budget { heap: 0, total: 12 };
        let p = pending_of(&[
            lendable(3) + heap(1),
            lendable(3) + heap(1),
            lendable(3) + heap(2),
            lendable(3) + heap(4),
        ]);
        assert_eq!(next_wave(&p, none), 1, "as measured: one instance per wave");
        let raised = none.holding_any(&p);
        assert_eq!(raised, Budget { heap: 4, total: 16 });
        assert_eq!(next_wave(&p, raised), 3, "heap 1 + 1 + 2 within the largest instance's 4");
        assert_eq!(next_wave(&p[3..], raised), 1);
    }

    #[test]
    fn the_room_left_counts_what_is_held_in_both() {
        // 6 lendable bytes fill the range of 6, 1 heap byte is in the heap.
        let room = Budget { heap: 4, total: 10 }.room(lendable(6) + heap(1), 0);
        assert_eq!(room, Budget { heap: 3, total: 3 }, "the lent range is full");
        assert!(room.fits(heap(3)));
        assert!(!room.fits(lendable(4)), "a lendable buffer now spills into the heap");
        // No lent range: everything held is in the heap.
        assert_eq!(budget(100).room(heap(30) + lendable(10), 0), budget(60));
    }

    #[test]
    fn held_buffers_count_where_they_landed() {
        // A wave of 20 lendable bytes filled the range (10) and spilled 10 into the heap. The ones
        // in the range were computed and freed: the 10 still held are in the heap, the range empty.
        let all = Budget { heap: 10, total: 20 };
        let room = all.room(lendable(10), 10);
        assert_eq!(room, Budget { heap: 0, total: 10 });
        assert!(!room.fits(heap(10)), "the heap is full, whatever kind the held bytes are");
        assert!(room.fits(lendable(10)), "the range is free");
        // The same 10 held in the range instead: the heap is free.
        assert_eq!(all.room(lendable(10), 0), Budget { heap: 10, total: 10 });

        // The range held by others: this collect's heap buffers are in the heap all the same.
        assert_eq!(
            Budget { heap: 5, total: 15 }.room(heap(3) + lendable(2), 0),
            Budget { heap: 2, total: 2 }
        );
        // More free than the budget counted of the range: no more than it counted is used.
        assert_eq!(all.room(lendable(4), 50), Budget { heap: 6, total: 16 });
    }

    #[test]
    fn a_wave_takes_the_prefix_that_fits() {
        let p = pending(&[3, 4, 5, 1]);
        assert_eq!(next_wave(&p, budget(7)), 2);
        assert_eq!(next_wave(&p, budget(12)), 3);
        assert_eq!(next_wave(&p, budget(100)), 4);
    }

    #[test]
    fn a_fitting_prefix_may_be_empty() {
        let p = pending(&[10, 1]);
        assert_eq!(fitting_prefix(&p, budget(5)), 0, "the first instance alone exceeds the room");
        assert_eq!(fitting_prefix(&p, budget(10)), 1);
        assert_eq!(fitting_prefix(&p, budget(11)), 2);
        assert_eq!(fitting_prefix(&[], budget(11)), 0);
    }

    #[test]
    fn a_wave_never_comes_out_empty() {
        let p = pending(&[10, 1]);
        assert_eq!(next_wave(&p, budget(5)), 1, "an instance larger than the room goes alone");
        assert_eq!(next_wave(&p, budget(0)), 1);
    }

    #[test]
    fn the_next_wave_waits_for_half_the_budget() {
        assert!(may_start(heap(0), heap(1_000), budget(100), 0), "nothing held: always");
        assert!(!may_start(heap(60), heap(10), budget(100), 0), "more than half held");
        assert!(may_start(heap(50), heap(50), budget(100), 0));
        assert!(
            !may_start(heap(50), heap(51), budget(100), 0),
            "the next instance does not fit beside"
        );
        // A range of 60 that holds the 50 lendable bytes: 10 of it free.
        let with_lent = Budget { heap: 40, total: 100 };
        assert!(may_start(lendable(50), heap(40), with_lent, 10));
        assert!(
            !may_start(lendable(50), heap(41), with_lent, 10),
            "heap bytes never use the lent range"
        );
        assert!(
            !may_start(lendable(50), heap(40), with_lent, 60),
            "the range is empty: the 50 held bytes are in the heap"
        );
    }

    #[test]
    fn the_log_shows_what_the_budget_is_made_of_only_with_a_lent_range() {
        const GIB: u64 = 1 << 30;
        let bytes = lendable(6 * GIB) + heap(GIB);
        assert_eq!(describe(bytes, budget(8 * GIB)), "7.0 GiB of a 8.0 GiB budget");
        assert_eq!(
            describe(bytes, Budget { heap: 2 * GIB, total: 12 * GIB }),
            "7.0 GiB of a 12.0 GiB budget (2.0 GiB of heap + 10.0 GiB lent; 1.0 GiB in buffers \
             that only go to the heap)"
        );
    }

    #[test]
    fn the_waves_log_names_a_raised_budget() {
        const GIB: u64 = 1 << 30;
        let bytes = lendable(6 * GIB) + heap(GIB);
        let measured = Budget { heap: 0, total: 12 * GIB };
        assert_eq!(describe_waves(bytes, measured, measured), describe(bytes, measured));
        assert_eq!(
            describe_waves(bytes, measured, Budget { heap: GIB / 2, total: 12 * GIB + GIB / 2 }),
            "7.0 GiB of a 12.0 GiB budget (0.0 GiB of heap + 12.0 GiB lent; 1.0 GiB in buffers \
             that only go to the heap), raised to 12.5 GiB (0.5 GiB of heap) to hold the largest \
             instance"
        );
        // Without a lent range the budget is one figure, raised or not.
        assert_eq!(
            describe_waves(bytes, budget(0), budget(7 * GIB)),
            "7.0 GiB of a 0.0 GiB budget, raised to 7.0 GiB to hold the largest instance"
        );
    }

    #[test]
    fn only_queued_and_running_instances_stay_held() {
        let mut held = Held::default();
        held.add(&pending(&[1, 2, 4, 8, 16]));
        held.refresh(|g| match g {
            0 => WitnessState::Queued,
            1 => WitnessState::Running,
            2 => WitnessState::Done,
            3 => WitnessState::Evicted,
            _ => WitnessState::Absent,
        });
        assert_eq!(held.bytes(), heap(3));
        assert_eq!(held.len(), 2);
    }

    #[test]
    fn the_wait_ends_when_enough_is_computed() {
        let mut held = Held::default();
        held.add(&pending(&[40, 40]));
        // Instance 0 is computed after a few polls.
        let polls = Cell::new(0);
        let state = |g: usize| {
            polls.set(polls.get() + 1);
            if g == 0 && polls.get() > 6 {
                WitnessState::Done
            } else {
                WitnessState::Running
            }
        };
        wait_for_room(&mut held, heap(30), budget(100), || 0, state, || false).unwrap();
        assert_eq!(held.bytes(), heap(40));
    }

    #[test]
    fn a_cancelled_proof_ends_the_wait() {
        let mut held = Held::default();
        held.add(&pending(&[100]));
        let started = Instant::now();
        let result =
            wait_for_room(&mut held, heap(1), budget(100), || 0, |_| WitnessState::Queued, || true);
        assert!(matches!(result, Err(ExecutorError::CollectCancelled)));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn the_wait_does_not_start_when_there_is_room() {
        let mut held = Held::default();
        held.add(&pending(&[10]));
        let cancelled = Cell::new(false);
        wait_for_room(
            &mut held,
            heap(10),
            budget(100),
            || 0,
            |_| WitnessState::Running,
            || {
                cancelled.set(true);
                true
            },
        )
        .unwrap();
        assert!(!cancelled.get(), "no wait, no cancellation check");
    }
}
