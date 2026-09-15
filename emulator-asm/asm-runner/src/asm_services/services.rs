use super::stdio::StdioService;
use crate::{
    AsmRunnerOptions, MemoryOperationsResponse, MinimalTraceResponse, RomHistogramResponse,
    NAMESPACE,
};

use anyhow::{Context, Result};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::time::Duration;
use std::{fmt, path::Path, process::Command};

/// Live `AsmServices` per shmem prefix — the refcount behind [`PrefixLease`].
///
/// The segments are keyed by `pid` + `local_rank` + hints mode, *not* by
/// program, so one set serves every program set up on this worker. That makes
/// both ends of their lifetime shared, and each end has a way to go wrong:
///
/// - **Creating twice.** Re-running the creation helpers for a second program
///   would `shm_unlink` the segments out from under the services already
///   mapping them and create fresh inodes under the same names. `/dev/shm`
///   would look unchanged while every earlier generation stayed resident and
///   pinned with nothing able to reach it again — 14.9 GiB per program.
/// - **Destroying too early.** `cleanup_shm_prefix` unlinks by prefix, so one
///   program's teardown would take the whole set with it while other programs
///   were still using it.
///
/// An entry exists exactly while at least one `AsmServices` holds the prefix;
/// the last lease out unlinks the segments and removes the entry, so a later
/// setup creates them again.
static PREFIX_LEASES: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());

/// Proof that this `AsmServices` may use `shm_prefix`'s segments, and that they
/// will outlive it.
///
/// Exists as a guard rather than a pair of bookkeeping calls so the count cannot
/// drift: `AsmServices::new` can fail at several points after the segments are
/// in place, and every one of those paths drops the lease on the way out.
struct PrefixLease {
    shm_prefix: String,
}

impl Drop for PrefixLease {
    fn drop(&mut self) {
        let mut leases = PREFIX_LEASES.lock().unwrap_or_else(|p| p.into_inner());
        let Some(count) = leases.get_mut(&self.shm_prefix) else {
            tracing::error!("Prefix lease for '{}' released twice", self.shm_prefix);
            return;
        };
        *count -= 1;
        if *count > 0 {
            return;
        }
        // Last user out: the segments are now unreachable, so unlink them and
        // forget the prefix. A later setup will create a fresh set.
        leases.remove(&self.shm_prefix);
        tracing::debug!("Last user of shmem prefix {} — unlinking its segments", self.shm_prefix);
        super::janitor::cleanup_shm_prefix(&self.shm_prefix);
    }
}

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

        if options.unlock_mapped_memory {
            command.arg("-u");
        }

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
/// `Clone` shares a single `AsmServicesInner` via `Arc`: the runner threads
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
    /// Keeps this prefix's shared segments alive; the last one out unlinks them.
    /// Declared last so it is released only after `Drop` has stopped the children.
    _prefix_lease: PrefixLease,
}

impl AsmServices {
    /// Array of all services, used for iteration in setup and cleanup.
    pub const SERVICES: [AsmService; 3] = [AsmService::MO, AsmService::MT, AsmService::RH];

    /// Returns the shared memory prefix `ZISK_{pid}_{rank}` (plus `_h` with hints).
    /// Shared by every program set up on this worker — see [`CREATED_SHMEM_PREFIXES`].
    pub fn shm_prefix(&self) -> &str {
        &self.inner.shm_prefix
    }

    /// Returns the semaphore prefix `ZISK_{pid}_{hash}_{rank}` (plus `_h` with hints).
    /// Per-program, unlike the shmem prefix.
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

        // The hints mode belongs on both prefixes: `get_precompile_results()` comes
        // from the generated assembly, so the hints binary variant creates a
        // `_precompile` segment the non-hints one neither creates nor opens. The two
        // modes therefore do not have the same *set* of segments and cannot share one.
        // Two sets per worker at most, one per mode, each reused by every program in it.
        //
        // `_h1`/`_h0` rather than `_h`/`""` so that neither prefix is a prefix of the
        // other: `janitor::cleanup_prefix` unlinks by `starts_with`, and with an empty
        // marker a rollback in one mode would silently destroy the other mode's live
        // segments. Keep any future marker prefix-free for the same reason.
        let hints = if with_hints { "_h1" } else { "_h0" };
        let shm_prefix = format!("{NAMESPACE}_{pid}_{local_rank}{hints}");
        let sem_prefix = format!("{NAMESPACE}_{pid}_{hash8}_{local_rank}{hints}");

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
        // Phase 1: create the shmem segments — once per prefix, not once per program.
        // The lease keeps them alive for as long as any program is using them; every
        // failure path below drops it, so a failed setup cannot strand the prefix.
        let prefix_lease =
            Self::acquire_prefix(world_rank, &shm_prefix, &sem_prefix, stripped_path, &options)?;

