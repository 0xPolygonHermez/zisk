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
use proofman_common::{initialize_logger, DebugInfo, ParamsGPU};
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;
use zisk_common::{info_file, ZiskLib, ZiskLibInitFn};

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

pub struct ZiskServerParams {
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

    pub asm_port: Option<u16>,

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

    /// Time when the server was launched
    pub launch_time: Instant,

    /// Unique identifier for the server instance
    pub server_id: Uuid,

    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,

    pub gpu_params: ParamsGPU,

    pub unlock_mapped_memory: bool,

    pub shared_tables: bool,
}

#[allow(clippy::too_many_arguments)]
impl ZiskServerParams {
    pub fn new(
        port: u16,
        elf: PathBuf,
        witness_lib: PathBuf,
        asm: Option<PathBuf>,
        asm_rom: Option<PathBuf>,
        asm_port: Option<u16>,
        custom_commits_map: HashMap<String, PathBuf>,
        emulator: bool,
        proving_key: PathBuf,
        verbose: u8,
        debug: DebugInfo,
        verify_constraints: bool,
        aggregation: bool,
        final_snark: bool,
        gpu_params: ParamsGPU,
        unlock_mapped_memory: bool,
        shared_tables: bool,
    ) -> Self {
        Self {
            port,
            elf,
            witness_lib,
            asm,
            asm_rom,
            asm_port,
            custom_commits_map,
            emulator,
            proving_key,
            verbose,
            debug_info: debug,
            launch_time: Instant::now(),
            server_id: Uuid::new_v4(),
            verify_constraints,
            aggregation,
            final_snark,
            gpu_params,
            unlock_mapped_memory,
            shared_tables,
        }
    }
}

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

    /// Additional options for the ASM runner
    pub asm_runner_options: AsmRunnerOptions,

    pub verify_constraints: bool,
    pub aggregation: bool,
    pub final_snark: bool,

    pub gpu_params: ParamsGPU,

    pub shared_tables: bool,
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
        asm_runner_options: AsmRunnerOptions,
        verify_constraints: bool,
        aggregation: bool,
        final_snark: bool,
        gpu_params: ParamsGPU,
        shared_tables: bool,
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
            asm_runner_options,
            verify_constraints,
            aggregation,
            final_snark,
            gpu_params,
            shared_tables,
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
    witness_lib: Arc<Box<dyn ZiskLib<Goldilocks>>>,
    proofman: Arc<ProofMan<Goldilocks>>,
    asm_services: Option<AsmServices>,
    is_busy: Arc<AtomicBool>,
    pending_handles: Vec<std::thread::JoinHandle<()>>,
}

