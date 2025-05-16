use std::{net::TcpListener, path::PathBuf, sync::Arc, time::Instant};

use proofman_common::DebugInfo;
use tracing::{error, info};
use uuid::Uuid;

use crate::handle_client;

pub struct ServerConfig {
    /// Port number for the server to listen on
    pub port: u16,

    /// Path to the ELF file
    pub elf: PathBuf,

    /// Path to the witness computation dynamic library
    pub witness_lib: PathBuf,

    /// Path to the ASM file (optional)
    pub asm: Option<PathBuf>,

    /// Flag indicating whether to use the prebuilt emulator
    pub emulator: bool,

    /// Path to the proving key
    pub proving_key: PathBuf,

    /// Indicates whether the proof includes recursive aggregation.
    pub aggregation: bool,

    /// Indicates whether the prover should produce a final SNARK.
    pub final_snark: bool,

    /// Indicates whether the prover should verify the produced proofs.
    pub verify_proofs: bool,

    /// Verbosity level for logging
    pub verbose: u8,

    /// Debug information
    pub debug: DebugInfo,

    /// Path to the SHA256f script
    pub sha256f_script: PathBuf,

    /// Time when the server was launched
    pub launch_time: Instant,

    /// Unique identifier for the server instance
    pub server_id: Uuid,
}

impl ServerConfig {
    pub fn new(
        port: u16,
        elf: PathBuf,
        witness_lib: PathBuf,
        asm: Option<PathBuf>,
        emulator: bool,
        proving_key: PathBuf,
        aggregation: bool,
        final_snark: bool,
        verify_proofs: bool,
        verbose: u8,
        debug: DebugInfo,
        sha256f_script: PathBuf,
    ) -> Self {
        Self {
            port,
            elf,
            witness_lib,
            asm,
            emulator,
            proving_key,
            aggregation,
            final_snark,
            verify_proofs,
            verbose,
            debug,
            sha256f_script,
            launch_time: Instant::now(),
            server_id: Uuid::new_v4(),
        }
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
            self.config.elf.display(),
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
