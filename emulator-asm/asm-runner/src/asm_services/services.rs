use super::stdio::{self, StdioTransport};
use super::tcp::TcpTransport;
use super::{
    FromResponsePayload, MemoryOperationsRequest, MemoryOperationsResponse, MinimalTraceRequest,
    MinimalTraceResponse, PingRequest, PingResponse, RequestData, ResponseData, ShutdownRequest,
    ShutdownResponse, ToRequestPayload,
};
use crate::{AsmRunnerOptions, RomHistogramRequest, RomHistogramResponse};
use anyhow::{Context, Result};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::time::Duration;
use std::{fmt, path::Path, process::Command};

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

    /// Returns the `--gen=N` index expected by the ASM C binary.
    pub fn gen_index(&self) -> u8 {
        match self {
            AsmService::MT => 1,
            AsmService::RH => 2,
            AsmService::MO => 7,
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
        write!(f, "{s}")
    }
}

const ASM_SERVICE_BASE_PORT: u16 = 23115;

#[derive(Clone)]
enum Transport {
    Tcp(TcpTransport),
    Stdio(StdioTransport),
}

#[derive(Clone)]
pub struct AsmServices {
    transport: Transport,
    shm_prefix: String,
    sem_prefix: String,
}

impl AsmServices {
    const MO_SERVICE_OFFSET: u64 = 0;
    const MT_SERVICE_OFFSET: u64 = 1;
    const RH_SERVICE_OFFSET: u64 = 2;

    pub const SERVICES: [AsmService; 3] = [AsmService::MO, AsmService::MT, AsmService::RH];

    pub fn new(
        world_rank: i32,
        local_rank: i32,
        base_port: Option<u16>,
        stdio: bool,
        hash_id: Option<&str>,
    ) -> Self {
        let port = base_port.unwrap_or(ASM_SERVICE_BASE_PORT);
        let (transport, shm_prefix, sem_prefix) = if stdio {
            let pid = std::process::id();
            let shm_prefix = format!("ZISK_{pid}_{local_rank}");
            let hash8 = hash_id.map(|h| &h[..h.len().min(8)]).unwrap_or("00000000");
            let sem_prefix = format!("ZISK_{pid}_{hash8}_{local_rank}");
            let transport = Transport::Stdio(StdioTransport::new(world_rank, local_rank));
            (transport, shm_prefix, sem_prefix)
        } else {
            let shm_prefix =
                format!("ZISK_{}_{}", Self::port_base_for(base_port, local_rank), local_rank);
            let transport = Transport::Tcp(TcpTransport::new(world_rank, local_rank, port));
            let sem_prefix = shm_prefix.clone();
            (transport, shm_prefix, sem_prefix)
        };
        AsmServices { transport, shm_prefix, sem_prefix }
    }

    /// Returns the shared memory prefix (e.g. `ZISK_{pid}_{rank}` for stdio, `ZISK_{port}_{rank}` for TCP).
    pub fn shm_prefix(&self) -> &str {
        &self.shm_prefix
    }

    /// Returns the semaphore prefix (includes hash_id for stdio mode).
    pub fn sem_prefix(&self) -> &str {
        &self.sem_prefix
    }

    /// Update the semaphore prefix for a new program (stdio mode).
    /// Called when a subsequent setup reuses shmem but needs new semaphores.
    pub fn set_sem_prefix(&mut self, hash_id: &str) {
        if let Transport::Stdio(_) = &self.transport {
            let pid = std::process::id();
            let hash8 = &hash_id[..hash_id.len().min(8)];
            self.sem_prefix = format!("ZISK_{pid}_{hash8}_{}", self.local_rank());
        }
    }

    pub fn base_port(&self) -> u16 {
        match &self.transport {
            Transport::Tcp(t) => t.base_port,
            Transport::Stdio(_) => 0,
        }
    }

    pub fn local_rank(&self) -> i32 {
        match &self.transport {
            Transport::Tcp(t) => t.local_rank,
            Transport::Stdio(s) => s.local_rank,
        }
    }

    pub fn world_rank(&self) -> i32 {
        match &self.transport {
            Transport::Tcp(t) => t.world_rank,
            Transport::Stdio(s) => s.world_rank,
        }
    }

