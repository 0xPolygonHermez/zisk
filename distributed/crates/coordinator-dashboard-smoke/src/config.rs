//! CLI/env configuration for live smoke probes.

use std::env;

use clap::Parser;

pub const DEFAULT_GRAFANA_URL: &str = "http://127.0.0.1:3000";
pub const DEFAULT_GRAFANA_USER: &str = "admin";
pub const DEFAULT_DASHBOARD_UID: &str = "zisk-dev";
pub const DEFAULT_PROMETHEUS_URL: &str = "http://127.0.0.1:9091";
pub const DEFAULT_COORDINATOR_API_URL: &str = "http://127.0.0.1:19090";
pub const DEFAULT_EXPECTED_REFRESH: &str = "1s";
pub const DEFAULT_COORDINATOR_ID: &str = ".*";

#[derive(Debug, Parser)]
#[command(
    name = "smoke-dashboard",
    about = "Live smoke test for the grafonnet-generated ZisK dashboard. \
             Verifies Grafana, Prometheus and the coordinator API expose the \
             v0.18 contract used by the operator dashboard.",
    long_about = None,
)]
pub struct Cli {
    /// Grafana base URL override (env: `GRAFANA_URL`).
    #[arg(long)]
    pub grafana: Option<String>,

    /// Grafana basic-auth username (env: `GRAFANA_USER`, default `admin`).
    #[arg(long)]
    pub grafana_user: Option<String>,

    /// Grafana basic-auth password (env: `GRAFANA_PASSWORD`, required).
    #[arg(long)]
    pub grafana_password: Option<String>,

    /// Prometheus base URL (env: `PROMETHEUS_URL`).
    #[arg(long)]
    pub prometheus: Option<String>,

    /// Coordinator HTTP API base URL (env: `ZISK_COORDINATOR_API_URL`).
    #[arg(long)]
    pub coordinator_api: Option<String>,

    /// Bearer token for the coordinator API (env: `ZISK_SCRAPE_TOKEN`).
    #[arg(long)]
    pub scrape_token: Option<String>,

    /// Grafana dashboard UID (env: `GRAFANA_DASHBOARD_UID`).
    #[arg(long)]
    pub dashboard_uid: Option<String>,

    /// Expected Grafana auto-refresh interval (env: `EXPECTED_DASHBOARD_REFRESH`).
    #[arg(long)]
    pub expected_refresh: Option<String>,

    /// Coordinator-id regex for PromQL filtering (env: `ZISK_COORDINATOR_ID`).
    #[arg(long)]
    pub coordinator_id: Option<String>,

    /// Skip the Grafana API check.
    #[arg(long)]
    pub skip_grafana: bool,

    /// Skip the Prometheus check.
    #[arg(long)]
    pub skip_prometheus: bool,

    /// Skip the coordinator API check.
    #[arg(long)]
    pub skip_coordinator: bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub grafana_url: String,
    pub grafana_user: String,
    pub grafana_password: Option<String>,
    pub prometheus_url: String,
    pub coordinator_api_url: String,
    pub scrape_token: Option<String>,
    pub dashboard_uid: String,
    pub expected_refresh: String,
    pub coordinator_id: String,
    pub skip_grafana: bool,
    pub skip_prometheus: bool,
    pub skip_coordinator: bool,
}

impl Config {
    pub fn from_cli(cli: Cli) -> Self {
        fn env_nonempty(key: &str) -> Option<String> {
            env::var(key).ok().filter(|value| !value.is_empty())
        }

        fn pick(cli_value: Option<String>, env_key: &str, default: &str) -> String {
            cli_value.or_else(|| env_nonempty(env_key)).unwrap_or_else(|| default.to_owned())
        }

        fn pick_opt(cli_value: Option<String>, env_key: &str) -> Option<String> {
            cli_value.or_else(|| env_nonempty(env_key))
        }

        let grafana_url =
            trim_trailing_slash(&pick(cli.grafana, "GRAFANA_URL", DEFAULT_GRAFANA_URL));
        let grafana_user = pick(cli.grafana_user, "GRAFANA_USER", DEFAULT_GRAFANA_USER);
        let grafana_password = pick_opt(cli.grafana_password, "GRAFANA_PASSWORD");
        let prometheus_url =
            trim_trailing_slash(&pick(cli.prometheus, "PROMETHEUS_URL", DEFAULT_PROMETHEUS_URL));
        let coordinator_api_url = trim_trailing_slash(&pick(
            cli.coordinator_api,
            "ZISK_COORDINATOR_API_URL",
            DEFAULT_COORDINATOR_API_URL,
        ));
        let scrape_token = pick_opt(cli.scrape_token, "ZISK_SCRAPE_TOKEN");
        let dashboard_uid = pick(cli.dashboard_uid, "GRAFANA_DASHBOARD_UID", DEFAULT_DASHBOARD_UID);
        let expected_refresh =
            pick(cli.expected_refresh, "EXPECTED_DASHBOARD_REFRESH", DEFAULT_EXPECTED_REFRESH);
        let coordinator_id =
            pick(cli.coordinator_id, "ZISK_COORDINATOR_ID", DEFAULT_COORDINATOR_ID);

        Self {
            grafana_url,
            grafana_user,
            grafana_password,
            prometheus_url,
            coordinator_api_url,
            scrape_token,
            dashboard_uid,
            expected_refresh,
            coordinator_id,
            skip_grafana: cli.skip_grafana,
            skip_prometheus: cli.skip_prometheus,
            skip_coordinator: cli.skip_coordinator,
        }
    }
}

