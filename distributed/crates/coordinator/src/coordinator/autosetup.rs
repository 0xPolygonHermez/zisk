//! Setting up again, when a job needs it, a program the coordinator knows but has not set up
//! since it started.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{broadcast::error::RecvError, OnceCell};
use tracing::{debug, info, warn};
use zisk_cluster_common::JobId;
use zisk_common::{SetupKey, ZiskPaths};

use crate::{
    coordinator_errors::{CoordinatorError, CoordinatorResult},
    job_events::CoordinatorJobEvent,
    Coordinator,
};

/// How long an autosetup waits for workers that are computing or recovering before it gives up.
const AUTOSETUP_BUSY_WAIT: Duration = Duration::from_secs(120);
/// How often it looks again whether they are free.
const AUTOSETUP_BUSY_POLL: Duration = Duration::from_millis(250);
/// How long it waits for the workers to set the program up from their caches. A cache that holds
/// the program's setup but not its emulator binaries makes a worker build those, which takes a
/// while.
const AUTOSETUP_TIMEOUT: Duration = Duration::from_secs(600);

/// One autosetup of a program in a mode, shared by the jobs that wait for it: its outcome, or why
/// it failed.
pub(crate) type Autosetup = Arc<OnceCell<Result<(), String>>>;

impl Coordinator {
    /// Makes sure `hash_id` is set up in the hints mode a job needs before the job is dispatched.
    ///
    /// The coordinator forgets its setups when it restarts, but keeps the ELFs registered with it,
    /// and its workers keep in their caches the setups they made. A job for a registered program
    /// that is not set up sets it up again, from the workers' caches only: a worker sets it up only
    /// if its cache holds the program's setup for the proving key it runs, so the program's
    /// verification key cannot change under its clients. Jobs for the same program share one
    /// autosetup.
    ///
    /// Fails with `ProgramNotFound` when no ELF was registered for `hash_id`, and with
    /// `ProgramNotSetup` when no worker could set it up from its cache (a new proving key, an
    /// emptied cache) or the workers stayed busy: an explicit setup is then needed.
    pub(crate) async fn ensure_set_up(
        self: &Arc<Self>,
        hash_id: &str,
        with_hints: bool,
    ) -> CoordinatorResult<()> {
        if self.is_set_up(hash_id, with_hints).await {
            return Ok(());
        }
        if !ZiskPaths::global().elf_cache(hash_id).exists() {
            return Err(CoordinatorError::ProgramNotFound(hash_id.to_string()));
        }

        let key = SetupKey::new(hash_id, with_hints, false);
        let autosetup = Arc::clone(self.autosetups.lock().await.entry(key.clone()).or_default());
        let outcome = autosetup
            .get_or_init(|| {
                // On its own task, so a client that gives up does not cut it short for the rest.
                let coordinator = Arc::clone(self);
                let key = key.clone();
                async move {
                    tokio::spawn(async move { coordinator.autosetup(&key).await })
                        .await
                        .unwrap_or_else(|e| Err(format!("autosetup task failed: {e}")))
                }
            })
            .await
            .clone();

        // Done with: a later job that finds the program not set up tries again.
        let mut autosetups = self.autosetups.lock().await;
        if autosetups.get(&key).is_some_and(|current| Arc::ptr_eq(current, &autosetup)) {
            autosetups.remove(&key);
        }
        drop(autosetups);

        outcome.map_err(|reason| {
            warn!(
                "[Setup] Could not set {} up again from the workers' caches: {}",
                hash_id, reason
            );
            CoordinatorError::ProgramNotSetup(hash_id.to_string())
        })
    }

    /// Runs one cached-only setup of `key`, waiting first for workers that are computing or
    /// recovering, as an explicit setup cannot overlap them.
    async fn autosetup(&self, key: &SetupKey) -> Result<(), String> {
        info!(
            "[Setup] {} is registered but not set up: setting it up from the workers' caches",
            key.hash_id
        );
        let busy_until = Instant::now() + AUTOSETUP_BUSY_WAIT;
        let job_id = loop {
            match self
                .setup_program_with(
                    &key.hash_id,
                    key.hash_id.clone(),
                    key.with_hints,
                    key.emulator_only,
                    true,
                )
                .await
            {
                Ok(job_id) => break job_id,
                Err(CoordinatorError::InvalidRequest(busy)) if Instant::now() < busy_until => {
                    debug!("[Setup] Autosetup of {} waits: {}", key.hash_id, busy);
                    tokio::time::sleep(AUTOSETUP_BUSY_POLL).await;
                }
                Err(e) => return Err(e.to_string()),
            }
        };
        match tokio::time::timeout(AUTOSETUP_TIMEOUT, self.setup_outcome(&job_id)).await {
            Ok(outcome) => outcome,
            Err(_) => Err(format!("no outcome in {:?}", AUTOSETUP_TIMEOUT)),
        }
    }

    /// How the setup `job_id` ended.
    async fn setup_outcome(&self, job_id: &JobId) -> Result<(), String> {
        let mut events = self
            .subscribe_job_events(job_id)
            .await
            .ok_or_else(|| format!("setup {job_id} has no events"))?;
        loop {
            // Subscribed first, so a terminal event fired before is stashed and one fired after
            // is received.
            let event = match self.get_terminal_event(job_id).await {
                Some(terminal) => terminal,
                None => match events.recv().await {
                    Ok(event) => event,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => self
                        .get_terminal_event(job_id)
                        .await
                        .ok_or_else(|| format!("setup {job_id} ended with no outcome"))?,
                },
            };
            match event {
                CoordinatorJobEvent::Completed(_) => return Ok(()),
                CoordinatorJobEvent::Failed(reason) => return Err(reason),
                CoordinatorJobEvent::Cancelled => return Err("cancelled".into()),
                _ => {}
            }
        }
    }
}
