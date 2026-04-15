use super::EmbeddedClient;
use crate::{
    embedded::EmbeddedProver,
    job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList},
    JobEvent,
};
use anyhow::Result;
use std::{sync::Arc, time::Duration};
use zisk_common::{ProofKind, ZiskProgramVK, ZiskProofWithPublicValues, ZiskPublics};
use zisk_prover_backend::ProverEngine;

impl EmbeddedClient {
    pub(crate) fn do_wrap(
        &self,
        proof_with_publics: &ZiskProofWithPublicValues,
        proof_kind: ProofKind,
        override_publics: Option<ZiskPublics>,
        override_program_vk: Option<ZiskProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ZiskProofWithPublicValues>> {
        let subs_cloned = Arc::clone(&subs);
        let prover = self.prover.clone();
        let proof_with_publics = proof_with_publics.clone();

        let handle = tokio::task::spawn_blocking(move || {
            fire_event(&subs_cloned, JobEvent::Started);

            let result = Self::do_wrap_inner(
                prover,
                &proof_with_publics,
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
        proof_with_publics: &ZiskProofWithPublicValues,
        proof_kind: ProofKind,
        override_publics: Option<&ZiskPublics>,
        override_program_vk: Option<&ZiskProgramVK>,
    ) -> Result<ZiskProofWithPublicValues> {
        let publics = override_publics.unwrap_or(&proof_with_publics.publics);
        let program_vk = override_program_vk.unwrap_or(&proof_with_publics.program_vk);

        match prover.as_ref() {
            EmbeddedProver::Emu(p) => {
                p.prover.wrap_proof(&proof_with_publics.proof, publics, program_vk, proof_kind)
            }
            EmbeddedProver::Asm(p) => {
                p.prover.wrap_proof(&proof_with_publics.proof, publics, program_vk, proof_kind)
            }
        }
    }
}
