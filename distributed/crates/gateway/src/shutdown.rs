//! Graceful shutdown signal — resolves on SIGTERM or SIGINT (Ctrl-C).
//!
//! Pass the future returned by [`shutdown_signal`] to
//! `tonic::Server::serve_with_shutdown` so the server drains in-flight
//! requests before exiting.

use futures::stream::StreamExt;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;

/// Returns a future that resolves when SIGTERM or SIGINT is received.
pub async fn shutdown_signal() {
    let mut signals = Signals::new([SIGTERM, SIGINT]).expect("failed to register signal handlers");

    if let Some(sig) = signals.next().await {
        match sig {
            SIGTERM => tracing::info!("received SIGTERM — initiating graceful shutdown"),
            SIGINT => tracing::info!("received SIGINT  — initiating graceful shutdown"),
            _ => tracing::info!(signal = sig, "received signal — initiating graceful shutdown"),
        }
    }
}
