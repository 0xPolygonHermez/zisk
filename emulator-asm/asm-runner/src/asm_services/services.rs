use super::stdio::StdioService;
use crate::{
    AsmRunnerOptions, MemoryOperationsResponse, MinimalTraceResponse, RomHistogramResponse,
    NAMESPACE,
};
use anyhow::{Context, Result};

use std::process::Stdio;
use std::sync::Arc;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::time::Duration;
use std::{fmt, path::Path, process::Command};

/// This enum represents the different assembly services (MO, MT, RH) that can be run as separate processes. It provides methods to get the command path for each service, build the command to run the service with the appropriate options and shared memory/semaphore prefixes, and handle shutdown and cleanup of resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsmService {
    /// Memory Operations service, responsible for collecting memory operation traces.
    MO,
    /// Minimal Trace service, responsible for collecting minimal execution traces.
    MT,
    /// ROM Histogram service, responsible for collecting ROM histogram data.
    RH,
}

impl AsmService {
    /// Returns a string representation of the service, used for command paths and logging.
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

    /// Array index for per-service slots (MO=0, MT=1, RH=2).
    pub const fn as_index(&self) -> usize {
        match self {
            AsmService::MO => 0,
            AsmService::MT => 1,
            AsmService::RH => 2,
        }
    }

    /// Returns the command path for a given service based on the trimmed base path.
    pub fn command_path_for(&self, trimmed_path: &str) -> String {
        format!("{}-{}.bin", trimmed_path, self)
    }

