//! Graceful shutdown signal — resolves on SIGTERM or SIGINT (Ctrl-C).
//!
//! Pass the future returned by [`shutdown_signal`] to
//! `tonic::Server::serve_with_shutdown` so the server drains in-flight
//! requests before exiting.

use futures::stream::StreamExt;
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook_tokio::Signals;

/// Returns a future that resolves when SIGTERM or SIGINT is received.
///
/// If signal handler registration fails (rare system-level failure), logs a warning
/// and returns a future that never resolves so the server keeps running.
pub async fn shutdown_signal() {
    let mut signals = match Signals::new([SIGTERM, SIGINT]) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                "failed to register signal handlers: {e} — graceful shutdown unavailable, \
                 send SIGKILL to stop the process"
            );
            std::future::pending::<()>().await;
            return;
        }
    };

    if let Some(sig) = signals.next().await {
        match sig {
            SIGTERM => tracing::info!("received SIGTERM — initiating graceful shutdown"),
            SIGINT => tracing::info!("received SIGINT  — initiating graceful shutdown"),
            _ => tracing::info!(signal = sig, "received signal — initiating graceful shutdown"),
        }
    }
}
