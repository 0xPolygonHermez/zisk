use super::EmbeddedClient;
use crate::{
    embedded::EmbeddedProver,
    job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList},
    JobEvent,
};
use anyhow::Result;
use std::{sync::Arc, time::Duration};
use zisk_common::{ProgramVK, Proof, ProofKind, PublicValues};
use zisk_prover_backend::{ProveOutput, ProverEngine};

impl EmbeddedClient {
    pub(crate) fn do_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveOutput>> {
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();
        let proof = proof.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_wrap_inner(
                prover,
                &proof,
                proof_kind,
                override_publics.as_ref(),
                override_program_vk.as_ref(),
            );

            fire_result_event(&subs_cloned, &result);

            result
        });

        Ok(JobHandle::new_embedded(handle, subs, timeout))
    }

    fn do_wrap_inner(
        prover: Arc<EmbeddedProver>,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<&PublicValues>,
        override_program_vk: Option<&ProgramVK>,
    ) -> Result<ProveOutput> {
        let publics = override_publics.unwrap_or(&proof.publics);
        let program_vk = override_program_vk.unwrap_or(&proof.program_vk);

        match prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                p.prover.wrap_proof(&proof.proof_bytes, publics, program_vk, proof_kind)
            }
            EmbeddedProver::Asm(p) => {
                p.prover.wrap_proof(&proof.proof_bytes, publics, program_vk, proof_kind)
            }
        }
    }
}
