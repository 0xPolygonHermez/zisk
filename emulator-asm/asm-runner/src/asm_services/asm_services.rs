use super::{
    FromResponsePayload, MinimalTraceRequest, MinimalTraceResponse, PingRequest, PingResponse,
    ResponseData, ShutdownRequest, ShutdownResponse, ToRequestPayload,
};
use crate::{AsmRunnerOptions, AsmRunnerTraceLevel};
use anyhow::{Context, Result};
use std::{
    fmt,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    process::{self, Command},
    thread::sleep,
    time::{Duration, Instant},
};

pub enum AsmService {
    MT,
    RH,
    MO,
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
    pub fn start_asm_services(ziskemuasm_path: &Path, options: AsmRunnerOptions) -> Result<()> {
        // ! TODO Remove this when we have a proper way to find the path
        let path_str = ziskemuasm_path.to_string_lossy();
        let trimmed_path = &path_str[..path_str.len().saturating_sub(7)];

        let services = [
            AsmService::MT,
            // AsmService::RH,
            // AsmService::MO,
        ];

        // Check if a service is already running
        for service in &services {
            let port = Self::port_for(service);
            let addr = format!("127.0.0.1:{}", port);

            match TcpStream::connect(&addr) {
                Ok(_) => {
                    tracing::info!(
                        "Service {} is already running on {}. Shutting it down.",
                        service,
                        addr
                    );
                    Self::send_shutdown_request(service).with_context(|| {
                        format!("Service {} failed to respond to shutdown", service)
                    })?;
                }
                Err(_) => {}
            }
        }

        let start = std::time::Instant::now();

        for service in &services {
            Self::start_asm_service(service, trimmed_path, &options);
        }

        for service in &services {
            let port = Self::port_for(service);
            Self::wait_for_service_ready(service, port);
        }

        // Ping status for all services
        for service in &services {
            Self::send_status_request(service)
                .with_context(|| format!("Service {} failed to respond to ping", service))?;
        }

        tracing::info!(
            "All ASM services are ready. Time taken: {} seconds",
            start.elapsed().as_secs_f32()
        );

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

    fn start_asm_service(asm_service: &AsmService, trimmed_path: &str, options: &AsmRunnerOptions) {
        // Prepare command
        let command_path = trimmed_path.to_string() + &format!("-{}.bin", asm_service);

        let mut command = Command::new(command_path);

        match asm_service {
            AsmService::MT => {
                command.arg("--generate_minimal_trace");
            }
            AsmService::RH => {
                command.arg("--generate_rom_histogram");
            }
            AsmService::MO => {
                unimplemented!("MO service is not implemented yet");
            }
        }

        command.arg("-s");

        // command.stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit());

        if !options.log_output {
            command.arg("-o");
            command.stdout(process::Stdio::null());
            command.stderr(process::Stdio::null());
        }
        if options.metrics {
            command.arg("-m");
        }
        // if options.verbose {
        command.arg("-v");
        // }
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

    const fn port_for(asm_service: &AsmService) -> u16 {
        match asm_service {
            AsmService::MT => MT_ASM_SERVICE_DEFAULT_PORT,
            AsmService::RH => RH_ASM_SERVICE_DEFAULT_PORT,
            AsmService::MO => MO_ASM_SERVICE_DEFAULT_PORT,
        }
    }

    pub fn send_status_request(service: &AsmService) -> Result<PingResponse> {
        Self::send_request(service, &PingRequest {})
    }

    pub fn send_shutdown_request(service: &AsmService) -> Result<ShutdownResponse> {
        Self::send_request(service, &ShutdownRequest {})
    }

    pub fn send_minimal_trace_request(
        max_steps: u64,
        chunk_len: u64,
    ) -> Result<MinimalTraceResponse> {
        Self::send_request(&AsmService::MT, &MinimalTraceRequest { max_steps, chunk_len })
    }

    fn send_request<Req, Res>(service: &AsmService, req: &Req) -> Result<Res>
    where
        Req: ToRequestPayload,
        Res: FromResponsePayload,
    {
        let port = Self::port_for(service);
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
