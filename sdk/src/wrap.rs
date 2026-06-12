use std::time::Duration;

use anyhow::Result;
use zisk_common::{ProgramVK, Proof, ProofKind, PublicValues};

use crate::job_handle::{new_subscriber_list, JobHandle};
use crate::prove::ProveResult;
use crate::{Client, ClientSync};

/// Builder for a proof wrapping/conversion request.
///
/// Obtain via `client.wrap_proof(&proof, mode)`.
pub struct WrapRequest<'a, C> {
    client: &'a C,
    proof: &'a Proof,
    proof_kind: ProofKind,
    override_publics: Option<PublicValues>,
    override_program_vk: Option<ProgramVK>,
    timeout: Option<Duration>,
}

#[allow(private_bounds)]
impl<'a, C: Client> WrapRequest<'a, C> {
    pub(crate) fn new(client: &'a C, proof: &'a Proof, proof_kind: ProofKind) -> Self {
        Self {
            client,
            proof,
            proof_kind,
            override_publics: None,
            override_program_vk: None,
            timeout: None,
        }
    }

    /// Override the public inputs used during wrapping.
    #[must_use]
    pub fn with_publics(mut self, publics: PublicValues) -> Self {
        self.override_publics = Some(publics);
        self
    }

    /// Override the program verification key used during wrapping.
    #[must_use]
    pub fn with_program_vk(mut self, program_vk: ProgramVK) -> Self {
        self.override_program_vk = Some(program_vk);
        self
    }

    /// Set a timeout for the wrap job.
    #[must_use]
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Submit the wrap, returning a [`JobHandle<ProveResult>`].
    pub fn run(self) -> Result<JobHandle<ProveResult>> {
        let subs = new_subscriber_list();
        self.client.run_wrap(
            self.proof,
            self.proof_kind,
            self.override_publics,
            self.override_program_vk,
            self.timeout,
            subs,
        )
    }
}

#[allow(private_bounds)]
impl<'a, C: ClientSync> WrapRequest<'a, C> {
    /// Wrap/convert the proof synchronously, returning the result directly.
    ///
    /// Unlike [`run`](Self::run), this drives the work on the calling thread and
    /// requires no async runtime — use it when embedding the SDK in a
    /// synchronous program. Available only for clients that implement
    /// [`ClientSync`] (the embedded client).
    pub fn run_sync(self) -> Result<ProveResult> {
        let subs = new_subscriber_list();
        self.client.run_wrap_sync(
            self.proof,
            self.proof_kind,
            self.override_publics,
            self.override_program_vk,
            subs,
        )
    }
}
