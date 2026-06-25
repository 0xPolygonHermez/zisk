use super::EmbeddedClient;
use crate::{
    embedded::EmbeddedProver,
    job_handle::{fire_event, fire_result_event, JobHandle, SubscriberList},
    prove::ProveResult,
    JobEvent,
};
use crate::{Result, SdkError};
use std::{sync::Arc, time::Duration};
use zisk_common::{ProgramVK, Proof, ProofBody, ProofKind, PublicValues, PROGRAM_VK_LEN};
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
        let (proof_words, default_publics_full) = match &proof.body {
            ProofBody::Vadcop { proof, publics_full, .. } => (proof.as_slice(), publics_full),
            ProofBody::Plonk { .. } => {
                return Err(SdkError::InvalidConfig("Cannot wrap a Plonk proof".to_string()));
            }
        };

        // Default path uses the proof's full-width publics verbatim. The
        // overrides come in as u32 `PublicValues`, so reconstructing through
        // them truncates publics above 32 bits — safe for standard proofs (the
        // override's intent), lossy for recurser publics. Prefer no override.
        let reconstructed;
        let publics_full: &[u64] = if override_publics.is_some() || override_program_vk.is_some() {
            let vk = override_program_vk
                .map(|v| v.vk.as_slice())
                .unwrap_or(&default_publics_full[..PROGRAM_VK_LEN]);
            let user = override_publics
                .map(|p| p.public_u64())
                .unwrap_or_else(|| default_publics_full[PROGRAM_VK_LEN..].to_vec());
            reconstructed = [vk.to_vec(), user].concat();
            &reconstructed
        } else {
            default_publics_full
        };

        match prover.as_ref() {
            EmbeddedProver::Emu(p) => p
                .prover
                .wrap_proof(proof_words, publics_full, proof_kind)
                .map(ProveResult::from)
                .map_err(SdkError::backend),
            EmbeddedProver::Asm(p) => p
                .prover
                .wrap_proof(proof_words, publics_full, proof_kind)
                .map(ProveResult::from)
                .map_err(SdkError::backend),
        }
    }
}
