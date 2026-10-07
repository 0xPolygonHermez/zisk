use std::{
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, ExitStatus, Stdio},
    sync::Mutex,
    thread,
};

use anyhow::{Context, Result};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::{debug, error};

use super::services::AsmServices;
use super::{
    AsmService, FromResponsePayload, PingRequest, PingResponse, ResetRequest, ResetResponse,
    ToRequestPayload,
};
use crate::{
    AsmRunError, AsmRunnerOptions, MemoryOperationsRequest, MemoryOperationsResponse,
    MinimalTraceRequest, MinimalTraceResponse, RomHistogramRequest, RomHistogramResponse,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use crate::{ShutdownRequest, ShutdownResponse};

pub(super) struct StdioHandle {
    stdin: ChildStdin,
    stdout: ChildStdout,
    _stderr_drain: thread::JoinHandle<()>,
    child: Child,
    /// How the process exited, once a request has found it gone. Kept so every later request
    /// fails with the cause, instead of with a broken pipe.
    exited: Option<String>,
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
        if let Some(how) = &self.exited {
            return Err(Self::died(service, how));
        }
        debug!("Sending request to stdio service {}", service);
        let out_buffer = super::codec::encode_request(req.to_request_payload());
        debug!("Encoded request for service {}: {} bytes", service, out_buffer.len());
        if let Err(e) = self.stdin.write_all(&out_buffer) {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Err(self.record_exit(service, status));
            }
            return Err(e)
                .with_context(|| format!("Failed to write request to stdio service {service}"));
        }

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
                return Err(self.record_exit(service, status));
            }
            error!("Service {service} process status unknown after read failure.");
            return Err(e)
                .with_context(|| format!("Failed to read response from stdio service {service}"));
        }

        debug!("Received response from stdio service {}: {} bytes", service, in_buffer.len());
        debug!("Raw response bytes from service {}: {:?}", service, &in_buffer);
        Ok(Res::from_response_payload(super::codec::decode_response(&in_buffer)?))
    }

    /// Remember that the process has exited, report it once, and return the error every request
    /// to it will now get.
    fn record_exit(&mut self, service: &AsmService, status: ExitStatus) -> anyhow::Error {
        let how = status.to_string();
        error!("Service {service} process crashed with {how}");
        let error = Self::died(service, &how);
        self.exited = Some(how);
        error
    }

    fn died(service: &AsmService, how: &str) -> anyhow::Error {
        AsmRunError::ServiceDied { service: service.to_string(), how: how.to_string() }.into()
    }

    /// Whether the process has exited. A handle that is busy with a request is alive by
    /// definition, and is not waited for.
    fn has_exited(handle: &Mutex<Self>) -> Option<String> {
        let mut handle = handle.try_lock().ok()?;
        if let Some(how) = &handle.exited {
            return Some(how.clone());
        }
        match handle.child.try_wait() {
            Ok(Some(status)) => {
                let how = status.to_string();
                handle.exited = Some(how.clone());
                Some(how)
            }
            _ => None,
        }
    }
}

pub(super) struct StdioService {
    state: [Mutex<StdioHandle>; 3],
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

        Ok(Self { state: handles, world_rank, local_rank })
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

        Ok(StdioHandle { stdin, stdout, _stderr_drain: stderr_drain, child, exited: None })
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(super) fn close(&self, service: &AsmService) {
        let mut guard = self.state[service.as_index()].lock().unwrap();
        let _ = guard.child.kill();
        let _ = guard.child.wait();
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

    /// The first service found to have exited, with how it exited. Busy services count as
    /// alive and are not waited for.
    pub(super) fn exited_service(&self) -> Option<(AsmService, String)> {
        AsmServices::SERVICES.iter().find_map(|service| {
            StdioHandle::has_exited(&self.state[service.as_index()]).map(|how| (*service, how))
        })
    }

    pub(super) fn send_status_request(&self, service: &AsmService) -> Result<PingResponse> {
        self.send_request(service, &PingRequest {})
    }

    pub(super) fn send_reset_request(&self, service: &AsmService) -> Result<ResetResponse> {
        self.send_request(service, &ResetRequest {})
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// A handle over a process that exits at once, standing in for a service that crashed.
    fn handle_of_an_exited_process() -> StdioHandle {
        let mut child = Command::new("sh")
            .args(["-c", "exit 3"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn sh");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let stderr_drain = thread::spawn(move || {
            let mut chunk = [0u8; 64];
            while matches!(stderr.read(&mut chunk), Ok(n) if n > 0) {}
        });
        StdioHandle { stdin, stdout, _stderr_drain: stderr_drain, child, exited: None }
    }

    fn died(error: &anyhow::Error) -> bool {
        matches!(error.downcast_ref::<AsmRunError>(), Some(AsmRunError::ServiceDied { .. }))
    }

    #[test]
    fn a_service_that_exited_fails_every_request_with_service_died() {
        let mut handle = handle_of_an_exited_process();

        let Err(first) = handle.send_request::<_, PingResponse>(&AsmService::RH, &PingRequest {})
        else {
            panic!("a request to an exited process must fail");
        };
        assert!(died(&first), "the first failure names the exit: {first:#}");

        // The exit is remembered: the second request fails the same way, without touching the
        // pipe the dead process left behind.
        let Err(second) = handle.send_request::<_, PingResponse>(&AsmService::RH, &PingRequest {})
        else {
            panic!("a later request must fail too");
        };
        assert!(died(&second), "a later failure names the exit too: {second:#}");
    }

    #[test]
    fn has_exited_reports_an_exited_process_and_not_a_busy_one() {
        let handle = Mutex::new(handle_of_an_exited_process());
        handle.lock().unwrap().child.wait().unwrap();
        assert!(StdioHandle::has_exited(&handle).is_some());

        let busy = Mutex::new(handle_of_an_exited_process());
        let _request_in_flight = busy.lock().unwrap();
        assert!(StdioHandle::has_exited(&busy).is_none(), "a busy handle is not waited for");
    }
}
