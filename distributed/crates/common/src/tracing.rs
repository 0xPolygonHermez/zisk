use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

// Re-export the WorkerGuard type for convenience
pub use tracing_appender::non_blocking::WorkerGuard;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Environment::Development => "development",
            Environment::Staging => "staging",
            Environment::Production => "production",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "pretty")]
    Pretty,
    #[serde(rename = "compact")]
    Compact,
}

impl fmt::Display for LogFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LogFormat::Json => "json",
            LogFormat::Pretty => "pretty",
            LogFormat::Compact => "compact",
        };
        write!(f, "{}", s)
    }
}

/// Initialize the tracing subscriber with the given configuration.
///
/// # Returns
///
/// Returns `Ok(Some(WorkerGuard))` if file logging is enabled. The caller **must**
/// keep this guard alive for the entire application lifetime to prevent log loss.
/// If the guard is dropped, the background logging thread will shut down and
/// buffered logs may be lost.
///
/// Returns `Ok(None)` if only console logging is configured.
pub fn init(logging_config: Option<&LoggingConfig>) -> Result<Option<WorkerGuard>> {
    // Prioritize logging_config values over environment variables
    let log_level =
        logging_config.map(|config| config.level.clone()).unwrap_or_else(|| "info".to_string());

    let log_format =
        logging_config.map(|config| config.format.clone()).unwrap_or(LogFormat::Pretty);

    let mut env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let trace_enabled =
        log_level == "trace" || std::env::var("RUST_LOG").unwrap_or_default().contains("trace");

    if trace_enabled {
        // When trace is enabled, set gRPC libraries to debug (less verbose than trace)
        for directive in ["h2=debug", "tonic=debug", "hyper=debug", "tower=debug"] {
            env_filter = env_filter.add_directive(directive.parse().unwrap());
        }
    } else {
        // When not in trace mode, suppress verbose gRPC logs
        for directive in ["h2=info", "tonic=info", "hyper=info", "tower=info"] {
            env_filter = env_filter.add_directive(directive.parse().unwrap());
        }
    }

    // Apply console logging with optional file logging
    if let Some(config) = logging_config {
        if let Some(file_path) = &config.file_path {
            // Create logs directory if it doesn't exist
            std::fs::create_dir_all("./logs").unwrap_or(());
            let file_appender = tracing_appender::rolling::daily("./logs", file_path);

            // Use non-blocking appender for better performance
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            match log_format {
                LogFormat::Json => {
                    tracing_subscriber::registry()
                        .with(
                            tracing_subscriber::fmt::layer()
                                .json()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_current_span(true)
                                .with_thread_ids(true)
                                .with_thread_names(true)
                                .event_format(proofman_common::RankFormatter)
                                .with_writer(std::io::stdout),
                        )
                        .with(
                            tracing_subscriber::fmt::layer()
                                .json()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_current_span(true)
                                .with_thread_ids(true)
                                .with_thread_names(true)
                                .event_format(proofman_common::RankFormatter)
                                .with_writer(non_blocking)
                                .with_ansi(false),
                        )
                        .with(env_filter)
                        .init();
                }
                LogFormat::Compact => {
                    tracing_subscriber::registry()
                        .with(
                            tracing_subscriber::fmt::layer()
                                .compact()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_target(true)
                                .with_thread_ids(true)
                                .event_format(proofman_common::RankFormatter)
                                .with_writer(std::io::stdout),
                        )
                        .with(
                            tracing_subscriber::fmt::layer()
                                .compact()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_target(true)
                                .with_thread_ids(true)
                                .event_format(proofman_common::RankFormatter)
                                .with_writer(non_blocking)
                                .with_ansi(false),
                        )
                        .with(env_filter)
                        .init();
                }
                LogFormat::Pretty => {
                    tracing_subscriber::registry()
                        .with(
                            tracing_subscriber::fmt::layer()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .event_format(proofman_common::RankFormatter)
                                .with_writer(std::io::stdout),
                        )
                        .with(
                            tracing_subscriber::fmt::layer()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_writer(non_blocking)
                                .event_format(proofman_common::RankFormatter)
                                .with_ansi(false),
                        )
                        .with(env_filter)
                        .init();
                }
            }

            return Ok(Some(guard));
        } else {
            // Console output only
            match log_format {
                LogFormat::Json => {
                    tracing_subscriber::fmt()
                        .json()
                        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                        .with_current_span(true)
                        .with_thread_ids(true)
                        .with_thread_names(true)
                        .with_env_filter(env_filter)
                        .event_format(proofman_common::RankFormatter)
                        .init();
                }
                LogFormat::Compact => {
                    tracing_subscriber::fmt()
                        .compact()
                        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_env_filter(env_filter)
                        .event_format(proofman_common::RankFormatter)
                        .init();
                }
                LogFormat::Pretty => {
                    tracing_subscriber::fmt()
                        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                        .with_env_filter(env_filter)
                        .event_format(proofman_common::RankFormatter)
                        .init();
                }
            }
        }
    } else {
        // Console output only (no config provided)
        match log_format {
            LogFormat::Json => {
                tracing_subscriber::fmt()
                    .json()
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_current_span(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_env_filter(env_filter)
                    .event_format(proofman_common::RankFormatter)
                    .init();
            }
            LogFormat::Compact => {
                tracing_subscriber::fmt()
                    .compact()
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_env_filter(env_filter)
                    .event_format(proofman_common::RankFormatter)
                    .init();
            }
            LogFormat::Pretty => {
                tracing_subscriber::fmt()
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_env_filter(env_filter)
                    .event_format(proofman_common::RankFormatter)
                    .init();
            }
        }
    }

    Ok(None)
}