        // Phase 2: start services and wait for them to be ready.
        let stdio_service = StdioService::start_services(
            world_rank,
            local_rank,
            stripped_path,
            &options,
            &shm_prefix,
            &sem_prefix,
        )?;

        let inner = AsmServicesInner {
            service: stdio_service,
            shm_prefix,
            sem_prefix,
            _prefix_lease: prefix_lease,
        };

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

    /// Take a lease on `shm_prefix`, creating its segments if this is the first.
    ///
    /// Creation and the refcount are decided under one lock, so two concurrent
    /// setups for the same prefix cannot both run the helpers, and a lease can
    /// never be granted on segments that a concurrent teardown is unlinking.
    fn acquire_prefix(
        world_rank: i32,
        shm_prefix: &str,
        sem_prefix: &str,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
    ) -> Result<PrefixLease> {
        let mut leases = PREFIX_LEASES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        match leases.get_mut(shm_prefix) {
            Some(count) => {
                tracing::debug!(
                    ">>> [{world_rank}] Reusing existing shmem for prefix {shm_prefix} \
                     ({count} user(s) before this one); skipping creation"
                );
                *count += 1;
            }
            None => {
                Self::create_shmem(world_rank, shm_prefix, sem_prefix, trimmed_path, options)?;
                leases.insert(shm_prefix.to_string(), 1);
            }
        }

        Ok(PrefixLease { shm_prefix: shm_prefix.to_string() })
    }

    /// The counting half of [`Self::acquire_prefix`], without the creation.
    /// Lets tests exercise the lease lifetime without spawning the C helpers.
    #[cfg(test)]
    fn acquire_prefix_uncreated(shm_prefix: &str) -> PrefixLease {
        let mut leases = PREFIX_LEASES.lock().unwrap_or_else(|p| p.into_inner());
        *leases.entry(shm_prefix.to_string()).or_insert(0) += 1;
        PrefixLease { shm_prefix: shm_prefix.to_string() }
    }

    /// Create all of the shared-memory segments.
    ///
    /// # Ordering
    ///
    /// The segments split into two groups by ownership:
    /// - **Shared** (`input`, `control_input`, `precompile`) — one copy per
    ///   process, created only by the index-0 service and *opened* read-only by the others.
    /// - **Per-service** (`output`, internal) — each service creates its own.
    ///
    /// # Errors
    ///
    /// Returns an error if any service's binary fails to spawn, can't be waited
    /// on, or exits unsuccessfully. On any failure, a best-effort cleanup of the
    /// segments that may have been created is attempted before returning.
    fn create_shmem(
        world_rank: i32,
        shm_prefix: &str,
        sem_prefix: &str,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
    ) -> Result<()> {
        // The index-0 service creates the shared segments; the rest open them.
        let creator = Self::SERVICES[0];
        let openers = &Self::SERVICES[1..];

        // The shared segments must exist before any opener runs, so create them
        // first and only then create the per-service ones. Every failure path
        // funnels here so they all share the best-effort cleanup below.
        let result = Self::launch_creator(
            world_rank,
            creator,
            trimmed_path,
            options,
            shm_prefix,
            sem_prefix,
        )
        .and_then(|()| {
            Self::launch_openers(world_rank, openers, trimmed_path, options, shm_prefix, sem_prefix)
        });

        if result.is_err() {
            // Roll back any segments the partial creation left behind. Unlinks
            // all `{shm_prefix}*` entries (per-service *and* the untagged
            // `_input`/`_precompile`/`_control` ones); the semaphore sweep is a
            // no-op here since no semaphores exist yet at creation time.
            super::janitor::cleanup_prefix(shm_prefix, sem_prefix);
        }
        result
    }

    /// Run `creator` to completion to create the process-shared `input`,
    /// `control_input` and `precompile` segments. It is spawned and waited
    /// synchronously: its clean exit is the signal that those segments durably
    /// exist, which every opener depends on.
    fn launch_creator(
        world_rank: i32,
        creator: AsmService,
        trimmed_path: &str,
        options: &AsmRunnerOptions,
        shm_prefix: &str,
        sem_prefix: &str,
    ) -> Result<()> {
        tracing::debug!(">>> [{world_rank}] Creating shmem for service (stdio): {creator}");
        let status = creator
            .build_create_shmem_command(trimmed_path, options, shm_prefix, sem_prefix, true)
            .spawn()
            .and_then(|mut child| child.wait())
            .with_context(|| format!("Failed to create shmem for service {creator}"))?;
        if !status.success() {
            anyhow::bail!("Shmem creation for {creator} failed with {status}");
        }
        Ok(())
    }

