use futures::stream::StreamExt;
use signal_hook_tokio::{Signals, SignalsInfo};
use std::io;
use tokio::signal;
use tracing::{info, warn};

/// Creates a future that resolves when a shutdown signal is received
pub async fn create_shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown");
        },
    }
}

/// Enhanced shutdown signal handler that can be used for more complex scenarios
pub struct ShutdownHandler {
    signals: SignalsInfo,
}

impl ShutdownHandler {
    pub fn new() -> io::Result<Self> {
        let signals = Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])?;
        Ok(Self { signals })
    }

    pub async fn wait_for_shutdown(&mut self) {
        while let Some(signal) = self.signals.next().await {
            match signal {
                signal_hook::consts::SIGTERM => {
                    info!("Received SIGTERM, shutting down gracefully");
                    break;
                }
                signal_hook::consts::SIGINT => {
                    info!("Received SIGINT (Ctrl+C), shutting down gracefully");
                    break;
                }
                _ => {
                    warn!("Received unexpected signal: {}", signal);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_shutdown_handler_creation() {
        let result = ShutdownHandler::new();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_signal_timeout() {
        // Test that the shutdown signal doesn't resolve immediately
        let result = timeout(Duration::from_millis(100), create_shutdown_signal()).await;
        assert!(result.is_err()); // Should timeout
    }
}
