//! Assertions for observed dashboard-facing coordinator state.

use chrono::Utc;

use crate::recorder::{Sample, Snapshot};
use crate::scenarios::{CleanProofSuccessSpec, PhaseCode};

#[derive(Debug, Clone)]
pub struct Check {
    pub label: String,
    pub passed: bool,
    pub detail: Option<String>,
}

impl Check {
    pub fn pass(label: impl Into<String>) -> Self {
        Self { label: label.into(), passed: true, detail: None }
    }

    pub fn fail(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { label: label.into(), passed: false, detail: Some(detail.into()) }
    }
}

/// Baseline values captured before terminal assertions.
#[derive(Debug, Clone)]
pub struct Baseline {
    pub jobs_total_success: f64,
    pub phase_duration_count: std::collections::HashMap<String, f64>,
    pub job_duration_count: f64,
    pub executed_steps_total: f64,
    pub last_successful_timestamp: f64,
    pub program_jobs_count: Option<u64>,
}

pub fn assert_running_contributions(
    snapshot: &Snapshot,
    program_alias: &str,
    spec: &CleanProofSuccessSpec,
) -> (Vec<Check>, Baseline) {
    let mut checks = Vec::new();

    let current = &snapshot.current;
    checks.push(check(
        "/api/v1/jobs/current.status=running",
        current.status == "running",
        format!("got status={}", current.status),
    ));
    checks.push(check(
        "/api/v1/jobs/current.phase=Contributions",
        current.phase == "Contributions" && current.phase_code == PhaseCode::Contributions.code(),
        format!("phase={} code={}", current.phase, current.phase_code),
    ));
    checks.push(check(
        "/api/v1/jobs/current.age_seconds>=0",
        current.age_seconds.is_some(),
        "age_seconds was null".to_owned(),
    ));
    checks.push(check(
        "/api/v1/jobs/current.workers_count>=1",
        current.workers_count.is_some_and(|count| count >= 1),
        format!("workers_count={:?}", current.workers_count),
    ));

    let running_in_contribs = snapshot.workers.iter().any(|worker| {
        worker.status == "running"
            && worker.phase.as_deref() == Some(PhaseCode::Contributions.label())
    });
    checks.push(check(
        "/api/v1/workers >=1 status=running phase=Contributions",
        running_in_contribs,
        format!("observed {} workers", snapshot.workers.len()),
    ));

    let active_value = snapshot.metrics.value_or_zero(
        "coordinator_active_jobs",
        &[("kind", spec.job_kind.as_str()), ("program", program_alias)],
    );
    checks.push(check(
        "coordinator_active_jobs{kind=prove,program=*}=1",
        (active_value - 1.0).abs() < f64::EPSILON,
        format!("active_jobs={active_value}"),
    ));

    let running_workers =
        snapshot.metrics.sum("coordinator_workers_by_status", &[("status", "running")]);
    checks.push(check(
        "coordinator_workers_by_status{status=running}>=1",
        running_workers >= 1.0,
        format!("running={running_workers}"),
    ));

    let heartbeat_violations = snapshot
        .metrics
        .iter("coordinator_worker_heartbeat_lag_seconds", &[])
        .filter(|sample| sample.value >= spec.heartbeat_lag_max_seconds)
        .map(format_sample)
        .collect::<Vec<_>>();
    checks.push(check(
        format!("coordinator_worker_heartbeat_lag_seconds<{}s", spec.heartbeat_lag_max_seconds),
        heartbeat_violations.is_empty(),
        format!("violations: {heartbeat_violations:?}"),
    ));

    let baseline = Baseline {
        jobs_total_success: snapshot
            .metrics
            .sum("coordinator_jobs_total", &[("outcome", "success"), ("program", program_alias)]),
        phase_duration_count: spec
            .phase_duration_counts
            .iter()
            .map(|phase| {
                let value = snapshot.metrics.sum(
                    "coordinator_phase_duration_seconds_count",
                    &[("phase", phase.label()), ("program", program_alias)],
                );
                (phase.label().to_owned(), value)
            })
            .collect(),
        job_duration_count: snapshot
            .metrics
            .sum("coordinator_job_duration_seconds_count", &[("program", program_alias)]),
        executed_steps_total: snapshot
            .metrics
            .sum("coordinator_job_executed_steps_total", &[("program", program_alias)]),
        last_successful_timestamp: snapshot
            .metrics
            .value_or_zero("coordinator_last_successful_job_timestamp_seconds", &[]),
        program_jobs_count: None,
    };

    (checks, baseline)
}