    pub fn start_asm_services(
        &self,
        ziskemuasm_path: &Path,
        mut options: AsmRunnerOptions,
    ) -> Result<()> {
        let path_str = ziskemuasm_path.to_string_lossy();
        let trimmed_path = &path_str[..path_str.len().saturating_sub(7)];

        match &self.transport {
            Transport::Tcp(t) => {
                options.share_input_shmem = true;
                // For TCP: shut down any already-running services before starting fresh ones.
                for service in t.check_running() {
                    let port = Self::port_for(&service, self.base_port(), self.local_rank());
                    tracing::info!(
                        "Service {} is already running on 127.0.0.1:{}. Shutting it down.",
                        service,
                        port
                    );
                    let _ = self.send_shutdown_and_wait(&service);
                }
                t.start_services(trimmed_path, &mut options, &self.shm_prefix)?;
            }
            Transport::Stdio(s) => {
                // Phase 0: Clean up stale shared memory from dead processes.
                cleanup_stale_shmem();

                // Phase 1: Create shared memory for each service.
                for (i, service) in Self::SERVICES.iter().enumerate() {
                    tracing::debug!(
                        ">>> [{}] Creating shmem for service (stdio): {}",
                        self.world_rank(),
                        service
                    );
                    let status = build_create_shmem_command(
                        service,
                        trimmed_path,
                        &options,
                        &self.shm_prefix,
                    )
                    .status()
                    .with_context(|| {
                        format!("Failed to spawn shmem creation for service {service}")
                    })?;
                    if !status.success() {
                        // Clean up shmem from previously-created services.
                        for prev in &Self::SERVICES[..i] {
                            let _ = cleanup_shmem_for_prefix(&self.shm_prefix, prev);
                        }
                        return Err(anyhow::anyhow!(
                            "Shmem creation for service {service} failed with {status}"
                        ));
                    }
                }

                // Phase 2: Start services with --open_all_shm.
                s.start_services(trimmed_path, &mut options, &self.shm_prefix, &self.sem_prefix)?;
            }
        }

        // Final ping for all services.
        for service in &Self::SERVICES {
            self.send_status_request(service)
                .with_context(|| format!("Service {service} failed to respond to ping"))?;
        }

        Ok(())
    }

    pub fn stop_asm_services(&self) -> Result<()> {
        let running = match &self.transport {
            Transport::Tcp(t) => t.check_running(),
            Transport::Stdio(s) => s.check_running(),
        };

        for service in running {
            match &self.transport {
                Transport::Tcp(_) => {
                    let port = Self::port_for(&service, self.base_port(), self.local_rank());
                    tracing::info!(
                        "Shutting down service {} running on 127.0.0.1:{}.",
                        service,
                        port
                    );
                }
                Transport::Stdio(_) => {
                    tracing::info!("Shutting down stdio service {}.", service);
                }
            }
            let _ = self.send_shutdown_and_wait(&service);
        }

        Ok(())
    }

    const fn service_offset(asm_service: &AsmService) -> u16 {
        match asm_service {
            AsmService::MT => Self::MT_SERVICE_OFFSET as u16,
            AsmService::RH => Self::RH_SERVICE_OFFSET as u16,
            AsmService::MO => Self::MO_SERVICE_OFFSET as u16,
        }
    }

    pub const fn default_port(asm_service: &AsmService, local_rank: i32) -> u16 {
        Self::port_for(asm_service, ASM_SERVICE_BASE_PORT, local_rank)
    }

    pub const fn port_for(asm_service: &AsmService, base_port: u16, local_rank: i32) -> u16 {
        let rank_offset = local_rank as u16 * Self::SERVICES.len() as u16;
        base_port + Self::service_offset(asm_service) + rank_offset
    }

