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

/// Bytes the collectors may hold at once, or `None` to collect everything in one pass.
///
/// `host_share` measures this process's share of the host memory available, after the margin
/// (`host_memory::host_memory_share`); the budget leaves `reserve` of it for work of its
/// own that runs alongside the collect. An explicit size is taken as is, without measuring.
pub fn collect_budget(reserve: u64, host_share: impl FnOnce() -> Option<u64>) -> Option<u64> {
    match std::env::var(BUDGET_ENV) {
        Ok(value) if value.eq_ignore_ascii_case("off") => None,
        Ok(value) if !value.is_empty() && !value.eq_ignore_ascii_case("auto") => {
            match parse_size(&value) {
                Some(bytes) => Some(bytes),
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
/// `reserve`. `None` when the platform reports no memory figures, so nothing is held back on a
/// guess.
fn available_budget(reserve: u64, host_share: impl FnOnce() -> Option<u64>) -> Option<u64> {
    let Some(share) = host_share() else {
        tracing::warn!("no memory figures from the system; collecting without a budget");
        return None;
    };
    Some(share.saturating_sub(reserve))
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

/// Bytes in GiB, for the log.
pub fn gib(bytes: u64) -> f64 {
    bytes as f64 / (1u64 << 30) as f64
}

/// An instance to collect, with the bytes its collectors reserve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    pub global_id: usize,
    pub bytes: u64,
}

/// The instances of one call that exceed the budget, in the order to collect them.
#[derive(Debug)]
pub struct WavePlan {
    pub budget: u64,
    /// Bytes all of them reserve together.
    pub total: u64,
    pub pending: Vec<Pending>,
}

/// How many instances from the front of `pending` fit in `room` together; possibly none.
pub fn fitting_prefix(pending: &[Pending], room: u64) -> usize {
    let mut used = 0u64;
    pending
        .iter()
        .take_while(|p| {
            used = used.saturating_add(p.bytes);
            used <= room
        })
        .count()
}

/// How many instances to take from the front of `pending` so their bytes fit in `room`.
///
/// At least one, so an instance larger than the room is collected rather than never
/// ([`holding_any`] sizes the waves' budget so that this takes no more than the budget).
pub fn next_wave(pending: &[Pending], room: u64) -> usize {
    fitting_prefix(pending, room).max(1).min(pending.len())
}

/// `budget`, raised so that every instance of `pending` fits in it alone.
///
/// A wave is never empty: an instance that exceeds the budget is collected anyway, alone. The waves
/// may as well hold that much at all times. Without this floor, a budget of zero sends every
/// instance alone, and the collect replays the chunks once per instance.
pub fn holding_any(budget: u64, pending: &[Pending]) -> u64 {
    budget.max(pending.iter().map(|p| p.bytes).max().unwrap_or(0))
}

/// `bytes` against the measured `budget`, for the log of a collect in waves, which ran within
/// `used`: when that is the budget raised by [`holding_any`], the log says to what.
pub fn describe_waves(bytes: u64, budget: u64, used: u64) -> String {
    let mut out = format!("{:.1} GiB of a {:.1} GiB budget", gib(bytes), gib(budget));
    if used != budget {
        out += &format!(", raised to {:.1} GiB to hold the largest instance", gib(used));
    }
    out
}

/// Whether the next wave may start: once nothing collected is still held, or once what is held is
/// at most half the budget and the next instance fits beside it.
///
/// The half keeps the waves large. A wave started at the first freed byte would hold one instance
/// and replay its chunks on its own, which is what collecting one instance at a time costs.
pub fn may_start(held: u64, next: u64, budget: u64) -> bool {
    held == 0 || (held <= budget / 2 && held.saturating_add(next) <= budget)
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
    pub fn bytes(&self) -> u64 {
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

/// Waits until the next wave, whose first instance reserves `next` bytes, may start.
///
/// Only the instances in `held` can make room, and proofman computes them on its own threads. The
/// wait ends with an error when the proof is cancelled, since they would then never be computed.
/// Without progress it only reports, every [`STALL_REPORT`], which instances it waits on.
pub fn wait_for_room(
    held: &mut Held,
    next: u64,
    budget: u64,
    state: impl Fn(usize) -> WitnessState,
    cancelled: impl Fn() -> bool,
) -> ExecutorResult<()> {
    let started = Instant::now();
    let mut last_progress = started;
    held.refresh(&state);
    while !may_start(held.bytes(), next, budget) {
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

    fn pending(sizes: &[u64]) -> Vec<Pending> {
        sizes.iter().enumerate().map(|(i, &bytes)| Pending { global_id: i, bytes }).collect()
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
    fn the_budget_is_the_host_share_less_the_reserve() {
        const GIB: u64 = 1 << 30;
        assert_eq!(available_budget(4 * GIB, || Some(10 * GIB)), Some(6 * GIB));
        assert_eq!(
            available_budget(12 * GIB, || Some(10 * GIB)),
            Some(0),
            "a reserve above the share"
        );
        assert_eq!(available_budget(0, || None), None, "no memory figures, no budget");
    }

    #[test]
    fn a_wave_takes_the_prefix_that_fits() {
        let p = pending(&[3, 4, 5, 1]);
        assert_eq!(next_wave(&p, 7), 2);
        assert_eq!(next_wave(&p, 12), 3);
        assert_eq!(next_wave(&p, 100), 4);
    }

    #[test]
    fn a_fitting_prefix_may_be_empty() {
        let p = pending(&[10, 1]);
        assert_eq!(fitting_prefix(&p, 5), 0, "the first instance alone exceeds the room");
        assert_eq!(fitting_prefix(&p, 10), 1);
        assert_eq!(fitting_prefix(&p, 11), 2);
        assert_eq!(fitting_prefix(&[], 11), 0);
    }

    #[test]
    fn a_wave_never_comes_out_empty() {
        let p = pending(&[10, 1]);
        assert_eq!(next_wave(&p, 5), 1, "an instance larger than the room goes alone");
        assert_eq!(next_wave(&p, 0), 1);
    }

    #[test]
    fn the_waves_budget_holds_any_single_instance() {
        assert_eq!(holding_any(10, &pending(&[3, 7, 5])), 10, "nothing to raise");
        assert_eq!(holding_any(4, &pending(&[3, 7, 5])), 7, "raised to the largest instance");
        assert_eq!(holding_any(4, &[]), 4);
    }

    #[test]
    fn a_budget_of_zero_does_not_send_every_instance_alone() {
        let p = pending(&[1, 1, 2, 4]);
        assert_eq!(next_wave(&p, 0), 1, "one instance per wave");
        let raised = holding_any(0, &p);
        assert_eq!(raised, 4);
        assert_eq!(next_wave(&p, raised), 3, "1 + 1 + 2 within the largest instance's 4");
        assert_eq!(next_wave(&p[3..], raised), 1);
    }

    #[test]
    fn the_waves_log_names_a_raised_budget() {
        const GIB: u64 = 1 << 30;
        assert_eq!(describe_waves(7 * GIB, 2 * GIB, 2 * GIB), "7.0 GiB of a 2.0 GiB budget");
        assert_eq!(
            describe_waves(7 * GIB, 0, GIB),
            "7.0 GiB of a 0.0 GiB budget, raised to 1.0 GiB to hold the largest instance"
        );
    }

    #[test]
    fn the_next_wave_waits_for_half_the_budget() {
        assert!(may_start(0, 1_000, 100), "nothing held: always");
        assert!(!may_start(60, 10, 100), "more than half held");
        assert!(may_start(50, 50, 100));
        assert!(!may_start(50, 51, 100), "the next instance does not fit beside");
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
        assert_eq!(held.bytes(), 3);
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
        wait_for_room(&mut held, 30, 100, state, || false).unwrap();
        assert_eq!(held.bytes(), 40);
    }

    #[test]
    fn a_cancelled_proof_ends_the_wait() {
        let mut held = Held::default();
        held.add(&pending(&[100]));
        let started = Instant::now();
        let result = wait_for_room(&mut held, 1, 100, |_| WitnessState::Queued, || true);
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
            10,
            100,
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
