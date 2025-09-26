use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fmt::{self, Display},
};
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
    let log_level = if let Some(config) = logging_config {
        config.level.clone()
    } else {
        env::var("DISTRIBUTED_LOGGING_LEVEL").unwrap_or_else(|_| "info".to_string())
    };

    let log_format = if let Some(config) = logging_config {
        config.format.clone()
    } else {
        let format_str =
            env::var("DISTRIBUTED_LOGGING_FORMAT").unwrap_or_else(|_| "pretty".to_string());
        match format_str.as_str() {
            "json" => LogFormat::Json,
            "compact" => LogFormat::Compact,
            _ => LogFormat::Pretty,
        }
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

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
                                .with_writer(std::io::stdout),
                        )
                        .with(
                            tracing_subscriber::fmt::layer()
                                .json()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_current_span(true)
                                .with_thread_ids(true)
                                .with_thread_names(true)
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
                                .with_writer(std::io::stdout),
                        )
                        .with(
                            tracing_subscriber::fmt::layer()
                                .compact()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_target(true)
                                .with_thread_ids(true)
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
                                .with_writer(std::io::stdout),
                        )
                        .with(
                            tracing_subscriber::fmt::layer()
                                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                                .with_writer(non_blocking)
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
                        .init();
                }
                LogFormat::Compact => {
                    tracing_subscriber::fmt()
                        .compact()
                        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                        .with_target(true)
                        .with_thread_ids(true)
                        .with_env_filter(env_filter)
                        .init();
                }
                LogFormat::Pretty => {
                    tracing_subscriber::fmt()
                        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                        .with_env_filter(env_filter)
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
                    .init();
            }
            LogFormat::Compact => {
                tracing_subscriber::fmt()
                    .compact()
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_env_filter(env_filter)
                    .init();
            }
            LogFormat::Pretty => {
                tracing_subscriber::fmt()
                    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                    .with_env_filter(env_filter)
                    .init();
            }
        }
    }

    Ok(None)
}
