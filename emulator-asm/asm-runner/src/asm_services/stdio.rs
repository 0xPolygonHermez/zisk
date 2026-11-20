use std::{
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, ExitStatus, Stdio},
    sync::{Mutex, MutexGuard, TryLockError},
    thread,
};

use anyhow::{Context, Result};
use tracing::{debug, error};

use super::services::AsmServices;
use super::{
    AsmService, FromResponsePayload, PingRequest, PingResponse, ResetMode, ResetRequest,
    ResetResponse, ToRequestPayload,
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
    /// Why the process can serve no request, once found: how it exited, or why it was
    /// stopped. Kept so every later request fails with the cause, instead of with a
    /// broken pipe.
    exited: Option<String>,
}

impl Drop for StdioHandle {
    /// A service runs until stopped, writing the shared segments, so a handle takes its
    /// process with it.
    fn drop(&mut self) {
        self.kill_and_reap();
    }
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
            if let Some(status) = self.status_after_pipe_failure() {
                return Err(self.retire(service, status.to_string()));
            }
            return Err(e)
                .with_context(|| format!("Failed to write request to stdio service {service}"));
        }

        debug!("Request sent to stdio service {}, waiting for response...", service);
        let mut in_buffer = [0u8; 40];
        if let Err(e) = self.stdout.read_exact(&mut in_buffer) {
            error!("Failed to read response from stdio service {}: {}", service, e);
            if let Some(status) = self.status_after_pipe_failure() {
                return Err(self.retire(service, status.to_string()));
            }
            error!("Service {service} process status unknown after read failure.");
            return Err(e)
                .with_context(|| format!("Failed to read response from stdio service {service}"));
        }

        debug!("Received response from stdio service {}: {} bytes", service, in_buffer.len());
        debug!("Raw response bytes from service {}: {:?}", service, &in_buffer);
        let payload = super::codec::decode_response(&in_buffer)?;
        if payload[0] != Res::RESPONSE_ID {
            // Whatever it answers next would belong to another request.
            let how = format!(
                "answered out of turn (response id {}, expected {})",
                payload[0],
                Res::RESPONSE_ID
            );
            return Err(self.retire(service, how));
        }
        Ok(Res::from_response_payload(payload))
    }

    /// The exit status of a process its pipe failed on. The services never close their
    /// stdin or stdout, so a failed pipe means the process is exiting: wait for it.
    fn status_after_pipe_failure(&mut self) -> Option<ExitStatus> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status),
            Ok(None) => self.child.wait().ok(),
            Err(_) => None,
        }
    }

    /// Make the process serve no more requests, because of `how`: stop it if it still
    /// runs, report it once, and return the error every request to it will now get.
    fn retire(&mut self, service: &AsmService, how: String) -> anyhow::Error {
        error!("Service {service} can serve no more requests: {how}");
        self.kill_and_reap();
        let error = Self::died(service, &how);
        self.exited = Some(how);
        error
    }

    fn died(service: &AsmService, how: &str) -> anyhow::Error {
        AsmRunError::ServiceDied { service: service.to_string(), how: how.to_string() }.into()
    }

    /// Kill the process and reap it; one already gone is only reaped.
    fn kill_and_reap(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// A handle poisoned by a panic during a request, in a state nobody knows: it is
    /// retired, so every later request fails rather than reads another's response.
    fn recover<'a>(service: &AsmService, mut handle: MutexGuard<'a, Self>) -> MutexGuard<'a, Self> {
        if handle.exited.is_none() {
            handle.retire(service, "left in an unknown state by a panic".to_string());
        }
        handle
    }
}

/// What a program's services are started with, kept to start one again; see
/// [`StdioService::restart`].
pub(super) struct Launch {
    pub(super) trimmed_path: String,
    pub(super) options: AsmRunnerOptions,
    pub(super) shm_prefix: String,
    pub(super) sem_prefix: String,
}

pub(super) struct StdioService {
    state: [Mutex<StdioHandle>; 3],
    launch: Launch,
    pub(super) world_rank: i32,
    pub(super) local_rank: i32,
}

impl StdioService {
    pub(super) fn start_services(world_rank: i32, local_rank: i32, launch: Launch) -> Result<Self> {
        // On a failure, every service that did start is stopped before the caller's
        // claim on the segments ends with it: dropping their handles kills them.
        let handles = AsmServices::SERVICES
            .iter()
            .map(|service| {
                debug!(">>> [{}] Starting ASM service (stdio): {}", world_rank, service);
                Self::start_service(service, &launch)
            })
            .collect::<Result<Vec<_>>>()?;

        let handles: [Mutex<StdioHandle>; 3] = handles
            .into_iter()
            .map(Mutex::new)
            .collect::<Vec<_>>()
            .try_into()
            .expect("expected exactly 3 services");

        Ok(Self { state: handles, launch, world_rank, local_rank })
    }