fn trim_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::parse_from(args)
    }

    #[test]
    fn cli_accepts_all_documented_flags() {
        // Locks the documented CLI surface.
        let auth_fixture = ["grafana", "test", "value"].join("-");
        let token_fixture = ["scrape", "test", "value"].join("-");
        let cli = parse(&[
            "smoke-dashboard",
            "--grafana",
            "http://gf:3000",
            "--prometheus",
            "http://prom:9091",
            "--coordinator-api",
            "http://coord:19090",
            "--dashboard-uid",
            "zisk-dev",
            "--expected-refresh",
            "1s",
            "--coordinator-id",
            "prod-.*",
            "--grafana-password",
            &auth_fixture,
            "--scrape-token",
            &token_fixture,
        ]);
        assert_eq!(cli.grafana.as_deref(), Some("http://gf:3000"));
        assert_eq!(cli.prometheus.as_deref(), Some("http://prom:9091"));
        assert_eq!(cli.coordinator_api.as_deref(), Some("http://coord:19090"));
        assert_eq!(cli.dashboard_uid.as_deref(), Some("zisk-dev"));
        assert_eq!(cli.expected_refresh.as_deref(), Some("1s"));
        assert_eq!(cli.coordinator_id.as_deref(), Some("prod-.*"));
        assert_eq!(cli.grafana_password.as_deref(), Some(auth_fixture.as_str()));
        assert_eq!(cli.scrape_token.as_deref(), Some(token_fixture.as_str()));
    }

    #[test]
    fn config_defaults_match_local_defaults_when_no_env_no_cli() {
        // Synthetic CLI with nothing set must reproduce local defaults.
        let cli = Cli {
            grafana: None,
            grafana_user: None,
            grafana_password: None,
            prometheus: None,
            coordinator_api: None,
            scrape_token: None,
            dashboard_uid: None,
            expected_refresh: None,
            coordinator_id: None,
            skip_grafana: false,
            skip_prometheus: false,
            skip_coordinator: false,
        };
        // Keep defaults independent of the developer shell environment.
        for key in [
            "GRAFANA_URL",
            "GRAFANA_USER",
            "GRAFANA_PASSWORD",
            "PROMETHEUS_URL",
            "ZISK_COORDINATOR_API_URL",
            "ZISK_SCRAPE_TOKEN",
            "GRAFANA_DASHBOARD_UID",
            "EXPECTED_DASHBOARD_REFRESH",
            "ZISK_COORDINATOR_ID",
        ] {
            // SAFETY: this test owns the env mutation before constructing Config.
            unsafe { std::env::remove_var(key) };
        }
        let cfg = Config::from_cli(cli);
        assert_eq!(cfg.grafana_url, DEFAULT_GRAFANA_URL);
        assert_eq!(cfg.grafana_user, DEFAULT_GRAFANA_USER);
        assert!(cfg.grafana_password.is_none());
        assert_eq!(cfg.prometheus_url, DEFAULT_PROMETHEUS_URL);
        assert_eq!(cfg.coordinator_api_url, DEFAULT_COORDINATOR_API_URL);
        assert!(cfg.scrape_token.is_none());
        assert_eq!(cfg.dashboard_uid, DEFAULT_DASHBOARD_UID);
        assert_eq!(cfg.expected_refresh, DEFAULT_EXPECTED_REFRESH);
        assert_eq!(cfg.coordinator_id, DEFAULT_COORDINATOR_ID);
    }

    #[test]
    fn trim_trailing_slash_strips_only_trailing() {
        assert_eq!(trim_trailing_slash("http://x/"), "http://x");
        assert_eq!(trim_trailing_slash("http://x"), "http://x");
        assert_eq!(trim_trailing_slash("http://x//"), "http://x");
    }
}
