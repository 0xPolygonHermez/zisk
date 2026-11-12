//! The coordinator's recent history, rendered into `/metrics`.
//!
//! Counters and histograms say how many jobs ended and how long they took; a live view also wants
//! *which* jobs: the ones in flight, the last few, the last failures and why, and each proof's own
//! duration at the moment it ended. The `metrics` facade cannot express that (it has no
//! per-sample timestamps and cannot drop a series), so this module keeps a small record and
//! renders it in the Prometheus text format, after the recorder's output.
//!
//! Everything is bounded: [`RECENT_JOBS`] jobs, [`RECENT_EVENTS`] events, [`RECENT_FAILURES`]
//! failures and [`MAX_PENDING_SAMPLES`] queued samples per timed series. It is updated at job,
//! setup and worker lifecycle events, never on a proving path, and read when `/metrics` is
//! scraped.
//!
//! Timed series carry an explicit timestamp, one sample per scrape and per series. A scrape
//! takes the next queued sample, so they expect a single Prometheus scraping this coordinator:
//! two would split the samples between them.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use zisk_cluster_common::prometheus_text::{self, sample};
use zisk_cluster_common::JobId;

/// Jobs kept for `coordinator_recent_job`.
pub(crate) const RECENT_JOBS: usize = 20;
/// Events kept for `coordinator_recent_event`.
pub(crate) const RECENT_EVENTS: usize = 30;
/// Failures kept for `coordinator_job_failure_info`.
pub(crate) const RECENT_FAILURES: usize = 100;
/// Samples a timed series holds while nothing scrapes it; the oldest are dropped past this.
pub(crate) const MAX_PENDING_SAMPLES: usize = 600;
/// How long the last job that ended stays in `coordinator_job_now_seconds` while none runs.
const JUST_ENDED: Duration = Duration::from_secs(120);
/// How long a timed sample stays exposed once it becomes its series' current one: long enough to
/// outlast a few failed scrapes, short enough that it is never older than Prometheus' head block
/// (about two hours), past which every scrape would reject it as too old.
const TIMED_EXPOSURE: Duration = Duration::from_secs(600);
/// Longest client-influenced text (a failure reason, an error) carried in a label, in characters.
const MAX_TEXT_CHARS: usize = 160;
/// Characters of a job id shown in labels, as the coordinator's log lines show them.
const SHORT_ID_CHARS: usize = 8;

/// Phase label of a job in flight.
pub(crate) const PHASE_STARTING: &str = "starting";
pub(crate) const PHASE_CONTRIBUTIONS: &str = "contributions";
pub(crate) const PHASE_PROVE: &str = "prove";
pub(crate) const PHASE_AGGREGATION: &str = "aggregation";
pub(crate) const PHASE_EXECUTION: &str = "execution";

/// How a job ended, with what is known about it at that point.
pub(crate) struct JobEnded<'a> {
    pub job_id: &'a JobId,
    pub kind: &'static str,
    /// One of the `OUTCOME_*` constants.
    pub outcome: &'static str,
    /// Seconds from the job's first phase to its end, when known.
    pub duration: Option<f64>,
    /// A successful proof's three phases (contributions, prove, aggregation), in seconds.
    pub phases: Option<[f64; 3]>,
    pub steps: Option<u64>,
    pub instances: Option<u64>,
    /// Why a failed job failed.
    pub reason: Option<&'a str>,
}

/// The coordinator's bounded recent history. See the module documentation.
#[derive(Default)]
pub(crate) struct Journal {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    running: Vec<Running>,
    last_ended: Option<LastEnded>,
    jobs: VecDeque<JobRecord>,
    events: VecDeque<EventRecord>,
    failures: VecDeque<FailureRecord>,
    next_event: u64,
    next_failure: u64,
    /// Each proof's duration per phase ("1", "2", "3") and in total.
    proof_seconds: BTreeMap<&'static str, TimedSeries>,
    /// Each failed or cancelled job's time from its start to its end, by outcome.
    unsuccessful_seconds: BTreeMap<&'static str, TimedSeries>,
    /// Each proof's input size and steps, at the same time as its `proof_seconds`.
    proof_input_bytes: TimedSeries,
    proof_steps: TimedSeries,
    /// Instances per AIR in the last execution summary.
    air_instances: Vec<(String, u64)>,
    /// A worker registered since the last successful proof.
    cold: bool,
}