    pub fn port_base_for(base_port: Option<u16>, local_rank: i32) -> u16 {
        let rank_offset = local_rank as u16 * Self::SERVICES.len() as u16;
        base_port.unwrap_or(ASM_SERVICE_BASE_PORT) + rank_offset
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

    pub fn send_rom_histogram_request(&self, max_steps: u64) -> Result<RomHistogramResponse> {
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
        match &self.transport {
            Transport::Tcp(t) => t.send_request(service, req),
            Transport::Stdio(s) => s.send_request(service, req),
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    pub fn send_shutdown_and_wait(&self, service: &AsmService) -> Result<()> {
        let sem_name = format!("/{}_{}_shutdown_done", self.sem_prefix, service.as_str());

        let mut sem = named_sem::NamedSemaphore::create(&sem_name, 0)
            .map_err(|e| crate::AsmRunError::SemaphoreError(sem_name.clone(), e))?;

        let _ = sem.try_wait();

        self.send_shutdown_request(service).with_context(|| {
            format!("Service '{service}' failed to respond to shutdown request.")
        })?;

        loop {
            match sem.timed_wait(Duration::from_secs(60)) {
                Ok(_) => break,
                Err(named_sem::Error::WaitFailed(e))
                    if e.kind() == std::io::ErrorKind::Interrupted =>
                {
                    continue
                }
                Err(e) => {
                    tracing::error!(
                        "[{}] Timeout or error waiting on semaphore {}: {}",
                        self.world_rank(),
                        sem_name,
                        e
                    );
                    return Err(crate::AsmRunError::SemaphoreError(sem_name.clone(), e).into());
                }
            }
        }

        drop(sem);

        let cstr = std::ffi::CString::new(sem_name.clone())?;
        unsafe {
            if libc::sem_unlink(cstr.as_ptr()) != 0 {
                let errno = std::io::Error::last_os_error();
                return Err(anyhow::anyhow!("Failed to unlink semaphore {}: {}", sem_name, errno));
            }
        }

        // In stdio mode, drop the handle to close the pipes and release the child process.
        if let Transport::Stdio(s) = &self.transport {
            let idx = stdio::service_index(service);
            *s.state().handles[idx].lock().unwrap() = None;
        }

        Ok(())
    }

    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    pub fn send_shutdown_and_wait(&self, _: &AsmService) -> Result<()> {
        Ok(())
    }
}

pub(super) fn encode_request(request: RequestData) -> [u8; 40] {
    let mut buf = [0u8; 40];
    for (i, word) in request.iter().enumerate() {
        buf[i * 8..(i + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }
    buf
}

pub(super) fn decode_response(buf: &[u8; 40]) -> Result<ResponseData> {
    let mut response = ResponseData::default();
    for (i, chunk) in buf.chunks_exact(8).enumerate() {
        response[i] = u64::from_le_bytes(chunk.try_into()?);
    }
    Ok(response)
}

fn command_path_for(trimmed_path: &str, asm_service: &AsmService) -> String {
    format!("{}-{}.bin", trimmed_path, asm_service)
}

fn shm_unlink_by_name(name: &str) -> Result<()> {
    let cstr = std::ffi::CString::new(name)?;
    unsafe { libc::shm_unlink(cstr.as_ptr()) };
    Ok(())
}

pub(super) fn build_service_command(
    asm_service: &AsmService,
    trimmed_path: &str,
    options: &AsmRunnerOptions,
    shm_prefix: &str,
    sem_prefix: &str,
) -> Command {
    let mut command = Command::new(command_path_for(trimmed_path, asm_service));
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                libc::setpriority(libc::PRIO_PROCESS, 0, -5);
                Ok(())
            });
        }
    }
    options.apply_to_command(&mut command, asm_service, shm_prefix, sem_prefix);
    command
}

/// Build a command that creates shared memory segments and exits.
fn build_create_shmem_command(
    asm_service: &AsmService,
    trimmed_path: &str,
    options: &AsmRunnerOptions,
    shm_prefix: &str,
) -> Command {
    let mut command = Command::new(command_path_for(trimmed_path, asm_service));

    command
        .arg("-s")
        .arg(format!("--gen={}", asm_service.gen_index()))
        .arg("--just_create_all_shm")
        .arg("--shm_prefix")
        .arg(shm_prefix);

    if options.verbose {
        command.arg("-v");
    }

    command.stderr(if options.verbose {
        std::process::Stdio::inherit()
    } else {
        std::process::Stdio::null()
    });

    command
}

/// Clean up a specific service's shared memory entries for a given prefix.
fn cleanup_shmem_for_prefix(shm_prefix: &str, service: &AsmService) -> Result<()> {
    let pattern = format!("{}_{}_", shm_prefix, service.as_str());
    let exact = format!("{}_{}", shm_prefix, service.as_str());
    let dev_shm = std::path::Path::new("/dev/shm");
    if let Ok(entries) = std::fs::read_dir(dev_shm) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&pattern) || name == exact {
                    shm_unlink_by_name(name)?;
                }
            }
        }
    }
    Ok(())
}

/// Scan `/dev/shm` for stale `ZISK_*` entries from dead processes and unlink them.
fn cleanup_stale_shmem() {
    let dev_shm = std::path::Path::new("/dev/shm");
    let entries = match std::fs::read_dir(dev_shm) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        if !name.starts_with("ZISK_") {
            continue;
        }

        // Extract PID from `ZISK_{pid}_...`
        let parts: Vec<&str> = name.splitn(3, '_').collect();
        if parts.len() < 3 {
            continue;
        }
        let pid: u32 = match parts[1].parse() {
            Ok(p) => p,
            Err(_) => continue, // Not a PID-based name (e.g., port-based from TCP mode)
        };

        // Check if the process is still alive
        let alive = unsafe { libc::kill(pid as i32, 0) };
        if alive == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) {
            continue; // Process alive or belongs to another user — leave it
        }

        // Process is dead (ESRCH) — unlink stale entry
        tracing::debug!("Cleaning up stale shared memory: /dev/shm/{}", name);
        let _ = shm_unlink_by_name(&name);
    }
}
