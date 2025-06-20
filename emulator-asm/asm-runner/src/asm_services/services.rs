use super::{
    FromResponsePayload, MemoryOperationsRequest, MemoryOperationsResponse, MinimalTraceRequest,
    MinimalTraceResponse, PingRequest, PingResponse, ResponseData, ShutdownRequest,
    ShutdownResponse, ToRequestPayload,
};
use crate::{AsmRunError, AsmRunnerOptions, AsmRunnerTraceLevel};
use anyhow::{Context, Result};
use named_sem::NamedSemaphore;
use std::{
    fmt,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    process::Command,
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsmService {
    MT,
    RH,
    MO,
}

impl AsmService {
    pub fn as_str(&self) -> &'static str {
        match self {
            AsmService::MT => "MT",
            AsmService::RH => "RH",
            AsmService::MO => "MO",
        }
    }
}

impl fmt::Display for AsmService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AsmService::MT => "mt",
            AsmService::RH => "rh",
            AsmService::MO => "mo",
        };
        write!(f, "{}", s)
    }
}

const ASM_SERVICE_BASE_PORT: u16 = 23115;

pub struct AsmServices {
    world_rank: i32,
    local_rank: i32,
    base_port: u16,
}

impl AsmServices {
    const MO_SERVICE_OFFSET: u64 = 0; // Relative offset to base port. Should correspond to the order in SERVICES
    const MT_SERVICE_OFFSET: u64 = 1; // Relative offset to base port. Should correspond to the order in SERVICES
    const RH_SERVICE_OFFSET: u64 = 2; // Relative offset to base port. Should correspond to the order in SERVICES

    const SERVICES: [AsmService; 2] = [
        AsmService::MO,
        AsmService::MT,
        // AsmService::RH,
    ];

    pub fn new(world_rank: i32, local_rank: i32, port: Option<u16>) -> Self {
        Self { world_rank, local_rank, base_port: port.unwrap_or(ASM_SERVICE_BASE_PORT) }
    }

    pub fn shmem_prefix(&self) -> String {
        format!("ZISK_{}_{}", self.base_port, self.local_rank)
    }

    pub fn start_asm_services(
        &self,
        ziskemuasm_path: &Path,
        options: AsmRunnerOptions,
    ) -> Result<()> {
        // ! TODO Remove this when we have a proper way to find the path
        let path_str = ziskemuasm_path.to_string_lossy();
        let trimmed_path = &path_str[..path_str.len().saturating_sub(7)];

        // Check if a service is already running
        for service in &Self::SERVICES {
            let port = self.port_for(service);
            let addr = format!("127.0.0.1:{}", port);

            if TcpStream::connect(&addr).is_ok() {
                tracing::info!(
                    "Service {} is already running on {}. Shutting it down.",
                    service,
                    addr
                );
                let sem_chunk_done_name =
                    format!("/{}_{}_shutdown_done", self.shmem_prefix(), service.as_str());
                let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
                    .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

                self.send_shutdown_request(service).with_context(|| {
                    format!("Service {} failed to respond to shutdown", service)
                })?;

                match sem_chunk_done.timed_wait(Duration::from_secs(30)) {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!("[{}] Failed to wait for shutdown: {}", self.world_rank, e);
                        return Err(AsmRunError::SemaphoreError(sem_chunk_done_name, e).into());
                    }
                }
            }
        }

        let start = std::time::Instant::now();

        for service in &Self::SERVICES {
            self.start_asm_service(service, trimmed_path, &options);
        }

        for service in &Self::SERVICES {
            Self::wait_for_service_ready(service, self.port_for(service));
        }

        // Ping status for all services
        for service in &Self::SERVICES {
            self.send_status_request(service)
                .with_context(|| format!("Service {} failed to respond to ping", service))?;
        }

        tracing::info!(
            ">>> [{}] All ASM services are ready. Time taken: {} seconds",
            self.world_rank,
            start.elapsed().as_secs_f32()
        );