struct Running {
    job_id: JobId,
    kind: &'static str,
    phase: &'static str,
    workers: usize,
    started: Instant,
    input_bytes: Option<u64>,
    /// Steps the job's execution ran, from its workers' ASM speed and time, before it ends.
    steps_estimate: Option<u64>,
}

struct LastEnded {
    job_id: String,
    kind: &'static str,
    outcome: &'static str,
    workers: usize,
    duration: f64,
    mhz: Option<f64>,
    at: DateTime<Utc>,
    ended: Instant,
}

struct JobRecord {
    job_id: String,
    kind: &'static str,
    outcome: &'static str,
    at: DateTime<Utc>,
    duration: Option<f64>,
    phases: Option<[f64; 3]>,
    steps: Option<u64>,
    instances: Option<u64>,
    mhz: Option<f64>,
    input_bytes: Option<u64>,
}

struct EventRecord {
    seq: u64,
    at: DateTime<Utc>,
    kind: &'static str,
    detail: String,
}

struct FailureRecord {
    seq: u64,
    at: DateTime<Utc>,
    job_id: String,
    reason: String,
    cause: String,
    input_bytes: Option<u64>,
    steps: Option<u64>,
}

/// A series of samples carrying their own timestamps, at most one rendered per scrape.
#[derive(Default)]
struct TimedSeries {
    pending: VecDeque<(f64, i64)>,
    /// The sample being exposed, its timestamp, and since when it is exposed.
    current: Option<(f64, i64, Instant)>,
}

impl TimedSeries {
    fn push(&mut self, value: f64, at: DateTime<Utc>) {
        if self.pending.len() == MAX_PENDING_SAMPLES {
            self.pending.pop_front();
        }
        self.pending.push_back((value, at.timestamp_millis()));
    }

    /// The sample to expose on this scrape: the next queued one, else the current one while it
    /// is fresh.
    fn next(&mut self, now: Instant) -> Option<(f64, i64)> {
        if let Some((value, at)) = self.pending.pop_front() {
            self.current = Some((value, at, now));
        }
        self.current
            .filter(|(_, _, since)| now.duration_since(*since) <= TIMED_EXPOSURE)
            .map(|(value, at, _)| (value, at))
    }
}

impl Journal {
    fn state(&self) -> MutexGuard<'_, State> {
        // A panic while holding the lock leaves plain data behind; keep serving it.
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A job was accepted and dispatched to `workers` workers.
    pub(crate) fn job_started(&self, job_id: &JobId, kind: &'static str, workers: usize) {
        let mut state = self.state();
        state.running.retain(|r| &r.job_id != job_id);
        state.running.push(Running {
            job_id: job_id.clone(),
            kind,
            phase: PHASE_STARTING,
            workers,
            started: Instant::now(),
            input_bytes: None,
            steps_estimate: None,
        });
    }

    /// A job in flight entered `phase` (one of the `PHASE_*` constants).
    pub(crate) fn job_phase(&self, job_id: &JobId, phase: &'static str) {
        if let Some(running) = self.state().running.iter_mut().find(|r| &r.job_id == job_id) {
            running.phase = phase;
        }
    }

    /// A job in flight takes its input inline, `bytes` long.
    pub(crate) fn job_input(&self, job_id: &JobId, bytes: u64) {
        if let Some(running) = self.state().running.iter_mut().find(|r| &r.job_id == job_id) {
            running.input_bytes = Some(bytes);
        }
    }

    /// A worker reported how many steps a job in flight's execution ran.
    pub(crate) fn job_steps_estimate(&self, job_id: &JobId, steps: u64) {
        if let Some(running) = self.state().running.iter_mut().find(|r| &r.job_id == job_id) {
            running.steps_estimate = Some(steps);
        }
    }