impl ZiskService {
    pub fn new(params: &ZiskServerParams) -> Result<Self> {
        info_file!("Starting asm microservices...");
        let library =
            unsafe { Library::new(params.witness_lib.clone()).expect("Failed to load library") };
        let witness_lib_constructor: Symbol<ZiskLibInitFn<Goldilocks>> =
            unsafe { library.get(b"init_library").expect("Failed to get symbol") };

        let unlock_mapped_memory = params.unlock_mapped_memory;

        let mut witness_lib = witness_lib_constructor(
            params.verbose.into(),
            params.elf.clone(),
            params.asm.clone(),
            params.asm_rom.clone(),
            params.asm_port,
            unlock_mapped_memory,
            params.shared_tables,
        )
        .expect("Failed to initialize witness library");

        let proofman = ProofMan::<Goldilocks>::new(
            params.proving_key.clone(),
            params.custom_commits_map.clone(),
            params.verify_constraints,
            params.aggregation,
            params.final_snark,
            params.gpu_params.clone(),
            params.verbose.into(),
            witness_lib.get_packed_info(),
        )
        .expect("Failed to initialize proofman");

        let world_rank = proofman.get_world_rank();
        let local_rank = proofman.get_local_rank();

        initialize_logger(params.verbose.into(), Some(world_rank));

        let port = params.port + local_rank as u16;

        let asm_runner_options = AsmRunnerOptions::new()
            .with_verbose(params.verbose > 0)
            .with_base_port(params.asm_port)
            .with_world_rank(world_rank)
            .with_local_rank(local_rank)
            .with_unlock_mapped_memory(params.unlock_mapped_memory);

        let asm_services = if params.emulator {
            None
        } else {
            let asm_services = AsmServices::new(world_rank, local_rank, params.asm_port);
            asm_services
                .start_asm_services(params.asm.as_ref().unwrap(), asm_runner_options.clone())?;
            Some(asm_services)
        };

        proofman.register_witness(witness_lib.as_mut(), library);

        let witness_lib = Arc::new(witness_lib);

        let config = ServerConfig::new(
            port,
            params.elf.clone(),
            params.witness_lib.clone(),
            params.asm.clone(),
            params.asm_rom.clone(),
            params.custom_commits_map.clone(),
            params.emulator,
            params.proving_key.clone(),
            params.verbose,
            params.debug_info.clone(),
            asm_runner_options,
            params.verify_constraints,
            params.aggregation,
            params.final_snark,
            params.gpu_params.clone(),
            params.shared_tables,
        );

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
        if self.proofman.rank() == Some(0) || self.proofman.rank().is_none() {
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
        } else {
            // Other MPI ranks just wait for rank 0 instructions
            loop {
                self.receive_request()?;
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
                let mut bytes = Vec::new();
                bytes.push(0u8); // option 0 for verify constraints
                let serialized: Vec<u8> =
                    serde_json::to_vec(&payload).expect("Failed to serialize payload");
                bytes.extend_from_slice(&serialized);
                self.proofman.mpi_broadcast(&mut bytes);

                ZiskServiceVerifyConstraintsHandler::handle(
                    config.clone(),
                    payload,
                    self.witness_lib.clone(),
                    self.proofman.clone(),
                    self.is_busy.clone(),
                    self.config.debug_info.clone(),
                )
            }
            ZiskRequest::Prove { payload } => {
                let mut bytes = Vec::new();
                bytes.push(1u8); // option 1 for prove
                let serialized: Vec<u8> =
                    serde_json::to_vec(&payload).expect("Failed to serialize payload");
                bytes.extend_from_slice(&serialized);
                self.proofman.mpi_broadcast(&mut bytes);

                ZiskServiceProveHandler::handle(
                    config.clone(),
                    payload,
                    self.witness_lib.clone(),
                    self.proofman.clone(),
                    self.is_busy.clone(),
                )
            }
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

    fn receive_request(&self) -> std::io::Result<()> {
        let mut bytes: Vec<u8> = Vec::new();
        self.proofman.mpi_broadcast(&mut bytes);

        // extract byte 0 to decide the option
        let option = bytes.first().cloned();
        match option {
            Some(0) => {
                info_file!("Received process 'VerifyConstraints' request");
                // Deserialize the rest of bytes into ZiskVerifyConstraintsRequest
                let payload: ZiskVerifyConstraintsRequest =
                    serde_json::from_slice(&bytes[1..]).expect("Failed to deserialize payload");
                ZiskServiceVerifyConstraintsHandler::process_handle(
                    payload,
                    self.proofman.clone(),
                    self.config.debug_info.clone(),
                );
            }
            Some(1) => {
                info_file!("Received process 'Prove' request");
                // Prove request
                // Deserialize the rest of bytes into ZiskProveRequest
                let payload: ZiskProveRequest =
                    serde_json::from_slice(&bytes[1..]).expect("Failed to deserialize payload");
                ZiskServiceProveHandler::process_handle(payload, self.proofman.clone());
            }
            _ => {
                info_file!(
                    "Rank {} received unknown request: {:?}",
                    self.proofman.rank().unwrap_or(0),
                    option
                );
            }
        }
        Ok(())
    }
}