    /// Start `service` again in place of a process that can serve no more requests, and
    /// wait until the new one answers a ping. On a failure the old handle stays, so every
    /// request still fails with why it stopped, and a later restart can try again.
    ///
    /// The new process writes the shared segments as it starts, its guest RAM and ROM
    /// among them, and recreates the service's semaphores. So the caller holds the
    /// segments' claim, has quiesced the program they serve, and binds the writers to the
    /// semaphores and resets the guest memory afterwards: [`AsmServices::activate`].
    pub(super) fn restart(&self, service: &AsmService) -> Result<()> {
        debug!(">>> [{}] Restarting ASM service (stdio): {}", self.world_rank, service);
        let fresh = Self::start_service(service, &self.launch)?;
        // Dropping the old handle reaps its process.
        *self.handle(service) = fresh;
        self.send_status_request(service)
            .map(|_| ())
            .with_context(|| format!("Restarted service {service} failed to respond to ping"))
    }

    fn start_service(asm_service: &AsmService, launch: &Launch) -> Result<StdioHandle> {
        let mut command = asm_service.build_service_command(
            &launch.trimmed_path,
            &launch.options,
            &launch.shm_prefix,
            &launch.sem_prefix,
        );
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = super::spawner::spawn(command)
            .with_context(|| format!("Failed to spawn stdio service {asm_service}"))?;

        // Until the child is in a handle, which stops it when dropped, a failure must
        // stop it by hand: it writes the shared segments as it starts.
        let stop = |mut child: std::process::Child| {
            let _ = child.kill();
            let _ = child.wait();
        };
        let (Some(stdin), Some(stdout), Some(mut stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            stop(child);
            anyhow::bail!("Failed to open the pipes of stdio service {asm_service}");
        };
        let stderr_drain = match thread::Builder::new().spawn(move || {
            let mut chunk = [0u8; 4096];
            while matches!(stderr.read(&mut chunk), Ok(n) if n > 0) {}
        }) {
            Ok(stderr_drain) => stderr_drain,
            Err(e) => {
                stop(child);
                return Err(e).with_context(|| {
                    format!("Failed to drain stdio service {asm_service}'s stderr")
                });
            }
        };

        Ok(StdioHandle { stdin, stdout, _stderr_drain: stderr_drain, child, exited: None })
    }

    /// The handle of `service`'s process, waiting for any request in flight on it.
    fn handle(&self, service: &AsmService) -> MutexGuard<'_, StdioHandle> {
        self.state[service.as_index()]
            .lock()
            .unwrap_or_else(|poisoned| StdioHandle::recover(service, poisoned.into_inner()))
    }