    /// The kind a job in flight was started as.
    pub(crate) fn job_kind(&self, job_id: &JobId) -> Option<&'static str> {
        self.state().running.iter().find(|r| &r.job_id == job_id).map(|r| r.kind)
    }

    /// Whether a worker registered since the last successful proof, and start a new count.
    pub(crate) fn take_cold(&self) -> bool {
        std::mem::take(&mut self.state().cold)
    }

    /// A worker registered: the next proof is the first one after it.
    pub(crate) fn mark_cold(&self) {
        self.state().cold = true;
    }

    /// A job reached a terminal state.
    pub(crate) fn job_ended(&self, ended: &JobEnded<'_>) {
        let now = Utc::now();
        let mut state = self.state();
        let running = state
            .running
            .iter()
            .position(|r| &r.job_id == ended.job_id)
            .map(|i| state.running.remove(i));
        let duration =
            ended.duration.or_else(|| running.as_ref().map(|r| r.started.elapsed().as_secs_f64()));
        let mhz = match (ended.steps, duration) {
            (Some(steps), Some(d)) if d > 0.0 => Some(steps as f64 / d / 1e6),
            _ => None,
        };
        let job_id = short_id(ended.job_id);

        if let Some(d) = duration {
            state.last_ended = Some(LastEnded {
                job_id: job_id.clone(),
                kind: ended.kind,
                outcome: ended.outcome,
                workers: running.as_ref().map_or(0, |r| r.workers),
                duration: d,
                mhz,
                at: now,
                ended: Instant::now(),
            });
        }

        if ended.outcome == super::OUTCOME_SUCCESS {
            if let (Some(phases), Some(total)) = (ended.phases, duration) {
                for (phase, value) in [("1", phases[0]), ("2", phases[1]), ("3", phases[2])] {
                    state.proof_seconds.entry(phase).or_default().push(value, now);
                }
                state.proof_seconds.entry("total").or_default().push(total, now);
                if let Some(bytes) = running.as_ref().and_then(|r| r.input_bytes) {
                    state.proof_input_bytes.push(bytes as f64, now);
                }
                if let Some(steps) = ended.steps {
                    state.proof_steps.push(steps as f64, now);
                }
            }
        } else if let Some(d) = duration {
            state.unsuccessful_seconds.entry(ended.outcome).or_default().push(d, now);
        }

        push_bounded(
            &mut state.jobs,
            RECENT_JOBS,
            JobRecord {
                job_id: job_id.clone(),
                kind: ended.kind,
                outcome: ended.outcome,
                at: now,
                duration,
                phases: ended.phases,
                steps: ended.steps,
                instances: ended.instances,
                mhz,
                input_bytes: running.as_ref().and_then(|r| r.input_bytes),
            },
        );

        if ended.outcome == super::OUTCOME_CANCELLED {
            let detail = match (&running, duration) {
                (Some(r), Some(d)) => format!("{job_id} after {d:.1} s, in {}", r.phase),
                _ => job_id.clone(),
            };
            state.event(now, "job cancelled", detail);
        } else if ended.outcome == super::OUTCOME_FAILURE {
            let reason = ended.reason.unwrap_or_default();
            state.event(now, "job failed", format!("{job_id}: {reason}"));
            let seq = state.next_failure;
            state.next_failure += 1;
            push_bounded(
                &mut state.failures,
                RECENT_FAILURES,
                FailureRecord {
                    seq,
                    at: now,
                    job_id,
                    reason: clip(reason),
                    cause: clip(worker_cause(reason)),
                    input_bytes: running.as_ref().and_then(|r| r.input_bytes),
                    steps: ended.steps.or(running.as_ref().and_then(|r| r.steps_estimate)),
                },
            );
        }
    }

    /// Something happened that the live view lists: `kind` is a short fixed phrase.
    pub(crate) fn event(&self, kind: &'static str, detail: String) {
        self.state().event(Utc::now(), kind, detail);
    }

    /// The instances per AIR of the last execution, from proofman's summary line
    /// (`"Main: 20 | Mem: 5 | … | Total global instances: 57"`).
    pub(crate) fn execution_summary(&self, summary: &str) {
        let airs = parse_air_instances(summary);
        if !airs.is_empty() {
            self.state().air_instances = airs;
        }
    }

    /// Append the journal's series to a `/metrics` body.
    pub(crate) fn render(&self, out: &mut String) {
        let now = Instant::now();
        let mut state = self.state();

        family(out, "coordinator_job_now_seconds", "The jobs in flight (seconds since each started) or, while none runs and for two minutes after it ended, the last one (its duration)");
        if state.running.is_empty() {
            if let Some(e) =
                state.last_ended.as_ref().filter(|e| now.duration_since(e.ended) <= JUST_ENDED)
            {
                let mhz = e.mhz.map(|m| format!("{m:.1}")).unwrap_or_default();
                let workers = e.workers.to_string();
                let ended_at = e.at.format("%H:%M:%S").to_string();
                sample(
                    out,
                    "coordinator_job_now_seconds",
                    &[
                        ("job_id", &e.job_id),
                        ("kind", e.kind),
                        ("status", e.outcome),
                        ("phase", "ended"),
                        ("workers", &workers),
                        ("mhz", &mhz),
                        ("ended_at", &ended_at),
                    ],
                    e.duration,
                    None,
                );
            }
        }
        for r in &state.running {
            let workers = r.workers.to_string();
            let seconds = now.duration_since(r.started).as_secs_f64();
            sample(
                out,
                "coordinator_job_now_seconds",
                &[
                    ("job_id", &short_id(&r.job_id)),
                    ("kind", r.kind),
                    ("status", "running"),
                    ("phase", r.phase),
                    ("workers", &workers),
                    ("mhz", ""),
                    ("ended_at", ""),
                ],
                seconds,
                None,
            );
        }

        family(
            out,
            "coordinator_recent_job",
            "The last jobs that ended, one series each, valued at their end time in milliseconds",
        );
        for j in &state.jobs {
            let finished = j.at.format("%H:%M:%S").to_string();
            let duration = j.duration.map(|d| format!("{d:.3}")).unwrap_or_default();
            let phases = j
                .phases
                .map(|p| format!("{:.2}+{:.2}+{:.2}", p[0], p[1], p[2]))
                .unwrap_or_default();
            let steps = j.steps.map(|s| s.to_string()).unwrap_or_default();
            let instances = j.instances.map(|i| i.to_string()).unwrap_or_default();
            let mhz = j.mhz.map(|m| format!("{m:.1}")).unwrap_or_default();
            let input = j.input_bytes.map(|b| b.to_string()).unwrap_or_default();
            sample(
                out,
                "coordinator_recent_job",
                &[
                    ("job_id", &j.job_id),
                    ("kind", j.kind),
                    ("outcome", j.outcome),
                    ("finished", &finished),
                    ("duration_seconds", &duration),
                    ("phases", &phases),
                    ("steps", &steps),
                    ("instances", &instances),
                    ("mhz", &mhz),
                    ("input_bytes", &input),
                ],
                j.at.timestamp_millis() as f64,
                None,
            );
        }

        family(out, "coordinator_recent_event", "The last events (workers, setups, programs, cancels, failures, recoveries), one series each, valued at their time in milliseconds");
        for e in &state.events {
            let seq = e.seq.to_string();
            let at = e.at.format("%H:%M:%S").to_string();
            sample(
                out,
                "coordinator_recent_event",
                &[("seq", &seq), ("at", &at), ("kind", e.kind), ("detail", &e.detail)],
                e.at.timestamp_millis() as f64,
                None,
            );
        }

        family(out, "coordinator_job_failure_info", "The last failed jobs, one series each, valued at their time in milliseconds: the reason, the worker's error in it, the input size and the steps");
        for f in &state.failures {
            let seq = f.seq.to_string();
            let at = f.at.format("%H:%M:%S").to_string();
            let input = f.input_bytes.map(|b| b.to_string()).unwrap_or_default();
            let steps = f.steps.map(|s| s.to_string()).unwrap_or_default();
            sample(
                out,
                "coordinator_job_failure_info",
                &[
                    ("seq", &seq),
                    ("at", &at),
                    ("job_id", &f.job_id),
                    ("reason", &f.reason),
                    ("cause", &f.cause),
                    ("input_bytes", &input),
                    ("steps", &steps),
                ],
                f.at.timestamp_millis() as f64,
                None,
            );
        }

        family(
            out,
            "coordinator_last_execution_air_instances",
            "Instances per AIR in the last execution summary",
        );
        for (air, count) in &state.air_instances {
            sample(
                out,
                "coordinator_last_execution_air_instances",
                &[("air", air)],
                *count as f64,
                None,
            );
        }

        family(out, "coordinator_proof_seconds", "Each proof's duration per phase (1 contributions, 2 prove, 3 aggregation) and in total, at the time it ended");
        for (phase, series) in state.proof_seconds.iter_mut() {
            if let Some((value, at)) = series.next(now) {
                sample(out, "coordinator_proof_seconds", &[("phase", phase)], value, Some(at));
            }
        }

        family(
            out,
            "coordinator_job_unsuccessful_seconds",
            "Each failed or cancelled job's time from its start to its end, at the time it ended",
        );
        for (outcome, series) in state.unsuccessful_seconds.iter_mut() {
            if let Some((value, at)) = series.next(now) {
                sample(
                    out,
                    "coordinator_job_unsuccessful_seconds",
                    &[("outcome", outcome)],
                    value,
                    Some(at),
                );
            }
        }

        family(
            out,
            "coordinator_proof_input_bytes",
            "Each proof's input size, at the time it ended (as coordinator_proof_seconds)",
        );
        if let Some((value, at)) = state.proof_input_bytes.next(now) {
            sample(out, "coordinator_proof_input_bytes", &[], value, Some(at));
        }

        family(
            out,
            "coordinator_proof_steps",
            "Each proof's steps, at the time it ended (as coordinator_proof_seconds)",
        );
        if let Some((value, at)) = state.proof_steps.next(now) {
            sample(out, "coordinator_proof_steps", &[], value, Some(at));
        }
    }
}

