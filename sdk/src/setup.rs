use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use rom_setup::{get_elf_bin_verkey_file_path_with_hash, get_output_path};
use zisk_coordinator_api::dto::{DomainJobKindResponse, TerminalStatus};

use crate::job_handle::{new_subscriber_list, JobHandle, JobId};
use crate::lifecycle::SetupTarget;
use crate::Client;

pub struct SetupResult {
    pub job_id: Option<JobId>,
}

impl SetupResult {
    pub fn job_id(&self) -> Option<&JobId> {
        self.job_id.as_ref()
    }
}

/// Builder for a program or aggregator setup request.
///
/// Obtain via `client.setup(&program)` or `client.setup(&aggregator)`.
/// - Embedded: runs ROM / aggregator setup locally, idempotent.
/// - Remote: dispatches setup work to workers via the coordinator.
pub struct SetupRequest<'a, C> {
    client: &'a C,
    target: SetupTarget<'a>,
    with_hints: bool,
    emulator_only: bool,
    timeout: Option<Duration>,
    output_dir: Option<PathBuf>,
}

#[allow(private_bounds)]
impl<'a, C: Client> SetupRequest<'a, C> {
    pub(crate) fn new(client: &'a C, target: SetupTarget<'a>) -> Self {
        Self {
            client,
            target,
            with_hints: false,
            emulator_only: false,
            timeout: None,
            output_dir: None,
        }
    }

    /// Enable hints during ROM setup. Requires Assembly executor on the client
    /// builder. Only applies to [`SetupTarget::Program`].
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

    /// Set the directory where the verkey file will be stored after setup
    /// completes. Only applies to [`SetupTarget::Program`] — aggregator
    /// artifacts are written to an SDK-managed location.
    #[must_use]
    pub fn output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = Some(dir);
        self
    }

    /// Submit the setup, returning a [`JobHandle<SetupResult>`].
    pub fn run(self) -> Result<JobHandle<SetupResult>> {
        let subs = new_subscriber_list();
        match self.target {
            SetupTarget::Program(program) => {
                let mut handle = self.client.run_setup(
                    program,
                    self.with_hints,
                    self.emulator_only,
                    self.timeout,
                    subs,
                )?;

                let hash_id = program.program_id.hash_id.to_string();
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
            SetupTarget::Aggregator(agg) => {
                let mut handle = self.client.run_setup_aggregator(agg, self.timeout, subs)?;

                // Fill `agg.vk_cache` from the terminal response so a later
                // `agg.vk()` doesn't fall through to a disk read.
                let agg_clone = agg.clone();
                handle.set_pre_process(move |status: &TerminalStatus| {
                    if let TerminalStatus::Completed(DomainJobKindResponse::SetupAggregator {
                        vk,
                    }) = status
                    {
                        if vk.len() != 32 {
                            return Err(anyhow::anyhow!(
                                "coordinator returned a {}-byte recurser verkey; expected 32",
                                vk.len()
                            ));
                        }
                        let mut limbs = [0u64; 4];
                        for i in 0..4 {
                            let chunk: [u8; 8] = vk[i * 8..(i + 1) * 8].try_into().unwrap();
                            limbs[i] = u64::from_le_bytes(chunk);
                        }
                        let _ =
                            agg_clone.vk_cache.set(zisk_common::ProgramVK { vk: limbs.to_vec() });
                    }
                    Ok(())
                });
                Ok(handle)
            }
        }
    }
}
