use std::{
    collections::HashMap,
    fmt,
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
    handler_status::{ZiskStatusRequest, ZiskStatusResponse},
    handler_verify_constraints::{
        ZiskServiceVerifyConstraintsHandler, ZiskVerifyConstraintsRequest,
    },
    ZiskProveResponse, ZiskServiceStatusHandler, ZiskShutdownRequest, ZiskShutdownResponse,
    ZiskVerifyConstraintsResponse,
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
    pub debug_info: Arc<DebugInfo>,

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
            debug_info: Arc::new(debug),
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
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ZiskRequest {
    Status {
        #[serde(flatten)]
        payload: ZiskStatusRequest,
    },
    Shutdown {
        #[serde(flatten)]
        payload: ZiskShutdownRequest,
    },
    Prove {
        #[serde(flatten)]
        payload: ZiskProveRequest,
    },
    VerifyConstraints {
        #[serde(flatten)]
        payload: ZiskVerifyConstraintsRequest,
    },
}

impl fmt::Display for ZiskRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant = match self {
            ZiskRequest::Status { .. } => "Status",
            ZiskRequest::Shutdown { .. } => "Shutdown",
            ZiskRequest::Prove { .. } => "Prove",
            ZiskRequest::VerifyConstraints { .. } => "VerifyConstraints",
        };
        write!(f, "{variant}")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZiskCmdResult {
    Ok,
    Error,
    InProgress,
    Busy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ZiskResultCode {
    Ok = 0,
    Error = 1001,
    InvalidRequest = 1002,
    Busy = 1003,
}

// Serialize as a number
impl Serialize for ZiskResultCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(*self as u32)
    }
}

