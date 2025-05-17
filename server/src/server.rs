use std::{collections::HashMap, net::TcpListener, path::PathBuf, sync::Arc, time::Instant};

use proofman_common::DebugInfo;
use uuid::Uuid;

use tracing::error;
use zisk_common::info_file;

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

    /// Path to the ASM ROM file (optional)
    pub asm_rom: Option<PathBuf>,

    /// Map of custom commits
    pub custom_commits_map: HashMap<String, PathBuf>,

    /// Flag indicating whether to use the prebuilt emulator
    pub emulator: bool,

    /// Path to the proving key
    pub proving_key: PathBuf,

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
        asm_rom: Option<PathBuf>,
        custom_commits_map: HashMap<String, PathBuf>,
        emulator: bool,
        proving_key: PathBuf,
        verbose: u8,
        debug: DebugInfo,
        sha256f_script: PathBuf,
    ) -> Self {
        Self {
            port,
            elf,
            witness_lib,
            asm,
            asm_rom,
            custom_commits_map,
            emulator,
            proving_key,
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

        info_file!(
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
                            info_file!("{}", "Shutdown signal received. Exiting.");
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
