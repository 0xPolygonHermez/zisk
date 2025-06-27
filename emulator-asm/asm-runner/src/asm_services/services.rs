use super::{
    FromResponsePayload, MemoryOperationsRequest, MemoryOperationsResponse, MinimalTraceRequest,
    MinimalTraceResponse, PingRequest, PingResponse, ResponseData, ShutdownRequest,
    ShutdownResponse, ToRequestPayload,
};
use crate::{AsmRunError, AsmRunnerOptions, RomHistogramRequest, RomHistogramResponse};
use anyhow::{Context, Result};
use libc::sem_unlink;
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
    MO,
    MT,
    RH,
}

impl AsmService {
    pub fn as_str(&self) -> &'static str {
        match self {
            AsmService::MO => "MO",
            AsmService::MT => "MT",
            AsmService::RH => "RH",
        }
    }
}

impl fmt::Display for AsmService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            AsmService::MO => "mo",
            AsmService::MT => "mt",
            AsmService::RH => "rh",
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

    const SERVICES: [AsmService; 3] = [AsmService::MO, AsmService::MT, AsmService::RH];

    pub fn new(world_rank: i32, local_rank: i32, base_port: Option<u16>) -> Self {
        Self { world_rank, local_rank, base_port: base_port.unwrap_or(ASM_SERVICE_BASE_PORT) }
    }

    pub fn shmem_prefix(
        asm_service: &AsmService,
        base_port: Option<u16>,
        local_rank: i32,
    ) -> String {
        format!(
            "ZISK_{}_{}",
            Self::port_for(
                asm_service,
                base_port.unwrap_or(AsmServices::default_port(&AsmService::MO, local_rank)),
                local_rank
            ),
            local_rank
        )
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
            let port = Self::port_for(service, self.base_port, self.local_rank);
            let addr = format!("127.0.0.1:{}", port);

            if TcpStream::connect(&addr).is_ok() {
                tracing::info!(
                    "Service {} is already running on {}. Shutting it down.",
                    service,
                    addr
                );

                let _ = self.send_shutdown_and_wait(service);
            }
        }

        let start = std::time::Instant::now();

        for service in &Self::SERVICES {
            self.start_asm_service(service, trimmed_path, &options);
        }

        for service in &Self::SERVICES {
            Self::wait_for_service_ready(
                service,
                Self::port_for(service, self.base_port, self.local_rank),
            );
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
            let port = Self::port_for(service, self.base_port, self.local_rank);
            let addr = format!("127.0.0.1:{}", port);

            if TcpStream::connect(&addr).is_ok() {
                tracing::info!("Shutting down service {} running on {}.", service, addr);

                let _ = self.send_shutdown_and_wait(service);
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

        options.apply_to_command(&mut command, asm_service);

        if let Err(e) = command.spawn() {
            tracing::error!("Child process failed: {:?}", e);
        } else if options.verbose || options.log_output {
            tracing::info!("Child process launched successfully");
        }
    }

    pub const fn default_port(asm_service: &AsmService, local_rank: i32) -> u16 {
        let rank_offset = local_rank as u16 * Self::SERVICES.len() as u16;

        let service_offset = match asm_service {
            AsmService::MT => Self::MT_SERVICE_OFFSET,
            AsmService::RH => Self::RH_SERVICE_OFFSET,
            AsmService::MO => Self::MO_SERVICE_OFFSET,
        };
        ASM_SERVICE_BASE_PORT + service_offset as u16 + rank_offset
    }

    pub const fn port_for(asm_service: &AsmService, base_port: u16, local_rank: i32) -> u16 {
        let rank_offset = local_rank as u16 * Self::SERVICES.len() as u16;

        let service_offset = match asm_service {
            AsmService::MT => Self::MT_SERVICE_OFFSET,
            AsmService::RH => Self::RH_SERVICE_OFFSET,
            AsmService::MO => Self::MO_SERVICE_OFFSET,
        };

        base_port + service_offset as u16 + rank_offset
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

    pub fn send_rom_histogram_request(
        &self,
        max_steps: u64,
    ) -> Result<RomHistogramResponse> {
        self.send_request(&AsmService::RH, &RomHistogramRequest { max_steps })
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
        let port = Self::port_for(service, self.base_port, self.local_rank);
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

    pub fn send_shutdown_and_wait(&self, service: &AsmService) -> Result<()> {
        let sem_shutdown_done_name = format!(
            "/{}_{}_shutdown_done",
            Self::shmem_prefix(service, Some(self.base_port), self.local_rank),
            service.as_str()
        );
        let mut sem_shutdown_done = NamedSemaphore::create(sem_shutdown_done_name.clone(), 0)
            .map_err(|e| AsmRunError::SemaphoreError(sem_shutdown_done_name.clone(), e))?;

        let _ = sem_shutdown_done.try_wait();

        self.send_shutdown_request(service)
            .with_context(|| format!("Service {} failed to respond to shutdown", service))?;

        tracing::info!("Waiting for semaphore {sem_shutdown_done_name}");

        match sem_shutdown_done.timed_wait(Duration::from_secs(30)) {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("[{}] Failed to wait for shutdown: {}", self.world_rank, e);
                return Err(AsmRunError::SemaphoreError(sem_shutdown_done_name, e).into());
            }
        }

        tracing::info!("Done waiting for semaphore {sem_shutdown_done_name}");

        unsafe {
            sem_unlink(sem_shutdown_done_name.as_ptr() as *const i8);
        }

        Ok(())
    }
}
