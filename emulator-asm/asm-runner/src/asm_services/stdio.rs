use std::{
    io::{Read, Write},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tracing::debug;

use crate::{
    AsmRunnerOptions, MemoryOperationsRequest, MemoryOperationsResponse, MinimalTraceRequest,
    MinimalTraceResponse, RomHistogramRequest, RomHistogramResponse, ShutdownRequest,
    ShutdownResponse,
};

use super::services::AsmServices;
use super::{AsmService, FromResponsePayload, PingRequest, PingResponse, ToRequestPayload};

pub(super) struct StdioHandle {
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Child,
}

#[derive(Clone)]
pub(super) struct StdioService {
    pub(super) state: Arc<[Mutex<Option<StdioHandle>>; 3]>,
    pub(super) world_rank: i32,
    pub(super) local_rank: i32,
}

impl StdioService {
    pub(super) fn new(world_rank: i32, local_rank: i32) -> Self {
        Self { state: Arc::new(std::array::from_fn(|_| Mutex::new(None))), world_rank, local_rank }
    }

    pub(super) fn start_services(
        &self,
        trimmed_path: &str,
        options: &mut AsmRunnerOptions,
        shm_prefix: &str,
        sem_prefix: &str,
    ) -> Result<()> {
        for (i, service) in AsmServices::SERVICES.iter().enumerate() {
            debug!(">>> [{}] Starting ASM service (stdio): {}", self.world_rank, service);

            options.open_input_shmem = i != 0;
            let handle =
                Self::start_service(service, trimmed_path, options, shm_prefix, sem_prefix)?;
            *self.state[i].lock().unwrap() = Some(handle);

            // If this is the first service (MO), ping it to ensure it's ready before starting the others.
            if i == 0 {
                self.send_request::<PingRequest, PingResponse>(service, &PingRequest {})
                    .with_context(|| {
                        format!("Service {service} did not respond to stdio readiness ping")
                    })?;
            }
        }

        Ok(())
    }

    fn start_service(
        asm_service: &AsmService,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
        shm_prefix: &str,
        sem_prefix: &str,
    ) -> Result<StdioHandle> {
        let mut command =
            asm_service.build_service_command(trimmed_path, options, shm_prefix, sem_prefix);
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .with_context(|| format!("Failed to spawn stdio service {asm_service}"))?;

        let stdin = child.stdin.take().context("Failed to open stdin for stdio service")?;
        let stdout = child.stdout.take().context("Failed to open stdout for stdio service")?;
        let stderr = child.stderr.take().context("Failed to open stderr for stdio service")?;

        Ok(StdioHandle { stdin, stdout, stderr, child })
    }

    pub(super) fn running_services(&self) -> Vec<AsmService> {
        AsmServices::SERVICES
            .iter()
            .filter(|s| {
                let mut guard = self.state[s.as_index()].lock().unwrap();
                guard.as_mut().is_some_and(|h| matches!(h.child.try_wait(), Ok(None) | Err(_)))
            })
            .copied()
            .collect()
    }

    pub(super) fn send_status_request(&self, service: &AsmService) -> Result<PingResponse> {
        self.send_request(service, &PingRequest {})
    }

    pub(super) fn send_shutdown_request(&self, service: &AsmService) -> Result<ShutdownResponse> {
        self.send_request(service, &ShutdownRequest {})
    }

    pub(crate) fn send_minimal_trace_request(
        &self,
        max_steps: u64,
        chunk_len: u64,
    ) -> Result<MinimalTraceResponse> {
        self.send_request(&AsmService::MT, &MinimalTraceRequest { max_steps, chunk_len })
    }

    pub(crate) fn send_rom_histogram_request(
        &self,
        max_steps: u64,
    ) -> Result<RomHistogramResponse> {
        self.send_request(&AsmService::RH, &RomHistogramRequest { max_steps })
    }

    pub(crate) fn send_memory_ops_request(
        &self,
        max_steps: u64,
        chunk_len: u64,
    ) -> Result<MemoryOperationsResponse> {
        self.send_request(&AsmService::MO, &MemoryOperationsRequest { max_steps, chunk_len })
    }

    pub(super) fn send_request<Req, Res>(&self, service: &AsmService, req: &Req) -> Result<Res>
    where
        Req: ToRequestPayload,
        Res: FromResponsePayload,
    {
        let mut guard = self.state[service.as_index()].lock().unwrap();
        let handle =
            guard.as_mut().expect("stdio handle not initialized; call start_services first");

        let out_buffer = AsmServices::encode_request(req.to_request_payload());
        handle
            .stdin
            .write_all(&out_buffer)
            .with_context(|| format!("Failed to write request to stdio service {service}"))?;

        let mut in_buffer = [0u8; 40];
        if let Err(e) = handle.stdout.read_exact(&mut in_buffer) {
            // Give the process a moment to fully exit if it hasn't yet
            let status = match handle.child.try_wait() {
                Ok(Some(status)) => Some(status),
                Ok(None) => handle.child.wait().ok(), // Process may still be exiting; wait briefly
                Err(_) => None,
            };

            if let Some(status) = status {
                let stderr_output = {
                    let mut buf = Vec::new();
                    handle.stderr.read_to_end(&mut buf).ok();
                    String::from_utf8(buf).unwrap_or_default()
                };
                let stderr_snippet = if stderr_output.is_empty() {
                    String::from("(no stderr captured)")
                } else {
                    // Take last 2000 chars to avoid huge messages
                    let start = stderr_output.len().saturating_sub(2000);
                    stderr_output[start..].to_string()
                };
                tracing::error!(
                    "Service {service} process crashed with {status}.\nstderr:\n{stderr_snippet}"
                );
                return Err(anyhow::anyhow!(
                    "Service {service} process exited with {status} before responding.\nstderr:\n{stderr_snippet}"
                ));
            }
            return Err(e)
                .with_context(|| format!("Failed to read response from stdio service {service}"));
        }

        Ok(Res::from_response_payload(AsmServices::decode_response(&in_buffer)?))
    }
}