        Ok(())
    }

    pub fn stop_asm_services(&self) -> Result<()> {
        // Check if a service is already running
        for service in &Self::SERVICES {
            let port = self.port_for(service);
            let addr = format!("127.0.0.1:{}", port);

            if TcpStream::connect(&addr).is_ok() {
                tracing::info!("Shutting down service {} running on {}.", service, addr);

                self.send_shutdown_request(service).with_context(|| {
                    format!("Service {} failed to respond to shutdown", service)
                })?;
            }
        }

        Ok(())
    }

    fn wait_for_service_ready(service: &AsmService, port: u16) {
        let addr = format!("127.0.0.1:{}", port);
        let timeout = Duration::from_secs(60);
        let retry_delay = Duration::from_millis(100);
        let start = Instant::now();

        while start.elapsed() < timeout {
            match TcpStream::connect(&addr) {
                Ok(_) => {
                    return;
                }
                Err(_) => sleep(retry_delay),
            }
        }

        panic!("Timeout: service `{}` not ready on {}", service, addr);
    }

    fn start_asm_service(
        &self,
        asm_service: &AsmService,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
    ) {
        // Prepare command
        let command_path = trimmed_path.to_string() + &format!("-{}.bin", asm_service);

        let mut command = Command::new(command_path);

        command.arg("-p").arg(self.port_for(asm_service).to_string());

        command.arg("--shm_prefix").arg(self.shmem_prefix());

        match asm_service {
            AsmService::MT => {
                command.arg("--generate_minimal_trace");
            }
            AsmService::RH => {
                command.arg("--generate_rom_histogram");
            }
            AsmService::MO => {
                command.arg("--generate_mem_op");
            }
        }

        command.arg("-s");

        // command.stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit());

        if !options.log_output {
            command.arg("-o");
            command.stdout(std::process::Stdio::null());
            command.stderr(std::process::Stdio::null());
        }
        if options.metrics {
            command.arg("-m");
        }
        if options.verbose {
            command.arg("-v");
        }
        match options.trace_level {
            AsmRunnerTraceLevel::None => {}
            AsmRunnerTraceLevel::Trace => {
                command.arg("-t");
            }
            AsmRunnerTraceLevel::ExtendedTrace => {
                command.arg("-tt");
            }
        }
        if options.keccak_trace {
            command.arg("-k");
        }

        if let Err(e) = command.spawn() {
            tracing::error!("Child process failed: {:?}", e);
        } else if options.verbose || options.log_output {
            tracing::info!("Child process launched successfully");
        }
    }

    const fn port_for(&self, asm_service: &AsmService) -> u16 {
        let rank_offset = self.local_rank as u16 * Self::SERVICES.len() as u16;

        let service_offset = match asm_service {
            AsmService::MT => Self::MT_SERVICE_OFFSET,
            AsmService::RH => Self::RH_SERVICE_OFFSET,
            AsmService::MO => Self::MO_SERVICE_OFFSET,
        };

        self.base_port + service_offset as u16 + rank_offset
    }

    pub fn send_status_request(&self, service: &AsmService) -> Result<PingResponse> {
        self.send_request(service, &PingRequest {})
    }

    pub fn send_shutdown_request(&self, service: &AsmService) -> Result<ShutdownResponse> {
        self.send_request(service, &ShutdownRequest {})
    }

    pub fn send_minimal_trace_request(
        &self,
        max_steps: u64,
        chunk_len: u64,
    ) -> Result<MinimalTraceResponse> {
        self.send_request(&AsmService::MT, &MinimalTraceRequest { max_steps, chunk_len })
    }

    pub fn send_memory_ops_request(
        &self,
        max_steps: u64,
        chunk_len: u64,
    ) -> Result<MemoryOperationsResponse> {
        self.send_request(&AsmService::MO, &MemoryOperationsRequest { max_steps, chunk_len })
    }

    fn send_request<Req, Res>(&self, service: &AsmService, req: &Req) -> Result<Res>
    where
        Req: ToRequestPayload,
        Res: FromResponsePayload,
    {
        let port = self.port_for(service);
        let addr = format!("127.0.0.1:{}", port);

        let request = req.to_request_payload();

        // Encode RequestData as bytes
        let mut out_buffer = Vec::with_capacity(40);
        for word in request {
            out_buffer.extend_from_slice(&word.to_le_bytes());
        }

        let mut stream =
            TcpStream::connect(&addr).with_context(|| format!("Failed to connect to {}", addr))?;

        // Set a read timeout to avoid indefinite blocking
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .context("Failed to set read timeout")?;

        // Send request payload
        stream.write_all(&out_buffer).context("Failed to write request payload")?;

        // Read exactly 40 bytes
        let mut in_buffer = [0u8; 40];
        stream.read_exact(&mut in_buffer).context("Failed to read full response payload")?;

        // Decode bytes into ResponseData
        let mut response = ResponseData::default();
        for (i, chunk) in in_buffer.chunks_exact(8).enumerate() {
            response[i] = u64::from_le_bytes(chunk.try_into()?);
        }

        Ok(Res::from_response_payload(response))
    }
}
