use std::{net::TcpListener, path::PathBuf, sync::Arc, time::Instant};

use tracing::{error, info};
use uuid::Uuid;

use crate::handle_client;

#[derive(Debug)]
pub struct ServerConfig {
    pub elf_path: PathBuf,
    pub port: u16,
    pub launch_time: Instant,
    pub server_id: Uuid,
}

impl ServerConfig {
    pub fn new(elf_path: PathBuf, port: u16) -> Self {
        Self { elf_path, port, launch_time: Instant::now(), server_id: Uuid::new_v4() }
    }
}

pub struct Server {
    config: Arc<ServerConfig>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self { config: Arc::new(config) }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", self.config.port))?;

        info!(
            "Server started on port {} with ELF '{}' and ID {}.",
            self.config.port,
            self.config.elf_path.display(),
            self.config.server_id
        );

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let config = Arc::clone(&self.config);
                    if let Ok(should_shutdown) = handle_client(stream, config) {
                        if should_shutdown {
                            info!("{}", "Shutdown signal received. Exiting.");
                            break;
                        }
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }

        Ok(())
    }
}
