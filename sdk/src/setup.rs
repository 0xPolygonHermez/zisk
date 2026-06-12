use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use rom_setup::{get_elf_bin_verkey_file_path_with_hash, get_output_path, HashMode};
use zisk_coordinator_api::dto::{DomainJobKindResponse, TerminalStatus};
use zisk_prover_backend::GuestProgram;

use crate::job_handle::{new_subscriber_list, JobHandle, JobId};
use crate::{Client, ClientSync};

pub struct SetupResult {
    pub job_id: Option<JobId>,
}

impl SetupResult {
    pub fn job_id(&self) -> Option<&JobId> {
        self.job_id.as_ref()
    }
}

/// Builder for a program ROM setup request.
///
/// Obtain via `client.setup(&program)`.
///
/// - Embedded client: executes ROM setup locally if not already done.
/// - Remote client: registers the program on the coordinator for proving.
pub struct SetupRequest<'a, C> {
    client: &'a C,
    program: &'a GuestProgram,
    with_hints: bool,
    emulator_only: bool,
    timeout: Option<Duration>,
    output_dir: Option<PathBuf>,
}

#[allow(private_bounds)]
impl<'a, C: Client> SetupRequest<'a, C> {
    pub(crate) fn new(client: &'a C, program: &'a GuestProgram) -> Self {
        Self {
            client,
            program,
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
        let mut handle = self.client.run_setup(
            self.program,
            self.with_hints,
            self.emulator_only,
            self.timeout,
            subs,
        )?;

        let hash_id = self.program.program_id.hash_id.to_string();
        let output_dir = self.output_dir.clone();
        handle.set_pre_process(move |status: &TerminalStatus| {
            if let TerminalStatus::Completed(DomainJobKindResponse::Setup { vk, hash_mode }) =
                status
            {
                // The hash mode is dictated by the worker's proving key, not the
                // client; use the authoritative value returned with the setup to
                // name the verkey artifact.
                let hash_mode = hash_mode.parse::<HashMode>()?;
                let output_path = get_output_path(&output_dir)?;
                let path =
                    get_elf_bin_verkey_file_path_with_hash(&hash_id, &output_path, hash_mode)?;
                std::fs::write(&path, vk)?;
            }
            Ok(())
        });

        Ok(handle)
    }
}

#[allow(private_bounds)]
impl<'a, C: ClientSync> SetupRequest<'a, C> {
    /// Run ROM setup synchronously, returning the result directly.
    ///
    /// Unlike [`run`](Self::run), this drives the work on the calling thread and
    /// requires no async runtime — use it when embedding the SDK in a
    /// synchronous program. Available only for clients that implement
    /// [`ClientSync`] (the embedded client).
    pub fn run_sync(self) -> Result<SetupResult> {
        let subs = new_subscriber_list();
        self.client.run_setup_sync(self.program, self.with_hints, self.emulator_only, subs)
    }
}
