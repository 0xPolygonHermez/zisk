use super::EmbeddedClient;
use crate::{
    embedded::EmbeddedProver,
    job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList},
    prove::ProveResult,
    JobEvent,
};
use crate::{Result, SdkError};
use std::{sync::Arc, time::Duration};
use zisk_common::{ProgramVK, Proof, ProofBody, ProofKind, PublicValues};
use zisk_prover_backend::ProverEngine;

impl EmbeddedClient {
    pub(crate) fn do_wrap(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
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

    /// Wrap/convert a proof synchronously on the calling thread.
    ///
    /// Unlike [`do_wrap`](Self::do_wrap), this performs no `spawn_blocking`
    /// and returns the result directly, so it requires no async runtime.
    pub(crate) fn do_wrap_sync(
        &self,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<PublicValues>,
        override_program_vk: Option<ProgramVK>,
        subs: SubscriberList,
    ) -> Result<ProveResult> {
        fire_event(&subs, JobEvent::Started);
        let result = Self::do_wrap_inner(
            self.prover.clone(),
            proof,
            proof_kind,
            override_publics.as_ref(),
            override_program_vk.as_ref(),
        );
        fire_result_event(&subs, &result);
        result
    }

    fn do_wrap_inner(
        prover: Arc<EmbeddedProver>,
        proof: &Proof,
        proof_kind: ProofKind,
        override_publics: Option<&PublicValues>,
        override_program_vk: Option<&ProgramVK>,
    ) -> Result<ProveResult> {
        let publics = override_publics.unwrap_or(&proof.publics);
        let program_vk = override_program_vk.unwrap_or(&proof.program_vk);
        let proof_words = match &proof.body {
            ProofBody::Vadcop { proof, .. } => proof.as_slice(),
            ProofBody::Plonk { .. } => {
                return Err(SdkError::InvalidConfig("Cannot wrap a Plonk proof".to_string()));
            }
        };

        match prover.as_ref() {
            EmbeddedProver::Emu(p) => p
                .prover
                .wrap_proof(proof_words, publics, program_vk, proof_kind)
                .map(ProveResult::from)
                .map_err(SdkError::backend),
            EmbeddedProver::Asm(p) => p
                .prover
                .wrap_proof(proof_words, publics, program_vk, proof_kind)
                .map(ProveResult::from)
                .map_err(SdkError::backend),
        }
    }
}