    /// Create each opener's own `output`/`rom`/`ram`/`control_output` segments.
    /// The shared segments must already exist (openers only open them
    /// read-only), so this runs the openers concurrently. Every child that
    /// starts is reaped — even if a later spawn fails — to avoid orphans, and
    /// the spawn error is surfaced only afterwards.
    fn launch_openers(
        world_rank: i32,
        openers: &[AsmService],
        trimmed_path: &str,
        options: &AsmRunnerOptions,
        shm_prefix: &str,
        sem_prefix: &str,
    ) -> Result<()> {
        let mut children = Vec::with_capacity(openers.len());
        let spawn_result: Result<()> = openers.iter().try_for_each(|service| {
            tracing::debug!(">>> [{world_rank}] Creating shmem for service (stdio): {service}");
            let child = service
                .build_create_shmem_command(trimmed_path, options, shm_prefix, sem_prefix, false)
                .spawn()
                .with_context(|| format!("Failed to spawn shmem creation for service {service}"))?;
            children.push((*service, child));
            Ok(())
        });

        // Reap everything we started, regardless of the spawn error above.
        let mut any_failed = false;
        for (service, mut child) in children {
            match child.wait() {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    tracing::error!("Shmem creation for {service} failed with {status}");
                    any_failed = true;
                }
                Err(e) => {
                    tracing::error!("Failed to wait on shmem creation for {service}: {e}");
                    any_failed = true;
                }
            }
        }

        spawn_result?; // surface the spawn error only after reaping live children
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

    /// Re-initialize every service's guest RAM and ROM, and wait for all three
    /// to confirm.
    ///
    /// Call this before the first emulation after the active program changes.
    /// The `_ram`/`_rom` segments are shared across programs, and a service's
    /// own post-emulation `server_reset_slow` only leaves them correct for its
    /// own next run — it says nothing about what another program's services
    /// wrote there in between. The C side services the request synchronously,
    /// so a response means that service's memory is ready.
    ///
    /// Runs the three in parallel: each is a 512 MiB memset plus a ROM rewrite,
    /// and they are separate processes with independent stdio state.
    pub fn reset_services(&self) -> Result<()> {
        Self::SERVICES
            .par_iter()
            .try_for_each(|service| {
                self.inner
                    .service
                    .send_reset_request(service)
                    .with_context(|| format!("Service {service} failed to reset"))
                    .map(|_| ())
            })
            .context(
                "Failed to reset ASM services. If the services died with \
                 'Invalid request id', their cached binaries predate the reset request: \
                 delete ~/.zisk/cache so they are regenerated",
            )
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
    /// Unlink the semaphores this program owns.
    ///
    /// Only the semaphores: they carry the program hash, so they are this
    /// program's to remove. The shmem segments are shared with every other
    /// program on the same prefix, and unlinking those is `PrefixLease`'s job
    /// once the last of them is gone.
    fn cleanup_my_semaphores(&self) {
        super::janitor::cleanup_sem_prefix(&self.sem_prefix);
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

        self.cleanup_my_semaphores();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shmem segments are shared by every program on a prefix, so a single
    /// program's teardown must not take them with it — but the last one out has
    /// to, or the worker leaves ~15 GiB of `/dev/shm` behind on exit.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn prefix_segments_are_unlinked_only_when_the_last_program_releases() {
        let prefix = format!("ZISK_unittest_lease_{}", std::process::id());
        let seg = format!("{prefix}_MT_output_0");

        let c = std::ffi::CString::new(seg.clone()).unwrap();
        let exists = || unsafe {
            let fd = libc::shm_open(c.as_ptr(), libc::O_RDONLY, 0);
            if fd >= 0 {
                libc::close(fd);
                true
            } else {
                false
            }
        };
        unsafe {
            let fd = libc::shm_open(c.as_ptr(), libc::O_CREAT | libc::O_RDWR, 0o600);
            assert!(fd >= 0, "could not create the stand-in segment");
            libc::close(fd);
        }
        assert!(exists());

        // Two programs sharing one prefix.
        let first = AsmServices::acquire_prefix_uncreated(&prefix);
        let second = AsmServices::acquire_prefix_uncreated(&prefix);

        drop(first);
        assert!(exists(), "one program's teardown must not unlink the shared segments");

        drop(second);
        assert!(!exists(), "the last program out must unlink them");
        assert!(
            !PREFIX_LEASES.lock().unwrap().contains_key(&prefix),
            "the prefix must be forgotten so a later setup re-creates it"
        );
    }

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