    /// The process serving `service`.
    #[cfg(test)]
    pub(super) fn pid(&self, service: &AsmService) -> u32 {
        self.handle(service).child.id()
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub(super) fn close(&self, service: &AsmService) {
        self.handle(service).kill_and_reap();
    }

    pub(super) fn running_services(&self) -> Vec<AsmService> {
        AsmServices::SERVICES
            .iter()
            .filter(|s| matches!(self.handle(s).child.try_wait(), Ok(None) | Err(_)))
            .copied()
            .collect()
    }

    /// The services that can serve no more requests, each with why. Busy services count as
    /// alive and are not waited for.
    pub(super) fn exited(&self) -> Vec<(AsmService, String)> {
        AsmServices::SERVICES
            .iter()
            .filter_map(|service| self.has_exited(service).map(|how| (*service, how)))
            .collect()
    }

    /// Why `service` can serve no request, if it can serve none. A handle busy with a
    /// request is alive by definition, and is not waited for.
    fn has_exited(&self, service: &AsmService) -> Option<String> {
        let mut handle = match self.state[service.as_index()].try_lock() {
            Ok(handle) => handle,
            Err(TryLockError::WouldBlock) => return None,
            Err(TryLockError::Poisoned(poisoned)) => {
                StdioHandle::recover(service, poisoned.into_inner())
            }
        };
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

    pub(super) fn send_status_request(&self, service: &AsmService) -> Result<PingResponse> {
        self.send_request(service, &PingRequest {})
    }

    pub(super) fn send_reset_request(
        &self,
        service: &AsmService,
        mode: ResetMode,
    ) -> Result<ResetResponse> {
        self.send_request(service, &ResetRequest { mode })
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
        self.handle(service)
            .send_request(service, req)
            .with_context(|| format!("Failed to send request to stdio service {service}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{Duration, Instant};

    /// A handle over `sh -c script`, standing in for a service.
    fn handle_running(script: &str) -> StdioHandle {
        let mut child = Command::new("sh")
            .args(["-c", script])
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

    /// A handle over a process that exits at once, standing in for a service that crashed.
    fn handle_of_an_exited_process() -> StdioHandle {
        handle_running("exit 3")
    }

    /// Three services idling, one per [`AsmService`]. None can be started again.
    fn idle_services() -> StdioService {
        StdioService {
            state: [(); 3].map(|()| Mutex::new(handle_running("exec sleep 60"))),
            launch: Launch {
                trimmed_path: "/nonexistent/prog".to_string(),
                options: AsmRunnerOptions::new(),
                shm_prefix: String::new(),
                sem_prefix: String::new(),
            },
            world_rank: 0,
            local_rank: 0,
        }
    }

    fn died(error: &anyhow::Error) -> bool {
        error.chain().any(|cause| {
            matches!(cause.downcast_ref::<AsmRunError>(), Some(AsmRunError::ServiceDied { .. }))
        })
    }

    /// Whether the process `pid` still exists.
    fn alive(pid: u32) -> bool {
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
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

    /// A response with another request's id means the stream is out of step, so whatever
    /// the service answers next would be taken for the wrong request.
    #[test]
    fn a_service_that_answers_out_of_turn_is_stopped_for_good() {
        // Reads one request, answers with id 9, then idles.
        let mut handle = handle_running(
            r#"head -c 40 >/dev/null; printf '\011'; head -c 39 /dev/zero; exec sleep 60"#,
        );
        let pid = handle.child.id();

        let Err(error) = handle.send_request::<_, PingResponse>(&AsmService::MT, &PingRequest {})
        else {
            panic!("an answer out of turn must fail the request");
        };
        assert!(died(&error), "it names the service as unusable: {error:#}");
        assert!(!alive(pid), "the service must be stopped");

        let Err(later) = handle.send_request::<_, PingResponse>(&AsmService::MT, &PingRequest {})
        else {
            panic!("a later request must fail too");
        };
        assert!(died(&later), "a later failure says why: {later:#}");
    }

    /// A panic during a request leaves its handle in a state nobody knows, which must not
    /// make every later request, teardown included, panic in turn.
    #[test]
    fn a_service_whose_handle_a_panic_poisoned_fails_its_requests() {
        let services = idle_services();
        let pid = services.state[AsmService::MO.as_index()].lock().unwrap().child.id();
        thread::scope(|scope| {
            let poisoner = scope.spawn(|| {
                let _request = services.state[AsmService::MO.as_index()].lock().unwrap();
                panic!("a panic during a request");
            });
            assert!(poisoner.join().is_err());
        });

        let Err(error) = services.send_status_request(&AsmService::MO) else {
            panic!("a request to a poisoned service must fail");
        };
        assert!(died(&error), "the request fails as to a dead service: {error:#}");
        assert!(!alive(pid), "the service must be stopped");
        let exited: Vec<_> = services.exited().into_iter().map(|(service, _)| service).collect();
        assert_eq!(exited, [AsmService::MO]);
        assert_eq!(services.running_services(), [AsmService::MT, AsmService::RH]);
    }

    /// A restart that cannot start the service leaves the old handle, which still says why
    /// it stopped, so a later restart can try again.
    #[test]
    fn a_failed_restart_keeps_the_stopped_service() {
        let services = idle_services();
        let rh = &services.state[AsmService::RH.as_index()];
        *rh.lock().unwrap() = handle_of_an_exited_process();
        rh.lock().unwrap().child.wait().unwrap();
        assert!(services.has_exited(&AsmService::RH).is_some());

        assert!(services.restart(&AsmService::RH).is_err(), "no binary, no restart");
        let Err(error) = services.send_status_request(&AsmService::RH) else {
            panic!("a request to the stopped service must still fail");
        };
        assert!(died(&error), "it still says why it stopped: {error:#}");
    }

    #[test]
    fn has_exited_reports_an_exited_process_and_not_a_busy_one() {
        let services = idle_services();
        let exited = &services.state[AsmService::RH.as_index()];
        *exited.lock().unwrap() = handle_of_an_exited_process();
        exited.lock().unwrap().child.wait().unwrap();
        assert!(services.has_exited(&AsmService::RH).is_some());

        let _request_in_flight = services.state[AsmService::MT.as_index()].lock().unwrap();
        assert!(services.has_exited(&AsmService::MT).is_none(), "a busy handle is not waited for");
    }

    #[test]
    fn dropping_a_handle_stops_its_process() {
        let handle = handle_running("exec sleep 60");
        let pid = handle.child.id();
        drop(handle);

        let deadline = Instant::now() + Duration::from_secs(5);
        while alive(pid) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!alive(pid), "a dropped handle must take its process with it");
    }
}