// Deserialize from a number
impl<'de> Deserialize<'de> for ZiskResultCode {
    fn deserialize<D>(deserializer: D) -> Result<ZiskResultCode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        match value {
            0 => Ok(ZiskResultCode::Ok),
            1001 => Ok(ZiskResultCode::Error),
            1002 => Ok(ZiskResultCode::InvalidRequest),
            1003 => Ok(ZiskResultCode::Busy),
            _ => Err(serde::de::Error::custom(format!("Unknown ZiskResultCode: {value}"))),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskBaseResponse {
    pub cmd: String,
    pub result: ZiskCmdResult,
    pub code: ZiskResultCode,
    pub node: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZiskInvalidRequestResponse {
    #[serde(flatten)]
    pub base: ZiskBaseResponse,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "zisk_response", rename_all = "snake_case")]
pub enum ZiskResponse {
    ZiskStatusResponse(ZiskStatusResponse),
    ZiskShutdownResponse(ZiskShutdownResponse),
    ZiskProveResponse(ZiskProveResponse),
    ZiskVerifyConstraintsResponse(ZiskVerifyConstraintsResponse),
    ZiskErrorResponse(ZiskBaseResponse),
    ZiskInvalidRequestResponse { base: ZiskBaseResponse },
}

pub struct ZiskService {
    config: Arc<ServerConfig>,
    // It is important to keep the witness_lib declaration before the proofman declaration
    // to ensure that the witness library is dropped before the proofman.
    witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync>,
    proofman: Arc<ProofMan<Goldilocks>>,
    asm_services: Option<AsmServices>,
    is_busy: Arc<AtomicBool>,
    pending_handles: Vec<std::thread::JoinHandle<()>>,
}

impl ZiskService {
    pub fn new(config: ServerConfig, mpi_context: MpiContext) -> Result<Self> {
        info_file!("Starting asm microservices...");

        let world_rank = config.asm_runner_options.world_rank;
        let local_rank = config.asm_runner_options.local_rank;
        let base_port = config.asm_runner_options.base_port;
        let unlock_mapped_memory = config.asm_runner_options.unlock_mapped_memory;

        let asm_services = if config.emulator {
            None
        } else {
            let asm_services = AsmServices::new(world_rank, local_rank, base_port);
            asm_services.start_asm_services(
                config.asm.as_ref().unwrap(),
                config.asm_runner_options.clone(),
            )?;
            Some(asm_services)
        };

        let library =
            unsafe { Library::new(config.witness_lib.clone()).expect("Failed to load library") };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library").expect("Failed to get symbol") };

        let mut witness_lib = witness_lib_constructor(
            config.verbose.into(),
            config.elf.clone(),
            config.asm.clone(),
            config.asm_rom.clone(),
            config.chunk_size_bits,
            Some(world_rank),
            Some(local_rank),
            base_port,
            unlock_mapped_memory,
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
            let _ = mpi_context; // avoid unused variable warning
            proofman = ProofMan::<Goldilocks>::new(
                config.proving_key.clone(),
                config.custom_commits_map.clone(),
                config.verify_constraints,
                config.aggregation,
                config.final_snark,
                config.gpu_params.clone(),
                config.verbose.into(),
            )
            .expect("Failed to initialize proofman");
        }

        proofman.register_witness(witness_lib.as_mut(), library);

        let witness_lib: Arc<dyn WitnessLibrary<Goldilocks> + Send + Sync> = Arc::from(witness_lib);

        Ok(Self {
            config: Arc::new(config),
            proofman: Arc::new(proofman),
            witness_lib,
            asm_services,
            is_busy: Arc::new(AtomicBool::new(false)),
            pending_handles: Vec::new(),
        })
    }

    pub fn print_waiting_message(config: &ServerConfig) {
        info_file!(
            "ZisK Server waiting for requests on port {} for ELF '{}'",
            config.port,
            config.elf.display()
        );
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", self.config.port))?;

        Self::print_waiting_message(&self.config);

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
                let response = ZiskResponse::ZiskInvalidRequestResponse {
                    base: ZiskBaseResponse {
                        cmd: "invalid_request".to_string(),
                        result: ZiskCmdResult::Error,
                        code: ZiskResultCode::InvalidRequest,
                        msg: Some(format!("Invalid request format or data. {e}")),
                        node: config.asm_runner_options.world_rank,
                    },
                };
                Self::send_json(&mut stream, &response)?;
                return Ok(false);
            }
        };

        info_file!("Received '{}' request", request);

        let mut must_shutdown = false;

        if self.is_busy.load(std::sync::atomic::Ordering::SeqCst)
            && !matches!(request, ZiskRequest::Status { .. })
        {
            let response = ZiskResponse::ZiskErrorResponse(ZiskBaseResponse {
                cmd: "busy".to_string(),
                result: ZiskCmdResult::InProgress,
                code: ZiskResultCode::Busy,
                msg: Some("Server is busy, please try again later.".to_string()),
                node: config.asm_runner_options.world_rank,
            });
            Self::send_json(&mut stream, &response)?;
            return Ok(false);
        }

        // Wait for all pending handles to finish
        for handle in self.pending_handles.drain(..) {
            handle.join().expect("Failed to join thread");
        }

        let (response, handle) = match request {
            ZiskRequest::Status { payload } => {
                let result =
                    ZiskServiceStatusHandler::handle(&config, payload, self.is_busy.clone());
                Self::print_waiting_message(&config);
                result
            }
            ZiskRequest::Shutdown { payload } => {
                must_shutdown = true;
                ZiskServiceShutdownHandler::handle(&config, payload, self.asm_services.as_ref())
            }
            ZiskRequest::VerifyConstraints { payload } => {
                ZiskServiceVerifyConstraintsHandler::handle(
                    config.clone(),
                    payload,
                    self.witness_lib.clone(),
                    self.proofman.clone(),
                    self.is_busy.clone(),
                    self.config.debug_info.clone(),
                )
            }

            ZiskRequest::Prove { payload } => ZiskServiceProveHandler::handle(
                config.clone(),
                payload,
                self.witness_lib.clone(),
                self.proofman.clone(),
                self.is_busy.clone(),
            ),
        };

        if let Some(handle) = handle {
            self.pending_handles.push(handle);
        }

        Self::send_json(&mut stream, &response)?;
        Ok(must_shutdown)
    }

    fn send_json(stream: &mut TcpStream, response: &ZiskResponse) -> std::io::Result<()> {
        let json = serde_json::to_string(response)?;
        stream.write_all(json.as_bytes())?;
        stream.flush()
    }
}