impl State {
    /// Record an event; its detail, which can quote a worker, is clipped to one short line.
    fn event(&mut self, at: DateTime<Utc>, kind: &'static str, detail: String) {
        let seq = self.next_event;
        self.next_event += 1;
        let detail = clip(&detail);
        push_bounded(&mut self.events, RECENT_EVENTS, EventRecord { seq, at, kind, detail });
    }
}

fn push_bounded<T>(queue: &mut VecDeque<T>, bound: usize, item: T) {
    if queue.len() == bound {
        queue.pop_front();
    }
    queue.push_back(item);
}

/// The id as the coordinator's log lines show it.
pub(crate) fn short_id(job_id: &JobId) -> String {
    job_id.as_str().chars().take(SHORT_ID_CHARS).collect()
}

/// The worker's own error inside a failure reason that quotes it
/// (`"… on worker WorkerId(1): <error>"`), or `""`.
fn worker_cause(reason: &str) -> &str {
    reason
        .find("WorkerId(")
        .and_then(|i| reason[i..].find("): ").map(|j| &reason[i + j + 3..]))
        .unwrap_or("")
}

/// `text` as one line of at most [`MAX_TEXT_CHARS`] characters.
fn clip(text: &str) -> String {
    let mut out: String =
        text.chars().map(|c| if c.is_control() { ' ' } else { c }).take(MAX_TEXT_CHARS).collect();
    if text.chars().count() > MAX_TEXT_CHARS {
        out.pop();
        out.push('…');
    }
    out
}

