//! Logging initialisation for the zisklet daemon.
//!
//! Call [`init`] once at process start, before any `tracing` macros fire.
//!
//! # Formats
//! Controlled by `logging.format` in `node.toml`:
//!
//! | Value    | Output                                      | Best for               |
//! |----------|---------------------------------------------|------------------------|
//! | `pretty` | Human-readable, coloured (default)          | Local dev / debugging  |
//! | `json`   | One JSON object per line (structured logs)  | Production / log ships |
//!
//! # Level filter
//! `RUST_LOG` env var takes precedence over `logging.level` in the config file,
//! following the standard `tracing_subscriber::EnvFilter` rules.

use crate::config::LoggingConfig;
use anyhow::Result;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialise the global tracing subscriber from `config`.
///
/// Must be called exactly once, before any `tracing` macros are used.
/// Returns an error only if a global subscriber has already been installed.
pub fn init(config: &LoggingConfig) -> Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    match config.format.as_str() {
        "json" => {
            tracing_subscriber::registry().with(filter).with(fmt::layer().json()).try_init()?;
        }
        _ => {
            tracing_subscriber::registry().with(filter).with(fmt::layer()).try_init()?;
        }
    }

    Ok(())
}