pub fn assert_phase_transition(
    previous: PhaseCode,
    new_snapshot: &Snapshot,
    expected_next: PhaseCode,
) -> Vec<Check> {
    let mut checks = Vec::new();
    let observed = new_snapshot.current.phase_code_enum();
    checks.push(check(
        format!("phase_code {} -> {}", previous.code(), expected_next.code()),
        observed == expected_next,
        format!(
            "observed phase={} code={}",
            new_snapshot.current.phase, new_snapshot.current.phase_code
        ),
    ));

    let phase_age = new_snapshot.current.phase_age_seconds.unwrap_or(u64::MAX);
    checks.push(check(
        "phase_age_seconds reset (<5s)",
        phase_age < 5,
        format!("phase_age_seconds={phase_age}"),
    ));

    checks
}

pub fn assert_terminal(
    snapshot: &Snapshot,
    program_alias: &str,
    expected_executed_steps: Option<u64>,
    baseline: &Baseline,
    spec: &CleanProofSuccessSpec,
) -> Vec<Check> {
    let mut checks = Vec::new();

    let current = &snapshot.current;
    checks.push(check(
        "/api/v1/jobs/current.status=idle",
        current.status == "idle",
        format!("got status={}", current.status),
    ));

    let active = snapshot.metrics.value_or_zero(
        "coordinator_active_jobs",
        &[("kind", spec.job_kind.as_str()), ("program", program_alias)],
    );
    checks.push(check(
        "coordinator_active_jobs{program=*}=0",
        active.abs() < f64::EPSILON,
        format!("active_jobs={active}"),
    ));

    let now_success = snapshot
        .metrics
        .sum("coordinator_jobs_total", &[("outcome", "success"), ("program", program_alias)]);
    let delta_success = now_success - baseline.jobs_total_success;
    checks.push(check(
        "coordinator_jobs_total{outcome=success} +1",
        (delta_success - 1.0).abs() < f64::EPSILON,
        format!(
            "baseline={} now={} delta={}",
            baseline.jobs_total_success, now_success, delta_success
        ),
    ));

    for phase in &spec.phase_duration_counts {
        let now = snapshot.metrics.sum(
            "coordinator_phase_duration_seconds_count",
            &[("phase", phase.label()), ("program", program_alias)],
        );
        let baseline_value =
            baseline.phase_duration_count.get(phase.label()).copied().unwrap_or(0.0);
        let delta = now - baseline_value;
        checks.push(check(
            format!("coordinator_phase_duration_seconds_count{{phase={}}} +1", phase.label()),
            (delta - 1.0).abs() < f64::EPSILON,
            format!("baseline={baseline_value} now={now} delta={delta}"),
        ));
    }

    let now_job_duration = snapshot
        .metrics
        .sum("coordinator_job_duration_seconds_count", &[("program", program_alias)]);
    let delta_job_duration = now_job_duration - baseline.job_duration_count;
    checks.push(check(
        "coordinator_job_duration_seconds_count +1",
        (delta_job_duration - 1.0).abs() < f64::EPSILON,
        format!(
            "baseline={} now={} delta={}",
            baseline.job_duration_count, now_job_duration, delta_job_duration
        ),
    ));

    let now_steps =
        snapshot.metrics.sum("coordinator_job_executed_steps_total", &[("program", program_alias)]);
    let delta_steps = now_steps - baseline.executed_steps_total;
    match expected_executed_steps {
        Some(expected) => {
            checks.push(check(
                "coordinator_job_executed_steps_total advanced by job.executed_steps",
                (delta_steps - expected as f64).abs() < 1.0,
                format!("expected +{expected} got +{delta_steps}"),
            ));
        }
        None => {
            checks.push(check(
                "coordinator_job_executed_steps_total advanced (>0)",
                delta_steps > 0.0,
                format!("delta_steps={delta_steps}"),
            ));
        }
    }

    let now_ts =
        snapshot.metrics.value_or_zero("coordinator_last_successful_job_timestamp_seconds", &[]);
    let now_wall = Utc::now().timestamp() as f64;
    let was_bumped = now_ts > baseline.last_successful_timestamp;
    let is_recent = (now_wall - now_ts).abs() < 120.0;
    checks.push(check(
        "coordinator_last_successful_job_timestamp_seconds bumped to recent",
        was_bumped && is_recent,
        format!(
            "baseline_ts={} new_ts={} wall_now={} (bumped={was_bumped} recent={is_recent})",
            baseline.last_successful_timestamp, now_ts, now_wall,
        ),
    ));

    checks
}