    pub(super) fn build_service_command(
        &self,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
        shm_prefix: &str,
        sem_prefix: &str,
    ) -> Command {
        let binary_path = self.command_path_for(trimmed_path);
        tracing::debug!("Spawning ASM service {self} binary: {binary_path}");
        let mut command = Command::new(binary_path);
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(|| {
                    libc::setpriority(libc::PRIO_PROCESS, 0, -5);
                    libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                    Ok(())
                });
            }
        }
        options.apply_to_command(&mut command, self, shm_prefix, sem_prefix);
        command
    }

    /// Build a command that creates shared memory segments and exits.
    fn build_create_shmem_command(
        &self,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
        shm_prefix: &str,
        sem_prefix: &str,
        create_input: bool,
    ) -> Command {
        let mut command = Command::new(self.command_path_for(trimmed_path));

        command.arg("-s").arg(format!("--gen={}", self.gen_index())).arg("--share_input_shm");

        if create_input {
            command.arg("--just_create_all_shm");
        } else {
            command.arg("--just_create_non_input_shm");
        }

        command.arg("--shm_prefix").arg(shm_prefix);
        command.arg("--sem_prefix").arg(sem_prefix);

        if options.verbose {
            command.arg("-v");
        }

        command.stderr(if options.verbose { Stdio::inherit() } else { Stdio::null() });

        command
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

/// Handle to the ASM microservices for one `(pid, local_rank)`.
///
/// `Clone` shares a single [`AsmServicesInner`] via `Arc`: the runner threads
/// (MO/MT/RH) each hold a clone for the duration of a run. Teardown lives in
/// `Drop for AsmServicesInner`, so it fires exactly once when the last clone is
/// dropped — race-free, because that's driven by `Arc`'s atomic refcount rather
/// than a `strong_count()` snapshot that concurrent droppers could both misread.
#[derive(Clone)]
pub struct AsmServices {
    inner: Arc<AsmServicesInner>,
}

struct AsmServicesInner {
    service: StdioService,
    shm_prefix: String,
    sem_prefix: String,
}

impl AsmServices {
    /// Array of all services, used for iteration in setup and cleanup.
    pub const SERVICES: [AsmService; 3] = [AsmService::MO, AsmService::MT, AsmService::RH];

    /// Returns the shared memory prefix  `ZISK_{pid}_{rank}`.
    pub fn shm_prefix(&self) -> &str {
        &self.inner.shm_prefix
    }

    /// Returns the semaphore prefix `ZISK_{pid}_{hash}_{rank}`.
    pub fn sem_prefix(&self) -> &str {
        &self.inner.sem_prefix
    }

    /// Returns the local rank of the process.
    pub fn local_rank(&self) -> i32 {
        self.inner.service.local_rank
    }

    /// Returns the world rank of the process.
    pub fn world_rank(&self) -> i32 {
        self.inner.service.world_rank
    }

    /// Wrapper used by the CLI and the first worker setup.
    pub fn new(
        world_rank: i32,
        local_rank: i32,
        hash_id: String,
        ziskemuasm_path: &Path,
        with_hints: bool,
        options: AsmRunnerOptions,
    ) -> Result<AsmServices> {
        let pid = std::process::id();
        let hash8 = &hash_id[..hash_id.len().min(8)];

        let shm_prefix = format!("{NAMESPACE}_{pid}_{local_rank}");
        let sem_prefix = format!(
            "{NAMESPACE}_{pid}_{hash8}_{local_rank}{hints}",
            hints = if with_hints { "_h" } else { "" }
        );

        // Strip it to get the base path.
        // `ziskemuasm_path` expected format: "<base>-??.bin".
        // where "??" is a 2-character service identifier.
        // Total suffix length = 7 ("-??.bin").
        // We validate: is at least 7 chars long, ends with ".bin" and has "-"" before the service
        let path = ziskemuasm_path.to_string_lossy();
        let stripped_path =
            if path.len() >= 7 && path.ends_with(".bin") && path.as_bytes()[path.len() - 7] == b'-'
            {
                &path[..path.len() - 7]
            } else {
                return Err(anyhow::anyhow!("invalid path format: expected '-??.bin' suffix"));
            };
        // Phase 1: create shmem segments for this process.
        Self::create_shmem(world_rank, &shm_prefix, &sem_prefix, stripped_path, &options)?;

        // Phase 2: start services and wait for them to be ready.
        let stdio_service = StdioService::start_services(
            world_rank,
            local_rank,
            stripped_path,
            &options,
            &shm_prefix,
            &sem_prefix,
        )?;

        let inner = AsmServicesInner { service: stdio_service, shm_prefix, sem_prefix };

        for service in &Self::SERVICES {
            inner
                .service
                .send_status_request(service)
                .with_context(|| format!("Service {service} failed to respond to ping"))?;
        }

        Ok(AsmServices { inner: Arc::new(inner) })
    }

    /// Clean up all shared memory and semaphores for currently running services.
    /// Scan `/dev/shm` for stale `ZISK_*` shmem segments and `sem.ZISK_*` semaphores
    /// left by dead processes and unlink them.
    pub fn cleanup_stale_shmem() {
        super::janitor::cleanup_stale();
    }

    /// Create segments via `--just_create_all_shm`. Call once at worker startup.
    fn create_shmem(
        world_rank: i32,
        shm_prefix: &str,
        sem_prefix: &str,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
    ) -> Result<()> {
        let children: Vec<(AsmService, std::process::Child)> = Self::SERVICES
            .iter()
            .enumerate()
            .map(|(index, service)| {
                tracing::debug!(
                    ">>> [{}] Creating shmem for service (stdio): {}",
                    world_rank,
                    service
                );
                let child = service
                    .build_create_shmem_command(
                        trimmed_path,
                        options,
                        shm_prefix,
                        sem_prefix,
                        index == 0,
                    )
                    .spawn()
                    .with_context(|| {
                        format!("Failed to spawn shmem creation for service {service}")
                    })?;
                Ok((*service, child))
            })
            .collect::<Result<_>>()?;

        let mut any_failed = false;
        for (service, mut child) in children {
            let status = child
                .wait()
                .with_context(|| format!("Failed to wait on shmem creation for {service}"))?;
            if !status.success() {
                tracing::error!("Shmem creation for {service} failed with {status}");
                any_failed = true;
            }
        }

        if any_failed {
            // Roll back any segments the partial creation left behind. Unlinks
            // all `{shm_prefix}*` entries (per-service *and* the untagged
            // `_input`/`_precompile`/`_control` ones); the semaphore sweep is a
            // no-op here since no semaphores exist yet at creation time.
            super::janitor::cleanup_prefix(shm_prefix, sem_prefix);
            return Err(anyhow::anyhow!("One or more shmem creation commands failed"));
        }

        Ok(())
    }

    /// Send a minimal trace request to the MT service and return the response.
    pub(crate) fn send_minimal_trace_request(
        &self,
        max_steps: u64,
        chunk_len: u64,
    ) -> Result<MinimalTraceResponse> {
        self.inner.service.send_minimal_trace_request(max_steps, chunk_len)
    }

    /// Send a ROM histogram request to the RH service and return the response.
    pub(crate) fn send_rom_histogram_request(
        &self,
        max_steps: u64,
    ) -> Result<RomHistogramResponse> {
        self.inner.service.send_rom_histogram_request(max_steps)
    }

    /// Send a memory operations request to the MO service and return the response.
    pub(crate) fn send_memory_ops_request(
        &self,
        max_steps: u64,
        chunk_len: u64,
    ) -> Result<MemoryOperationsResponse> {
        self.inner.service.send_memory_ops_request(max_steps, chunk_len)
    }
}

impl AsmServicesInner {
    fn stop_asm_services(&self) -> Result<()> {
        let running = self.service.running_services();

        let mut errors = Vec::new();
        for service in running {
            tracing::info!("Shutting down stdio service {}.", service);
            if let Err(e) = self.send_shutdown_and_wait(&service) {
                errors.push(format!("{service}: {e:#}"));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "failed to shut down {} stdio service(s):\n{}",
                errors.len(),
                errors.join("\n")
            ))
        }
    }

    /// Sends a shutdown request to the specified service and waits for its completion.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn send_shutdown_and_wait(&self, service: &AsmService) -> Result<()> {
        // Graceful shutdown handshake.
        let handshake = self.graceful_shutdown(service);

        // Close pipes and reap the child process (best-effort, infallible).
        self.service.close(service);

        handshake
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn graceful_shutdown(&self, service: &AsmService) -> Result<()> {
        let sem_name = format!("/{}_{}_shutdown_done", self.sem_prefix, service.as_str());

        let mut sem = named_sem::NamedSemaphore::create(&sem_name, 0)
            .map_err(|e| crate::AsmRunError::SemaphoreError(sem_name.clone(), e))?;

        let _ = sem.try_wait();

        self.service.send_shutdown_request(service).with_context(|| {
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
                        self.service.world_rank,
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

        Ok(())
    }

    /// Sends a shutdown request to the specified service and waits for its
    /// completion. No-op off Linux-x86_64, where the ASM services never run.
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    fn send_shutdown_and_wait(&self, _: &AsmService) -> Result<()> {
        Ok(())
    }

    /// Unlink every `/dev/shm/{shm_prefix}*` shmem segment and
    /// `/dev/shm/sem.{sem_prefix}*` semaphore. The C-side `server_cleanup`
    /// only unlinks if `delete_input_shm`/`delete_output_shm` flags are
    /// set — which the long-running ASM service children don't have — so
    /// the parent has to do it. Call after `stop_asm_services` so the
    /// children are already detached from the segments.
    fn cleanup_my_shmem(&self) {
        super::janitor::cleanup_prefix(&self.shm_prefix, &self.sem_prefix);
    }
}

impl Drop for AsmServicesInner {
    /// RAII teardown for the ASM microservices and their `/dev/shm` segments.
    ///
    /// Runs exactly once: this is the sole owner behind the `Arc` in
    /// [`AsmServices`], so `drop` fires only when the last `AsmServices` clone
    /// is gone. No `strong_count` guard — the `Arc` refcount is the gate.
    fn drop(&mut self) {
        tracing::info!(">>> [{}] Stopping ASM microservices.", self.service.local_rank);
        if let Err(e) = self.stop_asm_services() {
            tracing::error!(
                ">>> [{}] Failed to stop ASM microservices: {}",
                self.service.local_rank,
                e
            );
        }

        self.cleanup_my_shmem();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_index_matches_c_binary_contract() {
        // These are the `--gen=N` values the ziskemuasm C binary expects.
        assert_eq!(AsmService::MT.gen_index(), 1);
        assert_eq!(AsmService::RH.gen_index(), 2);
        assert_eq!(AsmService::MO.gen_index(), 7);
    }

    #[test]
    fn as_str_is_uppercase_used_for_segment_names() {
        assert_eq!(AsmService::MO.as_str(), "MO");
        assert_eq!(AsmService::MT.as_str(), "MT");
        assert_eq!(AsmService::RH.as_str(), "RH");
    }

    #[test]
    fn display_is_lowercase_and_drives_binary_path() {
        // Display (lowercase) names the per-service binary; as_str (uppercase)
        // names the shmem segments. Keeping them distinct is deliberate.
        assert_eq!(AsmService::MO.to_string(), "mo");
        assert_eq!(AsmService::RH.to_string(), "rh");
        assert_eq!(AsmService::MO.command_path_for("/x/ziskemuasm"), "/x/ziskemuasm-mo.bin");
        assert_eq!(AsmService::RH.command_path_for("base"), "base-rh.bin");
    }

    #[test]
    fn services_array_is_indexed_consistently() {
        assert_eq!(AsmServices::SERVICES, [AsmService::MO, AsmService::MT, AsmService::RH]);
        for (i, s) in AsmServices::SERVICES.iter().enumerate() {
            assert_eq!(s.as_index(), i, "as_index must match position in SERVICES");
        }
    }
}
