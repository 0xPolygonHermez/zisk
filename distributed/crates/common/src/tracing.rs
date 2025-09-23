use anyhow::Result;
use std::env;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

pub fn init() -> Result<()> {
    let log_level = env::var("DISTRIBUTED_LOGGING_LEVEL").unwrap_or_else(|_| "info".to_string());
    let log_format = env::var("DISTRIBUTED_LOGGING_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    match log_format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .json()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_current_span(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_env_filter(env_filter)
                .init();
        }
        "compact" => {
            tracing_subscriber::fmt()
                .compact()
                .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
                .with_target(true)
                .with_thread_ids(true)
                .with_env_filter(env_filter)
                .init();
        }
        _ => {
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
