//! Live harness that observes one proof run and checks dashboard-facing signals.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

mod assert;
mod dashboard;
mod recorder;
mod report;
mod scenarios;

use crate::assert::{
    assert_phase_timings_present, assert_phase_transition, assert_running_contributions,
    assert_terminal,
};
use crate::dashboard::DashboardProbe;
use crate::recorder::{Recorder, Snapshot};
use crate::report::{Report, Stage};
use crate::scenarios::{CleanProofSuccessSpec, PhaseCode, ScenarioKind};

const EXIT_OK: u8 = 0;
const EXIT_ASSERT_FAILED: u8 = 1;
const EXIT_BAD_INPUT: u8 = 2;

struct ScenarioContext<'a> {
    recorder: &'a Recorder<'a>,
    client: &'a Client,
    headers: &'a HeaderMap,
    coordinator_api: &'a str,
    dashboard: &'a DashboardProbe<'a>,
}

#[derive(Debug, Parser)]
#[command(
    name = "integration-dashboard",
    about = "Live integration harness: poll the coordinator during a real proof run and \
             assert metrics/routes react correctly at each phase transition.",
    long_about = None,
)]
struct Cli {
    /// Coordinator HTTP API base URL.
    #[arg(long, default_value = "http://127.0.0.1:19090")]
    coordinator_api: String,

    /// Bearer token for the coordinator API.
    #[arg(long)]
    scrape_token: Option<String>,

    /// Grafana base URL.
    #[arg(long, default_value = "http://127.0.0.1:3001")]
    grafana: String,

    /// Grafana user for API checks.
    #[arg(long, default_value = "admin")]
    grafana_user: String,

    /// Grafana password for API checks.
    #[arg(long, env = "GRAFANA_PASSWORD")]
    grafana_password: String,

    /// Dashboard UID served by Grafana.
    #[arg(long, default_value = "zisk-dev")]
    dashboard_uid: String,

    /// Scenario slug. Currently `clean-proof-success`.
    #[arg(long, default_value = "clean-proof-success")]
    scenario: String,

    /// Overall wall-clock budget for the harness, in seconds.
    #[arg(long, default_value_t = 600)]
    timeout_seconds: u64,

