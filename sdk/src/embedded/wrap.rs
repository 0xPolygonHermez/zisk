use std::time::Duration;

use anyhow::Result;
use zisk_common::{ProofKind, ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};

use super::{spawn_embedded_job, EmbeddedClient};
use crate::job_handle::{JobHandle, SubscriberList};

pub(crate) fn run(
    client: EmbeddedClient,
    proof_with_publics: &ZiskProofWithPublicValues,
    proof_kind: ProofKind,
    override_publics: Option<ZiskPublics>,
    override_program_vk: Option<ZiskProgramVK>,
    timeout: Option<Duration>,
    subs: SubscriberList,
) -> Result<JobHandle<ZiskProofWithPublicValues>> {
    let proof = proof_with_publics.clone();
    spawn_embedded_job(
        move || {
            client.run_wrap(
                &proof,
                proof_kind,
                override_publics.as_ref(),
                override_program_vk.as_ref(),
            )
        },
        timeout,
        subs,
    )
}
