use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    net::TcpStream,
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

use asm_runner::{AsmRunnerOptions, AsmServices};
use colored::Colorize;
use executor::ZiskExecutionResult;
use libloading::{Library, Symbol};
use p3_goldilocks::Goldilocks;
use proofman::ProofMan;
use proofman_common::{DebugInfo, ProofOptions};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;
use zisk_common::{info_file, ZiskLibInitFn};

use anyhow::Result;

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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "command", rename_all = "lowercase")]
pub enum ZiskRequest {
    Status,
    Shutdown,
    Prove {
        #[serde(flatten)]
        payload: ZiskProveRequest,
    },
    VerifyConstraints {
        #[serde(flatten)]
        payload: ZiskVerifyConstraintsRequest,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ZiskResponse {
    Ok { message: String },
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskProveRequest {
    pub input: PathBuf,
    pub aggregation: bool,
    pub final_snark: bool,
    pub verify_proofs: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskProveResponse {
    pub success: bool,
    pub details: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ZiskVerifyConstraintsRequest {
    pub input: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskVerifyConstraintsResponse {
    pub success: bool,
    pub details: String,
}

pub struct ZiskService {
    config: Arc<ServerConfig>,
    // witness_lib: Option<Box<dyn WitnessLibrary<Goldilocks>>>,
}

impl ZiskService {
    pub fn new(config: ServerConfig) -> Result<Self> {
        info_file!("Starting asm microservices...");
        let options = AsmRunnerOptions::default();
        AsmServices::start_asm_services(config.asm.as_ref().unwrap(), options)?;

        Ok(Self { config: Arc::new(config) /*witness_lib: None*/ })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", self.config.port))?;

        info_file!(
            "Server started on 127.0.0.1:{} with ELF '{}' and ID {}.",
            self.config.port,
            self.config.elf.display(),
            self.config.server_id
        );

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let config = Arc::clone(&self.config);
                    if let Ok(should_shutdown) = self.handle_client(stream, config) {
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

    fn handle_client(
        &mut self,
        mut stream: TcpStream,
        config: Arc<ServerConfig>,
    ) -> std::io::Result<bool> {
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();

        reader.read_line(&mut line)?;

        let request: ZiskRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = ZiskResponse::Error { message: format!("Invalid JSON: {}", e) };
                Self::send_json(&mut stream, &response)?;
                return Ok(false);
            }
        };

        info_file!("Received request: {:?}", request);

        let mut must_shutdown = false;
        let response = match request {
            ZiskRequest::Status => self.handle_status(config.as_ref()),
            ZiskRequest::Shutdown => {
                must_shutdown = true;
                self.handle_shutdown()
            }
            ZiskRequest::VerifyConstraints { payload } => {
                self.handle_verify_constraints(config.as_ref(), payload)
            }
            ZiskRequest::Prove { payload } => self.handle_prove(config.as_ref(), payload),
        };

        Self::send_json(&mut stream, &response)?;
        Ok(must_shutdown)
    }

    fn handle_status(&self, config: &ServerConfig) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();
        let status = serde_json::json!({
            "server_id": config.server_id.to_string(),
            "elf_file": config.elf.display().to_string(),
            "uptime": format!("{:.2?}", uptime)
        });
        ZiskResponse::Ok { message: status.to_string() }
    }

    fn handle_shutdown(&self) -> ZiskResponse {
        let msg = serde_json::json!({
            "info": "Shutting down server"
        });
        ZiskResponse::Ok { message: msg.to_string() }
    }

    fn handle_prove(&self, config: &ServerConfig, payload: ZiskProveRequest) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();
        let status = serde_json::json!({
            "server_id": config.server_id.to_string(),
            "elf_file": config.elf.display().to_string(),
            "uptime": format!("{:.2?}", uptime),
            "command:": "prove",
            "payload:": {
                "input": payload.input.display().to_string(),
                "aggregation": payload.aggregation,
                "final_snark": payload.final_snark,
                "verify_proofs": payload.verify_proofs,
            },
        });

        ZiskResponse::Ok { message: status.to_string() }
    }

    fn handle_verify_constraints(
        &mut self,
        config: &ServerConfig,
        request: ZiskVerifyConstraintsRequest,
    ) -> ZiskResponse {
        let uptime = config.launch_time.elapsed();

        let status = serde_json::json!({
            "server_id": config.server_id.to_string(),
            "elf_file": config.elf.display().to_string(),
            "uptime": format!("{:.2?}", uptime),
            "command:": "VerifyConstraints",
            "payload:": {
                "input": request.input.display().to_string(),
            },
        });

        let start = std::time::Instant::now();

        let library =
            unsafe { Library::new(config.witness_lib.clone()).expect("Failed to load library") };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library").expect("Failed to get symbol") };
        let mut witness_lib = witness_lib_constructor(
            config.verbose.into(),
            config.elf.clone(),
            config.asm.clone(),
            config.asm_rom.clone(),
            Some(request.input),
            config.sha256f_script.clone(),
        )
        .expect("Failed to initialize witness library");

        ProofMan::<Goldilocks>::verify_proof_constraints_from_lib(
            &mut *witness_lib,
            config.proving_key.clone(),
            PathBuf::new(),
            config.custom_commits_map.clone(),
            ProofOptions::new(
                true,
                config.verbose.into(),
                false,
                false,
                false,
                config.debug.clone(),
            ),
        )
        .map_err(|e| anyhow::anyhow!("Error generating proof: {}", e))
        .expect("Failed to generate proof");

        let elapsed = start.elapsed();

        let result: ZiskExecutionResult = *witness_lib
            .get_execution_result()
            .ok_or_else(|| anyhow::anyhow!("No execution result found"))
            .expect("Failed to get execution result")
            .downcast::<ZiskExecutionResult>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast execution result"))
            .expect("Failed to downcast execution result");

        println!();
        info!(
            "{}",
            "    Zisk: --- VERIFY CONSTRAINTS SUMMARY ------------------------"
                .bright_green()
                .bold()
        );
        info!("              ► Statistics");
        info!(
            "                time: {} seconds, steps: {}",
            elapsed.as_secs_f32(),
            result.executed_steps
        );

        ZiskResponse::Ok { message: status.to_string() }
    }

    fn send_json(stream: &mut TcpStream, response: &ZiskResponse) -> std::io::Result<()> {
        let json = serde_json::to_string(response)?;
        stream.write_all(json.as_bytes())?;
        stream.flush()
    }
}
