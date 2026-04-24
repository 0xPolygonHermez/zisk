use std::{
    io::{Read, Write},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tracing::debug;

use crate::AsmRunnerOptions;

use super::services::{build_service_command, decode_response, encode_request, AsmServices};
use super::{AsmService, FromResponsePayload, PingRequest, PingResponse, ToRequestPayload};

pub(super) struct StdioHandle {
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
    child: Child,
}

pub(super) struct StdioState {
    /// Indexed by service offset: MO=0, MT=1, RH=2. Populated by `start_services`.
    pub(super) handles: [Mutex<Option<StdioHandle>>; 3],
}

impl StdioState {
    fn new() -> Self {
        Self { handles: std::array::from_fn(|_| Mutex::new(None)) }
    }

    fn handle_mut(&self, service: &AsmService) -> std::sync::MutexGuard<'_, Option<StdioHandle>> {
        self.handles[service_index(service)].lock().unwrap()
    }
}

pub(super) struct StdioTransport {
    state: Arc<StdioState>,
    pub(super) world_rank: i32,
    pub(super) local_rank: i32,
}

impl Clone for StdioTransport {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            world_rank: self.world_rank,
            local_rank: self.local_rank,
        }
    }
}

impl StdioTransport {
    pub(super) fn new(world_rank: i32, local_rank: i32) -> Self {
        Self { state: Arc::new(StdioState::new()), world_rank, local_rank }
    }

    pub(super) fn state(&self) -> &Arc<StdioState> {
        &self.state
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
            let handle = start_service(service, trimmed_path, options, shm_prefix, sem_prefix)?;
            *self.state.handles[i].lock().unwrap() = Some(handle);

            if i == 0 {
                // Ping MO before spawning MT/RH: ensures shmem is initialized.
                self.send_request::<PingRequest, PingResponse>(service, &PingRequest {})
                    .with_context(|| {
                        format!("Service {service} did not respond to stdio readiness ping")
                    })?;
            }
        }

        Ok(())
    }

    pub(super) fn check_running(&self) -> Vec<AsmService> {
        AsmServices::SERVICES
            .iter()
            .filter(|s| {
                let mut guard = self.state.handle_mut(s);
                guard.as_mut().is_some_and(|h| matches!(h.child.try_wait(), Ok(None) | Err(_)))
            })
            .copied()
            .collect()
    }

    pub(super) fn send_request<Req, Res>(&self, service: &AsmService, req: &Req) -> Result<Res>
    where
        Req: ToRequestPayload,
        Res: FromResponsePayload,
    {
        let mut guard = self.state.handle_mut(service);
        let handle =
            guard.as_mut().expect("stdio handle not initialized; call start_services first");

        let out_buffer = encode_request(req.to_request_payload());
        handle
            .stdin
            .write_all(&out_buffer)
            .with_context(|| format!("Failed to write request to stdio service {service}"))?;

        let mut in_buffer = [0u8; 40];
        if let Err(e) = handle.stdout.read_exact(&mut in_buffer) {
            // Give the process a moment to fully exit if it hasn't yet
            let status = match handle.child.try_wait() {
                Ok(Some(status)) => Some(status),
                Ok(None) => {
                    // Process may still be exiting; wait briefly
                    match handle.child.wait() {
                        Ok(status) => Some(status),
                        Err(_) => None,
                    }
                }
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

        Ok(Res::from_response_payload(decode_response(&in_buffer)?))
    }
}

pub(super) const fn service_index(service: &AsmService) -> usize {
    match service {
        AsmService::MO => 0,
        AsmService::MT => 1,
        AsmService::RH => 2,
    }
}

fn start_service(
    asm_service: &AsmService,
    trimmed_path: &str,
    options: &AsmRunnerOptions,
    shm_prefix: &str,
    sem_prefix: &str,
) -> Result<StdioHandle> {
    let mut command =
        build_service_command(asm_service, trimmed_path, options, shm_prefix, sem_prefix);
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child =
        command.spawn().with_context(|| format!("Failed to spawn stdio service {asm_service}"))?;

    let stdin = child.stdin.take().context("Failed to open stdin for stdio service")?;
    let stdout = child.stdout.take().context("Failed to open stdout for stdio service")?;
    let stderr = child.stderr.take().context("Failed to open stderr for stdio service")?;

    Ok(StdioHandle { stdin, stdout, stderr, child })
}
