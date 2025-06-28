use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

use asm_runner::{AsmRunnerOptions, AsmServices};
use fields::Goldilocks;
use libloading::{Library, Symbol};
use proofman::ProofMan;
use proofman_common::{DebugInfo, ParamsGPU};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;
use witness::WitnessLibrary;
use zisk_common::{info_file, MpiContext, ZiskLibInitFn};

use anyhow::Result;

use crate::{
    handler_prove::{ZiskProveRequest, ZiskServiceProveHandler},
    handler_shutdown::ZiskServiceShutdownHandler,
    handler_verify_constraints::{
        ZiskServiceVerifyConstraintsHandler, ZiskVerifyConstraintsRequest,
    },
    ZiskServiceStatusHandler,
};

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
    pub debug_info: DebugInfo,

    /// Path to the SHA256f script
    pub sha256f_script: PathBuf,

    /// Time when the server was launched
    pub launch_time: Instant,

    /// Unique identifier for the server instance
    pub server_id: Uuid,

    /// Size of the chunks in bits
    pub chunk_size_bits: Option<u64>,

    /// Additional options for the ASM runner
    pub asm_runner_options: AsmRunnerOptions,

    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,

    pub gpu_params: ParamsGPU,
}

#[allow(clippy::too_many_arguments)]
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
        chunk_size_bits: Option<u64>,
        asm_runner_options: AsmRunnerOptions,
        verify_constraints: bool,
        aggregation: bool,
        final_snark: bool,
        gpu_params: ParamsGPU,
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
            debug_info: debug,
            sha256f_script,
            launch_time: Instant::now(),
            server_id: Uuid::new_v4(),
            chunk_size_bits,
            asm_runner_options,
            verify_constraints,
            aggregation,
            final_snark,
            gpu_params,
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

pub struct ZiskService {
    config: Arc<ServerConfig>,
    proofman: ProofMan<Goldilocks>,
    witness_lib: Box<dyn WitnessLibrary<Goldilocks>>,
    asm_services: AsmServices,
    is_busy: AtomicBool,
}

impl ZiskService {
    pub fn new(config: ServerConfig, mpi_context: MpiContext) -> Result<Self> {
        info_file!("Starting asm microservices...");

        let world_rank = config.asm_runner_options.world_rank;
        let local_rank = config.asm_runner_options.local_rank;
        let base_port = config.asm_runner_options.base_port;
        let map_locked = config.asm_runner_options.map_locked;

        let asm_services = AsmServices::new(world_rank, local_rank, base_port);
        asm_services
            .start_asm_services(config.asm.as_ref().unwrap(), config.asm_runner_options.clone())?;

        let library =
            unsafe { Library::new(config.witness_lib.clone()).expect("Failed to load library") };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library").expect("Failed to get symbol") };

        let mut witness_lib = witness_lib_constructor(
            config.verbose.into(),
            config.elf.clone(),
            config.asm.clone(),
            config.asm_rom.clone(),
            config.sha256f_script.clone(),
            config.chunk_size_bits,
            Some(world_rank),
            Some(local_rank),
            base_port,
            map_locked,
        )
        .expect("Failed to initialize witness library");

        let proofman;
        #[cfg(distributed)]
        {
            proofman = ProofMan::<Goldilocks>::new(
                config.proving_key.clone(),
                config.custom_commits_map.clone(),
                config.verify_constraints,
                config.aggregation,
                config.final_snark,
                config.gpu_params.clone(),
                config.verbose.into(),
                Some(mpi_context.universe),
            )
            .expect("Failed to initialize proofman");
        }

        #[cfg(not(distributed))]
        {
            proofman = ProofMan::<Goldilocks>::new(
                config.proving_key.clone(),
                config.custom_commits_map.clone(),
                config.verify_constraints,
                config.aggregation,
                config.final_snark,
                config.gpu_params.clone(),
                config.verbose.into(),
                None,
            )
            .expect("Failed to initialize proofman");
        }

        proofman.register_witness(witness_lib.as_mut(), library);

        Ok(Self {
            config: Arc::new(config),
            proofman,
            witness_lib,
            asm_services,
            is_busy: AtomicBool::new(false),
        })
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
                let response = ZiskResponse::Error { message: format!("Invalid JSON: {e}") };
                Self::send_json(&mut stream, &response)?;
                return Ok(false);
            }
        };

        info_file!("Received request: {:?}", request);

        let mut must_shutdown = false;
        self.is_busy.store(true, std::sync::atomic::Ordering::SeqCst);
        let response = match request {
            ZiskRequest::Status => ZiskServiceStatusHandler::handle(&config),
            ZiskRequest::Shutdown => {
                must_shutdown = true;
                ZiskServiceShutdownHandler::handle(&self.asm_services, &self.config)
            }
            ZiskRequest::VerifyConstraints { payload } => {
                ZiskServiceVerifyConstraintsHandler::handle(
                    &config,
                    payload,
                    &self.proofman,
                    self.witness_lib.as_mut(),
                    &self.config.debug_info,
                )
            }

            ZiskRequest::Prove { payload } => ZiskServiceProveHandler::handle(
                &config,
                payload,
                &self.proofman,
                self.witness_lib.as_mut(),
            ),
        };
        self.is_busy.store(false, std::sync::atomic::Ordering::SeqCst);

        Self::send_json(&mut stream, &response)?;
        Ok(must_shutdown)
    }

    fn send_json(stream: &mut TcpStream, response: &ZiskResponse) -> std::io::Result<()> {
        let json = serde_json::to_string(response)?;
        stream.write_all(json.as_bytes())?;
        stream.flush()
    }
}
