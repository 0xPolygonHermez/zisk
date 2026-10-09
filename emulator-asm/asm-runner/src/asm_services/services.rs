use super::segments::{ActiveProgram, SegmentsClaim, SetOnDrop, SharedSegments};
use super::stdio::StdioService;
use crate::{
    sem_prefix_for, shm_prefix_for, AsmRunError, AsmRunnerOptions, MemoryOperationsResponse,
    MinimalTraceResponse, RomHistogramResponse,
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
    ///
    /// The memory-ops emulator comes in two forms with their own names, so a cache shared with
    /// a build that knows only the plain `-mo` form never hands either one the other's stream:
    /// `-moh` (values and steps, what the device witness needs) and `-mol` (one word per record).
    pub fn command_path_for(&self, trimmed_path: &str) -> String {
        if *self == AsmService::MO {
            return if crate::mops_light() {
                format!("{trimmed_path}-mol.bin")
            } else {
                format!("{trimmed_path}-moh.bin")
            };
        }
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
        if options.metrics {
            command.arg("-m");
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

/// Handle to one program's ASM microservices: its three service processes and
/// its semaphores. The shared-memory segments they use are shared with every
/// other program on the same `(pid, local_rank, hints mode)` (see
/// [`SharedSegments`]).
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

/// A program's services. The fields drop in order after `Drop` has stopped the
/// services, and also when it panics: `service`, which kills and reaps any process
/// left, then `stopped`, which reports it, then `segments`.
pub(super) struct AsmServicesInner {
    service: StdioService,
    sem_prefix: String,
    /// Set once the services are gone; see [`ActiveProgram`].
    stopped: SetOnDrop,
    /// The segments the services use.
    segments: Arc<SharedSegments>,
}

impl AsmServices {
    /// Array of all services, used for iteration in setup and cleanup.
    pub const SERVICES: [AsmService; 3] = [AsmService::MO, AsmService::MT, AsmService::RH];

    /// Returns the shared memory prefix `ZISK_{pid}_{rank}_h{0,1}`, where the
    /// trailing marker is the hints mode.
    pub fn shm_prefix(&self) -> &str {
        self.inner.segments.shm_prefix()
    }

    /// The shared segments the services use. Whatever else maps them, such as
    /// the parent's readers and writers, holds this too, so that they outlive it.
    pub fn segments(&self) -> &Arc<SharedSegments> {
        &self.inner.segments
    }

    /// Returns the semaphore prefix `ZISK_{pid}_{hash}_{rank}_h{0,1}`, where
    /// `hash` is the first 32 characters of the program hash.
    ///
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

    /// Start a program's services on this process's segments for its rank and
    /// hints mode, creating the segments if no program holds them yet.
    ///
    /// Returns them with the claim taken to start them: keep it until the program
    /// is active ([`Self::activate`]), so no other setup or job gets in between.
    pub fn new(
        world_rank: i32,
        local_rank: i32,
        hash_id: String,
        ziskemuasm_path: &Path,
        with_hints: bool,
        options: AsmRunnerOptions,
    ) -> Result<(AsmServices, SegmentsClaim)> {
        let pid = std::process::id();

        // The hints mode belongs on both prefixes: `get_precompile_results()` comes
        // from the generated assembly, so the hints binary variant creates a
        // `_precompile` segment the non-hints one neither creates nor opens. The two
        // modes therefore do not have the same *set* of segments and cannot share one.
        // Two sets per worker at most, one per mode, each reused by every program in it.
        let shm_prefix = shm_prefix_for(pid, local_rank, with_hints);
        let sem_prefix = sem_prefix_for(pid, &hash_id, local_rank, with_hints);

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
        // Phase 1: create the shmem segments, once per prefix, not once per program.
        let segments = SharedSegments::acquire(&shm_prefix, || {
            Self::create_shmem(world_rank, &shm_prefix, &sem_prefix, stripped_path, &options)
        })?;

        // Starting services writes the shared guest RAM and ROM, so no job may be
        // using the segments, and the program they serve must have finished with
        // them first.
        let claim = segments.claim();
        Self::quiesce_active(&segments);

        // Phase 2: start services and wait for them to be ready.
        // The services that started before one failed have created their semaphores.
        let stdio_service = StdioService::start_services(
            world_rank,
            local_rank,
            stripped_path,
            &options,
            &shm_prefix,
            &sem_prefix,
        )
        .inspect_err(|_| super::janitor::cleanup_sem_prefix(&sem_prefix))?;

        let inner = AsmServicesInner {
            service: stdio_service,
            sem_prefix,
            stopped: SetOnDrop::default(),
            segments,
        };

        for service in &Self::SERVICES {
            inner
                .service
                .send_status_request(service)
                .with_context(|| format!("Service {service} failed to respond to ping"))?;
        }

        Ok((AsmServices { inner: Arc::new(inner) }, claim))
    }

    /// Clean up all shared memory and semaphores for currently running services.
    /// Scan `/dev/shm` for stale `ZISK_*` shmem segments and `sem.ZISK_*` semaphores
    /// left by dead processes and unlink them.
    pub fn cleanup_stale_shmem() {
        super::janitor::cleanup_stale();
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
            // Roll back whatever the partial creation left behind: every
            // `{shm_prefix}*` segment, per-service and shared alike, and any
            // semaphore the helpers created.
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
            return Err(anyhow::anyhow!("One or more shmem creation commands failed"));
        }
        Ok(())
    }

    /// Make the shared segments serve this program, unless they already do.
    ///
    /// The segments serve one program at a time, so a switch takes three steps,
    /// in order: wait until the program they serve now has finished with them
    /// (`quiesce_active`), let the caller rebind whatever is per program
    /// (`bind`, the semaphores of the parent's shared writers), and rebuild this
    /// program's guest RAM and ROM (`reset_services`). It is recorded as
    /// active only once all three have succeeded.
    ///
    /// The record lives with the segments, not with the caller, so every client
    /// in the process that set up programs on them sees the same one.
    ///
    /// `claim` must be on this program's segments, held for the job that follows: without it,
    /// a second handoff could find no program recorded while the first waits for
    /// the outgoing one, skip that wait, and reset the guest memory under it.
    ///
    /// Fails with [`AsmRunError::ServiceDied`] if one of this program's services
    /// has exited: it would fail the job anyway, later and less clearly.
    pub fn activate(&self, claim: &SegmentsClaim, bind: impl FnOnce() -> Result<()>) -> Result<()> {
        let segments = &self.inner.segments;
        if !Arc::ptr_eq(claim.segments(), segments) {
            return Err(anyhow::anyhow!(
                "activating a program on {} under a claim on {}",
                segments.shm_prefix(),
                claim.segments().shm_prefix()
            ));
        }
        self.inner.service.check_alive()?;
        if segments.is_active(&self.inner) {
            return Ok(());
        }
        Self::quiesce_active(segments);
        bind()?;
        self.reset_services()?;

        segments.set_active(ActiveProgram {
            services: Arc::downgrade(&self.inner),
            stopped: Arc::clone(&self.inner.stopped.0),
        });
        Ok(())
    }

    /// Wait until no setup or job is using this program's shared segments, and
    /// hold them until the returned claim is dropped. See [`SegmentsClaim`].
    pub fn claim(&self) -> SegmentsClaim {
        self.inner.segments.claim()
    }

    /// Wait until the services of the program `segments` serve have finished
    /// every request already sent, and record that they serve none.
    ///
    /// A service keeps writing the shared guest RAM and ROM after it has
    /// answered: its request may still be in flight (the ROM histogram's is
    /// parked until a job reads it), and its own reset runs after the response.
    /// A ping is answered only after both, since each service reads its next
    /// request only when it is done with the last, and a request in flight holds
    /// that service's handle until its response arrives. A request its runner has
    /// not sent yet is not waited for: see `AsmJob` in `zisk-executor`. A
    /// service whose ping fails is taken as stopped. A program already dropped is
    /// not pinged but waited for, until its teardown has stopped its services.
    fn quiesce_active(segments: &SharedSegments) {
        let Some(outgoing) = segments.take_active() else { return };
        // Its last reference is gone, but its teardown, which holds no claim, may
        // still be stopping services that write the segments: wait for it to finish.
        let Some(outgoing) = outgoing.services.upgrade() else {
            outgoing.stopped.wait();
            return;
        };
        for service in &Self::SERVICES {
            if let Err(e) = outgoing.service.send_status_request(service) {
                tracing::warn!(
                    "Service {service} of the outgoing program did not answer before a switch; \
                     taking it as stopped: {e:#}"
                );
            }
        }
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
    /// Runs the three in parallel: each clears 640 MiB (512 MiB of RAM, 128 MiB of
    /// ROM) and rewrites its init data, and they are separate processes with
    /// independent stdio state. On threads of their own, not rayon's: this runs under
    /// the segments' claim, which a rayon worker must not wait on while it could steal
    /// a task that takes the claim too.
    fn reset_services(&self) -> Result<()> {
        std::thread::scope(|scope| {
            let resets: Vec<_> = Self::SERVICES
                .iter()
                .map(|service| scope.spawn(move || self.reset_service(service)))
                .collect();
            resets.into_iter().try_for_each(|reset| {
                reset.join().unwrap_or_else(|_| Err(anyhow::anyhow!("a service's reset panicked")))
            })
        })
        .context("Failed to reset ASM services")
    }

    /// One service's part of [`reset_services`](Self::reset_services).
    fn reset_service(&self, service: &AsmService) -> Result<()> {
        let response = self.inner.service.send_reset_request(service).map_err(|e| {
            // A binary generated before the reset request existed exits on
            // it, and its own message is lost with its stderr. The cache names
            // binaries by ELF hash and hints mode only, so say what fixes it.
            let died = e.chain().any(|cause| {
                matches!(cause.downcast_ref::<AsmRunError>(), Some(AsmRunError::ServiceDied { .. }))
            });
            if died {
                e.context(format!(
                    "Service {service} exited on the reset request. A binary cached \
                         before that request existed exits on it with status 255; if \
                         that is the status reported, clear the cached ASM binaries \
                         (~/.zisk/cache by default) to regenerate them"
                ))
            } else {
                e.context(format!("Service {service} failed to reset"))
            }
        })?;
        if response.result != 0 {
            return Err(anyhow::anyhow!(
                "ASM {service} service returned non-zero result to the reset request: {}",
                response.result
            ));
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

    /// Unlink the semaphores this program owns.
    ///
    /// Only the semaphores: they carry the program hash, so they are this
    /// program's to remove. The shmem segments are shared with every other
    /// program on the same prefix, and unlinking those is `SharedSegments`' job
    /// once the last of them is gone.
    fn cleanup_my_semaphores(&self) {
        super::janitor::cleanup_sem_prefix(&self.sem_prefix);
    }
}

impl Drop for AsmServicesInner {
    /// RAII teardown for this program's services and semaphores. The shared
    /// segments outlive it unless it was their last owner: its `segments`,
    /// dropped after this body, decides.
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

    /// A program's teardown holds no claim, and the activation record stops
    /// upgrading as soon as its last reference goes, before its services are
    /// stopped. A switch that finds it dropped must wait for that teardown rather
    /// than reset the segments under services still writing them.
    #[test]
    fn a_switch_waits_for_a_dropped_program_to_stop_its_services() {
        use super::super::segments::StoppedLatch;
        use std::sync::{mpsc, Weak};
        use std::time::Duration;

        let prefix = format!("ZISK_unittest_teardown_{}", std::process::id());
        let segments = SharedSegments::acquire(&prefix, || Ok(())).unwrap();
        let stopped = Arc::new(StoppedLatch::default());
        // A program whose last reference is gone: its `Weak` no longer upgrades.
        segments.set_active(ActiveProgram { services: Weak::new(), stopped: Arc::clone(&stopped) });

        let (switched, rx) = mpsc::channel();
        let switch = {
            let segments = Arc::clone(&segments);
            std::thread::spawn(move || {
                AsmServices::quiesce_active(&segments);
                switched.send(()).unwrap();
            })
        };
        assert!(
            rx.recv_timeout(Duration::from_millis(200)).is_err(),
            "a switch must wait while the dropped program's services are stopping"
        );

        stopped.set();
        rx.recv_timeout(Duration::from_secs(10)).expect("the teardown's end must let it in");
        switch.join().unwrap();
        assert!(segments.take_active().is_none(), "the switch must leave no program recorded");
    }

    /// A program whose services are stand-ins: shell scripts that answer pings and,
    /// unless `exit_on_reset`, resets, log each request's id, and exit on anything else,
    /// a shutdown included. One that exits on the reset is a binary cached before the
    /// reset request existed. Its directory goes when it is dropped.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    struct FakeProgram {
        dir: std::path::PathBuf,
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    impl FakeProgram {
        /// The path `AsmServices::new` takes.
        fn path(&self) -> std::path::PathBuf {
            self.dir.join("prog-mt.bin")
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    impl Drop for FakeProgram {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn fake_program(name: &str, exit_on_reset: bool) -> FakeProgram {
        use std::io::Write;

        let on_reset = if exit_on_reset { "exit 255" } else { r"answer '\024'" };
        let script = format!(
            r#"#!/bin/sh
for arg in "$@"; do case "$arg" in --just_create*) exit 0;; esac; done
answer() {{ printf "$1"; head -c 39 /dev/zero; }}
while :; do
  id=$(head -c 40 | od -An -tu8 -N8 | tr -d ' ')
  [ -n "$id" ] || exit 0
  echo "$id" >> "$0.log"
  case "$id" in
    1) answer '\002' ;;
    19) {on_reset} ;;
    *) exit 0 ;;
  esac
done
"#
        );
        let dir = std::env::temp_dir()
            .join(format!("zisk_unittest_services_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Written by a child process, so that this one never holds the file open for writing:
        // a child another test forks meanwhile would inherit that descriptor until it
        // execs, and running the script then would fail with ETXTBSY.
        // At the names the services run them by (the memory-ops emulator's has its form in it).
        let trimmed = dir.join("prog");
        for service in AsmServices::SERVICES {
            let path = service.command_path_for(&trimmed.to_string_lossy());
            let mut writer = Command::new("sh")
                .args(["-c", r#"cat > "$0" && chmod 755 "$0""#])
                .arg(&path)
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();
            writer.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
            assert!(writer.wait().unwrap().success(), "write {path}");
        }
        FakeProgram { dir }
    }

    /// The ids of the requests `service` of the program at `program` has received.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn requests(program: &Path, service: AsmService) -> Vec<u64> {
        let trimmed = program.with_file_name("prog");
        let log = format!("{}.log", service.command_path_for(&trimmed.to_string_lossy()));
        std::fs::read_to_string(log)
            .unwrap_or_default()
            .lines()
            .map(|id| id.parse().unwrap())
            .collect()
    }

    /// Starts `program` on `local_rank`'s segments, as its setup does.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn start(program: &Path, local_rank: i32, hash: char) -> (AsmServices, SegmentsClaim) {
        let hash_id = hash.to_string().repeat(64);
        AsmServices::new(0, local_rank, hash_id, program, false, AsmRunnerOptions::new())
            .expect("the stand-in services start")
    }

    const PING: u64 = super::super::CMD_PING_REQUEST_ID;
    const RESET: u64 = super::super::CMD_RESET_REQUEST_ID;

    /// A switch waits for the outgoing program's services, binds the incoming
    /// program's semaphores, and has its services rebuild their guest memory; a
    /// program already active is left alone.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn a_switch_waits_for_the_outgoing_program_and_resets_the_incoming_one() {
        let rank = 101;
        let (a_program, b_program) =
            (fake_program("switch_a", false), fake_program("switch_b", false));
        let (a_path, b_path) = (a_program.path(), b_program.path());
        let binds = std::cell::Cell::new(0);
        let bind = || {
            binds.set(binds.get() + 1);
            Ok(())
        };

        let (a, claim) = start(&a_path, rank, 'a');
        a.activate(&claim, bind).unwrap();
        drop(claim);
        for service in AsmServices::SERVICES {
            assert_eq!(
                requests(&a_path, service),
                [PING, RESET],
                "A's {service} starts, then resets"
            );
        }
        assert_eq!(binds.get(), 1);

        let claim = a.claim();
        a.activate(&claim, bind).unwrap();
        drop(claim);
        assert_eq!(
            requests(&a_path, AsmService::MT),
            [PING, RESET],
            "an active program is left alone"
        );
        assert_eq!(binds.get(), 1);

        // Starting B's services writes the shared segments, so A's are waited for first.
        let (b, claim) = start(&b_path, rank, 'b');
        b.activate(&claim, bind).unwrap();
        drop(claim);
        for service in AsmServices::SERVICES {
            assert_eq!(
                requests(&a_path, service),
                [PING, RESET, PING],
                "A's {service} is waited for"
            );
            assert_eq!(
                requests(&b_path, service),
                [PING, RESET],
                "B's {service} starts, then resets"
            );
        }

        let claim = a.claim();
        a.activate(&claim, bind).unwrap();
        drop(claim);
        for service in AsmServices::SERVICES {
            assert_eq!(
                requests(&b_path, service),
                [PING, RESET, PING],
                "B's {service} is waited for"
            );
            assert_eq!(
                requests(&a_path, service),
                [PING, RESET, PING, RESET],
                "A's {service} resets"
            );
        }
        assert_eq!(binds.get(), 3, "the semaphores are bound on every switch, and only then");
    }

    /// A program whose reset fails, as one whose binary predates the reset request,
    /// fails its activation with `ServiceDied` and says what fixes it; no program is
    /// recorded as active, and a later activation fails at once.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn a_program_that_cannot_reset_is_never_recorded_as_active() {
        let stale = fake_program("stale", true);
        let (program, claim) = start(&stale.path(), 102, 's');

        let error = program.activate(&claim, || Ok(())).unwrap_err();
        assert!(
            error.chain().any(|cause| matches!(
                cause.downcast_ref::<AsmRunError>(),
                Some(AsmRunError::ServiceDied { .. })
            )),
            "the activation fails with ServiceDied: {error:#}"
        );
        assert!(format!("{error:#}").contains("clear the cached ASM binaries"), "{error:#}");
        assert!(program.segments().take_active().is_none(), "no program is recorded as active");

        let again = program.activate(&claim, || Ok(())).unwrap_err();
        assert!(format!("{again:#}").contains("can serve no more requests"), "{again:#}");
    }

    /// A claim holds the segments it was taken on, and no others.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn a_program_is_not_activated_under_another_segments_claim() {
        let (a_program, b_program) =
            (fake_program("claim_a", false), fake_program("claim_b", false));
        let (a, a_claim) = start(&a_program.path(), 103, 'c');
        let (_b, b_claim) = start(&b_program.path(), 104, 'd');

        let error = a.activate(&b_claim, || Ok(())).unwrap_err();
        assert!(format!("{error:#}").contains("under a claim on"), "{error:#}");
        a.activate(&a_claim, || Ok(())).unwrap();
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
        // The memory-ops emulator has a form of its own either way, never the plain `-mo`.
        let mo = AsmService::MO.command_path_for("/x/ziskemuasm");
        assert_eq!(
            mo,
            if crate::mops_light() { "/x/ziskemuasm-mol.bin" } else { "/x/ziskemuasm-moh.bin" }
        );
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
