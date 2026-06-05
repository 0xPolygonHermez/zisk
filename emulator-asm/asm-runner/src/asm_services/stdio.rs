use std::{
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{Context, Result};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::{debug, error};

use super::services::AsmServices;
use super::{AsmService, FromResponsePayload, PingRequest, PingResponse, ToRequestPayload};
use crate::{
    AsmRunnerOptions, MemoryOperationsRequest, MemoryOperationsResponse, MinimalTraceRequest,
    MinimalTraceResponse, RomHistogramRequest, RomHistogramResponse,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::{ShutdownRequest, ShutdownResponse};

pub(super) struct StdioHandle {
    stdin: ChildStdin,
    stdout: ChildStdout,
    _stderr_drain: thread::JoinHandle<()>,
    child: Child,
}

impl std::fmt::Debug for StdioHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StdioHandle(..)")
    }
}

impl StdioHandle {
    pub(super) fn send_request<Req, Res>(&mut self, service: &AsmService, req: &Req) -> Result<Res>
    where
        Req: ToRequestPayload,
        Res: FromResponsePayload,
    {
        debug!("Sending request to stdio service {}", service);
        let out_buffer = AsmServices::encode_request(req.to_request_payload());
        debug!("Encoded request for service {}: {} bytes", service, out_buffer.len());
        self.stdin
            .write_all(&out_buffer)
            .with_context(|| format!("Failed to write request to stdio service {service}"))?;

        debug!("Request sent to stdio service {}, waiting for response...", service);
        let mut in_buffer = [0u8; 40];
        if let Err(e) = self.stdout.read_exact(&mut in_buffer) {
            error!("Failed to read response from stdio service {}: {}", service, e);
            // Give the process a moment to fully exit if it hasn't yet
            let status = match self.child.try_wait() {
                Ok(Some(status)) => Some(status),
                Ok(None) => self.child.wait().ok(), // Process may still be exiting; wait briefly
                Err(_) => None,
            };
            error!(
                "Checked stdio service {} process status after read failure: {:?}",
                service, status
            );

            if let Some(status) = status {
                error!("Service {service} process crashed with {status}");
                return Err(anyhow::anyhow!(
                    "Service {service} process exited with {status} before responding"
                ));
            }
            error!("Service {service} process status unknown after read failure.");
            return Err(e)
                .with_context(|| format!("Failed to read response from stdio service {service}"));
        }

        debug!("Received response from stdio service {}: {} bytes", service, in_buffer.len());
        debug!("Raw response bytes from service {}: {:?}", service, &in_buffer);
        Ok(Res::from_response_payload(AsmServices::decode_response(&in_buffer)?))
    }
}

#[derive(Clone)]
pub(super) struct StdioService {
    state: Arc<[Mutex<StdioHandle>; 3]>,
    pub(super) world_rank: i32,
    pub(super) local_rank: i32,
}

impl StdioService {
    pub(super) fn start_services(
        world_rank: i32,
        local_rank: i32,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
        shm_prefix: &str,
        sem_prefix: &str,
    ) -> Result<Self> {
        let handles: [Mutex<StdioHandle>; 3] = AsmServices::SERVICES
            .par_iter()
            .map(|service| {
                debug!(">>> [{}] Starting ASM service (stdio): {}", world_rank, service);
                let handle =
                    Self::start_service(service, trimmed_path, options, shm_prefix, sem_prefix)?;
                Ok(Mutex::new(handle))
            })
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .expect("expected exactly 3 services");

        Ok(Self { state: Arc::new(handles), world_rank, local_rank })
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
        let mut stderr = child.stderr.take().context("Failed to open stderr for stdio service")?;

        let stderr_drain = thread::spawn(move || {
            let mut chunk = [0u8; 4096];
            while matches!(stderr.read(&mut chunk), Ok(n) if n > 0) {}
        });

        Ok(StdioHandle { stdin, stdout, _stderr_drain: stderr_drain, child })
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(super) fn close(&self, service: &AsmService) {
        let mut guard = self.state[service.as_index()].lock().unwrap();
        let _ = guard.child.kill();
        let _ = guard.child.wait();
    }

    /// Number of `StdioService`/`AsmServices` clones sharing these child handles.
    pub(super) fn owner_count(&self) -> usize {
        Arc::strong_count(&self.state)
    }

    pub(super) fn running_services(&self) -> Vec<AsmService> {
        AsmServices::SERVICES
            .iter()
            .filter(|s| {
                let mut guard = self.state[s.as_index()].lock().unwrap();
                matches!(guard.child.try_wait(), Ok(None) | Err(_))
            })
            .copied()
            .collect()
    }

    pub(super) fn send_status_request(&self, service: &AsmService) -> Result<PingResponse> {
        self.send_request(service, &PingRequest {})
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
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
        debug!("Sending request to stdio service {}", service);
        self.state[service.as_index()]
            .lock()
            .unwrap()
            .send_request(service, req)
            .with_context(|| format!("Failed to send request to stdio service {service}"))
    }
}