    /// Polling cadence in milliseconds.
    #[arg(long, default_value_t = 1000)]
    poll_interval_ms: u64,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let scenario = match cli.scenario.parse::<ScenarioKind>() {
        Ok(kind) => kind,
        Err(error) => {
            eprintln!("ERROR: {error}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let client = match build_client() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("ERROR: {error}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let headers = match coordinator_headers(cli.scrape_token.as_deref()) {
        Ok(headers) => headers,
        Err(error) => {
            eprintln!("ERROR: {error}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let coordinator_api = trim_trailing_slash(&cli.coordinator_api);
    let recorder = Recorder::new(&client, &headers, &coordinator_api);

    if let Err(error) = recorder.snapshot() {
        eprintln!("ERROR: coordinator unreachable on first probe: {error}");
        return ExitCode::from(EXIT_BAD_INPUT);
    }

    let grafana_url = trim_trailing_slash(&cli.grafana);
    let dashboard = match DashboardProbe::new(
        &client,
        &grafana_url,
        &cli.grafana_user,
        &cli.grafana_password,
        &cli.dashboard_uid,
    ) {
        Ok(probe) => probe,
        Err(error) => {
            eprintln!("ERROR: Grafana dashboard probe failed: {error}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let total_timeout = Duration::from_secs(cli.timeout_seconds);
    let poll_interval = Duration::from_millis(cli.poll_interval_ms);

    let context = ScenarioContext {
        recorder: &recorder,
        client: &client,
        headers: &headers,
        coordinator_api: &coordinator_api,
        dashboard: &dashboard,
    };

    let mut report = Report::new();
    let outcome = match scenario {
        ScenarioKind::CleanProofSuccess => {
            run_clean_proof_success(&context, poll_interval, total_timeout, &mut report)
        }
    };

    print!("{}", report.render());

    match outcome {
        Ok(()) => {
            if report.is_pass() {
                ExitCode::from(EXIT_OK)
            } else {
                ExitCode::from(EXIT_ASSERT_FAILED)
            }
        }
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::from(EXIT_BAD_INPUT)
        }
    }
}

fn run_clean_proof_success(
    context: &ScenarioContext<'_>,
    poll_interval: Duration,
    total_timeout: Duration,
    report: &mut Report,
) -> Result<()> {
    let spec = CleanProofSuccessSpec::default();
    let started = Instant::now();
    let recorder = context.recorder;
    let dashboard = context.dashboard;

    let baseline_snapshot = recorder
        .poll_until(poll_interval, total_timeout, |snapshot| {
            snapshot.current.status == "running"
                && snapshot.current.phase_code_enum() == PhaseCode::Contributions
        })
        .context("waiting for a job to enter Contributions")?;

    let program_alias = baseline_snapshot
        .current
        .job_id
        .as_ref()
        .and_then(|_| baseline_snapshot.workers.iter().find_map(|w| w.program.clone()))
        .unwrap_or_else(|| "unknown".to_owned());

    let job_id = baseline_snapshot.current.job_id.clone().ok_or_else(|| {
        anyhow!("baseline snapshot has no job_id; coordinator API contract drift")
    })?;

    let (running_checks, mut baseline) =
        assert_running_contributions(&baseline_snapshot, &program_alias, &spec);

    baseline.program_jobs_count = dashboard.program_jobs_count(&program_alias).ok().flatten();

    report.push(Stage {
        elapsed_seconds: baseline_snapshot.elapsed_seconds,
        header: format!(
            "Observed: phase=Contributions phase_code=2 workers={}",
            baseline_snapshot.workers.len()
        ),
        checks: running_checks,
    });

    let mut previous_phase = PhaseCode::Contributions;
    for expected_next in spec.expected_phases.iter().skip(1).copied() {
        let next_snapshot = wait_for_phase_or_terminal(
            recorder,
            poll_interval,
            remaining(total_timeout, started),
            previous_phase,
            expected_next,
        )?;
        let checks = assert_phase_transition(previous_phase, &next_snapshot, expected_next);
        report.push(Stage {
            elapsed_seconds: next_snapshot.elapsed_seconds,
            header: format!("Transition {} -> {}", previous_phase.label(), expected_next.label()),
            checks,
        });
        previous_phase = expected_next;
    }

    let terminal_snapshot = recorder
        .poll_until(poll_interval, remaining(total_timeout, started), |snapshot| {
            snapshot.current.status != "running"
        })
        .context("waiting for terminal status")?;
    let terminal_job =
        fetch_job_by_id(context.client, context.headers, context.coordinator_api, &job_id);
    let expected_steps = terminal_job.as_ref().ok().and_then(|job| job.executed_steps);

    let mut terminal_checks =
        assert_terminal(&terminal_snapshot, &program_alias, expected_steps, &baseline, &spec);

    match terminal_job {
        Ok(job) => {
            terminal_checks
                .extend(assert_phase_timings_present(&job.phase_timings, &spec.expected_phases));
        }
        Err(error) => {
            terminal_checks.push(crate::assert::Check::fail(
                "/api/v1/jobs/{id} phase_timings fetched",
                error.to_string(),
            ));
        }
    }

    match dashboard.recent_history_contains_job(&job_id, &program_alias) {
        Ok(true) => terminal_checks.push(crate::assert::Check::pass(
            "Grafana Recent Proof History contains completed job",
        )),
        Ok(false) => terminal_checks.push(crate::assert::Check::fail(
            "Grafana Recent Proof History contains completed job",
            format!("job_id={job_id} program={program_alias} was not returned by the panel"),
        )),
        Err(error) => terminal_checks.push(crate::assert::Check::fail(
            "Grafana Recent Proof History queried",
            error.to_string(),
        )),
    }

    match dashboard.program_jobs_count(&program_alias) {
        Ok(Some(now_count)) => {
            let baseline_value = baseline.program_jobs_count;
            let passed = match baseline_value {
                Some(baseline) => now_count == baseline + 1,
                None => now_count >= 1,
            };
            let detail = format!("baseline={:?} now={}", baseline_value, now_count);
            terminal_checks.push(if passed {
                crate::assert::Check::pass("Grafana Program Performance Summary jobs +1")
            } else {
                crate::assert::Check::fail("Grafana Program Performance Summary jobs +1", detail)
            });
        }
        Ok(None) => {
            terminal_checks.push(crate::assert::Check::fail(
                "Grafana Program Performance Summary jobs +1",
                format!("no row returned for program={program_alias}"),
            ));
        }
        Err(error) => {
            terminal_checks.push(crate::assert::Check::fail(
                "Grafana Program Performance Summary queried",
                error.to_string(),
            ));
        }
    }

    report.push(Stage {
        elapsed_seconds: terminal_snapshot.elapsed_seconds,
        header: "Terminal: Completed".to_owned(),
        checks: terminal_checks,
    });

    Ok(())
}

fn wait_for_phase_or_terminal(
    recorder: &Recorder<'_>,
    poll_interval: Duration,
    budget: Duration,
    previous: PhaseCode,
    expected_next: PhaseCode,
) -> Result<Snapshot> {
    recorder.poll_until(poll_interval, budget, |snapshot| {
        let code = snapshot.current.phase_code_enum();
        code == expected_next || snapshot.current.status != "running" || code != previous
    })
}

fn remaining(total: Duration, started: Instant) -> Duration {
    total.saturating_sub(started.elapsed())
}

fn fetch_job_by_id(
    client: &Client,
    headers: &HeaderMap,
    coordinator_api: &str,
    job_id: &str,
) -> Result<zisk_coordinator::JobHistoryJob> {
    let url = format!("{coordinator_api}/api/v1/jobs/{job_id}");
    let response = client
        .get(&url)
        .headers(headers.clone())
        .send()
        .map_err(|error| anyhow!("{url} failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_else(|e| format!("<body read failed: {e}>"));
        return Err(anyhow!("{url} returned HTTP {}: {body}", status.as_u16()));
    }
    response.json::<zisk_coordinator::JobHistoryJob>().map_err(|error| anyhow!("decode: {error}"))
}

fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| anyhow!("failed to build HTTP client: {error}"))
}

fn coordinator_headers(scrape_token: Option<&str>) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    if let Some(token) = scrape_token {
        let value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|error| anyhow!("failed to encode coordinator auth header: {error}"))?;
        headers.insert(AUTHORIZATION, value);
    }
    Ok(headers)
}

fn trim_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_accepts_documented_flags() {
        let token_fixture = ["zisk", "test", "token"].join("-");
        let auth_fixture = ["grafana", "test", "value"].join("-");
        let cli = Cli::parse_from([
            "integration-dashboard",
            "--coordinator-api",
            "http://127.0.0.1:19090",
            "--scrape-token",
            &token_fixture,
            "--grafana",
            "http://127.0.0.1:3001",
            "--grafana-user",
            "admin",
            "--grafana-password",
            &auth_fixture,
            "--dashboard-uid",
            "zisk-dev",
            "--scenario",
            "clean-proof-success",
            "--timeout-seconds",
            "300",
            "--poll-interval-ms",
            "500",
        ]);
        assert_eq!(cli.coordinator_api, "http://127.0.0.1:19090");
        assert_eq!(cli.scrape_token.as_deref(), Some(token_fixture.as_str()));
        assert_eq!(cli.grafana, "http://127.0.0.1:3001");
        assert_eq!(cli.grafana_user, "admin");
        assert_eq!(cli.grafana_password, auth_fixture);
        assert_eq!(cli.dashboard_uid, "zisk-dev");
        assert_eq!(cli.scenario, "clean-proof-success");
        assert_eq!(cli.timeout_seconds, 300);
        assert_eq!(cli.poll_interval_ms, 500);
    }

    #[test]
    fn cli_requires_grafana_password() {
        let error = Cli::try_parse_from(["integration-dashboard"]).unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn trim_trailing_slash_strips_only_trailing() {
        assert_eq!(trim_trailing_slash("http://x/"), "http://x");
        assert_eq!(trim_trailing_slash("http://x"), "http://x");
        assert_eq!(trim_trailing_slash("http://x//"), "http://x");
    }
}
