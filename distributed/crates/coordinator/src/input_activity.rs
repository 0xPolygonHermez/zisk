//! How long a job has gone without the input or hints its client still has to send.

use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// When a job's client last sent it input or hints, and which of them it still has to send.
///
/// A client's data streams are the one part of a job that the client paces: a job whose sender
/// stalls holds its workers, which wait for the data, until something ends the job. The job
/// monitor fails a job that still expects data and has had none for
/// `input_idle_timeout_seconds`. Updated on every chunk, without locks.
#[derive(Debug)]
pub struct InputActivity {
    /// The instant `last` counts from.
    epoch: Instant,
    /// Milliseconds from `epoch` to the last chunk, or to when the job started expecting data.
    last: AtomicU64,
    /// The job still expects input from its client.
    input: AtomicBool,
    /// The job still expects hints from its client.
    hints: AtomicBool,
}

impl Default for InputActivity {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            last: AtomicU64::new(0),
            input: AtomicBool::new(false),
            hints: AtomicBool::new(false),
        }
    }
}

impl InputActivity {
    /// A chunk of input or hints arrived.
    pub fn progress(&self) {
        self.last.fetch_max(self.epoch.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    /// The job expects input from its client from now on, or no more. Expecting it restarts the
    /// idle clock.
    pub fn expect_input(&self, expected: bool) {
        Self::expect(&self.input, expected, || self.progress());
    }

    /// The job expects hints from its client from now on, or no more. Expecting them restarts
    /// the idle clock.
    pub fn expect_hints(&self, expected: bool) {
        Self::expect(&self.hints, expected, || self.progress());
    }

    fn expect(flag: &AtomicBool, expected: bool, progress: impl Fn()) {
        if expected {
            // Before the flag, which the monitor reads first: it never pairs a fresh
            // expectation with the time of an older chunk.
            progress();
        }
        flag.store(expected, Ordering::Release);
    }

    /// What the job still expects from its client and how long it has had none of it, at `now`;
    /// `None` when it expects nothing.
    pub(crate) fn idle_at(&self, now: Instant) -> Option<(&'static str, Duration)> {
        let what = match (self.input.load(Ordering::Acquire), self.hints.load(Ordering::Acquire)) {
            (false, false) => return None,
            (true, false) => "input",
            (false, true) => "hints",
            (true, true) => "input and hints",
        };
        let last = self.epoch + Duration::from_millis(self.last.load(Ordering::Relaxed));
        Some((what, now.saturating_duration_since(last)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_that_expects_nothing_is_never_idle() {
        let activity = InputActivity::default();
        assert!(activity.idle_at(Instant::now() + Duration::from_secs(3600)).is_none());
        activity.progress();
        assert!(activity.idle_at(Instant::now() + Duration::from_secs(3600)).is_none());
    }

    #[test]
    fn idle_time_counts_from_the_last_chunk() {
        let activity = InputActivity::default();
        activity.expect_input(true);
        let later = Instant::now() + Duration::from_secs(90);
        let (what, idle) = activity.idle_at(later).unwrap();
        assert_eq!(what, "input");
        assert!(idle >= Duration::from_secs(89), "{idle:?}");

        std::thread::sleep(Duration::from_millis(20));
        activity.progress();
        let (_, after_progress) = activity.idle_at(later).unwrap();
        assert!(after_progress < idle, "{after_progress:?} vs {idle:?}");
    }

    #[test]
    fn the_end_of_a_stream_stops_the_expectation() {
        let activity = InputActivity::default();
        activity.expect_input(true);
        activity.expect_hints(true);
        assert_eq!(activity.idle_at(Instant::now()).unwrap().0, "input and hints");
        activity.expect_input(false);
        assert_eq!(activity.idle_at(Instant::now()).unwrap().0, "hints");
        activity.expect_hints(false);
        assert!(activity.idle_at(Instant::now() + Duration::from_secs(3600)).is_none());
    }
}
