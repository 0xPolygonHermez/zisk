//! Remote dispatch for recurser-aggregator operations.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use zisk_common::Proof;
use zisk_coordinator_api::dto::{
    DomainAggregateRequest, DomainAggregatorSpec, DomainJobKind, DomainSetupAggregatorRequest,
};

use super::RemoteClient;
use crate::aggregator::RecurserAggregator;
use crate::job_handle::{JobHandle, SubscriberList};
use crate::prove::ProveResult;
use crate::setup::SetupResult;
use crate::upload::UploadResult;

impl RemoteClient {
    /// Pushes the aggregator spec to the coordinator; idempotent server-side.
    pub(crate) fn do_upload_aggregator(
        &self,
        agg: &RecurserAggregator,
    ) -> Result<UploadResult> {
        let spec = DomainAggregatorSpec {
            program_vks: agg.program_vks.clone(),
            n_private_inputs: agg.n_private_inputs as u64,
            prepare_publics_body: agg
                .prepare_publics_template
                .clone()
                .unwrap_or_else(|| recurser::templates::DEFAULT_PREPARE_PUBLICS.to_string()),
            check_publics_body: agg
                .check_publics_template
                .clone()
                .unwrap_or_else(|| recurser::templates::DEFAULT_CHECK_PUBLICS.to_string()),
            aggregate_publics_body: agg.aggregate_publics_template.clone(),
        };

        let returned = self
            .gw
            .register_recurser_aggregator(agg.recurser_id.clone(), spec)
            .context("RegisterRecurserAggregator failed")?;

        if returned != agg.recurser_id {
            return Err(anyhow!(
                "coordinator returned recurser_id '{}', expected '{}'",
                returned,
                agg.recurser_id
            ));
        }
        Ok(UploadResult::new(agg.recurser_id.clone()))
    }

    pub(crate) fn do_setup_aggregator(
        &self,
        agg: &RecurserAggregator,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<SetupResult>> {
        let job_kind = DomainJobKind::SetupAggregator(DomainSetupAggregatorRequest {
            recurser_id: agg.recurser_id.clone(),
        });
        let remote_job = self.gw.submit_job(job_kind)?;
        Ok(JobHandle::new_remote(remote_job, subs, timeout, None, None))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn do_aggregate_proof(
        &self,
        agg: &RecurserAggregator,
        proof_a: &Proof,
        proof_b: &Proof,
        private_inputs: &[u64],
        root_c_recurser_agg: Option<[u64; 4]>,
        timeout: Option<Duration>,
        subs: SubscriberList,
    ) -> Result<JobHandle<ProveResult>> {
        // Bincode each proof for the wire.
        let vfp_a = proof_a.get_vadcop_final_proof()?;
        let vfp_b = proof_b.get_vadcop_final_proof()?;
        let bytes_a = bincode::serde::encode_to_vec(&vfp_a, bincode::config::standard())
            .map_err(|e| anyhow!("failed to serialize proof_a: {e}"))?;
        let bytes_b = bincode::serde::encode_to_vec(&vfp_b, bincode::config::standard())
            .map_err(|e| anyhow!("failed to serialize proof_b: {e}"))?;

        // Server-side deadline not on the wire yet; `timeout` is honored client-side via JobHandle.
        let job_kind = DomainJobKind::Aggregate(DomainAggregateRequest {
            recurser_id: agg.recurser_id.clone(),
            proof_a: bytes_a,
            proof_b: bytes_b,
            private_inputs: private_inputs.to_vec(),
            root_c_recurser_agg,
        });
        let remote_job = self.gw.submit_job(job_kind)?;
        Ok(JobHandle::new_remote(remote_job, subs, timeout, None, None))
    }
}
