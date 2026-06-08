//! CLI that runs live dashboard smoke probes.

use std::process::ExitCode;

use clap::Parser;

mod config;
mod contract;
mod http;
mod probes;

use config::{Cli, Config};

const EXIT_OK: u8 = 0;
const EXIT_FAILED: u8 = 1;
const EXIT_BAD_INPUT: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = Config::from_cli(cli);

    if !config.skip_grafana && config.grafana_password.is_none() {
        eprintln!(
            "ERROR: GRAFANA_PASSWORD (or --grafana-password) is required to smoke-test the \
             Grafana API; pass --skip-grafana to skip"
        );
        return ExitCode::from(EXIT_BAD_INPUT);
    }

    let client = match http::build_client() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("ERROR: {error}");
            return ExitCode::from(EXIT_BAD_INPUT);
        }
    };

    let mut errors: Vec<String> = Vec::new();

    if !config.skip_prometheus {
        match probes::prometheus::run(&client, &config.prometheus_url, &config.coordinator_id) {
            Ok(mut probe_errors) => errors.append(&mut probe_errors),
            Err(error) => errors.push(error.to_string()),
        }
    }

    if !config.skip_coordinator {
        let coordinator_headers = match http::coordinator_headers(config.scrape_token.as_deref()) {
            Ok(headers) => headers,
            Err(error) => {
                eprintln!("ERROR: {error}");
                return ExitCode::from(EXIT_BAD_INPUT);
            }
        };
        match probes::coordinator::run(&client, &config.coordinator_api_url, &coordinator_headers) {
            Ok(mut probe_errors) => errors.append(&mut probe_errors),
            Err(error) => errors.push(error.to_string()),
        }
    }

    if !config.skip_grafana {
        let password =
            config.grafana_password.as_deref().expect("grafana_password gate enforced above");
        let grafana_auth = http::BasicAuth::new(&config.grafana_user, password);
        match probes::grafana::run(
            &client,
            &config.grafana_url,
            &config.dashboard_uid,
            &config.expected_refresh,
            &grafana_auth,
        ) {
            Ok(mut probe_errors) => errors.append(&mut probe_errors),
            Err(error) => errors.push(error.to_string()),
        }
    }

    if !errors.is_empty() {
        eprintln!("dashboard runtime smoke failed:");
        for error in &errors {
            eprintln!("- {error}");
        }
        return ExitCode::from(EXIT_FAILED);
    }

    println!("dashboard runtime smoke passed");
    ExitCode::from(EXIT_OK)
}
