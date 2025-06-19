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

const MT_ASM_SERVICE_DEFAULT_PORT: u16 = 23115;
const RH_ASM_SERVICE_DEFAULT_PORT: u16 = 23116;
const MO_ASM_SERVICE_DEFAULT_PORT: u16 = 23117;

pub struct AsmServices;

impl AsmServices {
    const SERVICES: [AsmService; 2] = [
        AsmService::MO,
        AsmService::MT,
        // AsmService::RH,
    ];

    pub fn start_asm_services(
        ziskemuasm_path: &Path,
        options: AsmRunnerOptions,
        world_rank: i32,
        local_rank: i32,
    ) -> Result<()> {
        // ! TODO Remove this when we have a proper way to find the path
        let path_str = ziskemuasm_path.to_string_lossy();
        let trimmed_path = &path_str[..path_str.len().saturating_sub(7)];

        // Check if a service is already running
        for service in &Self::SERVICES {
            let port = Self::port_for(service, local_rank);
            let addr = format!("127.0.0.1:{}", port);

            if TcpStream::connect(&addr).is_ok() {
                tracing::info!(
                    "Service {} is already running on {}. Shutting it down.",
                    service,
                    addr
                );
                let sem_chunk_done_name =
                    format!("/ZISK_{}_{}_shutdown_done", local_rank, service.as_str());
                let mut sem_chunk_done = NamedSemaphore::create(sem_chunk_done_name.clone(), 0)
                    .map_err(|e| AsmRunError::SemaphoreError(sem_chunk_done_name.clone(), e))?;

                Self::send_shutdown_request(service, local_rank).with_context(|| {
                    format!("Service {} failed to respond to shutdown", service)
                })?;

                match sem_chunk_done.timed_wait(Duration::from_secs(30)) {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!("[{}] Failed to wait for shutdown: {}", world_rank, e);
                        return Err(AsmRunError::SemaphoreError(sem_chunk_done_name, e).into());
                    }
                }
            }
        }

        let start = std::time::Instant::now();

        for service in &Self::SERVICES {
            Self::start_asm_service(service, trimmed_path, &options, local_rank);
        }

        for service in &Self::SERVICES {
            let port = Self::port_for(service, local_rank);
            Self::wait_for_service_ready(service, port);
        }

        // Ping status for all services
        for service in &Self::SERVICES {
            Self::send_status_request(service, local_rank)
                .with_context(|| format!("Service {} failed to respond to ping", service))?;
        }

        tracing::info!(
            ">>> [{}] All ASM services are ready. Time taken: {} seconds",
            world_rank,
            start.elapsed().as_secs_f32()
        );

        Ok(())
    }

    pub fn stop_asm_services(local_rank: i32) -> Result<()> {
        // Check if a service is already running
        for service in &Self::SERVICES {
            let port = Self::port_for(service, local_rank);
            let addr = format!("127.0.0.1:{}", port);

            if TcpStream::connect(&addr).is_ok() {
                tracing::info!("Shutting down service {} running on {}.", service, addr);

                Self::send_shutdown_request(service, local_rank).with_context(|| {
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
        asm_service: &AsmService,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
        local_rank: i32,
    ) {
        // Prepare command
        let command_path = trimmed_path.to_string() + &format!("-{}.bin", asm_service);

        let mut command = Command::new(command_path);

        command.arg("-p").arg(Self::port_for(asm_service, local_rank).to_string());

        let prefix = format!("ZISK_{}", local_rank);
        command.arg("--shm_prefix").arg(prefix);

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

    const fn port_for(asm_service: &AsmService, local_rank: i32) -> u16 {
        let offset = local_rank as u16 * Self::SERVICES.len() as u16;

        match asm_service {
            AsmService::MT => MT_ASM_SERVICE_DEFAULT_PORT + offset,
            AsmService::RH => RH_ASM_SERVICE_DEFAULT_PORT + offset,
            AsmService::MO => MO_ASM_SERVICE_DEFAULT_PORT + offset,
        }
    }

    pub fn send_status_request(service: &AsmService, local_rank: i32) -> Result<PingResponse> {
        Self::send_request(service, &PingRequest {}, local_rank)
    }

    pub fn send_shutdown_request(
        service: &AsmService,
        local_rank: i32,
    ) -> Result<ShutdownResponse> {
        Self::send_request(service, &ShutdownRequest {}, local_rank)
    }

    pub fn send_minimal_trace_request(
        max_steps: u64,
        chunk_len: u64,
        local_rank: i32,
    ) -> Result<MinimalTraceResponse> {
        Self::send_request(
            &AsmService::MT,
            &MinimalTraceRequest { max_steps, chunk_len },
            local_rank,
        )
    }

    pub fn send_memory_ops_request(
        max_steps: u64,
        chunk_len: u64,
        local_rank: i32,
    ) -> Result<MemoryOperationsResponse> {
        Self::send_request(
            &AsmService::MO,
            &MemoryOperationsRequest { max_steps, chunk_len },
            local_rank,
        )
    }

    fn send_request<Req, Res>(service: &AsmService, req: &Req, local_rank: i32) -> Result<Res>
    where
        Req: ToRequestPayload,
        Res: FromResponsePayload,
    {
        let port = Self::port_for(service, local_rank);
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