fn parse_air_instances(summary: &str) -> Vec<(String, u64)> {
    summary
        .split(" | ")
        .filter(|part| !part.starts_with("Total "))
        .filter_map(|part| {
            let (air, count) = part.rsplit_once(':')?;
            Some((air.trim().to_owned(), count.trim().parse().ok()?))
        })
        .collect()
}

/// Append a gauge family's `# HELP` and `# TYPE` lines.
fn family(out: &mut String, name: &str, help: &str) {
    prometheus_text::family(out, name, "gauge", help);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{OUTCOME_CANCELLED, OUTCOME_FAILURE, OUTCOME_SUCCESS};

    fn rendered(journal: &Journal) -> String {
        let mut out = String::new();
        journal.render(&mut out);
        out
    }

    /// The samples of `family` in a rendered body (its `# HELP`/`# TYPE` lines excluded).
    fn samples<'a>(body: &'a str, family: &str) -> Vec<&'a str> {
        body.lines()
            .filter(|l| !l.starts_with('#'))
            .filter(|l| l.strip_prefix(family).is_some_and(|rest| rest.starts_with(['{', ' '])))
            .collect()
    }

    fn ended<'a>(job_id: &'a JobId, outcome: &'static str) -> JobEnded<'a> {
        JobEnded {
            job_id,
            kind: "prove",
            outcome,
            duration: Some(3.5),
            phases: None,
            steps: None,
            instances: None,
            reason: None,
        }
    }

    #[test]
    fn a_running_job_shows_its_phase_and_workers() {
        let journal = Journal::default();
        let job = JobId::new();
        journal.job_started(&job, "prove", 2);
        journal.job_phase(&job, PHASE_PROVE);

        let body = rendered(&journal);
        let now = samples(&body, "coordinator_job_now_seconds");
        assert_eq!(now.len(), 1, "{body}");
        assert!(now[0].contains(r#"status="running""#), "{}", now[0]);
        assert!(now[0].contains(r#"phase="prove""#), "{}", now[0]);
        assert!(now[0].contains(r#"workers="2""#), "{}", now[0]);
        assert!(now[0].contains(&format!(r#"job_id="{}""#, short_id(&job))), "{}", now[0]);
    }

    #[test]
    fn a_proof_that_ended_is_listed_with_its_phases_and_speed() {
        let journal = Journal::default();
        let job = JobId::new();
        journal.job_started(&job, "prove", 1);
        journal.job_input(&job, 712_000);
        journal.job_ended(&JobEnded {
            phases: Some([1.0, 2.0, 0.5]),
            steps: Some(350_000_000),
            instances: Some(57),
            ..ended(&job, OUTCOME_SUCCESS)
        });

        let body = rendered(&journal);
        let recent = samples(&body, "coordinator_recent_job");
        assert_eq!(recent.len(), 1, "{body}");
        for label in [
            r#"outcome="success""#,
            r#"phases="1.00+2.00+0.50""#,
            r#"steps="350000000""#,
            r#"instances="57""#,
            r#"mhz="100.0""#,
            r#"duration_seconds="3.500""#,
            r#"input_bytes="712000""#,
        ] {
            assert!(recent[0].contains(label), "{label} missing in {}", recent[0]);
        }

        // Nothing runs: the job that just ended is the one shown.
        let now = samples(&body, "coordinator_job_now_seconds");
        assert_eq!(now.len(), 1, "{body}");
        assert!(now[0].contains(r#"status="success""#) && now[0].ends_with(" 3.5"), "{}", now[0]);

        // One timed sample per phase and the total, each with its timestamp.
        let proof = samples(&body, "coordinator_proof_seconds");
        assert_eq!(proof.len(), 4, "{body}");
        assert!(proof.iter().all(|s| s.split(' ').count() == 3), "{proof:?}");
        assert!(proof
            .iter()
            .any(|s| s.starts_with(r#"coordinator_proof_seconds{phase="total"} 3.5 "#)));

        // Its input size and steps, at the same time as its total.
        let at = |line: &str| line.rsplit(' ').next().map(str::to_owned);
        let total = proof.iter().find(|s| s.contains(r#"phase="total""#)).copied().unwrap();
        let input = samples(&body, "coordinator_proof_input_bytes");
        let steps = samples(&body, "coordinator_proof_steps");
        assert!(input.len() == 1 && input[0].starts_with("coordinator_proof_input_bytes 712000 "));
        assert!(steps.len() == 1 && steps[0].starts_with("coordinator_proof_steps 350000000 "));
        assert_eq!(at(input[0]), at(total));
        assert_eq!(at(steps[0]), at(total));
    }

    #[test]
    fn a_registration_makes_the_next_proof_cold_once() {
        let journal = Journal::default();
        assert!(!journal.take_cold());
        journal.mark_cold();
        assert!(journal.take_cold());
        assert!(!journal.take_cold());
    }

    #[test]
    fn a_failure_records_the_worker_error_input_and_steps() {
        let journal = Journal::default();
        let job = JobId::new();
        journal.job_started(&job, "prove", 1);
        journal.job_input(&job, 5_690_000);
        journal.job_steps_estimate(&job, 531_000_000);
        journal.job_ended(&JobEnded {
            reason: Some(
                "Task execution failed on worker WorkerId(1): Error aggregating proofs: \"bad\"\nmore",
            ),
            ..ended(&job, OUTCOME_FAILURE)
        });

        let body = rendered(&journal);
        let failures = samples(&body, "coordinator_job_failure_info");
        assert_eq!(failures.len(), 1, "{body}");
        let f = failures[0];
        assert!(f.contains(r#"cause="Error aggregating proofs: \"bad\" more""#), "{f}");
        assert!(f.contains(r#"input_bytes="5690000""#), "{f}");
        assert!(f.contains(r#"steps="531000000""#), "{f}");
        assert!(f.contains(r#"seq="0""#), "{f}");

        let events = samples(&body, "coordinator_recent_event");
        assert!(events.iter().any(|e| e.contains(r#"kind="job failed""#)), "{events:?}");
        let unsuccessful = samples(&body, "coordinator_job_unsuccessful_seconds");
        assert_eq!(unsuccessful.len(), 1, "{body}");
        assert!(unsuccessful[0]
            .starts_with(r#"coordinator_job_unsuccessful_seconds{outcome="failure"} 3.5 "#));
    }

    #[test]
    fn a_cancel_says_how_long_and_in_which_phase() {
        let journal = Journal::default();
        let job = JobId::new();
        journal.job_started(&job, "prove", 1);
        journal.job_phase(&job, PHASE_CONTRIBUTIONS);
        journal.job_ended(&ended(&job, OUTCOME_CANCELLED));

        let body = rendered(&journal);
        let events = samples(&body, "coordinator_recent_event");
        let expected = format!(r#"detail="{} after 3.5 s, in contributions""#, short_id(&job));
        assert!(events.iter().any(|e| e.contains(&expected)), "{events:?}");
        assert!(samples(&body, "coordinator_job_failure_info").is_empty());
    }

    #[test]
    fn jobs_events_and_failures_are_bounded() {
        let journal = Journal::default();
        for _ in 0..RECENT_FAILURES + 5 {
            let job = JobId::new();
            journal.job_ended(&JobEnded { reason: Some("x"), ..ended(&job, OUTCOME_FAILURE) });
        }
        let body = rendered(&journal);
        assert_eq!(samples(&body, "coordinator_recent_job").len(), RECENT_JOBS);
        assert_eq!(samples(&body, "coordinator_recent_event").len(), RECENT_EVENTS);
        assert_eq!(samples(&body, "coordinator_job_failure_info").len(), RECENT_FAILURES);
    }

    #[test]
    fn a_timed_series_exposes_one_queued_sample_per_scrape_then_expires() {
        let mut series = TimedSeries::default();
        let t0 = Utc::now();
        series.push(1.0, t0);
        series.push(2.0, t0 + chrono::Duration::seconds(1));
        let now = Instant::now();
        assert_eq!(series.next(now).map(|s| s.0), Some(1.0));
        assert_eq!(series.next(now).map(|s| s.0), Some(2.0));
        // Nothing queued: the current sample stays while it is fresh, then goes.
        assert_eq!(series.next(now + TIMED_EXPOSURE).map(|s| s.0), Some(2.0));
        assert_eq!(series.next(now + TIMED_EXPOSURE + Duration::from_secs(1)), None);
    }

    #[test]
    fn a_timed_series_nobody_scrapes_keeps_its_latest_samples() {
        let mut series = TimedSeries::default();
        let t0 = Utc::now();
        for i in 0..MAX_PENDING_SAMPLES + 3 {
            series.push(i as f64, t0);
        }
        assert_eq!(series.pending.len(), MAX_PENDING_SAMPLES);
        assert_eq!(series.next(Instant::now()).map(|s| s.0), Some(3.0));
    }

    #[test]
    fn the_air_summary_is_parsed_from_proofmans_line() {
        let journal = Journal::default();
        journal.execution_summary(
            "Add256: 1 | Main: 20 | VirtualTableZisk0: 1 | Total global instances: 57",
        );
        let body = rendered(&journal);
        let airs = samples(&body, "coordinator_last_execution_air_instances");
        assert_eq!(
            airs,
            [
                r#"coordinator_last_execution_air_instances{air="Add256"} 1"#,
                r#"coordinator_last_execution_air_instances{air="Main"} 20"#,
                r#"coordinator_last_execution_air_instances{air="VirtualTableZisk0"} 1"#,
            ]
        );
        // An unparseable line leaves the last summary in place.
        journal.execution_summary("no summary");
        assert_eq!(
            samples(&rendered(&journal), "coordinator_last_execution_air_instances").len(),
            3
        );
    }

    #[test]
    fn long_or_multiline_text_is_clipped_to_one_line() {
        let long = "x".repeat(MAX_TEXT_CHARS + 10);
        let clipped = clip(&long);
        assert_eq!(clipped.chars().count(), MAX_TEXT_CHARS);
        assert!(clipped.ends_with('…'));
        assert_eq!(clip("a\nb\tc"), "a b c");
    }

    #[test]
    fn the_worker_error_is_found_only_after_a_worker_id() {
        assert_eq!(worker_cause("on worker WorkerId(abc): boom: deep"), "boom: deep");
        assert_eq!(worker_cause("[Monitor] Phase Recurse timed out"), "");
    }

    #[test]
    fn an_event_quoting_a_worker_is_clipped_to_one_line() {
        let journal = Journal::default();
        journal.event("setup failed", format!("on w: {}\nsecond line", "e".repeat(500)));
        let body = rendered(&journal);
        let events = samples(&body, "coordinator_recent_event");
        assert_eq!(events.len(), 1);
        assert!(!events[0].contains("\\n") && events[0].contains('…'), "{}", events[0]);
    }
}
