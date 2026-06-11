use super::RemoteClient;
use crate::{
    job_handle::{new_subscriber_list, JobHandle, SubscriberList},
    setup::SetupResult,
};
use std::path::PathBuf;
use std::time::Duration;
use zisk_coordinator_api::dto::{
    DomainJobKind, DomainJobKindResponse, DomainSetupRequest, TerminalStatus,
};
use zisk_prover_backend::GuestProgram;

use anyhow::Result;
use rom_setup::{get_elf_bin_verkey_file_path_with_hash, get_output_path};

impl RemoteClient {
    pub(crate) fn do_setup(
        &self,
        program: &GuestProgram,
        with_hints: bool,
        emulator_only: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        self.do_setup_by_id(
            &program.program_id.hash_id,
            &program.program_id.name,
            with_hints,
            emulator_only,
            timeout,
            subs,
        )
    }

    /// Submit a setup job for an already-uploaded program identified by `hash_id`.
    ///
    /// The coordinator resolves the ELF bytes from its cache by `hash_id`, so the
    /// program must have been uploaded beforehand. `program_name` is metadata only
    /// (used for logging and ASM artifact filenames on the workers).
    pub(crate) fn do_setup_by_id(
        &self,
        hash_id: &str,
        program_name: &str,
        with_hints: bool,
        emulator_only: bool,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let job_kind = DomainJobKind::Setup(DomainSetupRequest {
            hash_id: hash_id.to_string(),
            program_name: program_name.to_string(),
            with_hints,
            emulator_only,
        });

        let remote_job = self.gw.submit_job(job_kind)?;

        Ok(JobHandle::new_remote(remote_job, subs, timeout, None, None))
    }
}

/// Builder for a remote ROM setup request keyed on an already-uploaded program's
/// `hash_id` (no ELF required).
///
/// Obtain via [`RemoteClient::setup_by_id`]. Unlike [`SetupRequest`](crate::SetupRequest),
/// this skips upload entirely — the coordinator must already hold the program's ELF.
/// Remote-only: the embedded client has no coordinator to resolve a hash against, so it
/// has no analog of this builder.
pub struct SetupByIdRequest<'a> {
    client: &'a RemoteClient,
    hash_id: String,
    program_name: String,
    with_hints: bool,
    emulator_only: bool,
    timeout: Option<Duration>,
    output_dir: Option<PathBuf>,
}

impl<'a> SetupByIdRequest<'a> {
    pub(crate) fn new(client: &'a RemoteClient, hash_id: String) -> Self {
        Self {
            client,
            // No ELF on this path, so we have no real name; the hash doubles as the
            // label. It only affects ASM artifact filenames on the worker (a cache-reuse
            // hint with a safe regenerate-on-miss fallback), so the hash is fine.
            program_name: hash_id.clone(),
            hash_id,
            with_hints: false,
            emulator_only: false,
            timeout: None,
            output_dir: None,
        }
    }

    /// Enable hints during ROM setup. Requires Assembly executor on the client builder.
    #[must_use]
    pub fn with_hints(mut self) -> Self {
        self.with_hints = true;
        self
    }

    /// Generate setup for emulator only (skips ASM service startup).
    #[must_use]
    pub fn emulator_only(mut self) -> Self {
        self.emulator_only = true;
        self
    }

    /// Set a timeout for the setup job.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Set the directory where the verkey file will be stored after setup completes.
    #[must_use]
    pub fn output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = Some(dir);
        self
    }

    /// Submit the setup, returning a [`JobHandle<SetupResult>`].
    pub fn run(self) -> Result<JobHandle<SetupResult>> {
        let subs = new_subscriber_list();
        let mut handle = self.client.do_setup_by_id(
            &self.hash_id,
            &self.program_name,
            self.with_hints,
            self.emulator_only,
            self.timeout,
            subs,
        )?;

        let hash_id = self.hash_id.clone();
        let output_dir = self.output_dir.clone();
        handle.set_pre_process(move |status: &TerminalStatus| {
            if let TerminalStatus::Completed(DomainJobKindResponse::Setup { vk }) = status {
                let output_path = get_output_path(&output_dir)?;
                let path = get_elf_bin_verkey_file_path_with_hash(&hash_id, &output_path)?;
                std::fs::write(&path, vk)?;
            }
            Ok(())
        });

        Ok(handle)
    }
}
