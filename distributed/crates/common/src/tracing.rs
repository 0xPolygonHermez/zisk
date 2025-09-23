use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{env, fmt};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub file_output: bool,
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

pub fn init(logging_config: Option<&LoggingConfig>) -> Result<()> {
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
            // Default to pretty format
            tracing_subscriber::fmt()
                // .pretty()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                // .with_thread_ids(true)
                // .with_thread_names(true)
                .with_env_filter(env_filter)
                .init();
        }
    }

    Ok(())
}