pub fn assert_phase_timings_present(
    phase_timings: &[zisk_coordinator::JobHistoryPhaseTiming],
    expected_phases: &[PhaseCode],
) -> Vec<Check> {
    expected_phases
        .iter()
        .map(|phase| {
            let present = phase_timings.iter().any(|timing| timing.phase == phase.label());
            check(
                format!("/api/v1/jobs/{{id}}.phase_timings contains {}", phase.label()),
                present,
                format!(
                    "available phases: {:?}",
                    phase_timings.iter().map(|t| t.phase.as_str()).collect::<Vec<_>>()
                ),
            )
        })
        .collect()
}

fn check(label: impl Into<String>, passed: bool, detail: impl Into<String>) -> Check {
    if passed {
        Check::pass(label)
    } else {
        Check::fail(label, detail)
    }
}

fn format_sample(sample: &Sample) -> String {
    let mut labels: Vec<_> = sample.labels.iter().collect();
    labels.sort_by(|left, right| left.0.cmp(right.0));
    let label_str = labels
        .iter()
        .map(|(key, value)| format!("{key}=\"{value}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!("{}{{{label_str}}}={}", sample.name, sample.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recorder::{CurrentJobRow, MetricSamples, Sample};

    fn sample(name: &str, labels: &[(&str, &str)], value: f64) -> Sample {
        Sample {
            name: name.to_owned(),
            labels: labels.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(),
            value,
        }
    }

    fn idle_snapshot_with(metrics: Vec<Sample>) -> Snapshot {
        Snapshot {
            elapsed_seconds: 0.0,
            current: CurrentJobRow {
                status: "idle".to_owned(),
                phase: "idle".to_owned(),
                phase_code: 0,
                coordinator_id: None,
                job_id: None,
                state: None,
                age_seconds: None,
                phase_age_seconds: None,
                update_age_seconds: None,
                workers_count: None,
            },
            workers: vec![],
            metrics: MetricSamples { samples: metrics },
        }
    }

    #[test]
    fn transition_check_detects_correct_advance() {
        let mut snap = idle_snapshot_with(vec![]);
        snap.current.phase = "Prove".to_owned();
        snap.current.phase_code = 3;
        snap.current.phase_age_seconds = Some(0);
        let checks = assert_phase_transition(PhaseCode::Contributions, &snap, PhaseCode::Prove);
        assert_eq!(checks.len(), 2);
        assert!(checks.iter().all(|c| c.passed), "{checks:?}");
    }

    #[test]
    fn transition_check_flags_stuck_phase() {
        let mut snap = idle_snapshot_with(vec![]);
        snap.current.phase = "Contributions".to_owned();
        snap.current.phase_code = 2;
        snap.current.phase_age_seconds = Some(0);
        let checks = assert_phase_transition(PhaseCode::Contributions, &snap, PhaseCode::Prove);
        let advance = &checks[0];
        assert!(!advance.passed, "expected phase_code advance to fail: {checks:?}");
    }

    #[test]
    fn terminal_check_diff_against_baseline() {
        let baseline = Baseline {
            jobs_total_success: 5.0,
            phase_duration_count: [
                ("Contributions".to_owned(), 5.0),
                ("Prove".to_owned(), 5.0),
                ("Aggregate".to_owned(), 5.0),
            ]
            .into_iter()
            .collect(),
            job_duration_count: 5.0,
            executed_steps_total: 100.0,
            last_successful_timestamp: 1_700_000_000.0,
            program_jobs_count: None,
        };

        let now_ts = chrono::Utc::now().timestamp() as f64;
        let metrics = vec![
            sample("coordinator_active_jobs", &[("kind", "prove"), ("program", "p")], 0.0),
            sample("coordinator_jobs_total", &[("outcome", "success"), ("program", "p")], 6.0),
            sample(
                "coordinator_phase_duration_seconds_count",
                &[("phase", "Contributions"), ("program", "p")],
                6.0,
            ),
            sample(
                "coordinator_phase_duration_seconds_count",
                &[("phase", "Prove"), ("program", "p")],
                6.0,
            ),
            sample(
                "coordinator_phase_duration_seconds_count",
                &[("phase", "Aggregate"), ("program", "p")],
                6.0,
            ),
            sample("coordinator_job_duration_seconds_count", &[("program", "p")], 6.0),
            sample(
                "coordinator_job_executed_steps_total",
                &[("program", "p")],
                100.0 + 4_516_996.0,
            ),
            sample("coordinator_last_successful_job_timestamp_seconds", &[], now_ts),
        ];
        let snap = idle_snapshot_with(metrics);
        let spec = CleanProofSuccessSpec::default();
        let checks = assert_terminal(&snap, "p", Some(4_516_996), &baseline, &spec);
        let failed = checks.iter().filter(|c| !c.passed).collect::<Vec<_>>();
        assert!(failed.is_empty(), "expected all terminal checks to pass, failed={failed:?}");
    }
}
