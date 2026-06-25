//! Remote dispatch for recurser operations.

use std::time::Duration;

use zisk_common::Proof;
use zisk_coordinator_api::dto::{
    DomainAggregateProofsRequest, DomainAggregationProgramSpec, DomainJobKind,
    DomainNormalizeGroup, DomainSetupAggregationProgramRequest,
};

use super::RemoteClient;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::prove::ProveResult;
use crate::recurser::Recurser;
use crate::setup::SetupResult;
use crate::upload::UploadResult;
use crate::{Result, SdkError};

impl RemoteClient {
    /// Pushes the recurser spec to the coordinator; idempotent server-side.
    pub(crate) fn do_upload_aggregation_program(&self, agg: &Recurser) -> Result<UploadResult> {
        let spec = DomainAggregationProgramSpec {
            program_vks: agg.program_vks.clone(),
            normalize_groups: agg
                .templates
                .normalize_groups
                .iter()
                .map(|g| DomainNormalizeGroup {
                    member_indices: g.member_indices.iter().map(|&i| i as u64).collect(),
                    body: g.body.clone(),
                    n_free_inputs: g.n_free_inputs as u64,
                })
                .collect(),
            aggregate_publics_body: agg.templates.aggregate_publics.clone(),
            aggregate_n_free_inputs: agg.templates.aggregate_n_free_inputs as u64,
        };

        let returned = self
            .gw
            .register_aggregation_program(agg.recurser_id.clone(), spec)
            .map_err(SdkError::backend)?;

        if returned != agg.recurser_id {
            return Err(SdkError::Recurser(format!(
                "coordinator returned recurser_id '{}', expected '{}'",
                returned, agg.recurser_id
            )));
        }
        Ok(UploadResult::new(agg.recurser_id.clone()))
    }

    pub(crate) fn do_setup_aggregation_program(
        &self,
        agg: &Recurser,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let job_kind =
            DomainJobKind::SetupAggregationProgram(DomainSetupAggregationProgramRequest {
                recurser_id: agg.recurser_id.clone(),
            });
        let remote_job = self.gw.submit_job(job_kind).map_err(SdkError::backend)?;
        Ok(JobHandle::new_remote(remote_job, subs, timeout, None, None))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_aggregate_proofs(
        &self,
        agg: &Recurser,
        proof_a: &Proof,
        proof_b: &Proof,
        free_inputs_a: &[u64],
        free_inputs_b: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        // Bincode each proof for the wire.
        let vfp_a = proof_a.get_vadcop_final_proof().map_err(SdkError::backend)?;
        let vfp_b = proof_b.get_vadcop_final_proof().map_err(SdkError::backend)?;
        let bytes_a = bincode::serde::encode_to_vec(&vfp_a, bincode::config::standard())
            .map_err(|e| SdkError::Serialization(format!("proof_a: {e}")))?;
        let bytes_b = bincode::serde::encode_to_vec(&vfp_b, bincode::config::standard())
            .map_err(|e| SdkError::Serialization(format!("proof_b: {e}")))?;

        // Server-side deadline not on the wire yet; `timeout` is honored client-side via JobHandle.
        let job_kind = DomainJobKind::AggregateProofs(DomainAggregateProofsRequest {
            recurser_id: agg.recurser_id.clone(),
            proof_a: bytes_a,
            proof_b: bytes_b,
            free_inputs_a: free_inputs_a.to_vec(),
            free_inputs_b: free_inputs_b.to_vec(),
            root_c_recurser_agg,
        });
        let remote_job = self.gw.submit_job(job_kind).map_err(SdkError::backend)?;
        Ok(JobHandle::new_remote(remote_job, subs, timeout, None, None))
    }
}
