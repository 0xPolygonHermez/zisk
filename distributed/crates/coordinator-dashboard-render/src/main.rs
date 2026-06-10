//! CLI that queries each dashboard panel through Grafana and checks expected rows.

use std::process::ExitCode;

use clap::Parser;
use serde_json::Value;

mod assert;
mod grafana;
mod panels;
mod report;

use assert::{expectation_for, OperatorState};
use panels::{extract_panels, Panel, TargetKind};
use report::{PanelReport, Summary, Verdict};

const EXIT_OK: u8 = 0;
const EXIT_FAILED: u8 = 1;
const EXIT_BAD_INPUT: u8 = 2;

#[derive(Debug, Parser)]
#[command(
    name = "render-dashboard",
    about = "Query each ZisK dashboard panel's data through Grafana and \
             assert populated/empty state per the operator expectation \
             table.",
    long_about = None,
)]
struct Cli {
    /// Grafana base URL (e.g. http://127.0.0.1:3001).
    #[arg(long)]
    grafana: String,

    /// Grafana basic-auth username (default `admin`).
    #[arg(long, default_value = "admin")]
    grafana_user: String,

    /// Grafana basic-auth password.
    #[arg(long, env = "GRAFANA_PASSWORD")]
    grafana_password: String,

    /// Dashboard UID to inspect (must match the dashboard JSON).
    #[arg(long)]
    dashboard_uid: String,

    /// Operator state for row-population checks (`idle`, `running`, or `terminal`).
    #[arg(long)]
    state: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let state = match OperatorState::parse(&cli.state) {
        Some(s) => s,
        None => {
            eprintln!(
                "ERROR: --state must be one of `idle`, `running`, `terminal` (got `{}`)",
                cli.state
            );
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let client = match grafana::build_client() {
        Ok(c) => c,
        Err(error) => {
            eprintln!("ERROR: {error}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };
    let auth = grafana::BasicAuth::new(&cli.grafana_user, &cli.grafana_password);

    let dashboard = match grafana::fetch_dashboard(
        &client,
        cli.grafana.trim_end_matches('/'),
        &cli.dashboard_uid,
        &auth,
    ) {
        Ok(d) => d,
        Err(error) => {
            eprintln!("ERROR: failed to fetch dashboard: {error}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let panels = extract_panels(&dashboard);
    if panels.is_empty() {
        eprintln!("ERROR: dashboard {} returned no panels", cli.dashboard_uid);
        return ExitCode::from(EXIT_BAD_INPUT);
    }

    let reports = run_panels(&client, cli.grafana.trim_end_matches('/'), &auth, &panels, state);
    for report in &reports {
        report::print_panel(report);
    }
    let summary = Summary::from_reports(&reports);
    report::print_summary(&summary, state);

    if summary.fail > 0 {
        ExitCode::from(EXIT_FAILED)
    } else {
        ExitCode::from(EXIT_OK)
    }
}

fn run_panels(
    client: &reqwest::blocking::Client,
    grafana_url: &str,
    auth: &grafana::BasicAuth,
    panels: &[Panel],
    state: OperatorState,
) -> Vec<PanelReport> {
    panels.iter().map(|panel| run_one_panel(client, grafana_url, auth, panel, state)).collect()
}

fn run_one_panel(
    client: &reqwest::blocking::Client,
    grafana_url: &str,
    auth: &grafana::BasicAuth,
    panel: &Panel,
    state: OperatorState,
) -> PanelReport {
    let expectation = expectation_for(&panel.title);

    let supported_targets: Vec<Value> = panel
        .targets
        .iter()
        .filter(|t| TargetKind::from_target(t) != TargetKind::Unsupported)
        .cloned()
        .collect();

    if supported_targets.is_empty() {
        let note = if panel.targets.is_empty() {
            "no targets".to_owned()
        } else {
            "no supported datasource".to_owned()
        };
        return PanelReport {
            title: panel.title.clone(),
            panel_type: panel.panel_type.clone(),
            expectation,
            verdict: Verdict::Skip,
            rows: 0,
            note: Some(note),
        };
    }

    let result = grafana::query_panel(client, grafana_url, auth, &supported_targets);
    PanelReport::from_result(
        panel.title.clone(),
        panel.panel_type.clone(),
        expectation,
        state,
        &result,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_parses_full_flag_set() {
        let auth_fixture = ["grafana", "test", "value"].join("-");
        let cli = Cli::parse_from([
            "render-dashboard",
            "--grafana",
            "http://127.0.0.1:3001",
            "--grafana-user",
            "admin",
            "--grafana-password",
            &auth_fixture,
            "--dashboard-uid",
            "zisk-dev",
            "--state",
            "idle",
        ]);
        assert_eq!(cli.grafana, "http://127.0.0.1:3001");
        assert_eq!(cli.grafana_user, "admin");
        assert_eq!(cli.grafana_password, auth_fixture);
        assert_eq!(cli.dashboard_uid, "zisk-dev");
        assert_eq!(cli.state, "idle");
    }

    #[test]
    fn cli_state_must_be_validatable() {
        assert!(OperatorState::parse("idle").is_some());
        assert!(OperatorState::parse("nonsense").is_none());
    }

    #[test]
    fn cli_password_is_required() {
        let error = Cli::try_parse_from([
            "render-dashboard",
            "--grafana",
            "http://127.0.0.1:3001",
            "--dashboard-uid",
            "zisk-dev",
            "--state",
            "running",
        ])
        .unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }
}
