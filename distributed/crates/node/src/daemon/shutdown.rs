use futures::stream::StreamExt;
use signal_hook_tokio::{Signals, SignalsInfo};
use std::io;
use tokio::signal;
use tracing::info;

/// Resolves when SIGTERM or SIGINT is received.
pub async fn wait_for_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}

pub struct ShutdownHandler {
    signals: SignalsInfo,
}

impl ShutdownHandler {
    pub fn new() -> io::Result<Self> {
        let signals = Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])?;
        Ok(Self { signals })
    }

    pub async fn wait(&mut self) {
        while let Some(signal) = self.signals.next().await {
            match signal {
                signal_hook::consts::SIGTERM => {
                    info!("Received SIGTERM, shutting down");
                    break;
                }
                signal_hook::consts::SIGINT => {
                    info!("Received SIGINT, shutting down");
                    break;
                }
                _ => {}
            }
        }
    }
}
