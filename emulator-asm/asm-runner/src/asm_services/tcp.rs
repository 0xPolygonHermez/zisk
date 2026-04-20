use std::{
    io::{Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use tracing::debug;

use crate::AsmRunnerOptions;

use super::services::{build_service_command, decode_response, encode_request, AsmServices};
use super::{AsmService, FromResponsePayload, ToRequestPayload};

#[derive(Clone)]
pub(super) struct TcpTransport {
    pub(super) base_port: u16,
    pub(super) local_rank: i32,
    pub(super) world_rank: i32,
}

impl TcpTransport {
    pub(super) fn new(world_rank: i32, local_rank: i32, base_port: u16) -> Self {
        Self { base_port, local_rank, world_rank }
    }

    pub(super) fn start_services(
        &self,
        trimmed_path: &str,
        options: &mut AsmRunnerOptions,
        shm_prefix: &str,
    ) -> Result<()> {
        let mut pending_wait = Vec::new();

        for (i, service) in AsmServices::SERVICES.iter().enumerate() {
            let port = AsmServices::port_for(service, self.base_port, self.local_rank);
            debug!(">>> [{}] Starting ASM service: {} on port {}", self.world_rank, service, port);

            options.open_input_shmem = i != 0;
            start_service(service, trimmed_path, options, shm_prefix);

            if i == 0 {
                wait_for_service_ready(self.world_rank, service, port)?;
            } else {
                pending_wait.push((service, port));
            }
        }

        for (service, port) in pending_wait {
            wait_for_service_ready(self.world_rank, service, port)?;
        }

        Ok(())
    }

    pub(super) fn check_running(&self) -> Vec<AsmService> {
        AsmServices::SERVICES
            .iter()
            .filter(|s| {
                let port = AsmServices::port_for(s, self.base_port, self.local_rank);
                TcpStream::connect(format!("127.0.0.1:{port}")).is_ok()
            })
            .copied()
            .collect()
    }

    pub(super) fn send_request<Req, Res>(&self, service: &AsmService, req: &Req) -> Result<Res>
    where
        Req: ToRequestPayload,
        Res: FromResponsePayload,
    {
        let port = AsmServices::port_for(service, self.base_port, self.local_rank);
        let addr = format!("127.0.0.1:{port}");
        let out_buffer = encode_request(req.to_request_payload());

        let mut stream =
            TcpStream::connect(&addr).with_context(|| format!("Failed to connect to {addr}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .context("Failed to set read timeout")?;
        stream
            .write_all(&out_buffer)
            .with_context(|| format!("Failed to write request to {addr}"))?;

        let total_timeout = Duration::from_secs(120);
        let start = Instant::now();
        let mut in_buffer = [0u8; 40];

        loop {
            if start.elapsed() >= total_timeout {
                return Err(anyhow::anyhow!("Total timeout exceeded"));
            }
            match stream.read_exact(&mut in_buffer) {
                Ok(_) => break,
                Err(e)
                    if e.kind() == std::io::ErrorKind::TimedOut
                        || e.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    debug!("Read timeout after {:?}, retrying...", start.elapsed());
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(Res::from_response_payload(decode_response(&in_buffer)?))
    }
}

fn wait_for_service_ready(world_rank: i32, service: &AsmService, port: u16) -> Result<()> {
    const TIMEOUT: Duration = Duration::from_secs(60);
    const CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
    const LOG_INTERVAL: Duration = Duration::from_secs(5);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let start = Instant::now();
    let mut last_log = start;

    while start.elapsed() < TIMEOUT {
        match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
            Ok(_) => {
                debug!(">>> [{}] ASM service {} is ready on port {}", world_rank, service, port);
                return Ok(());
            }
            Err(_) => {
                if last_log.elapsed() >= LOG_INTERVAL {
                    debug!(
                        ">>> [{}] Waiting for ASM service {} on port {} ({:.0}s elapsed), retrying...",
                        world_rank,
                        service,
                        port,
                        start.elapsed().as_secs_f32()
                    );
                    last_log = Instant::now();
                }
            }
        }
    }

    tracing::error!(
        ">>> [{}] Timeout waiting for ASM service {} to be ready on port {} after {:?}",
        world_rank,
        service,
        port,
        start.elapsed()
    );
    Err(anyhow::anyhow!("Timeout: service `{service}` not ready on {addr}"))
}

fn start_service(
    asm_service: &AsmService,
    trimmed_path: &str,
    options: &AsmRunnerOptions,
    shm_prefix: &str,
) {
    let mut command = build_service_command(asm_service, trimmed_path, options, shm_prefix);
    if let Err(e) = command.spawn() {
        tracing::error!("Child process failed: {:?}", e);
    }
}
